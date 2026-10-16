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
fn the_error_names_the_byte_it_points_at() {
    let error = parse("+++\ndue = tomorrow\n+++\n").unwrap_err();
    assert_eq!(error.to_string(), "`tomorrow` is not a value (at byte 10)");
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
fn a_toml_table_header_or_multi_line_string_left_open_is_refused() {
    let (header, _) = failure("+++\n[owner\nname = \"Ada\"\n+++\n");
    assert_eq!(header, "expected `]` to close the table header");

    let (array, _) = failure("+++\n[[owner]\nname = \"Ada\"\n+++\n");
    assert_eq!(array, "expected `]` to close the table header");

    let (basic, _) = failure("+++\ntext = \"\"\"\nlong\n+++\n");
    assert_eq!(basic, "the string is not closed");

    let (literal, _) = failure("+++\ntext = '''\nlong\n+++\n");
    assert_eq!(literal, "the string is not closed");
}

#[test]
fn a_toml_inline_table_stays_on_one_line() {
    let (message, _) = failure("+++\nt = { a = 1,\n  b = 2 }\n+++\n");
    assert_eq!(message, "expected a key");
}

/// The message an error names, with what it points at.
fn named(message: &str, at: &str) -> (String, String) {
    (message.to_owned(), at.to_owned())
}

/// [`failure`] with owned parts, to compare with [`named`].
fn failed(source: &str) -> (String, String) {
    let (message, at) = failure(source);
    (message, at.to_owned())
}

#[test]
fn a_tag_archie_does_not_read_is_refused_by_name() {
    assert_eq!(
        failed("---\norigin: !point 3,4\n---\n"),
        named("the tag `!point` is not one archie reads", "!point")
    );
    assert_eq!(
        failed("---\ndata: !!binary R0lG\n---\n"),
        named("the tag `!!binary` is not one archie reads", "!!binary")
    );
    assert_eq!(
        failed("---\norigin: !<tag:example.com,2026:point> 3,4\n---\n"),
        named(
            "the tag `!<tag:example.com,2026:point>` is not one archie reads",
            "!<tag:example.com,2026:point>"
        )
    );
    assert_eq!(
        failed("---\npoints: [!point 3]\n---\n"),
        named("the tag `!point` is not one archie reads", "!point")
    );
    assert_eq!(
        failed("---\n!point origin: 3\n---\n"),
        named("the tag `!point` is not one archie reads", "!point")
    );
}

#[test]
fn a_value_that_does_not_fit_its_tag_is_refused() {
    assert_eq!(
        failed("---\nn: !!int abc\n---\n"),
        named("`abc` is not a value of the tag `!!int`", "abc")
    );
    assert_eq!(
        failed("---\nratio: !!float 0x1F\n---\n"),
        named("`0x1F` is not a value of the tag `!!float`", "0x1F")
    );
    assert_eq!(
        failed("---\ndone: !!bool yes\n---\n"),
        named("`yes` is not a value of the tag `!!bool`", "yes")
    );
    assert_eq!(
        failed("---\nnothing: !!null \"x\"\n---\n"),
        named("`x` is not a value of the tag `!!null`", "\"x\"")
    );
    assert_eq!(
        failed("---\ndue: !!timestamp 2026-02-30\n---\n"),
        named(
            "`2026-02-30` is not a value of the tag `!!timestamp`",
            "2026-02-30"
        )
    );
    assert_eq!(
        failed("---\nowner: !!map Ada\n---\n"),
        named("`Ada` is not a value of the tag `!!map`", "Ada")
    );
    assert_eq!(
        failed("---\ntags: !!seq {a: 1}\n---\n"),
        named("a mapping is not a value of the tag `!!seq`", "{a: 1}")
    );
    assert_eq!(
        failed("---\ntitle: !!str\n  - a\n---\n"),
        named("a list is not a value of the tag `!!str`", "- a")
    );
    assert_eq!(
        failed("---\n!!int one: 1\n---\n"),
        named("`one` is not a value of the tag `!!int`", "one")
    );
}

#[test]
fn anchors_and_aliases_are_refused() {
    let refused = |at: &str| named("anchors and aliases are not read", at);
    assert_eq!(failed("---\nbase: &base 1\n---\n"), refused("&"));
    assert_eq!(failed("---\na: 1\nb: *a\n---\n"), refused("*"));
    assert_eq!(
        failed("---\nitems:\n  - &first 1\n  - [*first]\n---\n"),
        refused("&")
    );
    assert_eq!(failed("---\n&key a: 1\n---\n"), refused("&"));
    assert_eq!(failed("---\na: !!str *a\n---\n"), refused("*"));
}

#[test]
fn a_plain_value_cannot_hold_a_colon_and_a_space() {
    assert_eq!(
        refusal("---\ntitle: Notes: draft\n---\n"),
        (
            "a plain value cannot hold `: `; quote it".to_owned(),
            "Notes: draft"
        )
    );
    assert_eq!(
        refusal("---\ntags: [a: b]\n---\n"),
        (
            "a plain value cannot hold `: `; quote it".to_owned(),
            "a: b]"
        )
    );
}

#[test]
fn a_yaml_flow_collection_left_open_is_refused() {
    let (list, _) = failure("---\ntags: [a, b\n---\n");
    assert_eq!(list, "expected `,` or `]` in the list");

    let (mapping, _) = failure("---\nowner: {name: Ada\n---\n");
    assert_eq!(mapping, "expected `,` or `}` in the mapping");

    let (no_key, _) = failure("---\nowner: {: Ada}\n---\n");
    assert_eq!(no_key, "expected a key");
}

#[test]
fn a_block_scalar_with_a_bad_header_or_a_deeper_blank_line_is_refused() {
    let (header, _) = failure("---\ntext: | words\n  line\n---\n");
    assert_eq!(header, "expected the end of the line after the value");

    assert_eq!(
        refusal("---\ntext: |\n    \n  line\n---\n"),
        (
            "a blank line before the text of a block scalar is indented more than the text"
                .to_owned(),
            "  line"
        )
    );
}

#[test]
fn a_toml_date_that_does_not_exist_is_refused() {
    assert_eq!(
        failed("+++\ndue = 2026-02-30\n+++\n"),
        named("`2026-02-30` is not a value", "2026-02-30")
    );
    assert_eq!(
        failed("+++\nat = 2026-04-31 10:00:00\n+++\n"),
        named("`2026-04-31` is not a value", "2026-04-31")
    );
}

#[test]
fn yaml_indentation_that_matches_no_block_is_refused() {
    let (deeper, _) = failure("---\na: 1\n  b: 2\n---\n");
    assert_eq!(deeper, "this line is indented more than the keys above it");

    let (shallower, _) = failure("---\n  a: 1\nb: 2\n---\n");
    assert_eq!(shallower, "this line is indented less than the first key");
}
