//! Share destinations: other programs, mail, Office apps, folders, FTP,
//! PDF and the printer. Each takes the flattened image (written to a
//! temporary PNG named after the capture) unless it needs another format.

use crate::winutil;
use jiffshot_capture::{share, wide};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::Path;

/// Destinations offered in the Share panel, before the user's programs.
pub const BUILT_IN: &[&str] = &["Open with…", "Email", "Word", "PowerPoint", "Excel", "OneNote", "Paint", "Folder…", "FTP", "Print"];

/// Opens `file` in a program found on the App Paths list (`winword.exe`…).
pub fn open_in(program: &str, file: &Path) -> Result<(), String> {
    share::open_in(winutil::main_hwnd(), program, file)
}

/// Lets Windows ask which program to open the file with.
pub fn open_with(file: &Path) -> Result<(), String> {
    share::open_with(winutil::main_hwnd(), file)
}

pub fn print(file: &Path) -> Result<(), String> {
    share::print(winutil::main_hwnd(), file)
}

pub use share::to_folder;

/// Opens a new message in the default mail program with `file` attached.
pub fn email(file: &Path, subject: &str) -> Result<(), String> {
    share::email(winutil::main_hwnd(), file, subject)
}

/// The FTP destination: `ftp://user@host[:port]/folder` in the settings;
/// the password comes from Windows Credential Manager (a generic
/// credential named "Jiffshot FTP"), never from a file.
pub fn ftp(url: &str, file: &Path) -> Result<String, String> {
    let rest = url.strip_prefix("ftp://").ok_or("the FTP address must start with ftp://")?;
    let (authority, folder) = rest.split_once('/').unwrap_or((rest, ""));
    let (user, hostport) = authority.rsplit_once('@').unwrap_or(("anonymous", authority));
    let host = if hostport.contains(':') { hostport.to_owned() } else { format!("{hostport}:21") };
    let password = credential("Jiffshot FTP").unwrap_or_else(|| "jiffshot@".to_owned());

    let control = TcpStream::connect(&host).map_err(|e| format!("cannot reach {host}: {e}"))?;
    control.set_read_timeout(Some(std::time::Duration::from_secs(20))).ok();
    let mut reader = BufReader::new(control.try_clone().map_err(|e| e.to_string())?);
    let mut writer = control;
    let mut reply = || -> Result<(u32, String), String> {
        let mut line = String::new();
        loop {
            line.clear();
            reader.read_line(&mut line).map_err(|e| e.to_string())?;
            // Multi-line replies end with "NNN " (space after the code).
            if line.len() >= 4 && line.as_bytes()[3] == b' ' {
                let code = line[..3].parse().unwrap_or(0);
                return Ok((code, line.trim().to_owned()));
            }
        }
    };
    let mut send = |cmd: &str| writer.write_all(format!("{cmd}\r\n").as_bytes()).map_err(|e| e.to_string());
    let expect = |(code, text): (u32, String), ok: &[u32]| if ok.contains(&code) { Ok(text) } else { Err(format!("FTP server: {text}")) };
    expect(reply()?, &[220])?;
    send(&format!("USER {user}"))?;
    let (code, text) = reply()?;
    if code == 331 {
        send(&format!("PASS {password}"))?;
        expect(reply()?, &[230, 202])?;
    } else {
        expect((code, text), &[230])?;
    }
    send("TYPE I")?;
    expect(reply()?, &[200])?;
    if !folder.is_empty() {
        send(&format!("CWD /{folder}"))?;
        expect(reply()?, &[250])?;
    }
    send("PASV")?;
    let pasv = expect(reply()?, &[227])?;
    let nums: Vec<u16> = pasv
        .split(|c| c == '(' || c == ')')
        .nth(1)
        .ok_or("bad PASV reply")?
        .split(',')
        .filter_map(|n| n.trim().parse().ok())
        .collect();
    if nums.len() != 6 {
        return Err("bad PASV reply".into());
    }
    let data_addr = format!("{}.{}.{}.{}:{}", nums[0], nums[1], nums[2], nums[3], nums[4] * 256 + nums[5]);
    let mut data = TcpStream::connect(&data_addr).map_err(|e| e.to_string())?;
    let name = file.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "capture.png".into());
    send(&format!("STOR {name}"))?;
    expect(reply()?, &[125, 150])?;
    let mut bytes = Vec::new();
    std::fs::File::open(file).and_then(|mut f| f.read_to_end(&mut bytes)).map_err(|e| e.to_string())?;
    data.write_all(&bytes).map_err(|e| e.to_string())?;
    drop(data);
    expect(reply()?, &[226, 250])?;
    send("QUIT")?;
    Ok(format!("{}/{name}", url.trim_end_matches('/')))
}

/// A generic credential's password from Windows Credential Manager.
fn credential(target: &str) -> Option<String> {
    use windows_sys::Win32::Security::Credentials::{CredFree, CredReadW, CREDENTIALW, CRED_TYPE_GENERIC};
    let name = wide(target);
    let mut cred: *mut CREDENTIALW = std::ptr::null_mut();
    // SAFETY: CredReadW allocates the credential, freed with CredFree.
    unsafe {
        if CredReadW(name.as_ptr(), CRED_TYPE_GENERIC, 0, &mut cred) == 0 || cred.is_null() {
            return None;
        }
        let c = &*cred;
        let blob = std::slice::from_raw_parts(c.CredentialBlob, c.CredentialBlobSize as usize);
        let units: Vec<u16> = blob.chunks_exact(2).map(|b| u16::from_le_bytes([b[0], b[1]])).collect();
        let password = String::from_utf16_lossy(&units);
        CredFree(cred as *const _);
        Some(password)
    }
}
