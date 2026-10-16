use crate::parser::parse;

/// The message `source` fails with and the source text the error points at.
fn failure(source: &str) -> (String, &str) {
    let error = parse(source).unwrap_err();
    (error.message, &source[error.span])
}

/// The message `source` fails with and the rest of the line the error starts
/// on.
fn refusal(source: &str) -> (String, &str) {
    let error = parse(source).unwrap_err();
    let rest = &source[error.span.start..];
    (error.message, rest.lines().next().unwrap_or(rest))
}

#[test]
fn a_toml_key_written_twice_is_refused() {
    let (message, at) = failure("+++\na = 1\na = 2\n+++\n");
    assert_eq!(message, "the key `a` is written twice");
    assert_eq!(at, "a");
}

#[test]
fn a_yaml_key_written_twice_is_refused() {
    let (message, at) = failure("---\na: 1\na: 2\n---\n");
    assert_eq!(message, "the key `a` is written twice");
    assert_eq!(at, "a");
}

#[test]
fn the_error_names_the_byte_it_points_at() {
    let error = parse("+++\na = 1\na = 2\n+++\n").unwrap_err();
    assert_eq!(
        error.to_string(),
        "the key `a` is written twice (at byte 10)"
    );
}

#[test]
fn a_toml_value_that_is_no_type_is_refused() {
    let (message, at) = failure("+++\ndue = tomorrow\n+++\n");
    assert_eq!(message, "`tomorrow` is not a value");
    assert_eq!(at, "tomorrow");
}

#[test]
fn a_toml_string_left_open_is_refused() {
    let (message, _) = failure("+++\ntitle = \"open\n+++\n");
    assert_eq!(message, "the string is not closed");
}

#[test]
fn toml_constructs_not_read_yet_are_refused_by_name() {
    let (table, _) = failure("+++\n[owner]\nname = \"Ada\"\n+++\n");
    assert_eq!(table, "table headers are not read yet");

    let (dotted, _) = failure("+++\nowner.name = \"Ada\"\n+++\n");
    assert_eq!(dotted, "dotted keys are not read yet");

    let (multi_line, _) = failure("+++\ntext = \"\"\"\nlong\n\"\"\"\n+++\n");
    assert_eq!(multi_line, "multi-line strings are not read yet");
}

#[test]
fn a_toml_inline_table_stays_on_one_line() {
    let (message, _) = failure("+++\nt = { a = 1,\n  b = 2 }\n+++\n");
    assert_eq!(message, "expected a key");
}

#[test]
fn yaml_constructs_not_read_yet_are_refused_by_name() {
    let (block_scalar, _) = failure("---\ntext: |\n  line\n---\n");
    assert_eq!(block_scalar, "block scalars are not read yet");

    let (flow_mapping, _) = failure("---\nowner: {name: Ada}\n---\n");
    assert_eq!(flow_mapping, "flow mappings are not read yet");

    let (anchor, _) = failure("---\nbase: &base 1\n---\n");
    assert_eq!(anchor, "anchors, aliases and tags are not read yet");

    let (mapping_in_value, _) = failure("---\ntitle: Notes: draft\n---\n");
    assert_eq!(
        mapping_in_value,
        "a mapping written inside a value is not read yet"
    );

    let (list_over_two_lines, _) = failure("---\ntags: [a,\n  b]\n---\n");
    assert_eq!(list_over_two_lines, "expected `,` or `]` in the list");
}

#[test]
fn yaml_indentation_that_matches_no_block_is_refused() {
    let (deeper, _) = failure("---\na: 1\n  b: 2\n---\n");
    assert_eq!(deeper, "this line is indented more than the keys above it");

    let (shallower, _) = failure("---\n  a: 1\nb: 2\n---\n");
    assert_eq!(shallower, "this line is indented less than the first key");
}

#[test]
fn a_hugo_post_with_a_toml_table_is_refused() {
    let source = include_str!("fixtures/refused/hugo_post_with_table.md");
    assert_eq!(
        refusal(source),
        ("table headers are not read yet".to_owned(), "[params]")
    );
}

#[test]
fn a_toml_frontmatter_with_a_multi_line_string_is_refused() {
    let source = include_str!("fixtures/refused/toml_multi_line_string.md");
    assert_eq!(
        refusal(source),
        ("multi-line strings are not read yet".to_owned(), "\"\"")
    );
}

#[test]
fn a_pandoc_paper_with_a_block_scalar_is_refused() {
    let source = include_str!("fixtures/refused/pandoc_paper.md");
    assert_eq!(
        refusal(source),
        ("block scalars are not read yet".to_owned(), "|")
    );
}
