//! Straight-alpha sRGB colors, written as `#AARRGGBB` (the order Avalonia
//! uses).

use serde::{Deserialize, Serialize};
use tiny_skia::{Color, PremultipliedColorU8};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(into = "String", try_from = "String")]
pub struct Rgba {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Rgba {
    pub const TRANSPARENT: Rgba = Rgba::new(0, 0, 0, 0);
    pub const BLACK: Rgba = Rgba::rgb(0, 0, 0);
    pub const WHITE: Rgba = Rgba::rgb(255, 255, 255);
    pub const RED: Rgba = Rgba::rgb(0xE5, 0x39, 0x35);
    pub const YELLOW: Rgba = Rgba::rgb(0xFF, 0xEB, 0x3B);
    pub const BLUE: Rgba = Rgba::rgb(0x1E, 0x6F, 0xD9);

    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Rgba {
        Rgba { r, g, b, a }
    }

    pub const fn rgb(r: u8, g: u8, b: u8) -> Rgba {
        Rgba { r, g, b, a: 255 }
    }

    pub fn with_alpha(self, a: u8) -> Rgba {
        Rgba { a, ..self }
    }

    pub fn is_visible(self) -> bool {
        self.a > 0
    }

    pub fn to_color(self) -> Color {
        Color::from_rgba8(self.r, self.g, self.b, self.a)
    }

    /// The color with its alpha scaled by `opacity` (0..=1).
    pub fn faded(self, opacity: f32) -> Rgba {
        self.with_alpha((self.a as f32 * opacity.clamp(0.0, 1.0)).round() as u8)
    }

    pub fn premultiplied(self) -> PremultipliedColorU8 {
        let m = |c: u8| ((c as u16 * self.a as u16 + 127) / 255) as u8;
        PremultipliedColorU8::from_rgba(m(self.r), m(self.g), m(self.b), self.a).expect("channels never exceed alpha")
    }

    pub fn from_premultiplied(p: PremultipliedColorU8) -> Rgba {
        let c = p.demultiply();
        Rgba::new(c.red(), c.green(), c.blue(), c.alpha())
    }

    /// Relative luminance (0..=1), for picking readable text colors.
    pub fn luminance(self) -> f32 {
        (0.2126 * self.r as f32 + 0.7152 * self.g as f32 + 0.0722 * self.b as f32) / 255.0
    }

    /// Black or white, whichever reads better on this color.
    pub fn contrasting(self) -> Rgba {
        if self.luminance() > 0.6 {
            Rgba::BLACK
        } else {
            Rgba::WHITE
        }
    }

    /// Parses `#RGB`, `#RRGGBB` or `#AARRGGBB` (the `#` is optional).
    pub fn parse(text: &str) -> Option<Rgba> {
        let hex = text.trim().trim_start_matches('#');
        let digits: Vec<u8> = hex
            .chars()
            .map(|c| c.to_digit(16).map(|d| d as u8))
            .collect::<Option<_>>()?;
        let byte = |i: usize| digits[i] * 16 + digits[i + 1];
        match digits.len() {
            3 => Some(Rgba::rgb(digits[0] * 17, digits[1] * 17, digits[2] * 17)),
            6 => Some(Rgba::rgb(byte(0), byte(2), byte(4))),
            8 => Some(Rgba::new(byte(2), byte(4), byte(6), byte(0))),
            _ => None,
        }
    }

    pub fn to_hex(self) -> String {
        format!("#{:02X}{:02X}{:02X}{:02X}", self.a, self.r, self.g, self.b)
    }

    pub fn lerp(self, other: Rgba, t: f32) -> Rgba {
        let mix = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * t).round().clamp(0.0, 255.0) as u8;
        Rgba::new(mix(self.r, other.r), mix(self.g, other.g), mix(self.b, other.b), mix(self.a, other.a))
    }
}

impl From<Rgba> for String {
    fn from(value: Rgba) -> String {
        value.to_hex()
    }
}

impl TryFrom<String> for Rgba {
    type Error = String;

    fn try_from(value: String) -> Result<Rgba, String> {
        Rgba::parse(&value).ok_or_else(|| format!("invalid color {value:?}"))
    }
}

impl Default for Rgba {
    fn default() -> Rgba {
        Rgba::TRANSPARENT
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_formats() {
        assert_eq!(Rgba::parse("#FF0080"), Some(Rgba::rgb(255, 0, 128)));
        assert_eq!(Rgba::parse("80FF0000"), Some(Rgba::new(255, 0, 0, 128)));
        assert_eq!(Rgba::parse("#fff"), Some(Rgba::WHITE));
        assert_eq!(Rgba::parse("#12345"), None);
        assert_eq!(Rgba::new(1, 2, 3, 4).to_hex(), "#04010203");
        let json = serde_json::to_string(&Rgba::RED).unwrap();
        assert_eq!(json, "\"#FFE53935\"");
        assert_eq!(serde_json::from_str::<Rgba>(&json).unwrap(), Rgba::RED);
    }
}
