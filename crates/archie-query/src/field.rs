//! Frontmatter values with their types, independent of the format they were
//! written in.

use core::fmt;

const NANOS_PER_SECOND: i128 = 1_000_000_000;
const SECONDS_PER_DAY: i128 = 86_400;

/// A calendar date.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Date {
    pub d: u8,
    pub m: u8,
    pub y: i32,
}

/// A time of day.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Time {
    pub h: u8,
    pub m: u8,
    pub ns: u32,
    pub s: u8,
}

/// A date with a time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct DateTime {
    pub date: Date,
    /// Minutes east of UTC; `None` for a local datetime.
    pub offset_minutes: Option<i16>,
    pub time: Time,
}

/// One frontmatter value.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Field {
    Bool(bool),
    Date(Date),
    DateTime(DateTime),
    Float(f64),
    Int(i64),
    List(Vec<Self>),
    Map(Vec<(String, Self)>),
    Null,
    Str(String),
    Time(Time),
}

impl fmt::Display for Date {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}-{:02}-{:02}", self.y, self.m, self.d)
    }
}

impl fmt::Display for Time {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:02}:{:02}:{:02}", self.h, self.m, self.s)?;
        if self.ns > 0 {
            write!(f, ".{:09}", self.ns)?;
        }
        Ok(())
    }
}

impl fmt::Display for DateTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}T{}", self.date, self.time)?;
        match self.offset_minutes {
            None => Ok(()),
            Some(0) => write!(f, "Z"),
            Some(minutes) => {
                let sign = if minutes < 0 { '-' } else { '+' };
                let total = minutes.unsigned_abs();
                write!(f, "{sign}{:02}:{:02}", total / 60, total % 60)
            }
        }
    }
}

impl fmt::Display for Field {
    /// Scalars only; lists and maps print as `[...]` and `{...}`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bool(flag) => write!(f, "{flag}"),
            Self::Date(date) => write!(f, "{date}"),
            Self::DateTime(datetime) => write!(f, "{datetime}"),
            Self::Float(number) => write!(f, "{number}"),
            Self::Int(number) => write!(f, "{number}"),
            Self::List(_) => write!(f, "[...]"),
            Self::Map(_) => write!(f, "{{...}}"),
            Self::Null => write!(f, "null"),
            Self::Str(text) => write!(f, "{text}"),
            Self::Time(time) => write!(f, "{time}"),
        }
    }
}

impl Date {
    /// Nanoseconds since 1970-01-01 at midnight UTC.
    pub(crate) fn instant(self) -> i128 {
        i128::from(days_from_civil(self.y, self.m, self.d)) * SECONDS_PER_DAY * NANOS_PER_SECOND
    }

    /// Reads `YYYY-MM-DD`; the day must exist in that month.
    pub(crate) fn parse(text: &str) -> Option<Self> {
        if text.len() != 10 || text.get(4..5)? != "-" || text.get(7..8)? != "-" {
            return None;
        }
        let date = Self {
            d: digits(text.get(8..10)?)?.try_into().ok()?,
            m: digits(text.get(5..7)?)?.try_into().ok()?,
            y: digits(text.get(0..4)?)?.try_into().ok()?,
        };
        ((1..=12).contains(&date.m) && (1..=days_in_month(date.y, date.m)).contains(&date.d))
            .then_some(date)
    }
}

impl DateTime {
    /// Nanoseconds since 1970-01-01 UTC. A local datetime counts as UTC.
    pub(crate) fn instant(self) -> i128 {
        let seconds =
            i128::from(self.time.h) * 3600 + i128::from(self.time.m) * 60 + i128::from(self.time.s)
                - i128::from(self.offset_minutes.unwrap_or(0)) * 60;
        self.date.instant() + seconds * NANOS_PER_SECOND + i128::from(self.time.ns)
    }

    /// Reads RFC 3339 `YYYY-MM-DDTHH:MM:SS[.frac][Z|±HH:MM]`, or with no offset.
    pub(crate) fn parse(text: &str) -> Option<Self> {
        if !text.is_ascii() {
            return None;
        }
        let (date_text, rest) = text.split_once(['T', 't'])?;
        let date = Date::parse(date_text)?;
        if rest.get(2..3)? != ":" || rest.get(5..6)? != ":" {
            return None;
        }
        let mut time = Time {
            h: digits(rest.get(0..2)?)?.try_into().ok()?,
            m: digits(rest.get(3..5)?)?.try_into().ok()?,
            ns: 0,
            s: digits(rest.get(6..8)?)?.try_into().ok()?,
        };
        if time.h > 23 || time.m > 59 || time.s > 59 {
            return None;
        }
        let mut tail = rest.get(8..)?;
        if let Some(fraction) = tail.strip_prefix('.') {
            let count = fraction.bytes().take_while(u8::is_ascii_digit).count();
            if count == 0 {
                return None;
            }
            let padded = format!("{:0<9}", fraction.get(..count.min(9))?);
            time.ns = digits(&padded)?;
            tail = fraction.get(count..)?;
        }
        let offset_minutes = match tail {
            "" => None,
            "Z" | "z" => Some(0),
            signed => {
                let sign: i16 = match signed.get(0..1)? {
                    "+" => 1,
                    "-" => -1,
                    _ => return None,
                };
                if signed.len() != 6 || signed.get(3..4)? != ":" {
                    return None;
                }
                let hours = i16::try_from(digits(signed.get(1..3)?)?).ok()?;
                let minutes = i16::try_from(digits(signed.get(4..6)?)?).ok()?;
                if hours > 23 || minutes > 59 {
                    return None;
                }
                Some(sign * (hours * 60 + minutes))
            }
        };
        Some(Self {
            date,
            offset_minutes,
            time,
        })
    }
}

/// `text` as a number when every character is an ASCII digit.
fn digits(text: &str) -> Option<u32> {
    if text.is_empty() || !text.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    text.parse().ok()
}

/// How many days `month` has in `year`.
const fn days_in_month(year: i32, month: u8) -> u8 {
    match month {
        2 if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// Days since 1970-01-01 for a proleptic Gregorian date (Howard Hinnant's algorithm).
fn days_from_civil(year: i32, month: u8, day: u8) -> i64 {
    let shifted = i64::from(year) - i64::from(month <= 2);
    let era = shifted.div_euclid(400);
    let year_of_era = shifted - era * 400;
    let month_from_march = (i64::from(month) + 9).rem_euclid(12);
    let day_of_year = (153 * month_from_march + 2) / 5 + i64::from(day) - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

#[cfg(test)]
mod tests;
