// Included by the app's build script: turns `assets/jiffshot.ico` into a
// compiled resource file holding the program icon, and hands it to the
// linker. MSVC's linker takes `.res` files directly, so no resource compiler
// (or Windows SDK) is needed.

use std::path::Path;

/// Writes `ico` as a `.res` file: one RT_ICON per image, then the
/// RT_GROUP_ICON (id 1) that lists them, which is the icon Windows shows
/// for the executable.
fn write_icon_res(ico: &[u8], out: &Path) -> std::io::Result<()> {
    let u16_at = |i: usize| u16::from_le_bytes([ico[i], ico[i + 1]]);
    let u32_at = |i: usize| u32::from_le_bytes([ico[i], ico[i + 1], ico[i + 2], ico[i + 3]]);
    let bad = || std::io::Error::new(std::io::ErrorKind::InvalidData, "not an .ico file");
    if ico.len() < 6 || u16_at(0) != 0 || u16_at(2) != 1 {
        return Err(bad());
    }
    let count = u16_at(4) as usize;
    let mut res = Vec::new();
    // A resource file starts with an empty entry.
    entry(&mut res, 0, 0, 0, &[]);
    let mut group = Vec::new();
    group.extend_from_slice(&[0, 0, 1, 0]);
    group.extend_from_slice(&(count as u16).to_le_bytes());
    for i in 0..count {
        let e = 6 + 16 * i;
        if e + 16 > ico.len() {
            return Err(bad());
        }
        let (size, offset) = (u32_at(e + 8) as usize, u32_at(e + 12) as usize);
        let image = ico.get(offset..offset + size).ok_or_else(bad)?;
        let id = (i + 1) as u16;
        // RT_ICON, moveable and discardable.
        entry(&mut res, 3, id, 0x1010, image);
        // The group entry is the file's directory entry with the id in
        // place of the offset.
        group.extend_from_slice(&ico[e..e + 12]);
        group.extend_from_slice(&id.to_le_bytes());
    }
    // RT_GROUP_ICON.
    entry(&mut res, 14, 1, 0x1030, &group);
    std::fs::write(out, res)
}

/// One resource with a numeric type and name, in US English, padded to a
/// 4-byte boundary.
fn entry(res: &mut Vec<u8>, kind: u16, name: u16, flags: u16, data: &[u8]) {
    res.extend_from_slice(&(data.len() as u32).to_le_bytes());
    res.extend_from_slice(&32u32.to_le_bytes());
    for v in [0xFFFF, kind, 0xFFFF, name] {
        res.extend_from_slice(&u16::to_le_bytes(v));
    }
    res.extend_from_slice(&0u32.to_le_bytes());
    res.extend_from_slice(&flags.to_le_bytes());
    res.extend_from_slice(&(if kind == 0 { 0u16 } else { 0x0409 }).to_le_bytes());
    res.extend_from_slice(&0u32.to_le_bytes());
    res.extend_from_slice(&0u32.to_le_bytes());
    res.extend_from_slice(data);
    while res.len() % 4 != 0 {
        res.push(0);
    }
}

/// Embeds `ico` (a path relative to the crate) as the executable's icon.
fn embed_icon(ico: &str) {
    println!("cargo:rerun-if-changed={ico}");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") || std::env::var("CARGO_CFG_TARGET_ENV").as_deref() != Ok("msvc") {
        return;
    }
    let bytes = std::fs::read(ico).unwrap_or_else(|e| panic!("reading {ico}: {e}"));
    let out = Path::new(&std::env::var("OUT_DIR").expect("cargo sets OUT_DIR")).join("icon.res");
    write_icon_res(&bytes, &out).unwrap_or_else(|e| panic!("writing {}: {e}", out.display()));
    println!("cargo:rustc-link-arg-bins={}", out.display());
}
