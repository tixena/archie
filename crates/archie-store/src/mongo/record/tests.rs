//! The record: each kind of frontmatter value stored and read back, the
//! fixtures, an invalid document, records that do not read, the depth, the
//! filter and the revision.

use archie_core::parser::ast::{InvalidDocument, Mapping, NonFinite, RawDocument, Value};
use archie_core::parser::{from_json, parse, to_json};
use bson::oid::ObjectId;
use bson::{Bson, DateTime as BsonDateTime, Document, doc};
use chrono::{DateTime, NaiveDate, NaiveTime, Timelike as _, Utc};

use super::{StoredDocument, depth, from_record, frontmatter_is, revision_of, to_record};
use crate::StoreError;

const FIXTURES: [&str; 21] = [
    include_str!("../../../../archie-core/src/parser/tests/fixtures/api_reference.md"),
    include_str!("../../../../archie-core/src/parser/tests/fixtures/book_page.md"),
    include_str!("../../../../archie-core/src/parser/tests/fixtures/changelog.md"),
    include_str!("../../../../archie-core/src/parser/tests/fixtures/contact.md"),
    include_str!("../../../../archie-core/src/parser/tests/fixtures/daily_note.md"),
    include_str!("../../../../archie-core/src/parser/tests/fixtures/docusaurus_page.md"),
    include_str!("../../../../archie-core/src/parser/tests/fixtures/essay.md"),
    include_str!("../../../../archie-core/src/parser/tests/fixtures/hugo_post.md"),
    include_str!("../../../../archie-core/src/parser/tests/fixtures/hugo_post_with_table.md"),
    include_str!("../../../../archie-core/src/parser/tests/fixtures/issue_template.md"),
    include_str!("../../../../archie-core/src/parser/tests/fixtures/jekyll_post.md"),
    include_str!("../../../../archie-core/src/parser/tests/fixtures/meeting_notes.md"),
    include_str!("../../../../archie-core/src/parser/tests/fixtures/obsidian_note.md"),
    include_str!("../../../../archie-core/src/parser/tests/fixtures/pandoc_paper.md"),
    include_str!("../../../../archie-core/src/parser/tests/fixtures/readme.md"),
    include_str!("../../../../archie-core/src/parser/tests/fixtures/recipe.md"),
    include_str!("../../../../archie-core/src/parser/tests/fixtures/slash_command.md"),
    include_str!("../../../../archie-core/src/parser/tests/fixtures/task.md"),
    include_str!("../../../../archie-core/src/parser/tests/fixtures/toml_multi_line_string.md"),
    include_str!("../../../../archie-core/src/parser/tests/fixtures/yaml_list_of_mappings.md"),
    include_str!("../../../../archie-core/src/parser/tests/fixtures/zola_page.md"),
];

const ID: &str = "6ad42f681472f05ab2ed9be7";

fn data(record: &Document) -> &Document {
    record
        .get_document("doc")
        .unwrap()
        .get_document("data")
        .unwrap()
}

fn data_mut(record: &mut Document) -> &mut Document {
    record
        .get_document_mut("doc")
        .unwrap()
        .get_document_mut("data")
        .unwrap()
}

fn date() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 10, 20).unwrap()
}

fn frontmatter(stored: &StoredDocument) -> Option<&Mapping> {
    let RawDocument::Valid(document) = &stored.document else {
        return None;
    };
    document.frontmatter.as_ref()
}

fn instant(text: &str) -> BsonDateTime {
    BsonDateTime::parse_rfc3339_str(text).unwrap()
}

fn invalid(document: &RawDocument) -> Option<&InvalidDocument> {
    let RawDocument::Invalid(invalid) = document else {
        return None;
    };
    Some(invalid)
}

fn mapping(pairs: Vec<(&str, Value)>) -> Mapping {
    pairs
        .into_iter()
        .map(|(key, value)| (key.to_owned(), value))
        .collect()
}

/// A file whose frontmatter holds `count` mappings, each inside the one before.
fn nested_mappings(count: usize) -> StoredDocument {
    let mut value = Value::Null;
    for _ in 0..count {
        value = Value::Mapping(mapping(vec![("a", value)]));
    }
    with_frontmatter(mapping(vec![("a", value)]))
}

/// A paragraph inside `count` list items, each inside the one before.
fn nested_items(count: usize) -> String {
    let mut text = String::new();
    for level in 0..count {
        text.push_str(&"  ".repeat(level));
        text.push_str("- item\n");
    }
    text
}

fn offset(text: &str) -> Value {
    Value::OffsetDateTime(DateTime::parse_from_rfc3339(text).unwrap())
}

/// The tree `document` keeps once stored: each node's `kind` left out, as
/// JSON leaves it out.
fn plain_tree(document: &RawDocument) -> RawDocument {
    from_json(&to_json(document).unwrap()).unwrap()
}

fn stored(path: &str, text: &str) -> StoredDocument {
    StoredDocument {
        document: parse(text).document,
        id: ObjectId::parse_str(ID).unwrap(),
        path: path.to_owned(),
        revision: revision_of(text).0,
    }
}

fn string(text: &str) -> Value {
    Value::String(text.to_owned())
}

/// What MongoDB gives back for `value`: an offset written as UTC, a date-time
/// at midnight UTC as a date, a time to the millisecond.
fn through_mongodb(value: Value) -> Value {
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
                .map(|(key, item)| (key, through_mongodb(item)))
                .collect(),
        ),
        Value::OffsetDateTime(at) => in_utc(at.to_utc()),
        Value::Sequence(items) => Value::Sequence(items.into_iter().map(through_mongodb).collect()),
        Value::Time(time) => {
            let millisecond = time.nanosecond() / 1_000_000 * 1_000_000;
            Value::Time(time.with_nanosecond(millisecond).unwrap())
        }
        Value::Boolean(_)
        | Value::Date(_)
        | Value::Float(_)
        | Value::Integer(_)
        | Value::NonFinite(_)
        | Value::Null
        | Value::String(_)
        | _ => value,
    }
}

/// A file of one heading whose frontmatter is `frontmatter`.
fn with_frontmatter(frontmatter: Mapping) -> StoredDocument {
    let mut written = stored("notes/notes.md", "# Notes\n");
    if let RawDocument::Valid(document) = &mut written.document {
        document.frontmatter = Some(frontmatter);
    }
    written
}

#[test]
fn each_kind_of_value_is_stored_as_plain_bson_and_read_back() {
    let seven = NaiveTime::from_hms_opt(7, 32, 0).unwrap();
    let written = with_frontmatter(mapping(vec![
        ("title", string("Notes")),
        ("priority", Value::Integer(2)),
        ("ratio", Value::Float(0.5)),
        ("done", Value::Boolean(false)),
        ("none", Value::Null),
        (
            "tags",
            Value::Sequence(vec![string("parser"), string("design")]),
        ),
        (
            "owner",
            Value::Mapping(mapping(vec![("name", string("Ada"))])),
        ),
        ("due", Value::Date(date())),
        ("offset", offset("2026-10-20T09:00:00+02:00")),
        ("midnight", offset("2026-10-20T00:00:00Z")),
        (
            "local",
            Value::DateTime(date().and_hms_opt(9, 0, 0).unwrap()),
        ),
        ("at", Value::Time(seven)),
        (
            "at_ms",
            Value::Time(NaiveTime::from_hms_micro_opt(7, 32, 0, 250_123).unwrap()),
        ),
        ("infinity", Value::NonFinite(NonFinite::Infinity)),
        ("negative", Value::NonFinite(NonFinite::NegativeInfinity)),
        ("nan", Value::NonFinite(NonFinite::Nan)),
    ]));
    let mut record = to_record(&written).unwrap();
    let nan = data_mut(&mut record).remove("nan").unwrap();
    assert!(matches!(nan, Bson::Double(number) if number.is_nan()));
    let expected = doc! {
        "title": "Notes",
        "priority": 2_i64,
        "ratio": 0.5_f64,
        "done": false,
        "none": Bson::Null,
        "tags": ["parser", "design"],
        "owner": { "name": "Ada" },
        "due": instant("2026-10-20T00:00:00Z"),
        "offset": instant("2026-10-20T07:00:00Z"),
        "midnight": instant("2026-10-20T00:00:00Z"),
        "local": instant("2026-10-20T09:00:00Z"),
        "at": { "t": "time", "v": "07:32:00" },
        "at_ms": { "t": "time", "v": "07:32:00.250" },
        "infinity": f64::INFINITY,
        "negative": f64::NEG_INFINITY,
    };
    assert_eq!(data(&record), &expected);

    data_mut(&mut record).insert("nan", nan);
    let back = from_record(record).unwrap();
    let read = mapping(vec![
        ("title", string("Notes")),
        ("priority", Value::Integer(2)),
        ("ratio", Value::Float(0.5)),
        ("done", Value::Boolean(false)),
        ("none", Value::Null),
        (
            "tags",
            Value::Sequence(vec![string("parser"), string("design")]),
        ),
        (
            "owner",
            Value::Mapping(mapping(vec![("name", string("Ada"))])),
        ),
        ("due", Value::Date(date())),
        ("offset", offset("2026-10-20T07:00:00Z")),
        ("midnight", Value::Date(date())),
        ("local", offset("2026-10-20T09:00:00Z")),
        ("at", Value::Time(seven)),
        (
            "at_ms",
            Value::Time(NaiveTime::from_hms_milli_opt(7, 32, 0, 250).unwrap()),
        ),
        ("infinity", Value::NonFinite(NonFinite::Infinity)),
        ("negative", Value::NonFinite(NonFinite::NegativeInfinity)),
        ("nan", Value::NonFinite(NonFinite::Nan)),
    ]);
    assert_eq!(frontmatter(&back), Some(&read));
}

#[test]
fn a_file_with_a_frontmatter_is_stored_and_read_back() {
    let text = "---\nstatus: open\ndue: 2026-10-20\n---\n# Task\n";
    let written = stored("notes/task.md", text);
    let record = to_record(&written).unwrap();
    assert_eq!(data(&record).get_str("status").unwrap(), "open");
    assert!(matches!(data(&record).get("due"), Some(Bson::DateTime(_))));
    assert_eq!(from_record(record).unwrap(), written);
}

#[test]
fn each_fixture_reads_back_with_the_changes_mongodb_makes() {
    for source in FIXTURES {
        let written = stored("notes/fixture.md", source);
        let back = from_record(to_record(&written).unwrap()).unwrap();
        let mut expected = written.clone();
        expected.document = plain_tree(&written.document);
        if let RawDocument::Valid(document) = &mut expected.document {
            document.frontmatter = document.frontmatter.take().map(|kept| {
                kept.into_iter()
                    .map(|(key, value)| (key, through_mongodb(value)))
                    .collect()
            });
        }
        assert_eq!(back, expected, "{source}");
    }
}

#[test]
fn an_invalid_document_keeps_its_frontmatter_as_written() {
    let written = stored("notes/broken.md", "---\n: : :\n---\ntext\n");
    let kept = invalid(&written.document).unwrap();
    let record = to_record(&written).unwrap();
    let as_serde = bson::serialize_to_document(&kept.frontmatter).unwrap();
    assert_eq!(data(&record), &as_serde);
    assert_eq!(from_record(record).unwrap(), written);
}

#[test]
fn a_document_of_t_and_v_is_a_time_only_when_v_is_a_time() {
    let lookalike = mapping(vec![("t", string("time")), ("v", string("nine"))]);
    let clock = mapping(vec![("t", string("time")), ("v", string("07:32:00"))]);
    let written = with_frontmatter(mapping(vec![
        ("nine", Value::Mapping(lookalike.clone())),
        ("clock", Value::Mapping(clock)),
    ]));
    let mut record = to_record(&written).unwrap();
    data_mut(&mut record).insert("count", Bson::Int32(3));
    let back = from_record(record).unwrap();
    let read = mapping(vec![
        ("nine", Value::Mapping(lookalike)),
        (
            "clock",
            Value::Time(NaiveTime::from_hms_opt(7, 32, 0).unwrap()),
        ),
        ("count", Value::Integer(3)),
    ]);
    assert_eq!(frontmatter(&back), Some(&read));
}

#[test]
fn a_record_that_does_not_read_names_its_id_and_what_is_wrong() {
    let written = with_frontmatter(mapping(vec![("status", string("open"))]));
    let mut foreign = to_record(&written).unwrap();
    data_mut(&mut foreign).insert("owner", ObjectId::parse_str(ID).unwrap());
    let no_row = "the record ObjectId(\"6ad42f681472f05ab2ed9be7\") does not read: a frontmatter value cannot be a BSON ObjectId";
    assert_eq!(
        from_record(foreign),
        Err(StoreError::Backend(no_row.to_owned()))
    );

    let mut broken = to_record(&written).unwrap();
    broken.remove("path");
    broken.insert("text", "# Notes\n");
    let issues = "the record ObjectId(\"6ad42f681472f05ab2ed9be7\") does not read: path: missing: expected String\ntext: unknown: found String(\"# Notes\\n\")";
    assert_eq!(
        from_record(broken),
        Err(StoreError::Backend(issues.to_owned()))
    );
}

#[test]
fn a_record_deeper_than_mongodb_holds_is_not_stored() {
    assert_eq!(
        depth(&to_record(&stored("notes/one.md", "text\n")).unwrap()),
        11
    );
    let twelve = to_record(&stored("notes/deep.md", &nested_items(12))).unwrap();
    assert_eq!(depth(&twelve), 95);
    let refused = "notes/deep.md nests 102 levels, and MongoDB holds 100";
    assert_eq!(
        to_record(&stored("notes/deep.md", &nested_items(13))),
        Err(StoreError::Backend(refused.to_owned()))
    );
}

#[test]
fn a_frontmatter_value_98_mappings_deep_is_not_stored() {
    assert_eq!(depth(&to_record(&nested_mappings(97)).unwrap()), 100);
    let refused = "notes/notes.md nests 101 levels, and MongoDB holds 100";
    assert_eq!(
        to_record(&nested_mappings(98)),
        Err(StoreError::Backend(refused.to_owned()))
    );
}

#[test]
fn a_frontmatter_value_400_mappings_deep_is_refused_before_it_is_written() {
    let refused = "notes/notes.md nests 403 levels, and MongoDB holds 100";
    assert_eq!(
        to_record(&nested_mappings(400)),
        Err(StoreError::Backend(refused.to_owned()))
    );
}

#[test]
fn the_filter_on_a_frontmatter_key_names_doc_data() {
    let status = frontmatter_is("status", &string("open"));
    assert_eq!(status.to_string(), r#"{ "doc.data.status": "open" }"#);
    let due = frontmatter_is("due", &Value::Date(date()));
    assert_eq!(
        due,
        doc! { "doc.data.due": instant("2026-10-20T00:00:00Z") }
    );
}

#[test]
fn the_revision_is_the_sha_256_of_the_text() {
    assert_eq!(
        revision_of("").0,
        "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
}
