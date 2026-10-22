//! Each form of a document: its bytes, the document read back from them, and
//! the text a store keeps for them.

use chrono::{DateTime, NaiveTime, Timelike as _, Utc};

use super::{DecodeError, EncodeError, Form, FormError, decode, encode, stored_text};
use crate::parser::ast::{FrontmatterFormat, RawDocument, Value};
use crate::parser::parse;
use crate::parser::tests::json::unclaim;
use crate::parser::tests::rkyv::FIXTURES;
use crate::writer::{WriteError, WriteOptions, write};

const FORMS: [Form; 3] = [Form::Json, Form::Markdown, Form::Binary];

const TOML: WriteOptions = WriteOptions {
    frontmatter: FrontmatterFormat::Toml,
};

/// The binary form of `toml_multi_line_string.md`. A change to a model type
/// changes it, and raises `BINARY_VERSION` with it.
const PINNED: &[u8] =
    b"archie\0\x03\x07\0\0\0Body.\xff\xff\xff\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\
    \xdc\xff\xff\xff\x01\0\0\0\0\0\0\0\x04\0\0\0\xf0\xff\xff\xff\x01\0\0\0\0\0\0\0\0\0\0\0\0\0\
    \0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\xd0\xff\xff\xff\x01\0\0\0\0\0\0\0\0\0\
    \0\0\0\0\0\0\xec\xff\xff\xff\0\0\0\0\0\0\0\0\0\0\0\0\x01\0\0\0h\xff4h\xff4\xff\xff\xff\xff\
    \xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xffShippi\
    ng the parserdescriptionThe parser reads a file\nin two stages.\0\0\0\0\0title\xff\xff\xff\
    \t\0\0\0\x93\0\0\0\xab\xff\xff\xff\0\0\0\0\0\0\0\0\0\0\0\0\x8b\0\0\0\xaa\xff\xff\xff\t\0\0\
    \0\xa6\0\0\0\xa9\xff\xff\xff\0\0\0\0\0\0\0\0\0\0\0\0\x01\0\0\0,\xff\xff\xff\x01\0\0\0\x01\
    \0\0\0H\xff\xff\xff\x02\0\0\0\x03\0\0\0\xa4\xff\xff\xff";

/// The text a store keeps for `document` put into `form`.
fn stored(document: &RawDocument, form: Form, options: &WriteOptions) -> Result<String, FormError> {
    let bytes = encode(document, form, options)?;
    stored_text(&bytes, form, options)
}

/// `document` with each frontmatter value as the JSON form keeps it.
fn through_json(document: &RawDocument) -> RawDocument {
    let mut kept = document.clone();
    if let RawDocument::Valid(valid) = &mut kept
        && let Some(mapping) = &mut valid.frontmatter
    {
        for value in mapping.values_mut() {
            *value = value_through_json(value.clone());
        }
    }
    kept
}

/// What the JSON form keeps of `value`: a date-time in UTC, or a date when it
/// is at midnight UTC, and a time to the millisecond.
fn value_through_json(value: Value) -> Value {
    let in_utc = |at: DateTime<Utc>| {
        if at.time() == NaiveTime::MIN {
            Value::Date(at.date_naive())
        } else {
            Value::OffsetDateTime(at.fixed_offset())
        }
    };
    match value {
        Value::DateTime(at) => in_utc(at.and_utc()),
        Value::Mapping(inner) => Value::Mapping(
            inner
                .into_iter()
                .map(|(key, item)| (key, value_through_json(item)))
                .collect(),
        ),
        Value::OffsetDateTime(at) => in_utc(at.to_utc()),
        Value::Sequence(items) => {
            Value::Sequence(items.into_iter().map(value_through_json).collect())
        }
        Value::Time(time) => {
            let millisecond = time.nanosecond() / 1_000_000 * 1_000_000;
            Value::Time(time.with_nanosecond(millisecond).unwrap())
        }
        Value::Boolean(_)
        | Value::Date(_)
        | Value::Float(_)
        | Value::Integer(_)
        | Value::Null(())
        | Value::String(_) => value,
    }
}

/// The writer's error `form_error` holds, if it holds one.
fn write_error(form_error: FormError) -> Option<WriteError> {
    match form_error {
        FormError::Encode(EncodeError::Write(error)) => Some(error),
        FormError::Decode(_) | FormError::Encode(EncodeError::Binary(_) | EncodeError::Json(_)) => {
            None
        }
    }
}

#[test]
fn the_sample_puts_a_list_into_each_form() {
    let options = WriteOptions::default();
    let document = decode(b"* item\n", Form::Markdown).unwrap();
    let json = encode(&document, Form::Json, &options).unwrap();
    assert!(json.ends_with(b"\n"));
    let binary = encode(&document, Form::Binary, &options).unwrap();
    assert_eq!(&binary[..8], b"archie\0\x03");
    assert_eq!(
        stored_text(&binary, Form::Binary, &options).unwrap(),
        "- item\n"
    );
    assert_eq!(
        stored_text(&json, Form::Json, &options).unwrap(),
        "- item\n"
    );
    let mut wrong = binary;
    wrong[7] = 1;
    assert!(matches!(
        decode(&wrong, Form::Binary),
        Err(DecodeError::Version { found: 1 })
    ));
}

#[test]
fn each_fixture_is_stored_from_each_form_as_write_writes_it_and_a_second_pass_changes_nothing() {
    for source in FIXTURES {
        let document = parse(source).document;
        let kept = through_json(&document);
        for form in FORMS {
            let expected = if form == Form::Json { &kept } else { &document };
            for options in [WriteOptions::default(), TOML] {
                let text = stored(&document, form, &options).map_err(write_error);
                assert_eq!(
                    text,
                    write(expected, &options).map_err(Some),
                    "{form:?} {options:?}: {source}"
                );
                if let Ok(first) = text {
                    let again = stored(&parse(&first).document, form, &options);
                    assert_eq!(
                        again.map_err(write_error),
                        Ok(first),
                        "{form:?} {options:?}: {source}"
                    );
                }
            }
        }
    }
}

#[test]
fn each_fixture_reads_back_from_the_binary_form_every_kind_empty() {
    for source in FIXTURES {
        let mut document = parse(source).document;
        let bytes = encode(&document, Form::Binary, &WriteOptions::default()).unwrap();
        unclaim(&mut document);
        assert_eq!(decode(&bytes, Form::Binary).unwrap(), document, "{source}");
    }
}

#[test]
fn bytes_that_are_not_utf8_fail_naming_the_offset_of_the_first_byte_that_is_not() {
    for form in [Form::Json, Form::Markdown] {
        assert!(matches!(
            decode(b"a\xffb", form),
            Err(DecodeError::NotUtf8 { offset: 1 })
        ));
        assert!(matches!(
            stored_text(b"a\xffb", form, &WriteOptions::default()),
            Err(FormError::Decode(DecodeError::NotUtf8 { offset: 1 }))
        ));
    }
}

#[test]
fn binary_bytes_that_do_not_open_with_archie_a_zero_byte_and_a_version_are_not_archies() {
    for bytes in [b"bincode!".as_slice(), b"archie\0", b"archie", b""] {
        assert!(matches!(
            decode(bytes, Form::Binary),
            Err(DecodeError::NotArchie)
        ));
        assert!(matches!(
            stored_text(bytes, Form::Binary, &WriteOptions::default()),
            Err(FormError::Decode(DecodeError::NotArchie))
        ));
    }
}

#[test]
fn binary_bytes_of_another_version_fail_naming_it() {
    let document = parse(FIXTURES[0]).document;
    let bytes = encode(&document, Form::Binary, &WriteOptions::default()).unwrap();
    for version in [0, 1, 2, 4, u8::MAX] {
        let mut other = bytes.clone();
        other[7] = version;
        assert!(matches!(
            decode(&other, Form::Binary),
            Err(DecodeError::Version { found }) if found == version
        ));
        assert!(matches!(
            stored_text(&other, Form::Binary, &WriteOptions::default()),
            Err(FormError::Decode(DecodeError::Version { found })) if found == version
        ));
    }
}

#[test]
fn the_json_form_ends_with_one_newline_and_reads_with_or_without_it() {
    for source in FIXTURES {
        let mut document = parse(source).document;
        let json = encode(&document, Form::Json, &WriteOptions::default()).unwrap();
        let (&last, value) = json.split_last().unwrap();
        assert_eq!(last, b'\n', "{source}");
        assert!(!value.ends_with(b"\n"), "{source}");
        unclaim(&mut document);
        assert_eq!(decode(&json, Form::Json).unwrap(), document, "{source}");
        assert_eq!(decode(value, Form::Json).unwrap(), document, "{source}");
    }
}

#[test]
fn a_yaml_fixture_is_stored_with_toml_frontmatter_from_each_form_when_the_options_name_toml() {
    let source = include_str!("../parser/tests/fixtures/jekyll_post.md");
    assert!(source.starts_with("---\n"));
    let document = parse(source).document;
    for form in FORMS {
        let bytes = encode(&document, form, &WriteOptions::default()).unwrap();
        let text = stored_text(&bytes, form, &TOML).unwrap();
        assert!(text.starts_with("+++\n"), "{form:?}: {text}");
        assert_eq!(text, write(&document, &TOML).unwrap(), "{form:?}");
    }
}

#[test]
fn the_binary_form_of_the_smallest_fixture_is_pinned() {
    let source = include_str!("../parser/tests/fixtures/toml_multi_line_string.md");
    let document = parse(source).document;
    let bytes = encode(&document, Form::Binary, &WriteOptions::default()).unwrap();
    assert_eq!(bytes, PINNED);
}
