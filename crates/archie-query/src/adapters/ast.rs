//! Reads archie-core's frontmatter into format-neutral fields.

use archie_core::parser::{
    self,
    ast::{Date as AstDate, Mapping, ScalarValue, Time as AstTime, Value},
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
