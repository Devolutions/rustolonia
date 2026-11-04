//! Minimal editor for Android binary XML (`AndroidManifest.xml` inside an APK).
//!
//! The file is a `RES_XML_TYPE` chunk holding a string pool, a resource map
//! and a flat list of XML node chunks. Edits keep every chunk except the
//! string pool byte-for-byte: strings are replaced in place (indices stay
//! stable) or appended, and attribute typed values are patched where they sit.

use std::fmt;

const RES_XML_TYPE: u16 = 0x0003;
const RES_STRING_POOL_TYPE: u16 = 0x0001;
const RES_XML_START_ELEMENT_TYPE: u16 = 0x0102;
const UTF8_FLAG: u32 = 0x100;

const TYPE_STRING: u8 = 0x03;
const TYPE_INT_DEC: u8 = 0x10;
const NO_INDEX: u32 = 0xFFFF_FFFF;

#[derive(Debug, PartialEq, Eq)]
pub struct AxmlError(pub String);

impl fmt::Display for AxmlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "binary manifest: {}", self.0)
    }
}

impl std::error::Error for AxmlError {}

type Result<T> = std::result::Result<T, AxmlError>;

fn err<T>(message: impl Into<String>) -> Result<T> {
    Err(AxmlError(message.into()))
}

fn u16_at(data: &[u8], offset: usize) -> Result<u16> {
    data.get(offset..offset + 2)
        .map(|b| u16::from_le_bytes([b[0], b[1]]))
        .ok_or_else(|| AxmlError(format!("truncated at {offset}")))
}

fn u32_at(data: &[u8], offset: usize) -> Result<u32> {
    data.get(offset..offset + 4)
        .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        .ok_or_else(|| AxmlError(format!("truncated at {offset}")))
}

/// A parsed binary XML document.
#[derive(Clone, Debug)]
pub struct Axml {
    strings: Vec<String>,
    utf8: bool,
    pool_flags: u32,
    /// Every chunk after the string pool (resource map, nodes), verbatim.
    chunks: Vec<u8>,
}

/// One attribute of a start element, located inside `Axml::chunks`.
#[derive(Clone, Copy, Debug)]
struct Attribute {
    /// Offset of the 20-byte `ResXMLTree_attribute` within `chunks`.
    offset: usize,
    name: u32,
}

#[derive(Clone, Debug)]
struct Element {
    name: u32,
    attributes: Vec<Attribute>,
}

impl Axml {
    pub fn parse(data: &[u8]) -> Result<Self> {
        if u16_at(data, 0)? != RES_XML_TYPE {
            return err("not an Android binary XML document");
        }
        let header_size = u16_at(data, 2)? as usize;
        let total = u32_at(data, 4)? as usize;
        if total > data.len() {
            return err("document size exceeds the file");
        }
        let pool_start = header_size;
        if u16_at(data, pool_start)? != RES_STRING_POOL_TYPE {
            return err("the first chunk is not a string pool");
        }
        let pool_header = u16_at(data, pool_start + 2)? as usize;
        let pool_size = u32_at(data, pool_start + 4)? as usize;
        let count = u32_at(data, pool_start + 8)? as usize;
        let style_count = u32_at(data, pool_start + 12)?;
        let flags = u32_at(data, pool_start + 16)?;
        let strings_start = u32_at(data, pool_start + 20)? as usize;
        if style_count != 0 {
            return err("styled string pools are not supported");
        }
        let utf8 = flags & UTF8_FLAG != 0;
        let mut strings = Vec::with_capacity(count);
        for index in 0..count {
            let offset = u32_at(data, pool_start + pool_header + index * 4)? as usize;
            let at = pool_start + strings_start + offset;
            strings.push(if utf8 {
                read_utf8(data, at)?
            } else {
                read_utf16(data, at)?
            });
        }
        Ok(Self {
            strings,
            utf8,
            pool_flags: flags,
            chunks: data[pool_start + pool_size..total].to_vec(),
        })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let pool = self.string_pool();
        let mut out = Vec::with_capacity(8 + pool.len() + self.chunks.len());
        out.extend_from_slice(&RES_XML_TYPE.to_le_bytes());
        out.extend_from_slice(&8u16.to_le_bytes());
        out.extend_from_slice(&((8 + pool.len() + self.chunks.len()) as u32).to_le_bytes());
        out.extend_from_slice(&pool);
        out.extend_from_slice(&self.chunks);
        out
    }

    #[cfg(test)]
    pub fn strings(&self) -> &[String] {
        &self.strings
    }

    /// Replaces `from` by `to` in every string equal to `from` or starting
    /// with `from.` — the package name and everything derived from it
    /// (permissions, provider authorities, package-relative class names).
    /// Returns how many strings changed.
    pub fn rename_prefix(&mut self, from: &str, to: &str) -> usize {
        let dotted = format!("{from}.");
        let mut changed = 0;
        for value in &mut self.strings {
            if value == from {
                *value = to.to_owned();
                changed += 1;
            } else if let Some(rest) = value.strip_prefix(&dotted) {
                *value = format!("{to}.{rest}");
                changed += 1;
            }
        }
        changed
    }

    /// Sets attribute `attribute` of every `element` to the string `value`.
    /// Returns how many attributes were set.
    pub fn set_string(&mut self, element: &str, attribute: &str, value: &str) -> Result<usize> {
        let index = self.intern(value);
        self.set_typed(element, attribute, None, index, TYPE_STRING, index)
    }

    /// Sets attribute `attribute` of every `element` to a decimal integer.
    pub fn set_int(&mut self, element: &str, attribute: &str, value: u32) -> Result<usize> {
        self.set_typed(element, attribute, None, NO_INDEX, TYPE_INT_DEC, value)
    }

    /// Sets `attribute` to the string `value` on the `element`s whose
    /// `key_attribute` is the string `key` (for example the `value` of a
    /// `<meta-data android:name="...">`).
    pub fn set_string_where(
        &mut self,
        element: &str,
        key_attribute: &str,
        key: &str,
        attribute: &str,
        value: &str,
    ) -> Result<usize> {
        let index = self.intern(value);
        self.set_typed(
            element,
            attribute,
            Some((key_attribute, key)),
            index,
            TYPE_STRING,
            index,
        )
    }

    /// The string value of `attribute` on the first `element`, if it is a string.
    pub fn get_string(&self, element: &str, attribute: &str) -> Result<Option<String>> {
        for found in self.elements()? {
            if self.name(found.name) != Some(element) {
                continue;
            }
            for attr in &found.attributes {
                if self.name(attr.name) == Some(attribute) {
                    return Ok(self.string_value(attr));
                }
            }
        }
        Ok(None)
    }

    fn set_typed(
        &mut self,
        element: &str,
        attribute: &str,
        filter: Option<(&str, &str)>,
        raw: u32,
        data_type: u8,
        data: u32,
    ) -> Result<usize> {
        let mut targets = Vec::new();
        for found in self.elements()? {
            if self.name(found.name) != Some(element) {
                continue;
            }
            if let Some((key_attribute, key)) = filter {
                let matches = found.attributes.iter().any(|attr| {
                    self.name(attr.name) == Some(key_attribute)
                        && self.string_value(attr).as_deref() == Some(key)
                });
                if !matches {
                    continue;
                }
            }
            targets.extend(
                found
                    .attributes
                    .iter()
                    .filter(|attr| self.name(attr.name) == Some(attribute))
                    .map(|attr| attr.offset),
            );
        }
        for offset in &targets {
            let attr = &mut self.chunks[*offset..*offset + 20];
            attr[8..12].copy_from_slice(&raw.to_le_bytes());
            attr[12..14].copy_from_slice(&8u16.to_le_bytes());
            attr[14] = 0;
            attr[15] = data_type;
            attr[16..20].copy_from_slice(&data.to_le_bytes());
        }
        Ok(targets.len())
    }

    fn string_value(&self, attr: &Attribute) -> Option<String> {
        let at = attr.offset;
        if self.chunks[at + 15] != TYPE_STRING {
            return None;
        }
        let index = u32::from_le_bytes(self.chunks[at + 16..at + 20].try_into().ok()?);
        self.strings.get(index as usize).cloned()
    }

    fn name(&self, index: u32) -> Option<&str> {
        self.strings.get(index as usize).map(String::as_str)
    }

    fn intern(&mut self, value: &str) -> u32 {
        // Always append: an existing equal string may be shared with an
        // attribute that must keep its value.
        self.strings.push(value.to_owned());
        (self.strings.len() - 1) as u32
    }

    fn elements(&self) -> Result<Vec<Element>> {
        let mut elements = Vec::new();
        let mut at = 0;
        while at + 8 <= self.chunks.len() {
            let kind = u16_at(&self.chunks, at)?;
            let header = u16_at(&self.chunks, at + 2)? as usize;
            let size = u32_at(&self.chunks, at + 4)? as usize;
            if size < 8 || at + size > self.chunks.len() {
                return err(format!("malformed chunk at {at}"));
            }
            if kind == RES_XML_START_ELEMENT_TYPE {
                let ext = at + header;
                let name = u32_at(&self.chunks, ext + 4)?;
                let start = u16_at(&self.chunks, ext + 8)? as usize;
                let stride = u16_at(&self.chunks, ext + 10)? as usize;
                let count = u16_at(&self.chunks, ext + 12)? as usize;
                if stride < 20 || ext + start + stride * count > at + size {
                    return err(format!("malformed element at {at}"));
                }
                let attributes = (0..count)
                    .map(|i| {
                        let offset = ext + start + i * stride;
                        Ok(Attribute {
                            offset,
                            name: u32_at(&self.chunks, offset + 4)?,
                        })
                    })
                    .collect::<Result<_>>()?;
                elements.push(Element { name, attributes });
            }
            at += size;
        }
        Ok(elements)
    }

    fn string_pool(&self) -> Vec<u8> {
        let mut offsets = Vec::with_capacity(self.strings.len());
        let mut data = Vec::new();
        for value in &self.strings {
            offsets.push(data.len() as u32);
            if self.utf8 {
                write_utf8(&mut data, value);
            } else {
                write_utf16(&mut data, value);
            }
        }
        while data.len() % 4 != 0 {
            data.push(0);
        }
        let header = 28usize;
        let strings_start = header + 4 * offsets.len();
        let size = strings_start + data.len();
        let mut out = Vec::with_capacity(size);
        out.extend_from_slice(&RES_STRING_POOL_TYPE.to_le_bytes());
        out.extend_from_slice(&(header as u16).to_le_bytes());
        out.extend_from_slice(&(size as u32).to_le_bytes());
        out.extend_from_slice(&(offsets.len() as u32).to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        // Appended strings break any sort order.
        out.extend_from_slice(&(self.pool_flags & UTF8_FLAG).to_le_bytes());
        out.extend_from_slice(&(strings_start as u32).to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        for offset in offsets {
            out.extend_from_slice(&offset.to_le_bytes());
        }
        out.extend_from_slice(&data);
        out
    }
}

fn read_utf16(data: &[u8], at: usize) -> Result<String> {
    let first = u16_at(data, at)? as usize;
    let (len, start) = if first & 0x8000 != 0 {
        (
            ((first & 0x7fff) << 16) | u16_at(data, at + 2)? as usize,
            at + 4,
        )
    } else {
        (first, at + 2)
    };
    let units = (0..len)
        .map(|i| u16_at(data, start + i * 2))
        .collect::<Result<Vec<_>>>()?;
    String::from_utf16(&units).map_err(|_| AxmlError(format!("invalid UTF-16 string at {at}")))
}

fn read_utf8(data: &[u8], at: usize) -> Result<String> {
    let mut at = at;
    let mut length = || -> Result<usize> {
        let b0 = *data
            .get(at)
            .ok_or_else(|| AxmlError("truncated string".into()))? as usize;
        if b0 & 0x80 != 0 {
            let b1 = *data
                .get(at + 1)
                .ok_or_else(|| AxmlError("truncated string".into()))?;
            at += 2;
            Ok(((b0 & 0x7f) << 8) | b1 as usize)
        } else {
            at += 1;
            Ok(b0)
        }
    };
    let _utf16_len = length()?;
    let byte_len = length()?;
    let bytes = data
        .get(at..at + byte_len)
        .ok_or_else(|| AxmlError("truncated string".into()))?;
    String::from_utf8(bytes.to_vec()).map_err(|_| AxmlError("invalid UTF-8 string".into()))
}

fn write_utf16(out: &mut Vec<u8>, value: &str) {
    let units: Vec<u16> = value.encode_utf16().collect();
    if units.len() > 0x7fff {
        out.extend_from_slice(&((0x8000 | (units.len() >> 16)) as u16).to_le_bytes());
    }
    out.extend_from_slice(&(units.len() as u16).to_le_bytes());
    for unit in units {
        out.extend_from_slice(&unit.to_le_bytes());
    }
    out.extend_from_slice(&0u16.to_le_bytes());
}

fn write_utf8(out: &mut Vec<u8>, value: &str) {
    let mut length = |n: usize| {
        if n > 0x7f {
            out.push((0x80 | (n >> 8)) as u8);
        }
        out.push(n as u8);
    };
    length(value.encode_utf16().count());
    length(value.len());
    out.extend_from_slice(value.as_bytes());
    out.push(0);
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// A tiny binary manifest:
    /// `<manifest package=PKG versionCode=1><application label="Host">
    ///   <meta-data name="lib.key" value="placeholder"/></application></manifest>`
    pub(crate) fn sample(utf8: bool) -> Vec<u8> {
        let strings = [
            "package",                     // 0
            "versionCode",                 // 1
            "label",                       // 2
            "name",                        // 3
            "value",                       // 4
            "manifest",                    // 5
            "application",                 // 6
            "meta-data",                   // 7
            "net.example.host",            // 8
            "Host",                        // 9
            "lib.key",                     // 10
            "placeholder",                 // 11
            "net.example.host.PERMISSION", // 12
            "net.example.hostile.Other",   // 13
        ];
        let doc = Axml {
            strings: strings.iter().map(|s| s.to_string()).collect(),
            utf8,
            pool_flags: if utf8 { UTF8_FLAG } else { 0 },
            chunks: Vec::new(),
        };
        let mut chunks = Vec::new();
        let mut element = |name: u32, attrs: &[(u32, u8, u32)]| {
            let mut chunk = Vec::new();
            let size = 16 + 20 + attrs.len() * 20;
            chunk.extend_from_slice(&RES_XML_START_ELEMENT_TYPE.to_le_bytes());
            chunk.extend_from_slice(&16u16.to_le_bytes());
            chunk.extend_from_slice(&(size as u32).to_le_bytes());
            chunk.extend_from_slice(&1u32.to_le_bytes()); // line
            chunk.extend_from_slice(&NO_INDEX.to_le_bytes()); // comment
            chunk.extend_from_slice(&NO_INDEX.to_le_bytes()); // ns
            chunk.extend_from_slice(&name.to_le_bytes());
            chunk.extend_from_slice(&20u16.to_le_bytes()); // attributeStart
            chunk.extend_from_slice(&20u16.to_le_bytes()); // attributeSize
            chunk.extend_from_slice(&(attrs.len() as u16).to_le_bytes());
            chunk.extend_from_slice(&[0; 6]);
            for &(attr, kind, data) in attrs {
                chunk.extend_from_slice(&NO_INDEX.to_le_bytes());
                chunk.extend_from_slice(&attr.to_le_bytes());
                let raw = if kind == TYPE_STRING { data } else { NO_INDEX };
                chunk.extend_from_slice(&raw.to_le_bytes());
                chunk.extend_from_slice(&8u16.to_le_bytes());
                chunk.push(0);
                chunk.push(kind);
                chunk.extend_from_slice(&data.to_le_bytes());
            }
            chunks.extend_from_slice(&chunk);
        };
        element(5, &[(0, TYPE_STRING, 8), (1, TYPE_INT_DEC, 1)]);
        element(6, &[(2, TYPE_STRING, 9)]);
        element(7, &[(3, TYPE_STRING, 10), (4, TYPE_STRING, 11)]);
        Axml { chunks, ..doc }.to_bytes()
    }

    #[test]
    fn round_trips_both_encodings() {
        for utf8 in [false, true] {
            let bytes = sample(utf8);
            let parsed = Axml::parse(&bytes).unwrap();
            assert_eq!(parsed.strings()[8], "net.example.host");
            assert_eq!(parsed.to_bytes(), bytes, "utf8={utf8}");
        }
    }

    #[test]
    fn edits_survive_a_reparse() {
        for utf8 in [false, true] {
            let mut doc = Axml::parse(&sample(utf8)).unwrap();
            assert_eq!(doc.rename_prefix("net.example.host", "com.acme.app"), 2);
            assert_eq!(doc.set_int("manifest", "versionCode", 42).unwrap(), 1);
            assert_eq!(doc.set_string("application", "label", "Acme ✓").unwrap(), 1);
            assert_eq!(
                doc.set_string_where("meta-data", "name", "lib.key", "value", "acme")
                    .unwrap(),
                1
            );
            assert_eq!(
                doc.set_string_where("meta-data", "name", "other", "value", "x")
                    .unwrap(),
                0
            );

            let doc = Axml::parse(&doc.to_bytes()).unwrap();
            let get = |e, a| doc.get_string(e, a).unwrap();
            assert_eq!(get("manifest", "package").as_deref(), Some("com.acme.app"));
            assert_eq!(get("application", "label").as_deref(), Some("Acme ✓"));
            assert_eq!(get("meta-data", "value").as_deref(), Some("acme"));
            assert!(doc
                .strings()
                .contains(&"com.acme.app.PERMISSION".to_owned()));
            // Only whole dotted prefixes are renamed.
            assert!(doc
                .strings()
                .contains(&"net.example.hostile.Other".to_owned()));
        }
    }

    #[test]
    fn rejects_other_documents() {
        assert!(Axml::parse(b"PK\x03\x04").is_err());
        assert!(Axml::parse(&[]).is_err());
    }
}
