//! Process monitoring functionality.

use super::{ProcessData, ProcessInfo, ProcessStaticInfo};
use std::collections::{HashMap, HashSet};
use std::ffi::OsString;
use std::fmt::Debug;
use std::time::{SystemTime, UNIX_EPOCH};
use sysinfo::ProcessStatus;

#[cfg(windows)]
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
#[cfg(windows)]
use windows_sys::Win32::{
    Foundation::FILETIME,
    System::Threading::{
        GetProcessTimes, OpenProcess, TerminateProcess, PROCESS_QUERY_LIMITED_INFORMATION,
        PROCESS_TERMINATE,
    },
};

pub struct PendingKill {
    pid: u32,
    #[cfg(windows)]
    handle: OwnedHandle,
    #[cfg(not(windows))]
    identity: u64,
}

impl PendingKill {
    pub fn pid(&self) -> u32 {
        self.pid
    }
}

fn os_string_vec_to_string_vec(v: &[OsString]) -> Vec<String> {
    v.iter()
        .map(|c| c.to_string_lossy().into_owned())
        .collect::<Vec<_>>()
}

/// Monitors and manages system processes.
#[derive(Debug)]
pub struct ProcessMonitor {
    process_cache: HashMap<(u32, u64), ProcessStaticInfo>,
}

impl ProcessMonitor {
    pub fn new() -> Self {
        Self {
            process_cache: HashMap::new(),
        }
    }

    pub fn collect_processes(&mut self, sys: &sysinfo::System) -> Result<Vec<ProcessInfo>, String> {
        let current_time = Self::get_current_time()?;
        let processes_data = self.collect_process_data(sys, current_time);
        Ok(self.build_process_info(processes_data))
    }

    pub fn prepare_kill(sys: &sysinfo::System, pid: u32, identity: u64) -> Option<PendingKill> {
        #[cfg(windows)]
        {
            let _ = sys;
            let handle = open_process(pid, PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_TERMINATE)?;
            if process_identity_from_handle(&handle)? != identity {
                return None;
            }
            Some(PendingKill { pid, handle })
        }
        #[cfg(not(windows))]
        {
            sys.process(sysinfo::Pid::from(pid as usize))
                .filter(|process| process.start_time() == identity)
                .map(|_| PendingKill { pid, identity })
        }
    }

    pub fn kill_process(sys: &sysinfo::System, pending: PendingKill) -> bool {
        #[cfg(windows)]
        {
            let _ = sys;
            unsafe { TerminateProcess(pending.handle.as_raw_handle() as _, 1) != 0 }
        }
        #[cfg(not(windows))]
        {
            sys.process(sysinfo::Pid::from(pending.pid as usize))
                .filter(|process| process.start_time() == pending.identity)
                .map(|process| process.kill())
                .unwrap_or(false)
        }
    }

    fn process_identity(pid: u32, start_time: u64) -> u64 {
        #[cfg(windows)]
        {
            let _ = start_time;
            open_process(pid, PROCESS_QUERY_LIMITED_INFORMATION)
                .and_then(|handle| process_identity_from_handle(&handle))
                .unwrap_or(0)
        }
        #[cfg(not(windows))]
        {
            let _ = pid;
            start_time
        }
    }

    fn get_current_time() -> Result<u64, String> {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .map_err(|e| format!("Failed to get system time: {e}"))
    }

    fn collect_process_data(&self, sys: &sysinfo::System, current_time: u64) -> Vec<ProcessData> {
        sys.processes()
            .iter()
            .map(|(pid, process)| {
                let pid = pid.as_u32();
                let start_time = process.start_time();
                ProcessData {
                    pid,
                    identity: Self::process_identity(pid, start_time),
                    name: process.name().to_string_lossy().into_owned(),
                    cmd: os_string_vec_to_string_vec(process.cmd()),
                    user_id: process.user_id().map(|uid| uid.to_string()),
                    cpu_usage: process.cpu_usage(),
                    memory: process.memory(),
                    status: process.status(),
                    ppid: process.parent().map(|p| p.as_u32()),
                    environ: os_string_vec_to_string_vec(process.environ()),
                    root: process
                        .root()
                        .map(|p| p.to_string_lossy().into_owned())
                        .unwrap_or_default(),
                    virtual_memory: process.virtual_memory(),
                    start_time,
                    run_time: if start_time > 0 {
                        current_time.saturating_sub(start_time)
                    } else {
                        0
                    },
                    disk_usage: process.disk_usage(),
                    session_id: process.session_id().map(|id| id.as_u32()),
                    exe_path: process
                        .exe()
                        .map(|p| p.to_string_lossy().into_owned())
                        .unwrap_or_default(),
                }
            })
            .collect()
    }

    fn build_process_info(&mut self, processes: Vec<ProcessData>) -> Vec<ProcessInfo> {
        let current_identities = processes
            .iter()
            .map(|data| (data.pid, data.identity))
            .collect::<HashSet<_>>();
        self.process_cache
            .retain(|identity, _| current_identities.contains(identity));

        processes
            .into_iter()
            .map(|data| {
                let cached_info = self
                    .process_cache
                    .entry((data.pid, data.identity))
                    .or_insert_with(|| ProcessStaticInfo {
                        name: data.name.clone(),
                        command: data.cmd.join(" "),
                        user: data.user_id.unwrap_or_else(|| "-".to_string()),
                    });

                ProcessInfo {
                    pid: data.pid,
                    identity: data.identity,
                    ppid: data.ppid.unwrap_or(0),
                    name: cached_info.name.clone(),
                    cpu_usage: data.cpu_usage,
                    memory_usage: data.memory,
                    status: Self::format_status(data.status),
                    user: cached_info.user.clone(),
                    command: cached_info.command.clone(),
                    threads: None,
                    environ: data.environ,
                    root: data.root,
                    virtual_memory: data.virtual_memory,
                    start_time: data.start_time,
                    run_time: data.run_time,
                    disk_usage: (data.disk_usage.read_bytes, data.disk_usage.written_bytes),
                    session_id: data.session_id,
                    exe_path: data.exe_path,
                }
            })
            .collect()
    }

    pub fn format_status(status: ProcessStatus) -> String {
        match status {
            ProcessStatus::Run => "Running",
            ProcessStatus::Sleep => "Sleeping",
            ProcessStatus::Idle => "Idle",
            ProcessStatus::Stop => "Stopped",
            ProcessStatus::Zombie => "Zombie",
            _ => "Unknown",
        }
        .to_string()
    }
}

#[cfg(windows)]
fn open_process(pid: u32, access: u32) -> Option<OwnedHandle> {
    let handle = unsafe { OpenProcess(access, 0, pid) };
    if handle.is_null() {
        None
    } else {
        Some(unsafe { OwnedHandle::from_raw_handle(handle as _) })
    }
}

#[cfg(windows)]
fn process_identity_from_handle(handle: &OwnedHandle) -> Option<u64> {
    let mut creation = FILETIME {
        dwLowDateTime: 0,
        dwHighDateTime: 0,
    };
    let mut exit = creation;
    let mut kernel = creation;
    let mut user = creation;
    let succeeded = unsafe {
        GetProcessTimes(
            handle.as_raw_handle() as _,
            &mut creation,
            &mut exit,
            &mut kernel,
            &mut user,
        )
    };
    (succeeded != 0)
        .then_some(((creation.dwHighDateTime as u64) << 32) | creation.dwLowDateTime as u64)
}

impl Default for ProcessMonitor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sysinfo::System;

    #[test]
    fn test_process_monitor_creation() {
        let monitor = ProcessMonitor::new();
        assert!(monitor.process_cache.is_empty());
    }

    #[test]
    fn test_process_collection() {
        let mut monitor = ProcessMonitor::new();
        let mut sys = System::new();
        sys.refresh_all();
        assert!(monitor.collect_processes(&sys).is_ok());
    }

    #[test]
    fn pending_kill_rejects_a_changed_process_identity() {
        let mut sys = System::new();
        sys.refresh_all();
        let pid = std::process::id();
        let process = sys
            .process(sysinfo::Pid::from(pid as usize))
            .expect("test process must be present");
        let identity = ProcessMonitor::process_identity(pid, process.start_time());

        assert_ne!(identity, 0);
        assert!(ProcessMonitor::prepare_kill(&sys, pid, identity).is_some());
        assert!(ProcessMonitor::prepare_kill(&sys, pid, identity.wrapping_add(1)).is_none());
    }
}
