pub fn bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    format!("{value:.1} {}", UNITS[unit])
}

pub fn cpu(percent: f32) -> String {
    format!("{percent:.1}%")
}

pub fn runtime(seconds: u64) -> String {
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    let remaining = seconds % 60;
    format!("{hours}h {minutes}m {remaining}s")
}

pub fn start_time(timestamp: u64) -> String {
    if timestamp == 0 {
        return "-".to_owned();
    }
    format!("{timestamp}")
}

pub fn uptime(seconds: u64) -> String {
    let days = seconds / 86400;
    let hours = (seconds % 86400) / 3600;
    let minutes = (seconds % 3600) / 60;
    format!("{days}d {hours}h {minutes}m")
}

pub fn disk_io(read: u64, write: u64) -> String {
    format!("{}/{} MB", mb(read), mb(write))
}

fn mb(bytes: u64) -> String {
    format!("{:.1}", bytes as f64 / (1024.0 * 1024.0))
}

pub fn percent(part: u64, total: u64) -> f64 {
    if total == 0 {
        0.0
    } else {
        (part as f64 / total as f64) * 100.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_runtime_and_cpu() {
        assert_eq!(cpu(12.34), "12.3%");
        assert_eq!(runtime(3661), "1h 1m 1s");
    }
}
