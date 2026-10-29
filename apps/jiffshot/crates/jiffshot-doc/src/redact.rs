//! Smart Redact: finds sensitive text (email addresses, phone numbers,
//! payment card numbers, IP addresses, web addresses, long ID numbers) in
//! recognized text and returns the areas to cover.

use crate::geom::IRect;
use crate::smart::TextLine;

/// What a match looked like, for the status line.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
    Email,
    Phone,
    Card,
    Ip,
    Url,
    Number,
}

impl Kind {
    pub fn name(self) -> &'static str {
        match self {
            Kind::Email => "email address",
            Kind::Phone => "phone number",
            Kind::Card => "card number",
            Kind::Ip => "IP address",
            Kind::Url => "web address",
            Kind::Number => "ID number",
        }
    }
}

/// Luhn checksum, so random digit runs are not taken for card numbers.
fn luhn(digits: &[u8]) -> bool {
    let mut sum = 0u32;
    for (i, d) in digits.iter().rev().enumerate() {
        let mut v = *d as u32;
        if i % 2 == 1 {
            v *= 2;
            if v > 9 {
                v -= 9;
            }
        }
        sum += v;
    }
    sum % 10 == 0
}

fn is_email(w: &str) -> bool {
    let w = w.trim_matches(|c: char| !c.is_alphanumeric());
    let Some((user, host)) = w.split_once('@') else { return false };
    !user.is_empty() && host.contains('.') && !host.starts_with('.') && !host.ends_with('.') && host.chars().all(|c| c.is_alphanumeric() || c == '.' || c == '-')
}

fn is_url(w: &str) -> bool {
    let l = w.to_ascii_lowercase();
    l.starts_with("http://") || l.starts_with("https://") || l.starts_with("www.")
}

fn is_ip(w: &str) -> bool {
    let w = w.trim_matches(|c: char| !c.is_ascii_digit());
    let parts: Vec<&str> = w.split('.').collect();
    parts.len() == 4 && parts.iter().all(|p| !p.is_empty() && p.len() <= 3 && p.parse::<u16>().is_ok_and(|v| v <= 255))
}

/// Characters that may sit inside a phone or card number.
fn joins_number(c: char) -> bool {
    c.is_ascii_digit() || matches!(c, ' ' | '-' | '.' | '(' | ')' | '+' | '/')
}

/// Spans (byte ranges) of `text` holding something sensitive.
pub fn find(text: &str) -> Vec<(usize, usize, Kind)> {
    let mut found = Vec::new();
    // Word-shaped things.
    let mut start = None;
    for (i, c) in text.char_indices().chain(std::iter::once((text.len(), ' '))) {
        match (c.is_whitespace(), start) {
            (false, None) => start = Some(i),
            (true, Some(s)) => {
                let w = &text[s..i];
                let kind = if is_email(w) {
                    Some(Kind::Email)
                } else if is_url(w) {
                    Some(Kind::Url)
                } else if is_ip(w) {
                    Some(Kind::Ip)
                } else {
                    None
                };
                if let Some(k) = kind {
                    found.push((s, i, k));
                }
                start = None;
            }
            _ => {}
        }
    }
    // Digit runs, allowing separators between groups.
    let bytes: Vec<(usize, char)> = text.char_indices().collect();
    let mut i = 0;
    while i < bytes.len() {
        let (s, c) = bytes[i];
        if !(c.is_ascii_digit() || c == '+' || c == '(') {
            i += 1;
            continue;
        }
        let mut j = i;
        while j < bytes.len() && joins_number(bytes[j].1) {
            j += 1;
        }
        // Trim trailing separators.
        while j > i && !bytes[j - 1].1.is_ascii_digit() && bytes[j - 1].1 != ')' {
            j -= 1;
        }
        let e = bytes.get(j).map_or(text.len(), |b| b.0);
        let run = &text[s..e];
        let digits: Vec<u8> = run.chars().filter(|c| c.is_ascii_digit()).map(|c| c as u8 - b'0').collect();
        let overlaps = found.iter().any(|&(a, b, _)| s < b && a < e);
        if !overlaps {
            let kind = if (13..=19).contains(&digits.len()) && luhn(&digits) {
                Some(Kind::Card)
            } else if (10..=15).contains(&digits.len()) && run.chars().any(|c| !c.is_ascii_digit()) || digits.len() >= 10 && run.starts_with('+') {
                Some(Kind::Phone)
            } else if (7..=9).contains(&digits.len()) && run.contains('-') && digits.len() >= 7 {
                Some(Kind::Phone)
            } else if digits.len() >= 9 && run.chars().all(|c| c.is_ascii_digit() || c == '-') {
                Some(Kind::Number)
            } else {
                None
            };
            if let Some(k) = kind {
                found.push((s, e, k));
            }
        }
        i = j.max(i + 1);
    }
    found.sort_by_key(|f| f.0);
    found
}

/// Areas of `lines` to cover, with what they are. Matches map to the words
/// they touch; lines without word boxes are covered proportionally.
pub fn areas(lines: &[TextLine]) -> Vec<(IRect, Kind)> {
    let mut out = Vec::new();
    for line in lines {
        for (s, e, kind) in find(&line.text) {
            if line.words.is_empty() {
                let n = line.text.chars().count().max(1) as f32;
                let cs = line.text[..s].chars().count() as f32;
                let ce = line.text[..e].chars().count() as f32;
                let a = line.area;
                let x0 = a.x + (a.w as f32 * cs / n) as i32;
                let x1 = a.x + (a.w as f32 * ce / n).ceil() as i32;
                out.push((IRect::new(x0, a.y, x1 - x0, a.h), kind));
                continue;
            }
            // Locate each word in the line text to know which ones the match
            // covers.
            let mut cursor = 0;
            let mut cover: Option<IRect> = None;
            for (rect, word) in &line.words {
                let Some(found) = line.text[cursor..].find(word.as_str()) else { continue };
                let ws = cursor + found;
                let we = ws + word.len();
                cursor = we;
                if ws < e && s < we {
                    cover = Some(cover.map_or(*rect, |c| c.union(rect)));
                }
            }
            if let Some(c) = cover {
                out.push((c, kind));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(text: &str) -> Vec<(String, Kind)> {
        find(text).into_iter().map(|(s, e, k)| (text[s..e].to_owned(), k)).collect()
    }

    #[test]
    fn finds_sensitive_text() {
        assert_eq!(kinds("Write to jane.doe@example.com today"), [("jane.doe@example.com".into(), Kind::Email)]);
        assert_eq!(kinds("Call (555) 123-4567 now"), [("(555) 123-4567".into(), Kind::Phone)]);
        assert_eq!(kinds("Card 4111 1111 1111 1111 exp 12/29"), [("4111 1111 1111 1111".into(), Kind::Card)]);
        assert!(kinds("Order 4111 1111 1111 1112").iter().all(|(_, k)| *k != Kind::Card), "fails Luhn");
        assert_eq!(kinds("Server 10.0.12.254 is up"), [("10.0.12.254".into(), Kind::Ip)]);
        assert_eq!(kinds("See https://example.com/a"), [("https://example.com/a".into(), Kind::Url)]);
        assert!(kinds("Version 2025.4 released in 2026").is_empty());
        assert_eq!(kinds("+1 514 555 0199"), [("+1 514 555 0199".into(), Kind::Phone)]);
    }

    #[test]
    fn maps_matches_to_word_boxes() {
        let line = TextLine {
            area: IRect::new(0, 0, 300, 20),
            text: "Mail bob@x.io or call 555-123-4567".into(),
            words: vec![
                (IRect::new(0, 0, 40, 20), "Mail".into()),
                (IRect::new(45, 0, 80, 20), "bob@x.io".into()),
                (IRect::new(130, 0, 20, 20), "or".into()),
                (IRect::new(155, 0, 40, 20), "call".into()),
                (IRect::new(200, 0, 100, 20), "555-123-4567".into()),
            ],
        };
        let a = areas(&[line]);
        assert_eq!(a, vec![(IRect::new(45, 0, 80, 20), Kind::Email), (IRect::new(200, 0, 100, 20), Kind::Phone)]);
    }
}
