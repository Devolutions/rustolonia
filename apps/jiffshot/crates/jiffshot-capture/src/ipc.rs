//! Single-instance hand-off: the running Jiffshot listens on a per-session
//! named pipe, and a second launch sends it the file to open instead of
//! starting another editor.

use crate::wide;
use std::io;
use std::path::{Path, PathBuf};
use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, ERROR_PIPE_CONNECTED, HANDLE, INVALID_HANDLE_VALUE};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, FlushFileBuffers, ReadFile, WriteFile, FILE_FLAG_FIRST_PIPE_INSTANCE, FILE_GENERIC_WRITE, OPEN_EXISTING,
    PIPE_ACCESS_INBOUND,
};
use windows_sys::Win32::System::Pipes::{
    ConnectNamedPipe, CreateNamedPipeW, DisconnectNamedPipe, GetNamedPipeServerProcessId, PIPE_READMODE_MESSAGE,
    PIPE_REJECT_REMOTE_CLIENTS, PIPE_TYPE_MESSAGE, PIPE_WAIT,
};
use windows_sys::Win32::System::RemoteDesktop::ProcessIdToSessionId;
use windows_sys::Win32::System::Threading::GetCurrentProcessId;
use windows_sys::Win32::UI::WindowsAndMessaging::AllowSetForegroundWindow;

fn pipe_name() -> String {
    let mut session = 0;
    // SAFETY: plain Win32 call with a valid out pointer.
    unsafe { ProcessIdToSessionId(GetCurrentProcessId(), &mut session) };
    format!(r"\\.\pipe\Jiffshot.Editor.{session}")
}

/// Sends `path` to a running editor. Returns false when none is listening,
/// so the caller can start one. The editor is allowed to take the
/// foreground, since a hotkey press made this process the foreground one.
pub fn send_to_editor(path: &Path) -> bool {
    let name = wide(&pipe_name());
    // SAFETY: the handle is checked and closed; the buffer outlives WriteFile.
    unsafe {
        let pipe = CreateFileW(name.as_ptr(), FILE_GENERIC_WRITE, 0, std::ptr::null(), OPEN_EXISTING, 0, std::ptr::null_mut());
        if pipe == INVALID_HANDLE_VALUE {
            return false;
        }
        let mut server = 0;
        if GetNamedPipeServerProcessId(pipe, &mut server) != 0 {
            AllowSetForegroundWindow(server);
        }
        let message = path.to_string_lossy().into_owned().into_bytes();
        let mut written = 0;
        let ok = WriteFile(pipe, message.as_ptr(), message.len() as u32, &mut written, std::ptr::null_mut()) != 0;
        FlushFileBuffers(pipe);
        CloseHandle(pipe);
        ok
    }
}

/// Listens for paths on a background thread and passes each to `handler`.
/// Fails if another editor already owns the pipe.
pub fn serve(handler: impl Fn(PathBuf) + Send + 'static) -> io::Result<()> {
    let name = wide(&pipe_name());
    // SAFETY: NUL-terminated name; the handle is checked. The first-instance
    // flag makes creation fail when another editor is already listening.
    let pipe = unsafe {
        CreateNamedPipeW(
            name.as_ptr(),
            PIPE_ACCESS_INBOUND | FILE_FLAG_FIRST_PIPE_INSTANCE,
            PIPE_TYPE_MESSAGE | PIPE_READMODE_MESSAGE | PIPE_WAIT | PIPE_REJECT_REMOTE_CLIENTS,
            1,
            0,
            64 * 1024,
            0,
            std::ptr::null(),
        )
    };
    if pipe == INVALID_HANDLE_VALUE {
        return Err(io::Error::last_os_error());
    }
    let pipe = pipe as isize;
    std::thread::Builder::new().name("editor-ipc".into()).spawn(move || {
        let pipe = pipe as HANDLE;
        loop {
            // SAFETY: `pipe` is a server handle owned by this thread for the
            // life of the process.
            unsafe {
                let connected = ConnectNamedPipe(pipe, std::ptr::null_mut()) != 0 || GetLastError() == ERROR_PIPE_CONNECTED;
                if connected {
                    let mut buf = vec![0u8; 64 * 1024];
                    let mut read = 0;
                    if ReadFile(pipe, buf.as_mut_ptr(), buf.len() as u32, &mut read, std::ptr::null_mut()) != 0 && read > 0 {
                        buf.truncate(read as usize);
                        handler(PathBuf::from(String::from_utf8_lossy(&buf).into_owned()));
                    }
                }
                DisconnectNamedPipe(pipe);
            }
        }
    })?;
    Ok(())
}
