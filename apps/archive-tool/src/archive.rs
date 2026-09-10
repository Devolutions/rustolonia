//! Archive listing and extraction. Rust owns archive IO; the UI only presents it.

use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use std::collections::{BTreeSet, HashSet};
use std::fs::{self, File};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tar::{Builder, EntryType, Header};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArchiveKind {
    Zip,
    Tar,
    TarGz,
    TarBz2,
    TarXz,
    TarZstd,
    TarLz4,
    Gzip,
    Bzip2,
    Xz,
    Zstd,
    Lz4,
    SevenZ,
    Cab,
}

impl ArchiveKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Zip => "ZIP",
            Self::Tar => "TAR",
            Self::TarGz => "TAR.GZ",
            Self::TarBz2 => "TAR.BZ2",
            Self::TarXz => "TAR.XZ",
            Self::TarZstd => "TAR.ZST",
            Self::TarLz4 => "TAR.LZ4",
            Self::Gzip => "GZIP",
            Self::Bzip2 => "BZIP2",
            Self::Xz => "XZ",
            Self::Zstd => "ZSTD",
            Self::Lz4 => "LZ4",
            Self::SevenZ => "7Z",
            Self::Cab => "CAB",
        }
    }

    pub fn can_create(self) -> bool {
        matches!(self, Self::Zip | Self::Tar | Self::TarGz | Self::SevenZ)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArchiveEntry {
    pub path: String,
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
    pub compressed_size: Option<u64>,
    pub crc: Option<u32>,
    pub modified: String,
}

impl ArchiveEntry {
    pub fn kind_label(&self) -> &'static str {
        if self.is_dir {
            "Folder"
        } else {
            "File"
        }
    }

    pub fn size_label(&self) -> String {
        if self.is_dir {
            "—".to_string()
        } else {
            format_size(self.size)
        }
    }

    pub fn packed_label(&self) -> String {
        if self.is_dir {
            "—".to_string()
        } else {
            self.compressed_size
                .map(format_size)
                .unwrap_or_else(|| "—".to_string())
        }
    }

    pub fn crc_label(&self) -> String {
        self.crc
            .map(|value| format!("{value:08X}"))
            .unwrap_or_else(|| "—".to_string())
    }

    pub fn ratio_label(&self) -> String {
        if self.is_dir {
            return "—".to_string();
        }
        match (self.size, self.compressed_size) {
            (0, _) => "—".to_string(),
            (size, Some(packed)) => format!("{}%", packed.saturating_mul(100) / size),
            _ => "—".to_string(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExtractReport {
    pub extracted: usize,
    pub destination: PathBuf,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TestReport {
    pub files: usize,
    pub failed: usize,
}

impl TestReport {
    pub fn summary(&self) -> String {
        if self.failed == 0 {
            format!("Test OK · {} file{}", self.files, if self.files == 1 { "" } else { "s" })
        } else {
            format!("Test failed · {} of {} file(s) damaged", self.failed, self.files)
        }
    }
}

pub fn detect_kind(path: &Path) -> Result<ArchiveKind, String> {
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if name.ends_with(".tar.gz") || name.ends_with(".tgz") {
        return Ok(ArchiveKind::TarGz);
    }
    if name.ends_with(".tar.bz2") || name.ends_with(".tbz2") || name.ends_with(".tbz") {
        return Ok(ArchiveKind::TarBz2);
    }
    if name.ends_with(".tar.xz") || name.ends_with(".txz") {
        return Ok(ArchiveKind::TarXz);
    }
    if name.ends_with(".tar.zst") || name.ends_with(".tzst") {
        return Ok(ArchiveKind::TarZstd);
    }
    if name.ends_with(".tar.lz4") {
        return Ok(ArchiveKind::TarLz4);
    }
    if name.ends_with(".tar") {
        return Ok(ArchiveKind::Tar);
    }
    if name.ends_with(".zip") {
        return Ok(ArchiveKind::Zip);
    }
    if name.ends_with(".7z") {
        return Ok(ArchiveKind::SevenZ);
    }
    if name.ends_with(".cab") {
        return Ok(ArchiveKind::Cab);
    }
    if name.ends_with(".gz") {
        return Ok(ArchiveKind::Gzip);
    }
    if name.ends_with(".bz2") {
        return Ok(ArchiveKind::Bzip2);
    }
    if name.ends_with(".xz") {
        return Ok(ArchiveKind::Xz);
    }
    if name.ends_with(".zst") {
        return Ok(ArchiveKind::Zstd);
    }
    if name.ends_with(".lz4") {
        return Ok(ArchiveKind::Lz4);
    }
    sniff_kind(path)
}

pub fn list_entries(path: &Path) -> Result<(ArchiveKind, Vec<ArchiveEntry>), String> {
    let kind = detect_kind(path)?;
    let mut entries = match kind {
        ArchiveKind::Zip => list_zip(path)?,
        ArchiveKind::Tar
        | ArchiveKind::TarGz
        | ArchiveKind::TarBz2
        | ArchiveKind::TarXz
        | ArchiveKind::TarZstd
        | ArchiveKind::TarLz4 => list_tar(path, kind)?,
        ArchiveKind::Gzip
        | ArchiveKind::Bzip2
        | ArchiveKind::Xz
        | ArchiveKind::Zstd
        | ArchiveKind::Lz4 => list_single(path, kind)?,
        ArchiveKind::SevenZ => list_sevenz(path)?,
        ArchiveKind::Cab => list_cab(path)?,
    };
    synthesize_directories(&mut entries);
    entries.sort_by(|left, right| left.path.cmp(&right.path));
    Ok((kind, entries))
}

pub fn extract_entries(
    path: &Path,
    destination: &Path,
    selected: &[String],
) -> Result<ExtractReport, String> {
    let (kind, entries) = list_entries(path)?;
    let names = expand_selection(&entries, selected);
    if names.is_empty() {
        return Err("Nothing to extract.".to_string());
    }
    fs::create_dir_all(destination).map_err(|error| error.to_string())?;
    match kind {
        ArchiveKind::Zip => extract_zip(path, destination, &names)?,
        ArchiveKind::Tar
        | ArchiveKind::TarGz
        | ArchiveKind::TarBz2
        | ArchiveKind::TarXz
        | ArchiveKind::TarZstd
        | ArchiveKind::TarLz4 => extract_tar(path, kind, destination, &names)?,
        ArchiveKind::Gzip
        | ArchiveKind::Bzip2
        | ArchiveKind::Xz
        | ArchiveKind::Zstd
        | ArchiveKind::Lz4 => extract_single(path, kind, destination, &names)?,
        ArchiveKind::SevenZ => extract_sevenz(path, destination, &names)?,
        ArchiveKind::Cab => extract_cab(path, destination, &names)?,
    }
    Ok(ExtractReport {
        extracted: names.len(),
        destination: destination.to_path_buf(),
    })
}

pub fn expand_selection(entries: &[ArchiveEntry], selected: &[String]) -> Vec<String> {
    let mut names = Vec::new();
    for selected_path in selected {
        let normalized = normalize_archive_path(selected_path);
        if normalized.is_empty() {
            continue;
        }
        let prefix = format!("{normalized}/");
        let is_dir = entries
            .iter()
            .any(|entry| entry.path == normalized && entry.is_dir);
        if is_dir {
            for entry in entries {
                if !entry.is_dir && (entry.path == normalized || entry.path.starts_with(&prefix)) {
                    names.push(entry.path.clone());
                }
            }
        } else if entries.iter().any(|entry| entry.path == normalized && !entry.is_dir)
        {
            names.push(normalized);
        }
    }
    names.sort();
    names.dedup();
    names
}

pub fn parent_dir(dir: &str) -> String {
    match dir.rsplit_once('/') {
        Some((parent, _)) => parent.to_string(),
        None => String::new(),
    }
}

pub fn children_of<'a>(entries: &'a [ArchiveEntry], dir: &str) -> Vec<&'a ArchiveEntry> {
    let prefix = if dir.is_empty() {
        String::new()
    } else {
        format!("{dir}/")
    };
    entries
        .iter()
        .filter(|entry| {
            let rest = if prefix.is_empty() {
                entry.path.as_str()
            } else if let Some(rest) = entry.path.strip_prefix(&prefix) {
                rest
            } else {
                return false;
            };
            !rest.is_empty() && !rest.contains('/')
        })
        .collect()
}

pub fn top_level_names(entries: &[ArchiveEntry]) -> Vec<String> {
    let mut names = BTreeSet::new();
    for entry in entries {
        let name = entry.path.split('/').next().unwrap_or(&entry.path);
        if !name.is_empty() {
            names.insert(name.to_string());
        }
    }
    names.into_iter().collect()
}

pub fn smart_extract_destination(
    dest: &Path,
    archive_stem: &str,
    entries: &[ArchiveEntry],
) -> PathBuf {
    if top_level_names(entries).len() <= 1 {
        dest.to_path_buf()
    } else {
        dest.join(archive_stem)
    }
}

pub fn archive_totals(entries: &[ArchiveEntry]) -> (usize, u64, Option<u64>) {
    let files: Vec<_> = entries.iter().filter(|entry| !entry.is_dir).collect();
    let size = files.iter().map(|entry| entry.size).sum();
    let packed = files
        .iter()
        .map(|entry| entry.compressed_size)
        .collect::<Option<Vec<_>>>()
        .map(|values| values.into_iter().sum());
    (files.len(), size, packed)
}

pub fn is_nested_archive(path: &str) -> bool {
    let name = path.to_ascii_lowercase();
    name.ends_with(".zip")
        || name.ends_with(".tar")
        || name.ends_with(".tar.gz")
        || name.ends_with(".tgz")
        || name.ends_with(".tar.bz2")
        || name.ends_with(".tbz2")
        || name.ends_with(".tar.xz")
        || name.ends_with(".txz")
        || name.ends_with(".tar.zst")
        || name.ends_with(".7z")
        || name.ends_with(".cab")
        || name.ends_with(".gz")
        || name.ends_with(".bz2")
        || name.ends_with(".xz")
        || name.ends_with(".zst")
        || name.ends_with(".lz4")
}

pub fn test_archive(path: &Path) -> Result<TestReport, String> {
    let kind = detect_kind(path)?;
    match kind {
        ArchiveKind::Zip => test_zip(path),
        ArchiveKind::Tar
        | ArchiveKind::TarGz
        | ArchiveKind::TarBz2
        | ArchiveKind::TarXz
        | ArchiveKind::TarZstd
        | ArchiveKind::TarLz4 => test_tar(path, kind),
        ArchiveKind::Gzip
        | ArchiveKind::Bzip2
        | ArchiveKind::Xz
        | ArchiveKind::Zstd
        | ArchiveKind::Lz4 => test_single(path, kind),
        ArchiveKind::SevenZ => test_sevenz(path),
        ArchiveKind::Cab => test_cab(path),
    }
}

pub fn create_archive_from_folder(
    folder: &Path,
    destination: &Path,
    kind: ArchiveKind,
) -> Result<(), String> {
    if !kind.can_create() {
        return Err(format!("{} archives cannot be created yet", kind.label()));
    }
    let files = collect_folder_files(folder)?;
    if files.is_empty() {
        return Err("The folder is empty.".to_string());
    }
    let payload: Vec<(String, Vec<u8>)> = files
        .into_iter()
        .map(|(name, path)| {
            let bytes = fs::read(&path).map_err(|error| error.to_string())?;
            Ok((name, bytes))
        })
        .collect::<Result<Vec<_>, String>>()?;
    let refs: Vec<(&str, &[u8])> = payload
        .iter()
        .map(|(name, bytes)| (name.as_str(), bytes.as_slice()))
        .collect();
    match kind {
        ArchiveKind::Zip => write_zip(destination, &refs),
        ArchiveKind::Tar => write_tar(destination, ArchiveKind::Tar, &refs),
        ArchiveKind::TarGz => write_tar(destination, ArchiveKind::TarGz, &refs),
        ArchiveKind::SevenZ => write_sevenz(destination, &payload),
        _ => Err(format!("{} archives cannot be created yet", kind.label())),
    }
}

pub fn format_size(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    if bytes < 1024 {
        return format!("{bytes} B");
    }
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit + 1 < UNITS.len() {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{} {}", bytes, UNITS[unit])
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

pub fn safe_output_path(destination: &Path, entry_path: &str) -> Result<PathBuf, String> {
    let relative = relative_entry_path(entry_path)?;
    Ok(destination.join(relative))
}

fn relative_entry_path(entry_path: &str) -> Result<PathBuf, String> {
    let raw = entry_path.replace('\\', "/");
    if raw.starts_with('/') || has_windows_drive_prefix(&raw) {
        return Err(format!("unsafe archive path: {entry_path}"));
    }
    let normalized = normalize_archive_path(entry_path);
    if normalized.is_empty() {
        return Err("archive entry path is empty".to_string());
    }
    let mut output = PathBuf::new();
    for component in Path::new(&normalized).components() {
        match component {
            Component::Normal(name) => output.push(name),
            Component::CurDir => {}
            Component::Prefix(_) | Component::RootDir | Component::ParentDir => {
                return Err(format!("unsafe archive path: {entry_path}"));
            }
        }
    }
    if output.as_os_str().is_empty() {
        return Err(format!("unsafe archive path: {entry_path}"));
    }
    Ok(output)
}

fn has_windows_drive_prefix(path: &str) -> bool {
    let mut chars = path.chars();
    matches!(
        (chars.next(), chars.next()),
        (Some(letter), Some(':')) if letter.is_ascii_alphabetic()
    )
}

fn normalize_archive_path(path: &str) -> String {
    path.replace('\\', "/")
        .trim_matches('/')
        .trim()
        .to_string()
}

fn entry_name(path: &str) -> String {
    Path::new(path)
        .file_name()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .unwrap_or(path)
        .to_string()
}

fn sniff_kind(path: &Path) -> Result<ArchiveKind, String> {
    let mut file = File::open(path).map_err(|error| error.to_string())?;
    let mut header = [0_u8; 265];
    let read = file.read(&mut header).map_err(|error| error.to_string())?;
    if read >= 2 && header[0] == 0x50 && header[1] == 0x4B {
        return Ok(ArchiveKind::Zip);
    }
    if read >= 2 && header[0] == 0x1F && header[1] == 0x8B {
        return Ok(ArchiveKind::Gzip);
    }
    if read >= 3 && &header[0..3] == b"BZh" {
        return Ok(ArchiveKind::Bzip2);
    }
    if read >= 6 && header[0..6] == [0xFD, 0x37, 0x7A, 0x58, 0x5A, 0x00] {
        return Ok(ArchiveKind::Xz);
    }
    if read >= 4 && header[0..4] == [0x28, 0xB5, 0x2F, 0xFD] {
        return Ok(ArchiveKind::Zstd);
    }
    if read >= 4 && header[0..4] == [0x04, 0x22, 0x4D, 0x18] {
        return Ok(ArchiveKind::Lz4);
    }
    if read >= 6 && header[0..6] == [0x37, 0x7A, 0xBC, 0xAF, 0x27, 0x1C] {
        return Ok(ArchiveKind::SevenZ);
    }
    if read >= 4 && &header[0..4] == b"MSCF" {
        return Ok(ArchiveKind::Cab);
    }
    if read >= 262 && &header[257..262] == b"ustar" {
        return Ok(ArchiveKind::Tar);
    }
    Err(format!(
        "Unsupported archive: {}",
        path.file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("file")
    ))
}

fn list_zip(path: &Path) -> Result<Vec<ArchiveEntry>, String> {
    let file = File::open(path).map_err(|error| error.to_string())?;
    let mut reader = io::BufReader::with_capacity(64 * 1024, file);
    let archive = ZipArchive::new(&mut reader).map_err(|error| error.to_string())?;
    let count = archive.len();
    let cd_start = archive.central_directory_start();
    drop(archive);
    reader
        .seek(SeekFrom::Start(cd_start))
        .map_err(|error| error.to_string())?;
    let mut entries = Vec::with_capacity(count);
    for _ in 0..count {
        if let Some(entry) = read_zip_central_entry(&mut reader)? {
            entries.push(entry);
        }
    }
    Ok(entries)
}

fn read_zip_central_entry(reader: &mut impl Read) -> Result<Option<ArchiveEntry>, String> {
    let mut header = [0_u8; 46];
    reader.read_exact(&mut header).map_err(|error| error.to_string())?;
    if header[0..4] != [0x50, 0x4B, 0x01, 0x02] {
        return Err("invalid zip central directory".to_string());
    }
    let flags = u16::from_le_bytes(header[8..10].try_into().unwrap());
    let time = u16::from_le_bytes(header[12..14].try_into().unwrap());
    let date = u16::from_le_bytes(header[14..16].try_into().unwrap());
    let crc = u32::from_le_bytes(header[16..20].try_into().unwrap());
    let mut compressed = u32::from_le_bytes(header[20..24].try_into().unwrap()) as u64;
    let mut uncompressed = u32::from_le_bytes(header[24..28].try_into().unwrap()) as u64;
    let name_len = u16::from_le_bytes(header[28..30].try_into().unwrap()) as usize;
    let extra_len = u16::from_le_bytes(header[30..32].try_into().unwrap()) as usize;
    let comment_len = u16::from_le_bytes(header[32..34].try_into().unwrap()) as usize;
    let external_attr = u32::from_le_bytes(header[38..42].try_into().unwrap());
    let mut name_bytes = vec![0_u8; name_len];
    reader
        .read_exact(&mut name_bytes)
        .map_err(|error| error.to_string())?;
    let mut extra = vec![0_u8; extra_len];
    reader
        .read_exact(&mut extra)
        .map_err(|error| error.to_string())?;
    if comment_len > 0 {
        io::copy(
            &mut reader.take(comment_len as u64),
            &mut io::sink(),
        )
        .map_err(|error| error.to_string())?;
    }
    apply_zip64_sizes(&extra, &mut uncompressed, &mut compressed);
    let raw_name = if flags & (1 << 11) != 0 {
        String::from_utf8_lossy(&name_bytes).into_owned()
    } else {
        String::from_utf8_lossy(&name_bytes).into_owned()
    };
    let unix_mode = (external_attr >> 16) as u32;
    if unix_mode & 0o170000 == 0o120000 {
        return Ok(None);
    }
    let is_dir = raw_name.ends_with('/')
        || unix_mode & 0o170000 == 0o040000
        || external_attr & 0x10 != 0;
    let normalized = normalize_archive_path(&raw_name);
    if normalized.is_empty() {
        return Ok(None);
    }
    relative_entry_path(&normalized)?;
    let modified = zip::DateTime::try_from_msdos(date, time)
        .ok()
        .map(format_zip_datetime)
        .unwrap_or_default();
    Ok(Some(ArchiveEntry {
        name: entry_name(&normalized),
        path: normalized,
        is_dir,
        size: uncompressed,
        compressed_size: Some(compressed),
        crc: if is_dir { None } else { Some(crc) },
        modified,
    }))
}

fn apply_zip64_sizes(extra: &[u8], uncompressed: &mut u64, compressed: &mut u64) {
    let need_uncomp = *uncompressed == 0xFFFF_FFFF;
    let need_comp = *compressed == 0xFFFF_FFFF;
    if !need_uncomp && !need_comp {
        return;
    }
    let mut offset = 0;
    while offset + 4 <= extra.len() {
        let header_id = u16::from_le_bytes(extra[offset..offset + 2].try_into().unwrap());
        let size = u16::from_le_bytes(extra[offset + 2..offset + 4].try_into().unwrap()) as usize;
        let start = offset + 4;
        let end = start.saturating_add(size);
        if end > extra.len() {
            break;
        }
        if header_id == 0x0001 {
            let mut field = &extra[start..end];
            if need_uncomp && field.len() >= 8 {
                *uncompressed = u64::from_le_bytes(field[..8].try_into().unwrap());
                field = &field[8..];
            }
            if need_comp && field.len() >= 8 {
                *compressed = u64::from_le_bytes(field[..8].try_into().unwrap());
            }
            return;
        }
        offset = end;
    }
}

fn list_tar(path: &Path, kind: ArchiveKind) -> Result<Vec<ArchiveEntry>, String> {
    match kind {
        ArchiveKind::Tar => {
            let mut file = File::open(path).map_err(|error| error.to_string())?;
            list_tar_headers(&mut file, skip_seek)
        }
        ArchiveKind::TarGz => list_tar_gz(path),
        _ => {
            let mut reader = open_decoder(path, kind)?;
            let mut skip_buf = vec![0_u8; 256 * 1024];
            list_tar_headers(&mut reader, |reader, amount| {
                skip_bytes(reader, amount, &mut skip_buf)
            })
        }
    }
}

fn list_tar_gz(path: &Path) -> Result<Vec<ArchiveEntry>, String> {
    let file = File::open(path).map_err(|error| error.to_string())?;
    let mut decoder = GzDecoder::new(io::BufReader::with_capacity(512 * 1024, file));
    let mut data = Vec::new();
    if let Some(size) = gzip_uncompressed_size(path) {
        if (1..1_500_000_000).contains(&size) {
            data.reserve(size as usize);
        }
    }
    decoder
        .read_to_end(&mut data)
        .map_err(|error| error.to_string())?;
    let mut cursor = io::Cursor::new(data);
    list_tar_headers(&mut cursor, skip_seek)
}

fn skip_seek<R: Seek>(reader: &mut R, amount: u64) -> Result<(), String> {
    if amount == 0 {
        return Ok(());
    }
    reader
        .seek(SeekFrom::Current(amount as i64))
        .map(|_| ())
        .map_err(|error| error.to_string())
}

fn list_tar_headers<R: Read>(
    reader: &mut R,
    mut skip: impl FnMut(&mut R, u64) -> Result<(), String>,
) -> Result<Vec<ArchiveEntry>, String> {
    const BLOCK: u64 = 512;
    let mut entries = Vec::new();
    let mut header = [0_u8; 512];
    let mut pending_name: Option<String> = None;
    loop {
        match reader.read_exact(&mut header) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::UnexpectedEof => break,
            Err(error) => return Err(error.to_string()),
        }
        if header.iter().all(|&byte| byte == 0) {
            break;
        }
        let size = parse_tar_number(&header[124..136]);
        let mtime = parse_tar_number(&header[136..148]);
        let typeflag = header[156];
        let padded = size.div_ceil(BLOCK) * BLOCK;
        match typeflag {
            b'L' => {
                pending_name = Some(read_tar_string(reader, size)?);
                skip(reader, padded.saturating_sub(size))?;
                continue;
            }
            b'K' | b'x' | b'g' | b'X' => {
                skip(reader, padded)?;
                continue;
            }
            b'0' | 0 | b'7' | b'5' => {}
            _ => {
                skip(reader, padded)?;
                continue;
            }
        }
        let raw = pending_name.take().unwrap_or_else(|| ustar_name(&header));
        let normalized = normalize_archive_path(&raw);
        if normalized.is_empty() || normalized == "." || normalized == ".." {
            skip(reader, padded)?;
            continue;
        }
        if relative_entry_path(&normalized).is_err() {
            skip(reader, padded)?;
            continue;
        }
        let is_dir = typeflag == b'5' || normalized.ends_with('/');
        entries.push(ArchiveEntry {
            name: entry_name(&normalized),
            path: normalized,
            is_dir,
            size: if is_dir { 0 } else { size },
            compressed_size: None,
            crc: None,
            modified: format_unix_mtime(mtime),
        });
        skip(reader, padded)?;
    }
    Ok(entries)
}

fn ustar_name(header: &[u8; 512]) -> String {
    let name = tar_cstr(&header[0..100]);
    if header.len() >= 500 && header.get(257..262) == Some(b"ustar") {
        let prefix = tar_cstr(&header[345..500]);
        if !prefix.is_empty() && !name.is_empty() {
            return format!("{prefix}/{name}");
        }
        if !prefix.is_empty() {
            return prefix;
        }
    }
    name
}

fn tar_cstr(bytes: &[u8]) -> String {
    let end = bytes.iter().position(|&byte| byte == 0).unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).into_owned()
}

fn parse_tar_number(bytes: &[u8]) -> u64 {
    if bytes.is_empty() {
        return 0;
    }
    if bytes[0] & 0x80 != 0 {
        let mut value = u64::from(bytes[0] & 0x7F);
        for &byte in &bytes[1..] {
            value = (value << 8) | u64::from(byte);
        }
        return value;
    }
    let text = tar_cstr(bytes);
    let trimmed = text.trim();
    if trimmed.is_empty() {
        0
    } else {
        u64::from_str_radix(trimmed, 8).unwrap_or(0)
    }
}

fn read_tar_string(reader: &mut impl Read, size: u64) -> Result<String, String> {
    let mut bytes = vec![0_u8; size as usize];
    reader.read_exact(&mut bytes).map_err(|error| error.to_string())?;
    let end = bytes.iter().position(|&byte| byte == 0).unwrap_or(bytes.len());
    Ok(String::from_utf8_lossy(&bytes[..end]).into_owned())
}

fn skip_bytes(reader: &mut impl Read, mut remaining: u64, buf: &mut [u8]) -> Result<(), String> {
    while remaining > 0 {
        let chunk = remaining.min(buf.len() as u64) as usize;
        let read = reader
            .read(&mut buf[..chunk])
            .map_err(|error| error.to_string())?;
        if read == 0 {
            return Err("truncated tar archive".to_string());
        }
        remaining -= read as u64;
    }
    Ok(())
}

fn extract_zip(path: &Path, destination: &Path, names: &[String]) -> Result<(), String> {
    let file = File::open(path).map_err(|error| error.to_string())?;
    let reader = io::BufReader::with_capacity(64 * 1024, file);
    let mut archive = ZipArchive::new(reader).map_err(|error| error.to_string())?;
    for name in names {
        let mut file = archive.by_name(name).map_err(|error| error.to_string())?;
        if file.is_symlink() || file.is_dir() {
            continue;
        }
        let output = safe_output_path(destination, file.name())?;
        if let Some(parent) = output.parent() {
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        let mut created = File::create(&output).map_err(|error| error.to_string())?;
        io::copy(&mut file, &mut created).map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn extract_tar(
    path: &Path,
    kind: ArchiveKind,
    destination: &Path,
    names: &[String],
) -> Result<(), String> {
    let wanted: BTreeSet<&str> = names.iter().map(String::as_str).collect();
    let mut archive = open_tar(path, kind)?;
    for entry in archive.entries().map_err(|error| error.to_string())? {
        let mut entry = entry.map_err(|error| error.to_string())?;
        let header = entry.header().clone();
        if !header.entry_type().is_file() {
            continue;
        }
        let path = entry
            .path()
            .map_err(|error| error.to_string())?
            .to_string_lossy()
            .into_owned();
        let normalized = normalize_archive_path(&path);
        if !wanted.contains(normalized.as_str()) {
            continue;
        }
        let output = safe_output_path(destination, &normalized)?;
        if let Some(parent) = output.parent() {
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        let mut created = File::create(&output).map_err(|error| error.to_string())?;
        io::copy(&mut entry, &mut created).map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn open_decoder(path: &Path, kind: ArchiveKind) -> Result<Box<dyn Read>, String> {
    let file = File::open(path).map_err(|error| error.to_string())?;
    let input = io::BufReader::with_capacity(256 * 1024, file);
    let decoder: Box<dyn Read> = match kind {
        ArchiveKind::Tar => Box::new(input),
        ArchiveKind::TarGz | ArchiveKind::Gzip => Box::new(GzDecoder::new(input)),
        ArchiveKind::TarBz2 | ArchiveKind::Bzip2 => Box::new(bzip2::read::BzDecoder::new(input)),
        ArchiveKind::TarXz | ArchiveKind::Xz => Box::new(xz2::read::XzDecoder::new(input)),
        ArchiveKind::TarZstd | ArchiveKind::Zstd => Box::new(
            zstd::stream::read::Decoder::new(input).map_err(|error| error.to_string())?,
        ),
        ArchiveKind::TarLz4 | ArchiveKind::Lz4 => Box::new(lz4_flex::frame::FrameDecoder::new(input)),
        _ => return Err("not a tar archive".to_string()),
    };
    Ok(Box::new(io::BufReader::with_capacity(256 * 1024, decoder)))
}

fn open_tar(path: &Path, kind: ArchiveKind) -> Result<tar::Archive<Box<dyn Read>>, String> {
    Ok(tar::Archive::new(open_decoder(path, kind)?))
}

fn synthesize_directories(entries: &mut Vec<ArchiveEntry>) {
    let existing: HashSet<String> = entries.iter().map(|entry| entry.path.clone()).collect();
    let mut directories = BTreeSet::new();
    for entry in entries.iter() {
        let mut parent = Path::new(&entry.path);
        while let Some(next) = parent.parent() {
            if next.as_os_str().is_empty() {
                break;
            }
            directories.insert(next.to_string_lossy().replace('\\', "/"));
            parent = next;
        }
    }
    for path in directories {
        if existing.contains(&path) {
            continue;
        }
        entries.push(ArchiveEntry {
            name: entry_name(&path),
            path,
            is_dir: true,
            size: 0,
            compressed_size: None,
            crc: None,
            modified: String::new(),
        });
    }
}

fn test_zip(path: &Path) -> Result<TestReport, String> {
    let file = File::open(path).map_err(|error| error.to_string())?;
    let mut archive = ZipArchive::new(file).map_err(|error| error.to_string())?;
    let mut files = 0;
    let mut failed = 0;
    for index in 0..archive.len() {
        let mut file = archive.by_index(index).map_err(|error| error.to_string())?;
        if file.is_dir() || file.is_symlink() {
            continue;
        }
        files += 1;
        if io::copy(&mut file, &mut io::sink()).is_err() {
            failed += 1;
        }
    }
    Ok(TestReport { files, failed })
}

fn test_tar(path: &Path, kind: ArchiveKind) -> Result<TestReport, String> {
    let mut archive = open_tar(path, kind)?;
    let mut files = 0;
    let mut failed = 0;
    for entry in archive.entries().map_err(|error| error.to_string())? {
        let mut entry = match entry {
            Ok(entry) => entry,
            Err(_) => {
                failed += 1;
                continue;
            }
        };
        if !entry.header().entry_type().is_file() {
            continue;
        }
        files += 1;
        if io::copy(&mut entry, &mut io::sink()).is_err() {
            failed += 1;
        }
    }
    Ok(TestReport { files, failed })
}

fn collect_folder_files(folder: &Path) -> Result<Vec<(String, PathBuf)>, String> {
    let mut files = Vec::new();
    collect_folder_files_inner(folder, folder, &mut files)?;
    Ok(files)
}

fn collect_folder_files_inner(
    root: &Path,
    current: &Path,
    files: &mut Vec<(String, PathBuf)>,
) -> Result<(), String> {
    let entries = fs::read_dir(current).map_err(|error| error.to_string())?;
    for entry in entries {
        let entry = entry.map_err(|error| error.to_string())?;
        let path = entry.path();
        let file_type = entry.file_type().map_err(|error| error.to_string())?;
        if file_type.is_dir() {
            collect_folder_files_inner(root, &path, files)?;
            continue;
        }
        if !file_type.is_file() {
            continue;
        }
        let relative = path
            .strip_prefix(root)
            .map_err(|_| "file is outside the source folder".to_string())?
            .to_string_lossy()
            .replace('\\', "/");
        relative_entry_path(&relative)?;
        files.push((relative, path));
    }
    Ok(())
}

fn write_zip(path: &Path, files: &[(&str, &[u8])]) -> Result<(), String> {
    let file = File::create(path).map_err(|error| error.to_string())?;
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    for (name, bytes) in files {
        zip.start_file(*name, options)
            .map_err(|error| error.to_string())?;
        zip.write_all(bytes).map_err(|error| error.to_string())?;
    }
    zip.finish().map_err(|error| error.to_string())?;
    Ok(())
}

fn write_tar(path: &Path, kind: ArchiveKind, files: &[(&str, &[u8])]) -> Result<(), String> {
    let file = File::create(path).map_err(|error| error.to_string())?;
    match kind {
        ArchiveKind::Tar => append_tar(file, files),
        ArchiveKind::TarGz => append_tar(GzEncoder::new(file, Compression::default()), files),
        _ => Err(format!("{} archives cannot be created yet", kind.label())),
    }
}

fn single_entry_name(path: &Path, kind: ArchiveKind) -> String {
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("file")
        .to_string();
    let suffixes = match kind {
        ArchiveKind::Gzip => &[".tar.gz", ".gz"][..],
        ArchiveKind::Bzip2 => &[".tar.bz2", ".bz2"][..],
        ArchiveKind::Xz => &[".tar.xz", ".xz"][..],
        ArchiveKind::Zstd => &[".tar.zst", ".zst"][..],
        ArchiveKind::Lz4 => &[".tar.lz4", ".lz4"][..],
        _ => &[][..],
    };
    let lower = name.to_ascii_lowercase();
    for suffix in suffixes {
        if lower.ends_with(suffix) {
            return name[..name.len() - suffix.len()].to_string();
        }
    }
    name
}

fn open_single_decoder(path: &Path, kind: ArchiveKind) -> Result<Box<dyn Read>, String> {
    open_decoder(path, kind)
}

fn gzip_uncompressed_size(path: &Path) -> Option<u64> {
    let mut file = File::open(path).ok()?;
    let len = file.seek(SeekFrom::End(0)).ok()?;
    if len < 8 {
        return None;
    }
    file.seek(SeekFrom::End(-4)).ok()?;
    let mut isize = [0_u8; 4];
    file.read_exact(&mut isize).ok()?;
    Some(u32::from_le_bytes(isize) as u64)
}

fn list_single(path: &Path, kind: ArchiveKind) -> Result<Vec<ArchiveEntry>, String> {
    let packed = fs::metadata(path).map(|meta| meta.len()).unwrap_or(0);
    let size = match kind {
        ArchiveKind::Gzip => gzip_uncompressed_size(path).unwrap_or(packed),
        _ => packed,
    };
    let name = single_entry_name(path, kind);
    Ok(vec![ArchiveEntry {
        name: name.clone(),
        path: name,
        is_dir: false,
        size,
        compressed_size: Some(packed),
        crc: None,
        modified: String::new(),
    }])
}

fn extract_single(
    path: &Path,
    kind: ArchiveKind,
    destination: &Path,
    names: &[String],
) -> Result<(), String> {
    let entry_name = single_entry_name(path, kind);
    if !names.is_empty() && !names.iter().any(|name| name == &entry_name) {
        return Ok(());
    }
    fs::create_dir_all(destination).map_err(|error| error.to_string())?;
    let output = safe_output_path(destination, &entry_name)?;
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let mut decoder = open_single_decoder(path, kind)?;
    let mut created = File::create(&output).map_err(|error| error.to_string())?;
    io::copy(&mut decoder, &mut created).map_err(|error| error.to_string())?;
    Ok(())
}

fn test_single(path: &Path, kind: ArchiveKind) -> Result<TestReport, String> {
    let mut decoder = open_single_decoder(path, kind)?;
    io::copy(&mut decoder, &mut io::sink()).map_err(|error| error.to_string())?;
    Ok(TestReport {
        files: 1,
        failed: 0,
    })
}

fn list_sevenz(path: &Path) -> Result<Vec<ArchiveEntry>, String> {
    let reader = sevenz_rust2::ArchiveReader::open(path, sevenz_rust2::Password::empty())
        .map_err(|error| error.to_string())?;
    let mut entries = Vec::new();
    for file in reader.archive().files.iter() {
        let raw = file.name();
        let normalized = normalize_archive_path(raw);
        if normalized.is_empty() {
            continue;
        }
        relative_entry_path(&normalized)?;
        let is_dir = file.is_directory();
        entries.push(ArchiveEntry {
            name: entry_name(&normalized),
            path: normalized,
            is_dir,
            size: file.size(),
            compressed_size: Some(file.compressed_size),
            crc: if is_dir || !file.has_crc {
                None
            } else {
                Some(file.crc as u32)
            },
            modified: String::new(),
        });
    }
    Ok(entries)
}

fn extract_sevenz(path: &Path, destination: &Path, names: &[String]) -> Result<(), String> {
    let mut reader = sevenz_rust2::ArchiveReader::open(path, sevenz_rust2::Password::empty())
        .map_err(|error| error.to_string())?;
    let wanted: BTreeSet<&str> = names.iter().map(String::as_str).collect();
    let entries: Vec<(String, bool)> = reader
        .archive()
        .files
        .iter()
        .map(|file| (file.name().to_string(), file.is_directory()))
        .collect();
    for (name, is_dir) in entries {
        let normalized = normalize_archive_path(&name);
        if !wanted.contains(normalized.as_str()) {
            continue;
        }
        let output = safe_output_path(destination, &normalized)?;
        if is_dir {
            fs::create_dir_all(&output).map_err(|error| error.to_string())?;
            continue;
        }
        if let Some(parent) = output.parent() {
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        let data = reader
            .read_file(&name)
            .map_err(|error| error.to_string())?;
        fs::write(&output, data).map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn test_sevenz(path: &Path) -> Result<TestReport, String> {
    let mut reader = sevenz_rust2::ArchiveReader::open(path, sevenz_rust2::Password::empty())
        .map_err(|error| error.to_string())?;
    let names: Vec<(String, bool)> = reader
        .archive()
        .files
        .iter()
        .map(|file| (file.name().to_string(), file.is_directory()))
        .collect();
    let mut files = 0;
    let mut failed = 0;
    for (name, is_dir) in names {
        if is_dir {
            continue;
        }
        files += 1;
        if reader.read_file(&name).is_err() {
            failed += 1;
        }
    }
    Ok(TestReport { files, failed })
}

fn write_sevenz(path: &Path, files: &[(String, Vec<u8>)]) -> Result<(), String> {
    let mut writer =
        sevenz_rust2::ArchiveWriter::create(path).map_err(|error| error.to_string())?;
    for (name, bytes) in files {
        let entry = sevenz_rust2::ArchiveEntry::new_file(name);
        writer
            .push_archive_entry(entry, Some(bytes.as_slice()))
            .map_err(|error| error.to_string())?;
    }
    writer.finish().map_err(|error| error.to_string())?;
    Ok(())
}

fn list_cab(path: &Path) -> Result<Vec<ArchiveEntry>, String> {
    let file = File::open(path).map_err(|error| error.to_string())?;
    let cabinet = cab::Cabinet::new(file).map_err(|error| error.to_string())?;
    let mut entries = Vec::new();
    for folder in cabinet.folder_entries() {
        for file in folder.file_entries() {
            let raw = file.name().to_string();
            let normalized = normalize_archive_path(&raw);
            if normalized.is_empty() {
                continue;
            }
            relative_entry_path(&normalized)?;
            entries.push(ArchiveEntry {
                name: entry_name(&normalized),
                path: normalized,
                is_dir: false,
                size: u64::from(file.uncompressed_size()),
                compressed_size: None,
                crc: None,
                modified: String::new(),
            });
        }
    }
    Ok(entries)
}

fn extract_cab(path: &Path, destination: &Path, names: &[String]) -> Result<(), String> {
    let file = File::open(path).map_err(|error| error.to_string())?;
    let mut cabinet = cab::Cabinet::new(file).map_err(|error| error.to_string())?;
    let wanted: BTreeSet<&str> = names.iter().map(String::as_str).collect();
    let names: Vec<String> = cabinet
        .folder_entries()
        .flat_map(|folder| {
            folder
                .file_entries()
                .map(|file| file.name().to_string())
                .collect::<Vec<_>>()
        })
        .collect();
    for name in names {
        let normalized = normalize_archive_path(&name);
        if !wanted.contains(normalized.as_str()) {
            continue;
        }
        let output = safe_output_path(destination, &normalized)?;
        if let Some(parent) = output.parent() {
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        let mut reader = cabinet
            .read_file(&name)
            .map_err(|error| error.to_string())?;
        let mut created = File::create(&output).map_err(|error| error.to_string())?;
        io::copy(&mut reader, &mut created).map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn test_cab(path: &Path) -> Result<TestReport, String> {
    let file = File::open(path).map_err(|error| error.to_string())?;
    let mut cabinet = cab::Cabinet::new(file).map_err(|error| error.to_string())?;
    let names: Vec<String> = cabinet
        .folder_entries()
        .flat_map(|folder| {
            folder
                .file_entries()
                .map(|file| file.name().to_string())
                .collect::<Vec<_>>()
        })
        .collect();
    let mut files = 0;
    let mut failed = 0;
    for name in names {
        files += 1;
        match cabinet.read_file(&name) {
            Ok(mut reader) => {
                if io::copy(&mut reader, &mut io::sink()).is_err() {
                    failed += 1;
                }
            }
            Err(_) => failed += 1,
        }
    }
    Ok(TestReport { files, failed })
}

fn append_tar<W: Write>(writer: W, files: &[(&str, &[u8])]) -> Result<(), String> {
    let mut builder = Builder::new(writer);
    for (name, bytes) in files {
        let mut header = Header::new_gnu();
        header.set_size(bytes.len() as u64);
        header.set_mode(0o644);
        header.set_entry_type(EntryType::Regular);
        header.set_mtime(1_700_000_000);
        header.set_cksum();
        builder
            .append_data(&mut header, name, *bytes)
            .map_err(|error| error.to_string())?;
    }
    builder.finish().map_err(|error| error.to_string())
}

fn format_zip_datetime(datetime: zip::DateTime) -> String {
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}",
        datetime.year(),
        datetime.month(),
        datetime.day(),
        datetime.hour(),
        datetime.minute()
    )
}

fn format_unix_mtime(mtime: u64) -> String {
    let Some(time) = UNIX_EPOCH.checked_add(std::time::Duration::from_secs(mtime)) else {
        return String::new();
    };
    format_system_time(time)
}

fn format_system_time(time: SystemTime) -> String {
    let Ok(duration) = time.duration_since(UNIX_EPOCH) else {
        return String::new();
    };
    let total = duration.as_secs();
    let seconds = total % 60;
    let minutes = (total / 60) % 60;
    let hours = (total / 3600) % 24;
    let days = total / 86400;
    // 1970-01-01 plus whole days; good enough for archive listing tests.
    let (year, month, day) = civil_from_days(days as i64);
    format!("{year:04}-{month:02}-{day:02} {hours:02}:{minutes:02}:{seconds:02}")
}

fn civil_from_days(days: i64) -> (i32, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if m <= 2 { y + 1 } else { y };
    (year as i32, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    fn temp_path(name: &str) -> PathBuf {
        let unique = format!(
            "rustolonia-archive-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|value| value.as_nanos())
                .unwrap_or(0)
        );
        let dir = env::temp_dir().join(unique);
        let _ = fs::create_dir_all(&dir);
        dir.join(name)
    }

    fn sample_files() -> [(&'static str, &'static [u8]); 3] {
        [
            ("readme.txt", b"Rustolonia Archive Explorer\n"),
            ("docs/notes.txt", b"Sample section 2\n"),
            ("data/hello.txt", b"Hello from a sample zip.\n"),
        ]
    }

    #[test]
    fn lists_zip_entries_and_synthesizes_folders() {
        let path = temp_path("sample.zip");
        write_zip(&path, &sample_files()).expect("write zip");
        let (kind, entries) = list_entries(&path).expect("list zip");
        assert_eq!(kind, ArchiveKind::Zip);
        let paths: Vec<_> = entries.iter().map(|entry| entry.path.as_str()).collect();
        assert!(paths.contains(&"readme.txt"));
        assert!(paths.contains(&"docs"));
        assert!(paths.contains(&"docs/notes.txt"));
        assert!(paths.contains(&"data/hello.txt"));
        let notes = entries
            .iter()
            .find(|entry| entry.path == "docs/notes.txt")
            .expect("notes");
        assert!(!notes.is_dir);
        assert!(notes.size > 0);
        assert!(notes.compressed_size.is_some());
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn listing_zip_does_not_read_file_payloads() {
        let path = temp_path("large.zip");
        let payload = vec![0_u8; 8 * 1024 * 1024];
        write_zip(&path, &[("big.bin", &payload)]).expect("write zip");
        let started = std::time::Instant::now();
        let (_, entries) = list_entries(&path).expect("list");
        assert!(
            started.elapsed().as_millis() < 1_500,
            "listing read payloads: {:?}",
            started.elapsed()
        );
        let big = entries.iter().find(|entry| entry.path == "big.bin").expect("big");
        assert_eq!(big.size, payload.len() as u64);
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn lists_and_extracts_tar_gz() {
        let path = temp_path("sample.tar.gz");
        write_tar(&path, ArchiveKind::TarGz, &sample_files()).expect("write tar.gz");
        let (kind, entries) = list_entries(&path).expect("list tar.gz");
        assert_eq!(kind, ArchiveKind::TarGz);
        assert!(entries.iter().any(|entry| entry.path == "readme.txt"));

        let dest = temp_path("out");
        let report = extract_entries(&path, &dest, &["docs".to_string()]).expect("extract");
        assert_eq!(report.extracted, 1);
        let notes = fs::read_to_string(dest.join("docs").join("notes.txt")).expect("notes");
        assert!(notes.contains("Sample section 2"));
        let _ = fs::remove_file(&path);
        let _ = fs::remove_dir_all(&dest);
    }

    #[test]
    fn lists_large_tar_gz_by_skipping_payloads() {
        let path = temp_path("large.tar.gz");
        let payload: Vec<u8> = (0..4 * 1024 * 1024).map(|index| (index % 251) as u8).collect();
        write_tar(
            &path,
            ArchiveKind::TarGz,
            &[
                ("blob.bin", payload.as_slice()),
                ("readme.txt", b"ok\n".as_slice()),
            ],
        )
        .expect("write");
        let started = std::time::Instant::now();
        let (kind, entries) = list_entries(&path).expect("list");
        assert!(
            started.elapsed().as_millis() < 800,
            "listing too slow: {:?}",
            started.elapsed()
        );
        assert_eq!(kind, ArchiveKind::TarGz);
        let blob = entries.iter().find(|entry| entry.path == "blob.bin").expect("blob");
        assert_eq!(blob.size, payload.len() as u64);
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn lists_devo_cli_tar_gz_quickly() {
        let path = PathBuf::from(r"C:\Users\mamoreau\Downloads\devo-cli-win-x64-2026.3.0.0.tar.gz");
        if !path.exists() {
            return;
        }
        let started = std::time::Instant::now();
        let (kind, entries) = list_entries(&path).expect("list de vo tar.gz");
        let elapsed = started.elapsed();
        assert_eq!(kind, ArchiveKind::TarGz);
        assert!(
            !entries.is_empty(),
            "expected entries in {}",
            path.display()
        );
        if !cfg!(debug_assertions) {
            assert!(
                elapsed.as_millis() < 1_500,
                "listing {} took {:?}",
                path.display(),
                elapsed
            );
        }
    }

    #[test]
    fn extract_selected_files_from_zip() {
        let path = temp_path("extract.zip");
        write_zip(&path, &sample_files()).expect("write zip");
        let dest = temp_path("extract-out");
        let report =
            extract_entries(&path, &dest, &["readme.txt".to_string()]).expect("extract readme");
        assert_eq!(report.extracted, 1);
        let body = fs::read_to_string(dest.join("readme.txt")).expect("readme");
        assert!(body.contains("Archive Explorer"));
        assert!(!dest.join("data").join("hello.txt").exists());
        let _ = fs::remove_file(&path);
        let _ = fs::remove_dir_all(&dest);
    }

    #[test]
    fn rejects_zip_slip_parent_paths() {
        let path = temp_path("slip.zip");
        write_zip(&path, &[("../evil.txt", b"nope\n")]).expect("write slip zip");
        let listed = list_entries(&path);
        assert!(listed.is_err(), "listing should reject parent paths");
        let dest = temp_path("slip-out");
        fs::create_dir_all(&dest).expect("dest");
        let extracted = extract_entries(&path, &dest, &["../evil.txt".to_string()]);
        assert!(extracted.is_err());
        assert!(!dest.join("evil.txt").exists());
        let _ = fs::remove_file(&path);
        let _ = fs::remove_dir_all(&dest);
    }

    #[test]
    fn rejects_absolute_entry_paths() {
        assert!(relative_entry_path("/tmp/evil.txt").is_err());
        assert!(relative_entry_path("C:/Windows/evil.txt").is_err());
        assert!(relative_entry_path("\\\\server\\share\\evil.txt").is_err());
    }

    #[test]
    fn expand_selection_includes_folder_children() {
        let entries = vec![
            ArchiveEntry {
                path: "docs".to_string(),
                name: "docs".to_string(),
                is_dir: true,
                size: 0,
                compressed_size: None,
                crc: None,
                modified: String::new(),
            },
            ArchiveEntry {
                path: "docs/notes.txt".to_string(),
                name: "notes.txt".to_string(),
                is_dir: false,
                size: 4,
                compressed_size: Some(4),
                crc: Some(1),
                modified: String::new(),
            },
            ArchiveEntry {
                path: "readme.txt".to_string(),
                name: "readme.txt".to_string(),
                is_dir: false,
                size: 4,
                compressed_size: Some(4),
                crc: Some(1),
                modified: String::new(),
            },
        ];
        let names = expand_selection(&entries, &["docs".to_string()]);
        assert_eq!(names, vec!["docs/notes.txt".to_string()]);
    }

    #[test]
    fn children_of_lists_one_directory_level() {
        let path = temp_path("children.zip");
        write_zip(&path, &sample_files()).expect("write zip");
        let (_, entries) = list_entries(&path).expect("list");
        let root: Vec<_> = children_of(&entries, "")
            .into_iter()
            .map(|entry| entry.path.as_str())
            .collect();
        assert!(root.contains(&"readme.txt"));
        assert!(root.contains(&"docs"));
        assert!(!root.contains(&"docs/notes.txt"));
        let docs: Vec<_> = children_of(&entries, "docs")
            .into_iter()
            .map(|entry| entry.path.as_str())
            .collect();
        assert_eq!(docs, vec!["docs/notes.txt"]);
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn smart_extract_uses_subfolder_when_multiple_roots() {
        let entries = vec![
            ArchiveEntry {
                path: "a.txt".to_string(),
                name: "a.txt".to_string(),
                is_dir: false,
                size: 1,
                compressed_size: Some(1),
                crc: None,
                modified: String::new(),
            },
            ArchiveEntry {
                path: "b.txt".to_string(),
                name: "b.txt".to_string(),
                is_dir: false,
                size: 1,
                compressed_size: Some(1),
                crc: None,
                modified: String::new(),
            },
        ];
        let dest = PathBuf::from("out");
        assert_eq!(
            smart_extract_destination(&dest, "bundle", &entries),
            PathBuf::from("out").join("bundle")
        );
    }

    #[test]
    fn test_and_create_round_trip() {
        let folder = temp_path("src-folder");
        fs::create_dir_all(folder.join("docs")).expect("dir");
        fs::write(folder.join("readme.txt"), b"hello\n").expect("write");
        fs::write(folder.join("docs").join("notes.txt"), b"notes\n").expect("write");
        let zip_path = temp_path("created.zip");
        create_archive_from_folder(&folder, &zip_path, ArchiveKind::Zip).expect("create");
        let report = test_archive(&zip_path).expect("test");
        assert_eq!(report.failed, 0);
        assert_eq!(report.files, 2);
        let (_, entries) = list_entries(&zip_path).expect("list");
        assert!(entries.iter().any(|entry| entry.path == "docs/notes.txt"));
        let _ = fs::remove_file(&zip_path);
        let _ = fs::remove_dir_all(&folder);
    }

    #[test]
    fn lists_gzip_from_trailer_without_decompressing_payload() {
        let path = temp_path("payload.bin.gz");
        let payload = vec![7_u8; 256 * 1024];
        {
            let file = File::create(&path).expect("create");
            let mut encoder = GzEncoder::new(file, Compression::default());
            encoder.write_all(&payload).expect("write");
            encoder.finish().expect("finish");
        }
        let started = std::time::Instant::now();
        let (kind, entries) = list_entries(&path).expect("list gzip");
        assert!(started.elapsed().as_millis() < 500, "{:?}", started.elapsed());
        assert_eq!(kind, ArchiveKind::Gzip);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].size, payload.len() as u64);
        let dest = temp_path("gzip-out");
        extract_entries(&path, &dest, &[entries[0].path.clone()]).expect("extract");
        let body = fs::read(dest.join("payload.bin")).expect("read");
        assert_eq!(body, payload);
        let _ = fs::remove_file(&path);
        let _ = fs::remove_dir_all(&dest);
    }

    #[test]
    fn lists_and_extracts_sevenz() {
        let path = temp_path("sample.7z");
        write_sevenz(
            &path,
            &[
                ("readme.txt".to_string(), b"sevenz hello\n".to_vec()),
                ("docs/notes.txt".to_string(), b"nested\n".to_vec()),
            ],
        )
        .expect("write 7z");
        let (kind, entries) = list_entries(&path).expect("list 7z");
        assert_eq!(kind, ArchiveKind::SevenZ);
        assert!(entries.iter().any(|entry| entry.path == "readme.txt"));
        let dest = temp_path("sevenz-out");
        let report = extract_entries(&path, &dest, &["readme.txt".to_string()]).expect("extract");
        assert_eq!(report.extracted, 1);
        let body = fs::read_to_string(dest.join("readme.txt")).expect("readme");
        assert!(body.contains("sevenz hello"));
        let _ = fs::remove_file(&path);
        let _ = fs::remove_dir_all(&dest);
    }
}
