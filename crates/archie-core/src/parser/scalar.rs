//! Typed scalars from their text: numbers, dates and times.

use core::str::FromStr;

use chrono::{FixedOffset, NaiveDate, NaiveDateTime, NaiveTime};

use crate::parser::ast::{NonFinite, ScalarValue};

const TOML_RADIXES: &[(&str, u32)] = &[("0x", 16), ("0o", 8), ("0b", 2)];
const YAML_RADIXES: &[(&str, u32)] = &[("0x", 16), ("0o", 8)];

/// The value of an unquoted TOML token, or `None` when it is not one.
pub fn toml(text: &str) -> Option<ScalarValue> {
    match text {
        "true" => Some(ScalarValue::Boolean(true)),
        "false" => Some(ScalarValue::Boolean(false)),
        "inf" | "+inf" => Some(ScalarValue::NonFinite(NonFinite::Infinity)),
        "-inf" => Some(ScalarValue::NonFinite(NonFinite::NegativeInfinity)),
        "nan" | "+nan" | "-nan" => Some(ScalarValue::NonFinite(NonFinite::Nan)),
        _ => {
            let number = text.replace('_', "");
            date_time(text)
                .or_else(|| date(text).map(ScalarValue::Date))
                .or_else(|| time(text).map(ScalarValue::Time))
                .or_else(|| integer(&number, TOML_RADIXES).map(ScalarValue::Integer))
                .or_else(|| float(&number).map(finite_or_infinite))
        }
    }
}

/// The value of a plain YAML scalar: whatever is not a null, a boolean, a
/// number or a date is a string.
pub fn yaml(text: &str) -> ScalarValue {
    match text {
        "" | "~" | "null" | "Null" | "NULL" => ScalarValue::Null,
        "true" | "True" | "TRUE" => ScalarValue::Boolean(true),
        "false" | "False" | "FALSE" => ScalarValue::Boolean(false),
        ".inf" | ".Inf" | ".INF" | "+.inf" | "+.Inf" | "+.INF" => {
            ScalarValue::NonFinite(NonFinite::Infinity)
        }
        "-.inf" | "-.Inf" | "-.INF" => ScalarValue::NonFinite(NonFinite::NegativeInfinity),
        ".nan" | ".NaN" | ".NAN" => ScalarValue::NonFinite(NonFinite::Nan),
        _ => integer(text, YAML_RADIXES)
            .map(ScalarValue::Integer)
            .or_else(|| float(text).map(finite_or_infinite))
            .or_else(|| date_time(text))
            .or_else(|| date(text).map(ScalarValue::Date))
            .unwrap_or_else(|| ScalarValue::String(text.to_owned())),
    }
}

pub fn is_date(text: &str) -> bool {
    date(text).is_some()
}

/// A date such as `2026-10-10`, when that day exists in the calendar.
fn date(text: &str) -> Option<NaiveDate> {
    let [_, _, _, _, b'-', _, _, b'-', _, _] = text.as_bytes() else {
        return None;
    };
    if !text.is_ascii() {
        return None;
    }
    NaiveDate::from_ymd_opt(
        digits(&text[0..4])?,
        digits(&text[5..7])?,
        digits(&text[8..10])?,
    )
}

/// A time of day such as `07:32:00`, with an optional fraction. Second 60 is
/// a leap second, which chrono holds as second 59 plus one more second of
/// nanoseconds.
fn time(text: &str) -> Option<NaiveTime> {
    let (clock, fraction) = text
        .split_once('.')
        .map_or((text, None), |(clock, fraction)| (clock, Some(fraction)));
    let [_, _, b':', _, _, b':', _, _] = clock.as_bytes() else {
        return None;
    };
    if !clock.is_ascii() {
        return None;
    }
    let fraction_nanoseconds = fraction.map_or(Some(0), nanoseconds)?;
    let (second, nanosecond) = match digits(&clock[6..8])? {
        60 => (59, fraction_nanoseconds + 1_000_000_000),
        written => (written, fraction_nanoseconds),
    };
    NaiveTime::from_hms_nano_opt(
        digits(&clock[0..2])?,
        digits(&clock[3..5])?,
        second,
        nanosecond,
    )
}

fn nanoseconds(fraction: &str) -> Option<u32> {
    if !is_digits(fraction) {
        return None;
    }
    let mut padded = fraction[..fraction.len().min(9)].to_owned();
    while padded.len() < 9 {
        padded.push('0');
    }
    padded.parse().ok()
}

/// A date, then `T`, `t` or a space, then a time: a `DateTime` when an offset
/// follows, a `LocalDateTime` when none does.
fn date_time(text: &str) -> Option<ScalarValue> {
    if !text.is_ascii() || text.len() < 19 || !matches!(text.as_bytes()[10], b'T' | b't' | b' ') {
        return None;
    }
    let (clock, minutes) = split_offset(&text[11..])?;
    let local = NaiveDateTime::new(date(&text[..10])?, time(clock)?);
    let Some(east) = minutes else {
        return Some(ScalarValue::LocalDateTime(local));
    };
    let offset = FixedOffset::east_opt(i32::from(east) * 60)?;
    local
        .and_local_timezone(offset)
        .single()
        .map(ScalarValue::DateTime)
}

/// Splits `07:32:00-04:00` into the time and its offset in minutes.
fn split_offset(rest: &str) -> Option<(&str, Option<i16>)> {
    if let Some(clock) = rest.strip_suffix(['Z', 'z']) {
        return Some((clock, Some(0)));
    }
    rest.rfind(['+', '-']).map_or(Some((rest, None)), |at| {
        Some((&rest[..at], Some(offset(&rest[at..])?)))
    })
}

fn offset(text: &str) -> Option<i16> {
    let [sign, _, _, b':', _, _] = text.as_bytes() else {
        return None;
    };
    let hours: i16 = digits(&text[1..3])?;
    let minutes: i16 = digits(&text[4..6])?;
    let total = hours * 60 + minutes;
    Some(if *sign == b'-' { -total } else { total })
}

fn integer(text: &str, radixes: &[(&str, u32)]) -> Option<i64> {
    for &(prefix, radix) in radixes {
        if let Some(rest) = text.strip_prefix(prefix) {
            if !rest.bytes().all(|byte| byte.is_ascii_alphanumeric()) {
                return None;
            }
            return i64::from_str_radix(rest, radix).ok();
        }
    }
    let unsigned = text.strip_prefix(['+', '-']).unwrap_or(text);
    if !is_digits(unsigned) {
        return None;
    }
    text.parse().ok()
}

fn float(text: &str) -> Option<f64> {
    let unsigned = text.strip_prefix(['+', '-']).unwrap_or(text);
    let (mantissa, exponent) = unsigned
        .split_once(['e', 'E'])
        .map_or((unsigned, None), |(mantissa, exponent)| {
            (mantissa, Some(exponent))
        });
    let (whole, fraction) = mantissa
        .split_once('.')
        .map_or((mantissa, None), |(whole, fraction)| {
            (whole, Some(fraction))
        });
    let exponent_digits = exponent.map(|part| part.strip_prefix(['+', '-']).unwrap_or(part));
    if !is_digits(whole)
        || !fraction.is_none_or(is_digits)
        || !exponent_digits.is_none_or(is_digits)
    {
        return None;
    }
    if fraction.is_none() && exponent.is_none() {
        return None;
    }
    text.parse().ok()
}

/// A `Float`, or an infinity for digits too large for a float to hold.
const fn finite_or_infinite(float: f64) -> ScalarValue {
    match NonFinite::of(float) {
        Some(non_finite) => ScalarValue::NonFinite(non_finite),
        None => ScalarValue::Float(float),
    }
}

fn digits<T>(text: &str) -> Option<T>
where
    T: FromStr,
{
    if !is_digits(text) {
        return None;
    }
    text.parse().ok()
}

fn is_digits(text: &str) -> bool {
    !text.is_empty() && text.bytes().all(|byte| byte.is_ascii_digit())
}
