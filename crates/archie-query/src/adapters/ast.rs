//! Reads archie-core's frontmatter into format-neutral fields.

use archie_core::parser::{
    self,
    ast::{Date as AstDate, Mapping, ScalarValue, Time as AstTime, Value},
    read_toml,
};

use crate::error::QuerySourceError;
use crate::field::{Date, DateTime, Field, Time};

/// The frontmatter of `source` as fields, in the order they were written.
/// `Ok(None)` when the source has no frontmatter.
pub fn frontmatter_fields(source: &str) -> Result<Option<Vec<(String, Field)>>, QuerySourceError> {
    let document = parser::parse(source).map_err(|err| QuerySourceError::Parse {
        message: err.message,
        span: err.span,
    })?;
    Ok(document
        .frontmatter
        .map(|frontmatter| mapping(&frontmatter.mapping)))
}

fn mapping(mapping: &Mapping) -> Vec<(String, Field)> {
    mapping
        .pairs
        .iter()
        .map(|(key, pair)| (key.clone(), value(&pair.value)))
        .collect()
}

fn value(value: &Value) -> Field {
    match value {
        Value::Mapping(inner) => Field::Map(mapping(inner)),
        Value::Scalar(scalar) => scalar_field(&scalar.value).unwrap_or(Field::Null),
        Value::Sequence(sequence) => Field::List(sequence.items.iter().map(self::value).collect()),
        _ => Field::Null,
    }
}

/// `None` for a scalar kind this adapter does not know yet.
fn scalar_field(value: &ScalarValue) -> Option<Field> {
    Some(match value {
        ScalarValue::Boolean(flag) => Field::Bool(*flag),
        ScalarValue::Date(date) => Field::Date(date_of(*date)),
        ScalarValue::DateTime(datetime) => Field::DateTime(DateTime {
            date: date_of(datetime.date),
            offset_minutes: datetime.offset,
            time: time_of(datetime.time),
        }),
        ScalarValue::Float(number) => Field::Float(*number),
        ScalarValue::Integer(number) => Field::Int(*number),
        ScalarValue::Null => Field::Null,
        ScalarValue::String(text) => Field::Str(text.clone()),
        ScalarValue::Time(time) => Field::Time(time_of(*time)),
        _ => return None,
    })
}

fn date_of(date: AstDate) -> Date {
    Date {
        d: date.day,
        m: date.month,
        y: i32::from(date.year),
    }
}

const fn time_of(time: AstTime) -> Time {
    Time {
        h: time.hour,
        m: time.minute,
        ns: time.nanosecond,
        s: time.second,
    }
}

/// A literal word or string read as a date, datetime, integer or float, by
/// asking archie-core to read `v = <text>`. `None` for anything else, and for a
/// date or time archie accepts but that is not real (day past the month's end,
/// hour 24, offset past 23:59).
pub fn read_literal(text: &str) -> Option<Field> {
    let allowed = |c: char| c.is_ascii_alphanumeric() || matches!(c, '_' | '+' | ':' | '.' | '-');
    if text.is_empty() || !text.chars().all(allowed) {
        return None;
    }
    let source = format!("v = {text}");
    let read = read_toml(&source, 0..source.len()).ok()?;
    if read.pairs.len() != 1 {
        return None;
    }
    let (_, pair) = read.pairs.first()?;
    let Value::Scalar(scalar) = &pair.value else {
        return None;
    };
    let field = scalar_field(&scalar.value)?;
    let accepted = match &field {
        Field::Date(date) => date.is_real(),
        Field::DateTime(datetime) => datetime.is_real() && offset_minutes_in_range(text),
        Field::Float(_) | Field::Int(_) => true,
        Field::Bool(_)
        | Field::List(_)
        | Field::Map(_)
        | Field::Null
        | Field::Str(_)
        | Field::Time(_) => false,
    };
    accepted.then_some(field)
}

/// archie folds `+05:60` into 360 minutes, so the written minutes of a
/// numeric offset are checked on the text. `Z` and a missing offset pass. It relies on archie having already
/// validated the text as a datetime, so it only looks at the offset's shape.
fn offset_minutes_in_range(text: &str) -> bool {
    let Some((_, clock)) = text.split_once(['T', 't']) else {
        return true;
    };
    let Some((_, offset)) = clock.rsplit_once(['+', '-']) else {
        return true;
    };
    if !offset.contains(':') {
        return true;
    }
    offset
        .split_once(':')
        .is_some_and(|(_, minutes)| minutes.parse::<u8>().is_ok_and(|m| m <= 59))
}
