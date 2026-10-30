//! Screen capture primitives for Jiffshot's capture side and editor.
//!
//! Everything here works in physical screen pixels; callers must make the
//! process per-monitor DPI aware (see [`screen::enable_per_monitor_dpi`])
//! before using it.

#![cfg(windows)]

pub mod clipboard;
pub mod config;
pub mod frame;
pub mod gdi;
pub mod geom;
pub mod ipc;
pub mod library;
pub mod media;
pub mod paths;
pub mod ocr;
pub mod screen;
pub mod share;
pub mod stitch;
pub mod targets;

pub use frame::Frame;
pub use geom::Rect;

/// Encodes `s` as a NUL-terminated UTF-16 string for Win32 calls.
pub fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Decodes a UTF-16 buffer up to its first NUL.
pub fn from_wide(buffer: &[u16]) -> String {
    let len = buffer.iter().position(|&c| c == 0).unwrap_or(buffer.len());
    String::from_utf16_lossy(&buffer[..len])
}
