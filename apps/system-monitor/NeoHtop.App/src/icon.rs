//! Extract small shell icons from Windows executables for process rows.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

fn cache() -> &'static Mutex<HashMap<String, Option<String>>> {
    static CACHE: OnceLock<Mutex<HashMap<String, Option<String>>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Returns a base64 BMP for the process image, or empty if extraction fails.
pub fn png_for_process(pid: u32, exe_path: &str, command: &str) -> String {
    let path = resolve_path(pid, exe_path, command);
    if path.is_empty() {
        return String::new();
    }
    if let Ok(guard) = cache().lock() {
        if let Some(existing) = guard.get(&path) {
            return existing.clone().unwrap_or_default();
        }
    }
    let encoded = extract(&path);
    if let Ok(mut guard) = cache().lock() {
        guard.insert(path, encoded.clone());
    }
    encoded.unwrap_or_default()
}

fn resolve_path(pid: u32, exe_path: &str, command: &str) -> String {
    if looks_like_path(exe_path) {
        return exe_path.to_owned();
    }
    if let Some(from_pid) = image_name_from_pid(pid) {
        return from_pid;
    }
    first_path_token(command).unwrap_or_default()
}

fn looks_like_path(value: &str) -> bool {
    let trimmed = value.trim().trim_matches('"');
    trimmed.len() > 2 && (trimmed.contains('\\') || trimmed.contains('/'))
}

fn first_path_token(command: &str) -> Option<String> {
    let trimmed = command.trim();
    if trimmed.starts_with('"') {
        return trimmed
            .split('"')
            .nth(1)
            .filter(|part| looks_like_path(part))
            .map(str::to_owned);
    }
    trimmed
        .split_whitespace()
        .next()
        .filter(|part| looks_like_path(part))
        .map(str::to_owned)
}

#[cfg(not(windows))]
fn image_name_from_pid(_pid: u32) -> Option<String> {
    None
}

#[cfg(windows)]
fn image_name_from_pid(pid: u32) -> Option<String> {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION,
    };

    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if handle.is_null() {
            return None;
        }
        let mut buffer = [0u16; 32768];
        let mut size = buffer.len() as u32;
        let ok = QueryFullProcessImageNameW(handle, 0, buffer.as_mut_ptr(), &mut size);
        CloseHandle(handle);
        if ok == 0 || size == 0 {
            return None;
        }
        Some(String::from_utf16_lossy(&buffer[..size as usize]))
    }
}

#[cfg(not(windows))]
fn extract(_path: &str) -> Option<String> {
    None
}

#[cfg(windows)]
fn extract(path: &str) -> Option<String> {
    windows_extract(path)
}

#[cfg(windows)]
fn windows_extract(path: &str) -> Option<String> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Graphics::Gdi::{
        CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, SelectObject, BITMAPINFO,
        BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS,
    };
    use windows_sys::Win32::UI::Shell::{SHGetFileInfoW, SHFILEINFOW, SHGFI_ICON, SHGFI_SMALLICON};
    use windows_sys::Win32::UI::WindowsAndMessaging::{DestroyIcon, DrawIconEx, DI_NORMAL};

    const SIZE: i32 = 16;
    let wide: Vec<u16> = std::ffi::OsStr::new(path)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let mut info: SHFILEINFOW = unsafe { std::mem::zeroed() };
    let result = unsafe {
        SHGetFileInfoW(
            wide.as_ptr(),
            0,
            &mut info,
            std::mem::size_of::<SHFILEINFOW>() as u32,
            SHGFI_ICON | SHGFI_SMALLICON,
        )
    };
    let result = if result == 0 || info.hIcon.is_null() {
        unsafe {
            SHGetFileInfoW(
                wide.as_ptr(),
                0x80,
                &mut info,
                std::mem::size_of::<SHFILEINFOW>() as u32,
                SHGFI_ICON | SHGFI_SMALLICON | 0x10,
            )
        }
    } else {
        result
    };
    if result == 0 || info.hIcon.is_null() {
        return None;
    }
    let hicon = info.hIcon;
    let bmp = unsafe {
        let hdc = CreateCompatibleDC(std::ptr::null_mut());
        if hdc.is_null() {
            DestroyIcon(hicon);
            return None;
        }
        let mut header: BITMAPINFO = std::mem::zeroed();
        header.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        header.bmiHeader.biWidth = SIZE;
        header.bmiHeader.biHeight = -SIZE;
        header.bmiHeader.biPlanes = 1;
        header.bmiHeader.biBitCount = 32;
        header.bmiHeader.biCompression = BI_RGB;
        let mut bits: *mut core::ffi::c_void = std::ptr::null_mut();
        let dib = CreateDIBSection(
            hdc,
            &header,
            DIB_RGB_COLORS,
            &mut bits,
            std::ptr::null_mut(),
            0,
        );
        if dib.is_null() || bits.is_null() {
            DeleteDC(hdc);
            DestroyIcon(hicon);
            return None;
        }
        let old = SelectObject(hdc, dib);
        let _ = DrawIconEx(
            hdc,
            0,
            0,
            hicon,
            SIZE,
            SIZE,
            0,
            std::ptr::null_mut(),
            DI_NORMAL,
        );
        let len = (SIZE * SIZE * 4) as usize;
        let pixels = std::slice::from_raw_parts(bits as *const u8, len).to_vec();
        SelectObject(hdc, old);
        DeleteObject(dib);
        DeleteDC(hdc);
        DestroyIcon(hicon);
        if pixels.iter().all(|byte| *byte == 0) {
            return None;
        }
        encode_bmp(SIZE as u32, SIZE as u32, &pixels)
    };
    Some(base64_encode(&bmp))
}

fn encode_bmp(width: u32, height: u32, bgra: &[u8]) -> Vec<u8> {
    let pixel_size = width * height * 4;
    let file_size = 14 + 40 + pixel_size;
    let mut out = Vec::with_capacity(file_size as usize);
    out.extend_from_slice(b"BM");
    out.extend_from_slice(&file_size.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&54u32.to_le_bytes());
    out.extend_from_slice(&40u32.to_le_bytes());
    out.extend_from_slice(&(width as i32).to_le_bytes());
    out.extend_from_slice(&(-(height as i32)).to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&32u16.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&pixel_size.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(bgra);
    out
}

fn base64_encode(bytes: &[u8]) -> String {
    const TABLE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let a = chunk[0] as u32;
        let b = chunk.get(1).copied().unwrap_or(0) as u32;
        let c = chunk.get(2).copied().unwrap_or(0) as u32;
        let triple = (a << 16) | (b << 8) | c;
        out.push(TABLE[((triple >> 18) & 63) as usize] as char);
        out.push(TABLE[((triple >> 12) & 63) as usize] as char);
        if chunk.len() > 1 {
            out.push(TABLE[((triple >> 6) & 63) as usize] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(TABLE[(triple & 63) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
}
