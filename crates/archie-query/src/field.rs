//! Frontmatter values with their types, independent of the format they were
//! written in.

use core::fmt;

use crate::adapters::ast::read_literal;

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

    /// Whether the day exists in its month.
    pub(crate) const fn is_real(self) -> bool {
        1 <= self.m && self.m <= 12 && 1 <= self.d && self.d <= days_in_month(self.y, self.m)
    }

    /// `text` as a date, when archie-core reads it as one and the day exists.
    pub(crate) fn parse(text: &str) -> Option<Self> {
        let Field::Date(date) = read_literal(text)? else {
            return None;
        };
        Some(date)
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

    /// Whether the date exists, the time is a time of day, and the offset is at most 23:59.
    pub(crate) const fn is_real(self) -> bool {
        self.date.is_real()
            && self.time.h <= 23
            && self.time.m <= 59
            && self.time.s <= 59
            && match self.offset_minutes {
                None => true,
                Some(minutes) => minutes.unsigned_abs() <= 23 * 60 + 59,
            }
    }

    /// `text` as a datetime, when archie-core reads it as one and every part is in range.
    pub(crate) fn parse(text: &str) -> Option<Self> {
        let Field::DateTime(datetime) = read_literal(text)? else {
            return None;
        };
        Some(datetime)
    }
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
