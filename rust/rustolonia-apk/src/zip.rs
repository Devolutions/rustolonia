//! Just enough ZIP to rebuild an APK: read entries as raw (still compressed)
//! data, and write a fresh archive with Android's alignment rules. No ZIP64,
//! no encryption, no data descriptors on output.

use std::fmt;
use std::io::Write;

#[derive(Debug)]
pub struct ZipError(pub String);

impl fmt::Display for ZipError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "zip: {}", self.0)
    }
}

impl std::error::Error for ZipError {}

type Result<T> = std::result::Result<T, ZipError>;

pub const STORED: u16 = 0;
pub const DEFLATED: u16 = 8;

const LOCAL_SIGNATURE: u32 = 0x0403_4b50;
const CENTRAL_SIGNATURE: u32 = 0x0201_4b50;
const EOCD_SIGNATURE: u32 = 0x0605_4b50;
/// zipalign's extra field: u16 alignment followed by zero padding.
const ALIGNMENT_EXTRA_ID: u16 = 0xd935;
const PAGE: usize = 16384;

/// One archive member, holding its data exactly as stored.
#[derive(Clone, Debug)]
pub struct Entry {
    pub name: String,
    pub method: u16,
    pub crc32: u32,
    pub uncompressed_size: u32,
    pub data: Vec<u8>,
    pub mod_time: u16,
    pub mod_date: u16,
    pub external_attributes: u32,
}

impl Entry {
    /// A new member, deflated unless `store`.
    pub fn new(name: impl Into<String>, contents: &[u8], store: bool) -> Self {
        let mut crc = flate2::Crc::new();
        crc.update(contents);
        let (method, data) = if store {
            (STORED, contents.to_vec())
        } else {
            let mut encoder =
                flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::best());
            encoder.write_all(contents).expect("in-memory deflate");
            (DEFLATED, encoder.finish().expect("in-memory deflate"))
        };
        Self {
            name: name.into(),
            method,
            crc32: crc.sum(),
            uncompressed_size: contents.len() as u32,
            data,
            // 1980-01-01 00:00, as build tools use for reproducible output.
            mod_time: 0,
            mod_date: 0x21,
            external_attributes: 0,
        }
    }

    /// The uncompressed contents.
    pub fn contents(&self) -> Result<Vec<u8>> {
        match self.method {
            STORED => Ok(self.data.clone()),
            DEFLATED => {
                let mut out = Vec::with_capacity(self.uncompressed_size as usize);
                let mut decoder = flate2::write::DeflateDecoder::new(&mut out);
                decoder
                    .write_all(&self.data)
                    .and_then(|_| decoder.finish().map(|_| ()))
                    .map_err(|e| ZipError(format!("{}: {e}", self.name)))?;
                Ok(out)
            }
            other => Err(ZipError(format!(
                "{}: unsupported method {other}",
                self.name
            ))),
        }
    }

    /// Android alignment for stored data: shared libraries on a page
    /// boundary (16 KiB, valid for 4 KiB-page devices too), the rest on 4.
    fn alignment(&self) -> usize {
        match (self.method, self.name.ends_with(".so")) {
            (STORED, true) => PAGE,
            (STORED, false) => 4,
            _ => 1,
        }
    }
}

fn u16_at(data: &[u8], at: usize) -> Result<u16> {
    data.get(at..at + 2)
        .map(|b| u16::from_le_bytes([b[0], b[1]]))
        .ok_or_else(|| ZipError(format!("truncated at {at}")))
}

fn u32_at(data: &[u8], at: usize) -> Result<u32> {
    data.get(at..at + 4)
        .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        .ok_or_else(|| ZipError(format!("truncated at {at}")))
}

/// Offsets of the central directory and the end-of-central-directory record.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Layout {
    pub central_directory: usize,
    pub eocd: usize,
}

pub fn layout(data: &[u8]) -> Result<Layout> {
    let min = data.len().saturating_sub(22 + 0xffff);
    let eocd = (min..=data.len().saturating_sub(22))
        .rev()
        .find(|&at| u32_at(data, at).ok() == Some(EOCD_SIGNATURE))
        .ok_or_else(|| ZipError("no end of central directory".into()))?;
    let central_directory = u32_at(data, eocd + 16)? as usize;
    if central_directory > eocd {
        return Err(ZipError("central directory offset is past its end".into()));
    }
    Ok(Layout {
        central_directory,
        eocd,
    })
}

pub fn read(data: &[u8]) -> Result<Vec<Entry>> {
    let layout = layout(data)?;
    let count = u16_at(data, layout.eocd + 10)? as usize;
    let mut entries = Vec::with_capacity(count);
    let mut at = layout.central_directory;
    for _ in 0..count {
        if u32_at(data, at)? != CENTRAL_SIGNATURE {
            return Err(ZipError(format!("bad central directory entry at {at}")));
        }
        let flags = u16_at(data, at + 8)?;
        if flags & 1 != 0 {
            return Err(ZipError("encrypted entries are not supported".into()));
        }
        let method = u16_at(data, at + 10)?;
        let mod_time = u16_at(data, at + 12)?;
        let mod_date = u16_at(data, at + 14)?;
        let crc32 = u32_at(data, at + 16)?;
        let compressed = u32_at(data, at + 20)? as usize;
        let uncompressed_size = u32_at(data, at + 24)?;
        let name_len = u16_at(data, at + 28)? as usize;
        let extra_len = u16_at(data, at + 30)? as usize;
        let comment_len = u16_at(data, at + 32)? as usize;
        let external_attributes = u32_at(data, at + 38)?;
        let local = u32_at(data, at + 42)? as usize;
        let name = data
            .get(at + 46..at + 46 + name_len)
            .ok_or_else(|| ZipError("truncated name".into()))?;
        let name = String::from_utf8_lossy(name).into_owned();
        if u32_at(data, local)? != LOCAL_SIGNATURE {
            return Err(ZipError(format!("{name}: bad local header")));
        }
        let start =
            local + 30 + u16_at(data, local + 26)? as usize + u16_at(data, local + 28)? as usize;
        let contents = data
            .get(start..start + compressed)
            .ok_or_else(|| ZipError(format!("{name}: truncated data")))?;
        entries.push(Entry {
            name,
            method,
            crc32,
            uncompressed_size,
            data: contents.to_vec(),
            mod_time,
            mod_date,
            external_attributes,
        });
        at += 46 + name_len + extra_len + comment_len;
    }
    Ok(entries)
}

pub fn write(entries: &[Entry]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut central = Vec::new();
    for entry in entries {
        let offset = out.len();
        let name = entry.name.as_bytes();
        let alignment = entry.alignment();
        let extra = if alignment > 1 {
            let data_start = offset + 30 + name.len() + 6;
            let padding = (alignment - data_start % alignment) % alignment;
            let mut extra = Vec::with_capacity(6 + padding);
            extra.extend_from_slice(&ALIGNMENT_EXTRA_ID.to_le_bytes());
            extra.extend_from_slice(&((2 + padding) as u16).to_le_bytes());
            extra.extend_from_slice(&(alignment as u16).to_le_bytes());
            extra.resize(6 + padding, 0);
            extra
        } else {
            Vec::new()
        };
        let header = |sink: &mut Vec<u8>, central: bool| {
            sink.extend_from_slice(
                &if central {
                    CENTRAL_SIGNATURE
                } else {
                    LOCAL_SIGNATURE
                }
                .to_le_bytes(),
            );
            if central {
                sink.extend_from_slice(&20u16.to_le_bytes()); // made by
            }
            sink.extend_from_slice(&20u16.to_le_bytes()); // needed
            sink.extend_from_slice(&0x0800u16.to_le_bytes()); // UTF-8 names
            sink.extend_from_slice(&entry.method.to_le_bytes());
            sink.extend_from_slice(&entry.mod_time.to_le_bytes());
            sink.extend_from_slice(&entry.mod_date.to_le_bytes());
            sink.extend_from_slice(&entry.crc32.to_le_bytes());
            sink.extend_from_slice(&(entry.data.len() as u32).to_le_bytes());
            sink.extend_from_slice(&entry.uncompressed_size.to_le_bytes());
            sink.extend_from_slice(&(name.len() as u16).to_le_bytes());
        };
        header(&mut out, false);
        out.extend_from_slice(&(extra.len() as u16).to_le_bytes());
        out.extend_from_slice(name);
        out.extend_from_slice(&extra);
        debug_assert_eq!(out.len() % alignment, 0);
        out.extend_from_slice(&entry.data);

        header(&mut central, true);
        central.extend_from_slice(&0u16.to_le_bytes()); // extra
        central.extend_from_slice(&0u16.to_le_bytes()); // comment
        central.extend_from_slice(&0u16.to_le_bytes()); // disk
        central.extend_from_slice(&0u16.to_le_bytes()); // internal attributes
        central.extend_from_slice(&entry.external_attributes.to_le_bytes());
        central.extend_from_slice(&(offset as u32).to_le_bytes());
        central.extend_from_slice(name);
    }
    let central_offset = out.len();
    out.extend_from_slice(&central);
    out.extend_from_slice(&EOCD_SIGNATURE.to_le_bytes());
    out.extend_from_slice(&[0; 4]); // disk numbers
    out.extend_from_slice(&(entries.len() as u16).to_le_bytes());
    out.extend_from_slice(&(entries.len() as u16).to_le_bytes());
    out.extend_from_slice(&(central.len() as u32).to_le_bytes());
    out.extend_from_slice(&(central_offset as u32).to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes()); // comment
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_and_aligns_stored_entries() {
        let entries = vec![
            Entry::new("AndroidManifest.xml", b"manifest", false),
            Entry::new("resources.arsc", b"table", true),
            Entry::new("lib/x86_64/libstored.so", &[7; 100], true),
            Entry::new("lib/x86_64/libdeflated.so", &[9; 5000], false),
        ];
        let bytes = write(&entries);
        let read_back = read(&bytes).unwrap();
        assert_eq!(read_back.len(), 4);
        for (written, read) in entries.iter().zip(&read_back) {
            assert_eq!(written.name, read.name);
            assert_eq!(written.contents().unwrap(), read.contents().unwrap());
        }
        assert!(read_back[3].data.len() < 5000, "deflated");
        let offset_of = |needle: &[u8]| {
            bytes
                .windows(needle.len())
                .position(|w| w == needle)
                .unwrap()
        };
        assert_eq!(offset_of(b"table") % 4, 0);
        assert_eq!(offset_of(&[7; 100]) % PAGE, 0);
    }
}
