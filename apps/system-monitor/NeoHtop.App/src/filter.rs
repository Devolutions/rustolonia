use crate::monitoring::ProcessInfo;
use regex::Regex;
use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Clone, Debug, PartialEq)]
pub struct Filters {
    pub cpu_enabled: bool,
    pub cpu_operator: String,
    pub cpu_value: f32,
    pub ram_enabled: bool,
    pub ram_operator: String,
    pub ram_value: f64,
    pub runtime_enabled: bool,
    pub runtime_operator: String,
    pub runtime_value: f64,
    pub status: String,
}

impl Default for Filters {
    fn default() -> Self {
        Self {
            cpu_enabled: false,
            cpu_operator: ">".to_owned(),
            cpu_value: 50.0,
            ram_enabled: false,
            ram_operator: ">".to_owned(),
            ram_value: 100.0,
            runtime_enabled: false,
            runtime_operator: ">".to_owned(),
            runtime_value: 60.0,
            status: String::new(),
        }
    }
}

fn compare_value(value: f64, operator: &str, target: f64) -> bool {
    match operator {
        ">" => value > target,
        "<" => value < target,
        "=" | "==" => (value - target).abs() < f64::EPSILON,
        ">=" => value >= target,
        "<=" => value <= target,
        _ => true,
    }
}

fn regex_for(term: &str, cache: &Mutex<HashMap<String, Option<Regex>>>) -> Option<Regex> {
    let mut cache = cache.lock().expect("regex cache lock poisoned");
    if let Some(existing) = cache.get(term) {
        return existing.clone();
    }
    let compiled = Regex::new(&format!("(?i){term}")).ok();
    cache.insert(term.to_owned(), compiled.clone());
    compiled
}

/// Matches the Svelte `filterProcesses` rules: comma-separated terms, name /
/// command / PID substring, then case-insensitive regex on the name.
pub fn filter_processes(
    processes: &[ProcessInfo],
    search_term: &str,
    filters: &Filters,
    regex_cache: &Mutex<HashMap<String, Option<Regex>>>,
) -> Vec<usize> {
    let terms: Vec<&str> = if search_term.is_empty() {
        Vec::new()
    } else {
        search_term
            .split(',')
            .map(str::trim)
            .filter(|term| !term.is_empty())
            .collect()
    };
    let statuses: Vec<String> = filters
        .status
        .split(',')
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| !value.is_empty())
        .collect();

    processes
        .iter()
        .enumerate()
        .filter(|(_, process)| {
            if !statuses.is_empty()
                && !statuses
                    .iter()
                    .any(|status| process.status.eq_ignore_ascii_case(status))
            {
                return false;
            }
            if filters.cpu_enabled
                && !compare_value(
                    f64::from(process.cpu_usage),
                    &filters.cpu_operator,
                    f64::from(filters.cpu_value),
                )
            {
                return false;
            }
            if filters.ram_enabled {
                let ram_mb = process.memory_usage as f64 / (1024.0 * 1024.0);
                if !compare_value(ram_mb, &filters.ram_operator, filters.ram_value) {
                    return false;
                }
            }
            if filters.runtime_enabled {
                let runtime_min = process.run_time as f64 / 60.0;
                if !compare_value(
                    runtime_min,
                    &filters.runtime_operator,
                    filters.runtime_value,
                ) {
                    return false;
                }
            }
            if terms.is_empty() {
                return true;
            }
            let name = process.name.to_ascii_lowercase();
            let command = process.command.to_ascii_lowercase();
            let pid = process.pid.to_string();
            terms.iter().any(|term| {
                let lower = term.to_ascii_lowercase();
                name.contains(&lower)
                    || command.contains(&lower)
                    || pid.contains(term)
                    || regex_for(term, regex_cache)
                        .map(|regex| regex.is_match(&process.name))
                        .unwrap_or(false)
            })
        })
        .map(|(index, _)| index)
        .collect()
}

pub fn toggle_operator(current: &str) -> &'static str {
    if current == "<" {
        ">"
    } else {
        "<"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::monitoring::ProcessInfo;

    fn process(name: &str, pid: u32, command: &str) -> ProcessInfo {
        ProcessInfo {
            pid,
            identity: pid as u64,
            ppid: 1,
            name: name.to_owned(),
            cpu_usage: 10.0,
            memory_usage: 50 * 1024 * 1024,
            status: "Running".to_owned(),
            user: "me".to_owned(),
            command: command.to_owned(),
            threads: None,
            environ: Vec::new(),
            root: String::new(),
            virtual_memory: 0,
            start_time: 0,
            run_time: 120,
            disk_usage: (0, 0),
            session_id: None,
            exe_path: String::new(),
        }
    }

    #[test]
    fn comma_terms_match_any() {
        let processes = vec![
            process("nginx", 100, "/usr/sbin/nginx"),
            process("python", 101, "python app.py"),
        ];
        let cache = Mutex::new(HashMap::new());
        let visible = filter_processes(&processes, "nginx, python", &Filters::default(), &cache);
        assert_eq!(visible, vec![0, 1]);
    }

    #[test]
    fn pid_and_regex_terms() {
        let processes = vec![
            process("sshd", 1234, "/usr/sbin/sshd"),
            process("kernel", 1, ""),
        ];
        let cache = Mutex::new(HashMap::new());
        assert_eq!(
            filter_processes(&processes, "1234", &Filters::default(), &cache),
            vec![0]
        );
        assert_eq!(
            filter_processes(&processes, "d$", &Filters::default(), &cache),
            vec![0]
        );
        assert_eq!(
            filter_processes(&processes, "^kernel", &Filters::default(), &cache),
            vec![1]
        );
    }

    #[test]
    fn cpu_filter_excludes_low_usage() {
        let mut processes = vec![process("idle", 2, "idle")];
        processes[0].cpu_usage = 1.0;
        let cache = Mutex::new(HashMap::new());
        let mut filters = Filters::default();
        filters.cpu_enabled = true;
        filters.cpu_operator = ">".to_owned();
        filters.cpu_value = 50.0;
        assert!(filter_processes(&processes, "", &filters, &cache).is_empty());
    }
}
