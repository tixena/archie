use core::fmt::Write as _;
use std::fs;
use std::path::Path;

use crate::field::Field;
use crate::resource::Resource;

fn fixture(name: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    fs::read_to_string(path).unwrap()
}

fn render(fields: &[(String, Field)]) -> String {
    let mut out = String::new();
    for (key, value) in fields {
        render_into(&mut out, key, value);
    }
    out
}

fn render_into(out: &mut String, path: &str, field: &Field) {
    match field {
        Field::Map(pairs) => {
            for (key, value) in pairs {
                render_into(out, &format!("{path}.{key}"), value);
            }
        }
        Field::List(items) => {
            for (index, value) in items.iter().enumerate() {
                render_into(out, &format!("{path}[{index}]"), value);
            }
        }
        Field::Str(text) => {
            let _ = writeln!(out, "{path} = \"{text}\"");
        }
        other @ (Field::Bool(_)
        | Field::Date(_)
        | Field::DateTime(_)
        | Field::Float(_)
        | Field::Int(_)
        | Field::Null
        | Field::Time(_)) => {
            let _ = writeln!(out, "{path} = {other}");
        }
    }
}

fn resource(source: &str) -> Resource {
    Resource::from_source(source).unwrap().unwrap()
}

#[test]
fn every_fixture_reads_to_its_expected_fields() {
    for name in [
        "toml_flat",
        "toml_dates",
        "yaml_flat",
        "yaml_lists",
        "toml_inline_table",
    ] {
        let read = resource(&fixture(&format!("{name}.md")));
        assert_eq!(
            render(&read.fields),
            fixture(&format!("{name}.fields.txt")),
            "{name}"
        );
    }
}

#[test]
fn a_toml_document_with_a_type_is_a_resource_of_that_kind() {
    assert_eq!(
        resource(&fixture("toml_flat.md")).kind.as_deref(),
        Some("task")
    );
}

#[test]
fn a_yaml_document_reads_the_same_fields() {
    let yaml = "---\ntype: task\ntitle: Write the parser\npriority: 2\nweight: 1.5\ndone: false\n---\n\nBody.\n";
    assert_eq!(
        resource(yaml).fields,
        resource(&fixture("toml_flat.md")).fields
    );
}

#[test]
fn no_frontmatter_is_not_a_resource() {
    assert!(
        Resource::from_source(&fixture("no_frontmatter.md"))
            .unwrap()
            .is_none()
    );
}

#[test]
fn a_non_string_type_gives_no_kind() {
    let read = resource("+++\ntype = 3\n+++\n");
    assert_eq!(read.kind, None);
    assert_eq!(read.field(&["type"]), Some(&Field::Int(3)));
}

#[test]
fn nested_fields_are_reachable_by_path() {
    let read = resource(&fixture("toml_inline_table.md"));
    assert_eq!(
        read.field(&["phone", "work"]),
        Some(&Field::Str(String::from("555-0199")))
    );
    assert_eq!(read.field(&["phone", "fax"]), None);
}

#[test]
fn a_local_datetime_keeps_no_offset() {
    let read = resource(&fixture("toml_dates.md"));
    assert!(matches!(
        read.field(&["local"]),
        Some(Field::DateTime(local)) if local.offset_minutes.is_none()
    ));
}
