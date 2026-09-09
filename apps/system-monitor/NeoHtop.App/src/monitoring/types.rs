use std::fmt::Debug;
use sysinfo::{DiskUsage, ProcessStatus};

/// Internal representation of process data collected from the system.
#[derive(Clone, Debug)]
pub(crate) struct ProcessData {
    pub pid: u32,
    pub identity: u64,
    pub name: String,
    pub cmd: Vec<String>,
    pub user_id: Option<String>,
    pub cpu_usage: f32,
    pub memory: u64,
    pub status: ProcessStatus,
    pub ppid: Option<u32>,
    pub environ: Vec<String>,
    pub root: String,
    pub virtual_memory: u64,
    pub start_time: u64,
    pub run_time: u64,
    pub disk_usage: DiskUsage,
    pub session_id: Option<u32>,
    pub exe_path: String,
}

/// Static information about a process that doesn't change frequently.
#[derive(Clone, Debug)]
pub struct ProcessStaticInfo {
    pub name: String,
    pub command: String,
    pub user: String,
}

/// Process information for the Avalonia view model.
#[derive(Clone, Debug)]
pub struct ProcessInfo {
    pub pid: u32,
    pub identity: u64,
    pub ppid: u32,
    pub name: String,
    pub cpu_usage: f32,
    pub memory_usage: u64,
    pub status: String,
    pub user: String,
    pub command: String,
    #[allow(dead_code)]
    pub threads: Option<u32>,
    pub environ: Vec<String>,
    pub root: String,
    pub virtual_memory: u64,
    pub start_time: u64,
    pub run_time: u64,
    pub disk_usage: (u64, u64),
    pub session_id: Option<u32>,
    pub exe_path: String,
}

/// System-wide statistics for the Avalonia view model.
#[derive(Clone, Debug, Default)]
pub struct SystemStats {
    pub cpu_usage: Vec<f32>,
    pub memory_total: u64,
    pub memory_used: u64,
    pub memory_free: u64,
    pub memory_cached: u64,
    pub uptime: u64,
    pub load_avg: [f64; 3],
    pub network_rx_bytes: u64,
    pub network_tx_bytes: u64,
    pub disk_total_bytes: u64,
    pub disk_used_bytes: u64,
    pub disk_free_bytes: u64,
}
