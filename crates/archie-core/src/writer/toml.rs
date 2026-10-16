//! Frontmatter written in TOML: text always quoted, the plain keys of a
//! table first, then a header for each table and list of tables it holds.

use core::fmt::{self, Write as _};

use crate::parser::ast::{Mapping, ScalarValue, Value};
use crate::writer::{WriteError, push_date_time, push_finite, push_local_date_time};

/// `mapping` between `+++` lines, or an error naming the key of the first
/// null, which TOML cannot hold.
pub fn frontmatter(mapping: &Mapping) -> Result<String, WriteError> {
    let mut content = String::new();
    table(&mut content, mapping, &[])?;
    Ok(format!("+++\n{content}+++\n"))
}

/// The plain keys of `mapping`, then, in the order they were read, a
/// `[path]` header for each mapping it holds and a `[[path]]` header for
/// each mapping of a list of mappings. `path` holds the keys of the table.
fn table(out: &mut String, mapping: &Mapping, path: &[&str]) -> Result<(), WriteError> {
    for (key, pair) in &mapping.pairs {
        if !is_header(&pair.value) {
            push_key(out, key);
            out.push_str(" = ");
            inline(out, &pair.value, &joined(path, key))?;
            out.push('\n');
        }
    }
    for (key, pair) in &mapping.pairs {
        let inner = joined(path, key);
        match &pair.value {
            Value::Mapping(nested) => {
                header(out, &inner, false);
                table(out, nested, &inner)?;
            }
            Value::Sequence(_) => {
                for item in tables(&pair.value).unwrap_or_default() {
                    header(out, &inner, true);
                    table(out, item, &inner)?;
                }
            }
            Value::Scalar(_) => {}
        }
    }
    Ok(())
}

/// Whether `value` is written under a header: a mapping, or a list of
/// mappings.
fn is_header(value: &Value) -> bool {
    matches!(value, Value::Mapping(_)) || tables(value).is_some()
}

/// The mappings of a list of one mapping or more that holds nothing else.
fn tables(value: &Value) -> Option<Vec<&Mapping>> {
    let Value::Sequence(sequence) = value else {
        return None;
    };
    if sequence.items.is_empty() {
        return None;
    }
    sequence
        .items
        .iter()
        .map(|item| match item {
            Value::Mapping(mapping) => Some(mapping),
            Value::Scalar(_) | Value::Sequence(_) => None,
        })
        .collect()
}

/// `[path]`, or `[[path]]` for an item of a list of tables, after a blank
/// line unless it opens the frontmatter.
fn header(out: &mut String, path: &[&str], array: bool) {
    if !out.is_empty() {
        out.push('\n');
    }
    out.push_str(if array { "[[" } else { "[" });
    push_path(out, path);
    out.push_str(if array { "]]\n" } else { "]\n" });
}

/// `value` on one line: an inline table, an array, or a scalar. `path`
/// names its key.
fn inline(out: &mut String, value: &Value, path: &[&str]) -> Result<(), WriteError> {
    match value {
        Value::Mapping(mapping) => {
            out.push('{');
            for (index, (key, pair)) in mapping.pairs.iter().enumerate() {
                out.push_str(if index > 0 { ", " } else { " " });
                push_key(out, key);
                out.push_str(" = ");
                inline(out, &pair.value, &joined(path, key))?;
            }
            out.push_str(if mapping.pairs.is_empty() { "}" } else { " }" });
        }
        Value::Scalar(scalar) => push_scalar(out, &scalar.value, path)?,
        Value::Sequence(sequence) => {
            out.push('[');
            for (index, item) in sequence.items.iter().enumerate() {
                if index > 0 {
                    out.push_str(", ");
                }
                inline(out, item, path)?;
            }
            out.push(']');
        }
    }
    Ok(())
}

/// Text in double quotes; booleans, numbers, dates and times bare. A null
/// is an error naming `path`.
fn push_scalar(out: &mut String, value: &ScalarValue, path: &[&str]) -> Result<(), WriteError> {
    match value {
        ScalarValue::Boolean(boolean) => out.push_str(if *boolean { "true" } else { "false" }),
        ScalarValue::Date(date) => {
            let _: fmt::Result = write!(out, "{date}");
        }
        ScalarValue::DateTime(date_time) => push_date_time(out, date_time),
        ScalarValue::Float(float) => {
            if float.is_nan() {
                out.push_str("nan");
            } else if float.is_infinite() {
                out.push_str(if float.is_sign_negative() {
                    "-inf"
                } else {
                    "inf"
                });
            } else {
                push_finite(out, *float);
            }
        }
        ScalarValue::Integer(integer) => {
            let _: fmt::Result = write!(out, "{integer}");
        }
        ScalarValue::LocalDateTime(date_time) => push_local_date_time(out, date_time),
        ScalarValue::Null => {
            let mut key = String::new();
            push_path(&mut key, path);
            return Err(WriteError {
                key,
                message: "TOML cannot hold a null".to_owned(),
            });
        }
        ScalarValue::String(text) => push_string(out, text),
        ScalarValue::Time(time) => {
            let _: fmt::Result = write!(out, "{time}");
        }
    }
    Ok(())
}

/// A bare key when it is one, or else a quoted one.
fn push_key(out: &mut String, key: &str) {
    let bare = !key.is_empty()
        && key
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-');
    if bare {
        out.push_str(key);
    } else {
        push_string(out, key);
    }
}

/// The keys of `path`, joined by `.`.
fn push_path(out: &mut String, path: &[&str]) {
    for (index, key) in path.iter().enumerate() {
        if index > 0 {
            out.push('.');
        }
        push_key(out, key);
    }
}

/// `text` in double quotes, with its quotes, backslashes and control
/// characters escaped.
fn push_string(out: &mut String, text: &str) {
    out.push('"');
    for character in text.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            control if control.is_control() => {
                let _: fmt::Result = write!(out, "\\u{:04X}", u32::from(control));
            }
            other => out.push(other),
        }
    }
    out.push('"');
}

/// `path` with `key` after it.
fn joined<'path>(path: &[&'path str], key: &'path str) -> Vec<&'path str> {
    let mut inner = path.to_vec();
    inner.push(key);
    inner
}
