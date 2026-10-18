//! One file as one MongoDB record. The frontmatter is stored as plain BSON
//! values, dates as BSON dates, so a query can name a frontmatter key.

use core::fmt;

use archie_core::parser::ast::{Mapping, NonFinite, RawDocument, Value, raw_document_schema};
use bson::oid::ObjectId;
use bson::{Bson, DateTime as BsonDateTime, Document, doc};
use chrono::{DateTime, NaiveTime, Utc};
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

use crate::{Revision, StoreError, file};

/// The deepest record MongoDB holds, each document and each array one level.
const MAX_DEPTH: usize = 100;

/// One file of a MongoDB store: its path, its revision and its document.
#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct StoredDocument {
    #[serde(rename = "doc")]
    pub document: RawDocument,
    #[serde(rename = "_id")]
    pub id: ObjectId,
    pub path: String,
    pub revision: String,
}

/// The record sent to MongoDB: serde's BSON of `stored`, with `doc.data` as
/// plain BSON values.
///
/// # Errors
///
/// `StoreError::Backend` naming the path when the record does not serialize,
/// or nests deeper than the 100 levels MongoDB holds.
pub fn to_record(stored: &StoredDocument) -> Result<Document, StoreError> {
    let failed =
        |error: &dyn fmt::Display| StoreError::Backend(format!("{}: {error}", stored.path));
    let too_deep = |levels: usize| {
        StoreError::Backend(format!(
            "{} nests {levels} levels, and MongoDB holds {MAX_DEPTH}",
            stored.path
        ))
    };
    let frontmatter = if let RawDocument::Valid(document) = &stored.document {
        document.frontmatter.as_ref()
    } else {
        None
    };
    // serde writes a deep frontmatter by recursion, so it is measured first.
    if let Some(levels) = frontmatter
        .map(frontmatter_depth)
        .filter(|levels| *levels > MAX_DEPTH)
    {
        return Err(too_deep(levels));
    }
    let mut record = bson::serialize_to_document(stored).map_err(|error| failed(&error))?;
    if let Some(mapping) = frontmatter {
        record
            .get_document_mut("doc")
            .map_err(|error| failed(&error))?
            .insert("data", plain_mapping(mapping));
    }
    let levels = depth(&record);
    if levels > MAX_DEPTH {
        return Err(too_deep(levels));
    }
    Ok(record)
}

/// A record read back: a valid document's `doc.data` read from plain BSON
/// values and put back in serde's shape, then the record read with
/// `StoredDocument::from_bson_piped`.
///
/// # Errors
///
/// `StoreError::Backend` naming the record's `_id` and either every issue or
/// the BSON type of a frontmatter value that `to_record` never writes.
pub fn from_record(mut record: Document) -> Result<StoredDocument, StoreError> {
    let id = record
        .get("_id")
        .map_or_else(|| "with no `_id`".to_owned(), ToString::to_string);
    let unread = |reason: &dyn fmt::Display| {
        StoreError::Backend(format!("the record {id} does not read: {reason}"))
    };
    if let Ok(doc) = record.get_document_mut("doc")
        && doc.get_str("t").is_ok_and(|tag| tag == "valid")
        && let Ok(data) = doc.get_document("data")
    {
        let mapping = typed_mapping(data).map_err(|error| unread(&error))?;
        let tagged = bson::serialize_to_bson(&mapping).map_err(|error| unread(&error))?;
        doc.insert("data", tagged);
    }
    StoredDocument::from_bson_piped(record, &[]).map_err(|unrecovered| unread(&unrecovered))
}

/// The filter on the frontmatter key `key`, which no typed path names:
/// `{ "doc.data.<key>": value }`, the value as `to_record` stores it.
#[must_use]
pub fn frontmatter_is(key: &str, value: &Value) -> Document {
    let mut filter = Document::new();
    filter.insert(format!("doc.data.{key}"), plain(value));
    filter
}

/// `sha256:` and the 64 hex digits of the SHA-256 of `text`, archie's form of
/// the document: the revision the file-system store gives for the same bytes.
#[must_use]
pub fn revision_of(text: &str) -> Revision {
    file::revision(text.as_bytes())
}

/// How deep `record` nests: the record is the first level, and each document
/// and each array in it one more. Counted from a list, not by recursion.
fn depth(record: &Document) -> usize {
    let mut deepest = 1_usize;
    let mut open: Vec<(usize, &Bson)> = record.values().map(|value| (2, value)).collect();
    while let Some((level, value)) = open.pop() {
        if let Some(document) = value.as_document() {
            deepest = deepest.max(level);
            open.extend(document.values().map(|inner| (level + 1, inner)));
        }
        if let Some(items) = value.as_array() {
            deepest = deepest.max(level);
            open.extend(items.iter().map(|inner| (level + 1, inner)));
        }
    }
    deepest
}

/// How deep a record nests through `frontmatter` as its `doc.data`, counted as
/// `depth` counts: `data` is level 3, and each mapping, list and time inside
/// it, each stored as a document or an array, one more.
fn frontmatter_depth(frontmatter: &Mapping) -> usize {
    let mut deepest = 3_usize;
    let mut open: Vec<(usize, &Value)> = frontmatter.values().map(|value| (4, value)).collect();
    while let Some((level, value)) = open.pop() {
        if let Value::Mapping(inner) = value {
            deepest = deepest.max(level);
            open.extend(inner.values().map(|item| (level + 1, item)));
        }
        if let Value::Sequence(items) = value {
            deepest = deepest.max(level);
            open.extend(items.iter().map(|item| (level + 1, item)));
        }
        if matches!(value, Value::Time(_)) {
            deepest = deepest.max(level);
        }
    }
    deepest
}

/// A value as it is stored. A kind of value this store does not know keeps
/// serde's tagged shape.
fn plain(value: &Value) -> Bson {
    match value {
        Value::Boolean(flag) => Bson::Boolean(*flag),
        Value::Date(date) => utc(date.and_time(NaiveTime::MIN).and_utc()),
        Value::DateTime(at) => utc(at.and_utc()),
        Value::Float(number) => Bson::Double(*number),
        Value::Integer(number) => Bson::Int64(*number),
        Value::Mapping(mapping) => Bson::Document(plain_mapping(mapping)),
        Value::NonFinite(NonFinite::Infinity) => Bson::Double(f64::INFINITY),
        Value::NonFinite(NonFinite::Nan) => Bson::Double(f64::NAN),
        Value::NonFinite(NonFinite::NegativeInfinity) => Bson::Double(f64::NEG_INFINITY),
        Value::Null => Bson::Null,
        Value::OffsetDateTime(at) => utc(at.to_utc()),
        Value::Sequence(items) => Bson::Array(items.iter().map(plain).collect()),
        Value::String(text) => Bson::String(text.clone()),
        Value::Time(time) => {
            let text = time.format("%H:%M:%S%.3f").to_string();
            Bson::Document(doc! { "t": "time", "v": text.trim_end_matches(".000") })
        }
        Value::NonFinite(_) | _ => bson::serialize_to_bson(value).unwrap_or(Bson::Null),
    }
}

fn plain_mapping(mapping: &Mapping) -> Document {
    mapping
        .iter()
        .map(|(key, value)| (key.clone(), plain(value)))
        .collect()
}

/// A stored value read back. A BSON date at midnight UTC is a date, any other
/// a date-time in UTC.
fn typed(value: &Bson) -> Result<Value, StoreError> {
    Ok(match value {
        Bson::Boolean(flag) => Value::Boolean(*flag),
        Bson::DateTime(stored) => {
            let at = DateTime::<Utc>::from_timestamp_millis(stored.timestamp_millis())
                .ok_or_else(|| StoreError::Backend(format!("the date {stored} is out of range")))?;
            if at.time() == NaiveTime::MIN {
                Value::Date(at.date_naive())
            } else {
                Value::OffsetDateTime(at.fixed_offset())
            }
        }
        Bson::Double(number) => {
            NonFinite::of(*number).map_or(Value::Float(*number), Value::NonFinite)
        }
        Bson::Int32(number) => Value::Integer(i64::from(*number)),
        Bson::Int64(number) => Value::Integer(*number),
        Bson::Null => Value::Null,
        Bson::String(text) => Value::String(text.clone()),
        Bson::Array(items) => Value::Sequence(items.iter().map(typed).collect::<Result<_, _>>()?),
        Bson::Document(document) => match time_of_day(document) {
            Some(time) => Value::Time(time),
            None => Value::Mapping(typed_mapping(document)?),
        },
        other @ (Bson::Binary(_)
        | Bson::DbPointer(_)
        | Bson::Decimal128(_)
        | Bson::JavaScriptCode(_)
        | Bson::JavaScriptCodeWithScope(_)
        | Bson::MaxKey
        | Bson::MinKey
        | Bson::ObjectId(_)
        | Bson::RegularExpression(_)
        | Bson::Symbol(_)
        | Bson::Timestamp(_)
        | Bson::Undefined) => {
            return Err(StoreError::Backend(format!(
                "a frontmatter value cannot be a BSON {:?}",
                other.element_type()
            )));
        }
    })
}

fn typed_mapping(document: &Document) -> Result<Mapping, StoreError> {
    document
        .iter()
        .map(|(key, value)| Ok((key.clone(), typed(value)?)))
        .collect()
}

/// The time a document holds when it has exactly the keys `t`, which is
/// `time`, and `v`, a valid time.
fn time_of_day(document: &Document) -> Option<NaiveTime> {
    if document.len() != 2 || document.get_str("t").ok()? != "time" {
        return None;
    }
    NaiveTime::parse_from_str(document.get_str("v").ok()?, "%H:%M:%S%.f").ok()
}

const fn utc(at: DateTime<Utc>) -> Bson {
    Bson::DateTime(BsonDateTime::from_millis(at.timestamp_millis()))
}

#[cfg(test)]
mod tests;
