//! Opt-in call tracing for diagnosing hangs: set JIFFSHOT_TRACE to a file
//! path and every traced call appends an enter line and an exit line with
//! the time it took.

use std::io::Write;
use std::sync::{Mutex, OnceLock};

fn file() -> Option<&'static Mutex<std::fs::File>> {
    static FILE: OnceLock<Option<Mutex<std::fs::File>>> = OnceLock::new();
    FILE.get_or_init(|| {
        let path = std::env::var_os("JIFFSHOT_TRACE")?;
        std::fs::File::create(path).ok().map(Mutex::new)
    })
    .as_ref()
}

pub fn line(text: &str) {
    if let Some(f) = file() {
        let mut f = f.lock().unwrap();
        let _ = writeln!(f, "{:?} {text}", std::thread::current().id());
        let _ = f.flush();
    }
}

/// Logs `name` now and again, with the elapsed time, when the returned
/// guard drops.
pub struct Span(&'static str, std::time::Instant);

pub fn span(name: &'static str) -> Option<Span> {
    file()?;
    line(&format!("> {name}"));
    Some(Span(name, std::time::Instant::now()))
}

impl Drop for Span {
    fn drop(&mut self) {
        line(&format!("< {} {} us", self.0, self.1.elapsed().as_micros()));
    }
}
