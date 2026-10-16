//! Frontmatter values with their types, independent of the format they were
//! written in.

use core::fmt;

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
