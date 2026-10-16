//! Frontmatter written in YAML: mappings in block form, lists of plain
//! values in flow form, and text in double quotes only when the reader would
//! read it plain as something else.

use core::fmt::{self, Write as _};

use crate::parser::ast::{Mapping, NonFinite, ScalarValue, Sequence, Value};
use crate::parser::scalar;
use crate::writer::{push_date_time, push_finite, push_local_date_time};

/// The characters that make a plain scalar starting with them something
/// else: the YAML indicators.
const INDICATORS: [char; 19] = [
    '!', '"', '#', '%', '&', '\'', '*', ',', '-', ':', '>', '?', '@', '[', ']', '`', '{', '|', '}',
];

/// The characters that end a plain scalar inside a flow collection.
const FLOW_INDICATORS: [char; 5] = [',', '[', ']', '{', '}'];

/// `mapping` between `---` lines.
pub fn frontmatter(mapping: &Mapping) -> String {
    let mut out = String::from("---\n");
    block_mapping(&mut out, mapping, 0, false);
    out.push_str("---\n");
    out
}

/// The pairs of `mapping`, each key at column `indent`. With `on_entry`, the
/// first key goes on the line a list entry's `-` started.
fn block_mapping(out: &mut String, mapping: &Mapping, indent: usize, on_entry: bool) {
    for (index, (key, pair)) in mapping.pairs.iter().enumerate() {
        if index > 0 || !on_entry {
            push_indent(out, indent);
        }
        push_text(out, key, false);
        out.push(':');
        after_indicator(out, &pair.value, indent + 2, false);
    }
}

/// The items of `sequence`, each `-` at column `indent`. With `on_entry`,
/// the first `-` goes on the line a list entry's `-` started.
fn block_sequence(out: &mut String, sequence: &Sequence, indent: usize, on_entry: bool) {
    for (index, item) in sequence.items.iter().enumerate() {
        if index > 0 || !on_entry {
            push_indent(out, indent);
        }
        out.push('-');
        after_indicator(out, item, indent + 2, true);
    }
}

/// `value` after a key's `:` or, `on_entry`, after a list entry's `-`: a
/// mapping, or a list that holds one, in block form at column `indent`, and
/// anything else on the same line.
fn after_indicator(out: &mut String, value: &Value, indent: usize, on_entry: bool) {
    let opening = if on_entry { ' ' } else { '\n' };
    match value {
        Value::Mapping(mapping) if !mapping.pairs.is_empty() => {
            out.push(opening);
            block_mapping(out, mapping, indent, on_entry);
        }
        Value::Sequence(sequence) if holds_mapping(sequence) => {
            out.push(opening);
            block_sequence(out, sequence, indent, on_entry);
        }
        Value::Mapping(_) | Value::Scalar(_) | Value::Sequence(_) => {
            out.push(' ');
            flow(out, value, false);
            out.push('\n');
        }
    }
}

/// Whether `sequence` holds a mapping, in itself or in a list inside it.
fn holds_mapping(sequence: &Sequence) -> bool {
    sequence.items.iter().any(|item| match item {
        Value::Mapping(_) => true,
        Value::Scalar(_) => false,
        Value::Sequence(inner) => holds_mapping(inner),
    })
}

/// `value` on one line, in flow form, inside a flow collection when
/// `in_flow`.
fn flow(out: &mut String, value: &Value, in_flow: bool) {
    match value {
        Value::Mapping(mapping) => {
            out.push('{');
            for (index, (key, pair)) in mapping.pairs.iter().enumerate() {
                if index > 0 {
                    out.push_str(", ");
                }
                push_text(out, key, true);
                out.push_str(": ");
                flow(out, &pair.value, true);
            }
            out.push('}');
        }
        Value::Scalar(scalar) => push_scalar(out, &scalar.value, in_flow),
        Value::Sequence(sequence) => {
            out.push('[');
            for (index, item) in sequence.items.iter().enumerate() {
                if index > 0 {
                    out.push_str(", ");
                }
                flow(out, item, true);
            }
            out.push(']');
        }
    }
}

/// A date or a date-time after the tag `!!timestamp`, a time of day as
/// quoted text, `null` for a null.
fn push_scalar(out: &mut String, value: &ScalarValue, in_flow: bool) {
    match value {
        ScalarValue::Boolean(boolean) => out.push_str(if *boolean { "true" } else { "false" }),
        ScalarValue::Date(date) => {
            let _: fmt::Result = write!(out, "!!timestamp {date}");
        }
        ScalarValue::DateTime(date_time) => {
            out.push_str("!!timestamp ");
            push_date_time(out, date_time);
        }
        ScalarValue::Float(float) => match NonFinite::of(*float) {
            Some(non_finite) => push_non_finite(out, non_finite),
            None => push_finite(out, *float),
        },
        ScalarValue::Integer(integer) => {
            let _: fmt::Result = write!(out, "{integer}");
        }
        ScalarValue::LocalDateTime(date_time) => {
            out.push_str("!!timestamp ");
            push_local_date_time(out, date_time);
        }
        ScalarValue::NonFinite(non_finite) => push_non_finite(out, *non_finite),
        ScalarValue::Null => out.push_str("null"),
        ScalarValue::String(text) => push_text(out, text, in_flow),
        ScalarValue::Time(time) => {
            let _: fmt::Result = write!(out, "\"{time}\"");
        }
    }
}

fn push_non_finite(out: &mut String, non_finite: NonFinite) {
    out.push_str(match non_finite {
        NonFinite::Infinity => ".inf",
        NonFinite::Nan => ".nan",
        NonFinite::NegativeInfinity => "-.inf",
    });
}

/// `text` plain, or in double quotes with its line breaks, quotes,
/// backslashes and control characters escaped.
fn push_text(out: &mut String, text: &str, in_flow: bool) {
    if !needs_quotes(text, in_flow) {
        out.push_str(text);
        return;
    }
    out.push('"');
    for character in text.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            control if control.is_control() => {
                let _: fmt::Result = write!(out, "\\x{:02X}", u32::from(control));
            }
            other => out.push(other),
        }
    }
    out.push('"');
}

/// Whether `text` written plain would read as something else: another type,
/// a key, a comment, an indicator, a flow collection's end, or text without
/// the spaces around it or its control characters.
fn needs_quotes(text: &str, in_flow: bool) -> bool {
    text.starts_with(INDICATORS)
        || text.starts_with(' ')
        || text.ends_with([' ', ':'])
        || text.contains(": ")
        || text.contains(" #")
        || text.contains(char::is_control)
        || (in_flow && text.contains(FLOW_INDICATORS))
        || !matches!(scalar::yaml(text), ScalarValue::String(_))
}

fn push_indent(out: &mut String, indent: usize) {
    out.push_str(&" ".repeat(indent));
}
