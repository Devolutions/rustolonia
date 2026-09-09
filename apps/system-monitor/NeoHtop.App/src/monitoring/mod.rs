//! System monitoring functionality copied from the Tauri backend.

mod process_monitor;
mod system_monitor;
mod types;

pub use process_monitor::{PendingKill, ProcessMonitor};
pub use system_monitor::SystemMonitor;
pub use types::*;
