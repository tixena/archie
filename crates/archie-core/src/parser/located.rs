//! Frontmatter values as the TOML and YAML readers build them: each with the
//! byte range it covers, and each key with every place it is written. A
//! reader's caller converts them into the model's `Mapping` and `Value`.

use core::ops::Range;

use chrono::{DateTime, FixedOffset, NaiveDate, NaiveDateTime, NaiveTime};
use indexmap::IndexMap;

use crate::parser::ast::{self, NonFinite};

/// Keys and their values, in the order they were written. A key appears once.
#[derive(Debug, Clone, PartialEq)]
pub struct Mapping {
    pub pairs: IndexMap<String, Pair>,
    pub span: Range<usize>,
}

impl Mapping {
    /// The model's mapping: the same keys in the same order, no spans.
    pub fn into_model(self) -> ast::Mapping {
        self.pairs
            .into_iter()
            .map(|(key, pair)| (key, pair.value.into_model()))
            .collect()
    }
}

/// The value under one key of a mapping. Its span runs from the key to the
/// end of the value, at the key's first occurrence. `repeats` holds the span
/// of each later occurrence of the same key, in file order.
#[derive(Debug, Clone, PartialEq)]
pub struct Pair {
    pub repeats: Vec<Range<usize>>,
    pub span: Range<usize>,
    pub value: Value,
}

/// What a key maps to, or what a sequence holds.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Mapping(Mapping),
    Scalar(Scalar),
    Sequence(Sequence),
}

impl Value {
    pub fn into_model(self) -> ast::Value {
        match self {
            Self::Mapping(mapping) => ast::Value::Mapping(mapping.into_model()),
            Self::Scalar(scalar) => scalar.value.into_model(),
            Self::Sequence(sequence) => {
                ast::Value::Sequence(sequence.items.into_iter().map(Self::into_model).collect())
            }
        }
    }

    /// The byte range the value covers.
    pub const fn span(&self) -> &Range<usize> {
        match self {
            Self::Mapping(mapping) => &mapping.span,
            Self::Scalar(scalar) => &scalar.span,
            Self::Sequence(sequence) => &sequence.span,
        }
    }
}

/// An ordered list of values, such as `["parser", "design"]`.
#[derive(Debug, Clone, PartialEq)]
pub struct Sequence {
    pub items: Vec<Value>,
    pub span: Range<usize>,
}

/// A single value, such as `2` or `"task"`.
#[derive(Debug, Clone, PartialEq)]
pub struct Scalar {
    pub span: Range<usize>,
    pub value: ScalarValue,
}

/// The typed content of a scalar.
#[derive(Debug, Clone, PartialEq)]
pub enum ScalarValue {
    Boolean(bool),
    Date(NaiveDate),
    /// A date and time with no offset.
    DateTime(NaiveDateTime),
    /// A finite float.
    Float(f64),
    Integer(i64),
    NonFinite(NonFinite),
    /// YAML only: TOML has no null.
    Null,
    /// A date and time with its offset from UTC.
    OffsetDateTime(DateTime<FixedOffset>),
    String(String),
    Time(NaiveTime),
}

impl ScalarValue {
    pub fn into_model(self) -> ast::Value {
        match self {
            Self::Boolean(boolean) => ast::Value::Boolean(boolean),
            Self::Date(date) => ast::Value::Date(date),
            Self::DateTime(date_time) => ast::Value::DateTime(date_time),
            Self::Float(float) => ast::Value::Float(float),
            Self::Integer(integer) => ast::Value::Integer(integer),
            Self::NonFinite(non_finite) => ast::Value::NonFinite(non_finite),
            Self::Null => ast::Value::Null,
            Self::OffsetDateTime(date_time) => ast::Value::OffsetDateTime(date_time),
            Self::String(text) => ast::Value::String(text),
            Self::Time(time) => ast::Value::Time(time),
        }
    }
}
