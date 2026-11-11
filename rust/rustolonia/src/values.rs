//! Idiomatic Rust value types for projected string-marshalled Avalonia properties:
//! [`GridTrack`] / [`GridTracks`], [`CivilDate`], [`CivilTime`], and [`Decimal`].

use crate::{Error, Result};
use std::fmt;
use std::ops::Deref;
use std::str::FromStr;

/// Length of a single [`GridTrack`], mirroring `Avalonia.Controls.GridLength`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GridLength {
    /// Sizes to the largest child in the track (`Auto`).
    Auto,
    /// Fixed size in device-independent pixels (`120`).
    Px(f64),
    /// Weighted share of remaining space (`*`, `2*`, `0.5*`).
    Star(f64),
}

impl GridLength {
    pub const fn auto() -> Self {
        Self::Auto
    }

    pub const fn px(pixels: f64) -> Self {
        Self::Px(pixels)
    }

    pub const fn star(weight: f64) -> Self {
        Self::Star(weight)
    }
}

impl fmt::Display for GridLength {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Auto => f.write_str("Auto"),
            Self::Px(px) => write_invariant_f64(f, px),
            Self::Star(weight) if (weight - 1.0).abs() < f64::EPSILON => f.write_str("*"),
            Self::Star(weight) => {
                write_invariant_f64(f, weight)?;
                f.write_str("*")
            }
        }
    }
}

impl FromStr for GridLength {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            return Err(Error::InvalidValue(
                "grid track length cannot be empty".into(),
            ));
        }
        if trimmed.eq_ignore_ascii_case("auto") {
            return Ok(Self::Auto);
        }
        if let Some(prefix) = trimmed.strip_suffix('*') {
            let prefix = prefix.trim();
            if prefix.is_empty() {
                return Ok(Self::Star(1.0));
            }
            let weight: f64 = prefix
                .parse()
                .map_err(|_| Error::InvalidValue(format!("invalid star grid track '{trimmed}'")))?;
            if !weight.is_finite() || weight < 0.0 {
                return Err(Error::InvalidValue(format!(
                    "star grid track weight must be non-negative finite, got '{trimmed}'"
                )));
            }
            return Ok(Self::Star(weight));
        }
        let num_str = trimmed
            .strip_suffix("px")
            .or_else(|| trimmed.strip_suffix("PX"))
            .unwrap_or(trimmed)
            .trim();
        let px: f64 = num_str
            .parse()
            .map_err(|_| Error::InvalidValue(format!("invalid pixel grid track '{trimmed}'")))?;
        if !px.is_finite() || px < 0.0 {
            return Err(Error::InvalidValue(format!(
                "pixel grid track length must be non-negative finite, got '{trimmed}'"
            )));
        }
        Ok(Self::Px(px))
    }
}

/// One column or row track definition in a `Grid`, optionally participating in
/// a `SharedSizeGroup`.
#[derive(Clone, Debug, PartialEq)]
pub struct GridTrack {
    pub length: GridLength,
    pub shared_size_group: Option<String>,
}

impl GridTrack {
    pub const fn new(length: GridLength) -> Self {
        Self {
            length,
            shared_size_group: None,
        }
    }

    pub const fn auto() -> Self {
        Self::new(GridLength::Auto)
    }

    pub const fn px(pixels: f64) -> Self {
        Self::new(GridLength::Px(pixels))
    }

    pub const fn star(weight: f64) -> Self {
        Self::new(GridLength::Star(weight))
    }

    pub fn shared_group(mut self, group: impl Into<String>) -> Self {
        self.shared_size_group = Some(group.into());
        self
    }
}

impl From<GridLength> for GridTrack {
    fn from(length: GridLength) -> Self {
        Self::new(length)
    }
}

impl From<f64> for GridTrack {
    fn from(pixels: f64) -> Self {
        Self::px(pixels)
    }
}

impl fmt::Display for GridTrack {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.length)?;
        if let Some(group) = &self.shared_size_group {
            write!(f, "@{group}")?;
        }
        Ok(())
    }
}

impl FromStr for GridTrack {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        let trimmed = s.trim();
        if let Some((len_part, group_part)) = trimmed.split_once('@') {
            if group_part.contains('@') || group_part.trim().is_empty() {
                return Err(Error::InvalidValue(format!(
                    "invalid grid track shared size group in '{trimmed}'"
                )));
            }
            Ok(Self {
                length: len_part.parse()?,
                shared_size_group: Some(group_part.trim().to_owned()),
            })
        } else {
            Ok(Self {
                length: trimmed.parse()?,
                shared_size_group: None,
            })
        }
    }
}

/// Ordered list of [`GridTrack`] definitions for `Grid::column_definitions` and
/// `Grid::row_definitions`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GridTracks(pub Vec<GridTrack>);

impl GridTracks {
    pub const fn new() -> Self {
        Self(Vec::new())
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn as_slice(&self) -> &[GridTrack] {
        &self.0
    }
}

impl Deref for GridTracks {
    type Target = [GridTrack];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl fmt::Display for GridTracks {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, track) in self.0.iter().enumerate() {
            if i > 0 {
                f.write_str(",")?;
            }
            write!(f, "{track}")?;
        }
        Ok(())
    }
}

impl FromStr for GridTracks {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            return Ok(Self::new());
        }
        let tracks = trimmed
            .split([',', ' '])
            .map(str::trim)
            .filter(|part| !part.is_empty())
            .map(GridTrack::from_str)
            .collect::<Result<Vec<_>>>()?;
        Ok(Self(tracks))
    }
}

impl From<Vec<GridTrack>> for GridTracks {
    fn from(tracks: Vec<GridTrack>) -> Self {
        Self(tracks)
    }
}

impl From<&[GridTrack]> for GridTracks {
    fn from(tracks: &[GridTrack]) -> Self {
        Self(tracks.to_vec())
    }
}

impl<const N: usize> From<[GridTrack; N]> for GridTracks {
    fn from(tracks: [GridTrack; N]) -> Self {
        Self(tracks.into())
    }
}

impl<const N: usize> From<[GridLength; N]> for GridTracks {
    fn from(lengths: [GridLength; N]) -> Self {
        Self(lengths.into_iter().map(GridTrack::from).collect())
    }
}

impl From<GridTracks> for String {
    fn from(tracks: GridTracks) -> Self {
        tracks.to_string()
    }
}

/// Proleptic Gregorian civil date (`YYYY-MM-DD`) without timezone or time-of-day.
///
/// Used for `Calendar`, `CalendarDatePicker`, and `DatePicker` date properties.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CivilDate {
    year: i32,
    month: u8,
    day: u8,
}

impl CivilDate {
    /// Creates a validated civil date in `0001-01-01..=9999-12-31`.
    pub fn new(year: i32, month: u8, day: u8) -> Result<Self> {
        Self::ymd(year, month, day).ok_or_else(|| {
            Error::InvalidValue(format!(
                "invalid calendar date {year:04}-{month:02}-{day:02}"
            ))
        })
    }

    /// Const-friendly validated constructor returning `None` if out of range.
    pub const fn ymd(year: i32, month: u8, day: u8) -> Option<Self> {
        if year < 1 || year > 9999 || month < 1 || month > 12 {
            return None;
        }
        let max_day = days_in_month(year, month);
        if day < 1 || day > max_day {
            return None;
        }
        Some(Self { year, month, day })
    }

    pub const fn year(self) -> i32 {
        self.year
    }

    pub const fn month(self) -> u8 {
        self.month
    }

    pub const fn day(self) -> u8 {
        self.day
    }

    pub(crate) fn to_abi_string(self) -> String {
        self.to_string()
    }

    /// Parses either `YYYY-MM-DD` or an ISO-8601 `DateTimeOffset` prefix
    /// (`YYYY-MM-DDTHH:mm:ss...`) returned by `AvnDateTimeOffset`.
    pub(crate) fn from_abi_str(s: &str) -> Result<Self> {
        let date_part = s.split('T').next().unwrap_or(s);
        date_part.parse()
    }
}

const fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

const fn days_in_month(year: i32, month: u8) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap_year(year) {
                29
            } else {
                28
            }
        }
        _ => 0,
    }
}

impl fmt::Display for CivilDate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

impl FromStr for CivilDate {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        let bytes = s.as_bytes();
        if bytes.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' || !s.is_ascii() {
            return Err(Error::InvalidValue(format!(
                "expected ISO-8601 date YYYY-MM-DD, got '{s}'"
            )));
        }
        let year: i32 = s[0..4]
            .parse()
            .map_err(|_| Error::InvalidValue(format!("invalid year in date '{s}'")))?;
        let month: u8 = s[5..7]
            .parse()
            .map_err(|_| Error::InvalidValue(format!("invalid month in date '{s}'")))?;
        let day: u8 = s[8..10]
            .parse()
            .map_err(|_| Error::InvalidValue(format!("invalid day in date '{s}'")))?;
        Self::new(year, month, day)
    }
}

/// 24-hour wall-clock time (`HH:MM:SS`) in `00:00:00..=23:59:59`.
///
/// Used for `TimePicker::selected_time`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CivilTime {
    hour: u8,
    minute: u8,
    second: u8,
}

impl CivilTime {
    pub fn new(hour: u8, minute: u8, second: u8) -> Result<Self> {
        Self::hms(hour, minute, second).ok_or_else(|| {
            Error::InvalidValue(format!(
                "invalid wall-clock time {hour:02}:{minute:02}:{second:02}"
            ))
        })
    }

    pub const fn hms(hour: u8, minute: u8, second: u8) -> Option<Self> {
        if hour < 24 && minute < 60 && second < 60 {
            Some(Self {
                hour,
                minute,
                second,
            })
        } else {
            None
        }
    }

    pub const fn hm(hour: u8, minute: u8) -> Option<Self> {
        Self::hms(hour, minute, 0)
    }

    pub const fn hour(self) -> u8 {
        self.hour
    }

    pub const fn minute(self) -> u8 {
        self.minute
    }

    pub const fn second(self) -> u8 {
        self.second
    }

    pub(crate) fn to_abi_string(self) -> String {
        self.to_string()
    }

    pub(crate) fn from_abi_str(s: &str) -> Result<Self> {
        s.parse()
    }
}

impl fmt::Display for CivilTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:02}:{:02}:{:02}", self.hour, self.minute, self.second)
    }
}

impl FromStr for CivilTime {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        if !s.is_ascii() {
            return Err(Error::InvalidValue(format!(
                "invalid wall-clock time '{s}'"
            )));
        }
        let parts: Vec<&str> = s.split(':').collect();
        match parts.as_slice() {
            [hh, mm] if hh.len() == 2 && mm.len() == 2 => {
                let h: u8 = hh
                    .parse()
                    .map_err(|_| Error::InvalidValue(format!("invalid hour in '{s}'")))?;
                let m: u8 = mm
                    .parse()
                    .map_err(|_| Error::InvalidValue(format!("invalid minute in '{s}'")))?;
                Self::new(h, m, 0)
            }
            [hh, mm, ss] if hh.len() == 2 && mm.len() == 2 && ss.len() == 2 => {
                let h: u8 = hh
                    .parse()
                    .map_err(|_| Error::InvalidValue(format!("invalid hour in '{s}'")))?;
                let m: u8 = mm
                    .parse()
                    .map_err(|_| Error::InvalidValue(format!("invalid minute in '{s}'")))?;
                let sec: u8 = ss
                    .parse()
                    .map_err(|_| Error::InvalidValue(format!("invalid second in '{s}'")))?;
                Self::new(h, m, sec)
            }
            _ => Err(Error::InvalidValue(format!(
                "expected 24-hour time HH:MM or HH:MM:SS, got '{s}'"
            ))),
        }
    }
}

/// Exact base-10 decimal value matching `.NET` `System.Decimal` (`96`-bit
/// magnitude, scale `0..=28`), used for `NumericUpDown` properties.
#[derive(Clone, Copy, Debug, Default)]
pub struct Decimal {
    mantissa: i128,
    scale: u8,
}

const MAX_DECIMAL_MAGNITUDE: u128 = (1_u128 << 96) - 1;
const MAX_DECIMAL_SCALE: u8 = 28;

impl Decimal {
    pub const ZERO: Self = Self {
        mantissa: 0,
        scale: 0,
    };
    pub const ONE: Self = Self {
        mantissa: 1,
        scale: 0,
    };

    /// Creates a `Decimal` representing `mantissa / 10^scale`.
    pub fn new(mantissa: i128, scale: u8) -> Result<Self> {
        if scale > MAX_DECIMAL_SCALE || mantissa.unsigned_abs() > MAX_DECIMAL_MAGNITUDE {
            return Err(Error::InvalidValue(format!(
                "decimal out of 96-bit/scale-28 range: mantissa={mantissa}, scale={scale}"
            )));
        }
        Ok(Self { mantissa, scale })
    }

    pub const fn from_i64(value: i64) -> Self {
        Self {
            mantissa: value as i128,
            scale: 0,
        }
    }

    pub const fn mantissa(self) -> i128 {
        self.mantissa
    }

    pub const fn scale(self) -> u8 {
        self.scale
    }

    /// Strips trailing fractional zeros (`1.50` -> `1.5`, `-0.0` -> `0`).
    pub const fn normalize(self) -> Self {
        if self.mantissa == 0 {
            return Self::ZERO;
        }
        let mut m = self.mantissa;
        let mut s = self.scale;
        while s > 0 && m % 10 == 0 {
            m /= 10;
            s -= 1;
        }
        Self {
            mantissa: m,
            scale: s,
        }
    }

    pub fn to_f64(self) -> f64 {
        let divisor = 10_f64.powi(i32::from(self.scale));
        (self.mantissa as f64) / divisor
    }

    pub(crate) fn to_abi_string(self) -> String {
        self.to_string()
    }

    pub(crate) fn from_abi_str(s: &str) -> Result<Self> {
        s.parse()
    }
}

impl PartialEq for Decimal {
    fn eq(&self, other: &Self) -> bool {
        let a = self.normalize();
        let b = other.normalize();
        a.mantissa == b.mantissa && a.scale == b.scale
    }
}

impl Eq for Decimal {}

impl std::hash::Hash for Decimal {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let norm = self.normalize();
        norm.mantissa.hash(state);
        norm.scale.hash(state);
    }
}

impl PartialOrd for Decimal {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Decimal {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let a = self.normalize();
        let b = other.normalize();
        if a.mantissa.signum() != b.mantissa.signum() {
            return a.mantissa.signum().cmp(&b.mantissa.signum());
        }
        if a.scale == b.scale {
            return a.mantissa.cmp(&b.mantissa);
        }
        // Compare via 10^diff scaling in i128 where possible, falling back to integer+fraction split.
        let target_scale = a.scale.max(b.scale);
        let scale_to = |d: Decimal, target: u8| -> Option<i128> {
            let diff = u32::from(target - d.scale);
            10_i128
                .checked_pow(diff)
                .and_then(|f| d.mantissa.checked_mul(f))
        };
        if let (Some(ma), Some(mb)) = (scale_to(a, target_scale), scale_to(b, target_scale)) {
            return ma.cmp(&mb);
        }
        let pow_a = 10_i128.pow(u32::from(a.scale));
        let pow_b = 10_i128.pow(u32::from(b.scale));
        let (int_a, rem_a) = (a.mantissa / pow_a, a.mantissa % pow_a);
        let (int_b, rem_b) = (b.mantissa / pow_b, b.mantissa % pow_b);
        int_a.cmp(&int_b).then_with(|| {
            // Cross-multiply fractional remainders (`rem < 10^28`, so `rem_a * pow_b` < 10^56; fits in i128? 10^56 > i128::MAX (~1.7e38), so compare digit by digit).
            let mut ra = rem_a.unsigned_abs();
            let mut rb = rem_b.unsigned_abs();
            let pa = pow_a as u128;
            let pb = pow_b as u128;
            for _ in 0..=MAX_DECIMAL_SCALE {
                let da = (ra * 10) / pa;
                let db = (rb * 10) / pb;
                if da != db {
                    return if a.mantissa >= 0 {
                        da.cmp(&db)
                    } else {
                        db.cmp(&da)
                    };
                }
                ra = (ra * 10) % pa;
                rb = (rb * 10) % pb;
            }
            std::cmp::Ordering::Equal
        })
    }
}

impl From<i32> for Decimal {
    fn from(value: i32) -> Self {
        Self::from_i64(i64::from(value))
    }
}

impl From<i64> for Decimal {
    fn from(value: i64) -> Self {
        Self::from_i64(value)
    }
}

impl TryFrom<f64> for Decimal {
    type Error = Error;

    fn try_from(value: f64) -> Result<Self> {
        if !value.is_finite() {
            return Err(Error::InvalidValue(format!(
                "cannot convert non-finite f64 '{value}' to Decimal"
            )));
        }
        let formatted = format!("{value}");
        formatted.parse()
    }
}

impl fmt::Display for Decimal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.scale == 0 {
            return write!(f, "{}", self.mantissa);
        }
        let sign = if self.mantissa < 0 { "-" } else { "" };
        let abs = self.mantissa.unsigned_abs();
        let divisor = 10_u128.pow(u32::from(self.scale));
        let int_part = abs / divisor;
        let frac_part = abs % divisor;
        let width = usize::from(self.scale);
        write!(f, "{sign}{int_part}.{frac_part:0width$}")
    }
}

impl FromStr for Decimal {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        let trimmed = s.trim();
        if trimmed.is_empty() || !trimmed.is_ascii() {
            return Err(Error::InvalidValue(format!("invalid decimal '{s}'")));
        }
        let (negative, rest) = if let Some(r) = trimmed.strip_prefix('-') {
            (true, r)
        } else if let Some(r) = trimmed.strip_prefix('+') {
            (false, r)
        } else {
            (false, trimmed)
        };
        if rest.is_empty() || rest == "." {
            return Err(Error::InvalidValue(format!("invalid decimal '{s}'")));
        }
        let (int_str, frac_str) = match rest.split_once('.') {
            Some((i, f)) => {
                if f.contains('.') {
                    return Err(Error::InvalidValue(format!("invalid decimal '{s}'")));
                }
                (i, f)
            }
            None => (rest, ""),
        };
        if !int_str.bytes().all(|b| b.is_ascii_digit())
            || !frac_str.bytes().all(|b| b.is_ascii_digit())
        {
            return Err(Error::InvalidValue(format!("invalid decimal '{s}'")));
        }
        let scale = u8::try_from(frac_str.len()).map_err(|_| {
            Error::InvalidValue(format!("decimal scale exceeds 28 digits in '{s}'"))
        })?;
        if scale > MAX_DECIMAL_SCALE {
            return Err(Error::InvalidValue(format!(
                "decimal scale {scale} exceeds maximum 28 in '{s}'"
            )));
        }
        let combined = format!("{int_str}{frac_str}");
        let magnitude: u128 = combined
            .parse()
            .map_err(|_| Error::InvalidValue(format!("decimal overflow in '{s}'")))?;
        if magnitude > MAX_DECIMAL_MAGNITUDE {
            return Err(Error::InvalidValue(format!(
                "decimal magnitude exceeds 96 bits in '{s}'"
            )));
        }
        let mantissa = if negative {
            -(magnitude as i128)
        } else {
            magnitude as i128
        };
        Ok(Self { mantissa, scale })
    }
}

fn write_invariant_f64(f: &mut fmt::Formatter<'_>, value: f64) -> fmt::Result {
    if value.fract() == 0.0 && value.abs() < (i64::MAX as f64) {
        write!(f, "{}", value as i64)
    } else {
        write!(f, "{value}")
    }
}

#[cfg(feature = "chrono")]
mod chrono_compat {
    use super::{CivilDate, CivilTime};
    use chrono::{Datelike, NaiveDate, NaiveTime, Timelike};

    impl From<NaiveDate> for CivilDate {
        fn from(d: NaiveDate) -> Self {
            Self {
                year: d.year(),
                month: d.month() as u8,
                day: d.day() as u8,
            }
        }
    }

    impl TryFrom<CivilDate> for NaiveDate {
        type Error = crate::Error;

        fn try_from(d: CivilDate) -> crate::Result<Self> {
            NaiveDate::from_ymd_opt(d.year, u32::from(d.month), u32::from(d.day))
                .ok_or_else(|| crate::Error::InvalidValue(format!("date out of chrono range: {d}")))
        }
    }

    impl From<NaiveTime> for CivilTime {
        fn from(t: NaiveTime) -> Self {
            Self {
                hour: t.hour() as u8,
                minute: t.minute() as u8,
                second: t.second() as u8,
            }
        }
    }

    impl From<CivilTime> for NaiveTime {
        fn from(t: CivilTime) -> Self {
            NaiveTime::from_hms_opt(u32::from(t.hour), u32::from(t.minute), u32::from(t.second))
                .expect("CivilTime invariants guarantee valid H:M:S")
        }
    }
}

#[cfg(feature = "time")]
mod time_compat {
    use super::{CivilDate, CivilTime};

    impl TryFrom<time::Date> for CivilDate {
        type Error = crate::Error;

        fn try_from(d: time::Date) -> crate::Result<Self> {
            Self::new(d.year(), u8::from(d.month()), d.day())
        }
    }

    impl TryFrom<CivilDate> for time::Date {
        type Error = crate::Error;

        fn try_from(d: CivilDate) -> crate::Result<Self> {
            let month = time::Month::try_from(d.month)
                .map_err(|_| crate::Error::InvalidValue(format!("invalid month in {d}")))?;
            time::Date::from_calendar_date(d.year, month, d.day)
                .map_err(|e| crate::Error::InvalidValue(e.to_string()))
        }
    }

    impl From<time::Time> for CivilTime {
        fn from(t: time::Time) -> Self {
            Self {
                hour: t.hour(),
                minute: t.minute(),
                second: t.second(),
            }
        }
    }

    impl From<CivilTime> for time::Time {
        fn from(t: CivilTime) -> Self {
            time::Time::from_hms(t.hour, t.minute, t.second)
                .expect("CivilTime invariants guarantee valid H:M:S")
        }
    }
}

#[cfg(feature = "rust_decimal")]
mod rust_decimal_compat {
    use super::Decimal;

    impl From<rust_decimal::Decimal> for Decimal {
        fn from(d: rust_decimal::Decimal) -> Self {
            Self {
                mantissa: d.mantissa(),
                scale: d.scale() as u8,
            }
        }
    }

    impl From<Decimal> for rust_decimal::Decimal {
        fn from(d: Decimal) -> Self {
            rust_decimal::Decimal::from_i128_with_scale(d.mantissa, u32::from(d.scale))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_tracks_parse_and_format_with_shared_groups() {
        let tracks = GridTracks::from([
            GridTrack::star(1.0),
            GridTrack::auto().shared_group("Labels"),
            GridTrack::px(120.0),
            GridTrack::star(2.5),
        ]);
        assert_eq!(tracks.to_string(), "*,Auto@Labels,120,2.5*");
        let parsed: GridTracks = "*, Auto@Labels, 120, 2.5*".parse().unwrap();
        assert_eq!(parsed, tracks);
        assert!(GridTracks::from_str("").unwrap().is_empty());
        assert!(GridTracks::from_str("Auto@@Bad").is_err());
    }

    #[test]
    fn civil_date_validates_leap_years_and_parses_datetimeoffset_prefixes() {
        assert!(CivilDate::new(2024, 2, 29).is_ok());
        assert!(CivilDate::new(2025, 2, 29).is_err());
        assert!(CivilDate::new(2026, 13, 1).is_err());
        let d = CivilDate::from_abi_str("2026-09-03T00:00:00.0000000-04:00").unwrap();
        assert_eq!(d, CivilDate::ymd(2026, 9, 3).unwrap());
        assert_eq!(d.to_string(), "2026-09-03");
    }

    #[test]
    fn civil_time_parses_hm_and_hms() {
        let hm: CivilTime = "09:05".parse().unwrap();
        assert_eq!(hm.to_string(), "09:05:00");
        let hms: CivilTime = "23:59:59".parse().unwrap();
        assert_eq!((hms.hour(), hms.minute(), hms.second()), (23, 59, 59));
        assert!("24:00:00".parse::<CivilTime>().is_err());
    }

    #[test]
    fn decimal_round_trips_and_compares_semantically() {
        let a: Decimal = "1.5".parse().unwrap();
        let b: Decimal = "1.50".parse().unwrap();
        let c: Decimal = "-0.25".parse().unwrap();
        assert_eq!(a, b);
        assert_eq!(b.to_string(), "1.50");
        assert!(c < Decimal::ZERO);
        assert!(Decimal::ZERO < a);
        assert_eq!(Decimal::try_from(42.125_f64).unwrap().to_string(), "42.125");
    }
}
