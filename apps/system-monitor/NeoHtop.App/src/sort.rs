use crate::monitoring::ProcessInfo;
use std::cmp::Ordering;
use std::collections::HashSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SortColumn {
    Name,
    Pid,
    Status,
    User,
    CpuUsage,
    MemoryUsage,
    VirtualMemory,
    DiskIo,
    RunTime,
    Command,
    Ppid,
    Root,
    SessionId,
    StartTime,
}

impl SortColumn {
    pub fn parse(name: &str) -> Option<Self> {
        match name.split(':').next().unwrap_or(name) {
            "Name" => Some(Self::Name),
            "Pid" => Some(Self::Pid),
            "Status" => Some(Self::Status),
            "User" => Some(Self::User),
            "CpuUsage" => Some(Self::CpuUsage),
            "MemoryUsage" => Some(Self::MemoryUsage),
            "VirtualMemory" => Some(Self::VirtualMemory),
            "DiskIo" => Some(Self::DiskIo),
            "RunTime" => Some(Self::RunTime),
            "Command" => Some(Self::Command),
            "Ppid" => Some(Self::Ppid),
            "Root" => Some(Self::Root),
            "SessionId" => Some(Self::SessionId),
            "StartTime" => Some(Self::StartTime),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Name => "Name",
            Self::Pid => "Pid",
            Self::Status => "Status",
            Self::User => "User",
            Self::CpuUsage => "CpuUsage",
            Self::MemoryUsage => "MemoryUsage",
            Self::VirtualMemory => "VirtualMemory",
            Self::DiskIo => "DiskIo",
            Self::RunTime => "RunTime",
            Self::Command => "Command",
            Self::Ppid => "Ppid",
            Self::Root => "Root",
            Self::SessionId => "SessionId",
            Self::StartTime => "StartTime",
        }
    }
}

pub fn sort_label(column: SortColumn, descending: bool) -> String {
    format!(
        "{} {}",
        column.as_str(),
        if descending {
            "Descending"
        } else {
            "Ascending"
        }
    )
}

pub fn process_key(process: &ProcessInfo) -> String {
    format!("{}:{}", process.pid, process.identity)
}

pub fn sort_visible_indices(
    processes: &[ProcessInfo],
    indices: &mut [usize],
    column: SortColumn,
    descending: bool,
    pinned: &HashSet<String>,
) {
    indices.sort_by(|left, right| {
        let left = &processes[*left];
        let right = &processes[*right];
        let left_pin = pinned.contains(&left.command);
        let right_pin = pinned.contains(&right.command);
        if left_pin != right_pin {
            return if left_pin {
                Ordering::Less
            } else {
                Ordering::Greater
            };
        }
        let comparison = match column {
            SortColumn::Name => left.name.cmp(&right.name),
            SortColumn::Pid => left.pid.cmp(&right.pid),
            SortColumn::Status => left.status.cmp(&right.status),
            SortColumn::User => left.user.cmp(&right.user),
            SortColumn::CpuUsage => left
                .cpu_usage
                .partial_cmp(&right.cpu_usage)
                .unwrap_or(Ordering::Equal),
            SortColumn::MemoryUsage => left.memory_usage.cmp(&right.memory_usage),
            SortColumn::VirtualMemory => left.virtual_memory.cmp(&right.virtual_memory),
            SortColumn::DiskIo => {
                let left_total = left.disk_usage.0.saturating_add(left.disk_usage.1);
                let right_total = right.disk_usage.0.saturating_add(right.disk_usage.1);
                left_total.cmp(&right_total)
            }
            SortColumn::RunTime => left.run_time.cmp(&right.run_time),
            SortColumn::Command => left.command.cmp(&right.command),
            SortColumn::Ppid => left.ppid.cmp(&right.ppid),
            SortColumn::Root => left.root.cmp(&right.root),
            SortColumn::SessionId => left.session_id.cmp(&right.session_id),
            SortColumn::StartTime => left.start_time.cmp(&right.start_time),
        };
        if descending {
            comparison.reverse()
        } else {
            comparison
        }
        .then_with(|| left.pid.cmp(&right.pid))
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::monitoring::ProcessInfo;

    fn process(name: &str, pid: u32, cpu: f32, command: &str) -> ProcessInfo {
        ProcessInfo {
            pid,
            identity: pid as u64,
            ppid: 1,
            name: name.to_owned(),
            cpu_usage: cpu,
            memory_usage: 1,
            status: "Running".to_owned(),
            user: "me".to_owned(),
            command: command.to_owned(),
            threads: None,
            environ: Vec::new(),
            root: String::new(),
            virtual_memory: 0,
            start_time: 0,
            run_time: 0,
            disk_usage: (0, 0),
            session_id: None,
            exe_path: String::new(),
        }
    }

    #[test]
    fn pinned_processes_sort_first() {
        let processes = vec![process("a", 1, 90.0, "a"), process("b", 2, 10.0, "b")];
        let mut indices = vec![0, 1];
        let mut pinned = HashSet::new();
        pinned.insert("b".to_owned());
        sort_visible_indices(
            &processes,
            &mut indices,
            SortColumn::CpuUsage,
            true,
            &pinned,
        );
        assert_eq!(indices, vec![1, 0]);
    }

    #[test]
    fn process_identity_distinguishes_reused_pids() {
        let first = process("first", 42, 1.0, "first");
        let mut replacement = process("replacement", 42, 1.0, "replacement");
        replacement.identity += 1;

        assert_ne!(process_key(&first), process_key(&replacement));
    }

    #[test]
    fn root_column_is_sortable() {
        let mut processes = vec![process("a", 1, 1.0, "a"), process("b", 2, 1.0, "b")];
        processes[0].root = "Z:\\".to_owned();
        processes[1].root = "C:\\".to_owned();
        let mut indices = vec![0, 1];

        sort_visible_indices(
            &processes,
            &mut indices,
            SortColumn::parse("Root").unwrap(),
            false,
            &HashSet::new(),
        );

        assert_eq!(indices, vec![1, 0]);
    }
}
