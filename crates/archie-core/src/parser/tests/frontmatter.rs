use super::{check, check_spans};
use crate::parser::parse;

/// The outline of a YAML frontmatter that holds no pair, before an empty body.
const EMPTY_YAML: &str = "Document
  Frontmatter format=Yaml
    Mapping
  MarkdownBlock
";

#[test]
fn a_file_with_no_delimiter_on_its_first_line_has_no_frontmatter() {
    check(
        "Just text.\n",
        r#"Document
  MarkdownBlock
    Section
      Paragraph
        Line
          Text "Just text."
"#,
    );
}

#[test]
fn an_opening_delimiter_with_no_closing_one_is_a_thematic_break() {
    check(
        "\
---
title: x

No closing line.
",
        r#"Document
  MarkdownBlock
    Section
      ThematicBreak
      Paragraph
        Line
          Text "title: x"
      Paragraph
        Line
          Text "No closing line."
"#,
    );
}

#[test]
fn an_empty_frontmatter_is_a_mapping_with_no_pairs() {
    check(
        "\
+++
+++
Body.
",
        r#"Document
  Frontmatter format=Toml
    Mapping
  MarkdownBlock
    Section
      Paragraph
        Line
          Text "Body."
"#,
    );
}

#[test]
fn a_delimiter_after_the_first_line_is_a_thematic_break() {
    check(
        "\
Text.

---

More.
",
        r#"Document
  MarkdownBlock
    Section
      Paragraph
        Line
          Text "Text."
      ThematicBreak
      Paragraph
        Line
          Text "More."
"#,
    );
}

#[test]
fn three_dots_also_close_a_yaml_frontmatter() {
    check(
        "\
---
a: 1
...
",
        "\
Document
  Frontmatter format=Yaml
    Mapping
      Pair key=a
        Scalar Integer 1
  MarkdownBlock
",
    );
}

#[test]
fn windows_line_endings_read_the_same() {
    check(
        "---\r\ntitle: x\r\n---\r\n# H\r\n\r\n- a\r\n  b\r\n",
        r#"Document
  Frontmatter format=Yaml
    Mapping
      Pair key=title
        Scalar String "x"
  MarkdownBlock
    Section
      Heading
        Text "H"
      List
        Item
          MarkdownBlock
            Section
              Paragraph
                Line
                  Text "a"
                Line
                  Text "b"
"#,
    );
}

#[test]
fn a_frontmatter_may_end_the_file_with_no_line_ending() {
    check(
        "---\na: 1\n---",
        "\
Document
  Frontmatter format=Yaml
    Mapping
      Pair key=a
        Scalar Integer 1
  MarkdownBlock
",
    );
}

#[test]
fn spaces_may_follow_a_delimiter_but_not_come_before_it() {
    check(
        "---  \na: 1\n---  \n",
        "\
Document
  Frontmatter format=Yaml
    Mapping
      Pair key=a
        Scalar Integer 1
  MarkdownBlock
",
    );
    check(
        " ---\na: 1\n---\n",
        r#"Document
  MarkdownBlock
    Section
      ThematicBreak
      Section
        Heading
          Text "a: 1"
"#,
    );
}

#[test]
fn a_yaml_frontmatter_after_a_byte_order_mark_is_read() {
    let source = "\u{feff}---\na: 1\n---\n# A\n";
    check(
        source,
        r#"Document
  Frontmatter format=Yaml
    Mapping
      Pair key=a
        Scalar Integer 1
  MarkdownBlock
    Section
      Heading
        Text "A"
"#,
    );
    check_spans(
        source,
        &[
            ("Document", source),
            ("Frontmatter format=Yaml", "---\na: 1\n---"),
            ("Mapping", "a: 1"),
            ("Pair key=a", "a: 1"),
            ("Scalar Integer 1", "1"),
            ("MarkdownBlock", "# A\n"),
            ("Section", "# A"),
            ("Heading", "# A"),
            (r#"Text "A""#, "A"),
        ],
    );
}

#[test]
fn a_toml_frontmatter_after_a_byte_order_mark_is_read() {
    let source = "\u{feff}+++\na = 1\n+++\nx\n";
    check(
        source,
        r#"Document
  Frontmatter format=Toml
    Mapping
      Pair key=a
        Scalar Integer 1
  MarkdownBlock
    Section
      Paragraph
        Line
          Text "x"
"#,
    );
    check_spans(
        source,
        &[
            ("Document", source),
            ("Frontmatter format=Toml", "+++\na = 1\n+++"),
            ("Mapping", "a = 1"),
            ("Pair key=a", "a = 1"),
            ("Scalar Integer 1", "1"),
            ("MarkdownBlock", "x\n"),
            ("Section", "x"),
            ("Paragraph", "x"),
            ("Line", "x"),
            (r#"Text "x""#, "x"),
        ],
    );
}

#[test]
fn a_yaml_pair_whose_key_is_a_list_or_a_mapping_is_dropped_and_the_rest_is_read() {
    check(
        "---\ntitle: Trip\n[48.85, 2.35]: Paris\nyear: 2026\n---\n# Trip\n",
        r#"Document
  Frontmatter format=Yaml
    Mapping
      Pair key=title
        Scalar String "Trip"
      Pair key=year
        Scalar Integer 2026
  MarkdownBlock
    Section
      Heading
        Text "Trip"
"#,
    );
    let first_kept = "Document
  Frontmatter format=Yaml
    Mapping
      Pair key=a
        Scalar Integer 1
  MarkdownBlock
";
    check("---\na: 1\n[x]: 2\n---\n", first_kept);
    check("---\na: 1\n{x: 2}: 3\n---\n", first_kept);
    check("---\na: 1\n!!seq [x]: y\n---\n", first_kept);
    check("---\na: 1\n[x]: 2\n[x]: 3\n---\n", first_kept);
    check("---\na: 1\n[x]:\n  b: 2\n---\n", first_kept);
}

#[test]
fn an_explicit_key_that_is_a_list_or_a_mapping_drops_its_pair() {
    check(
        "---\ntitle: Trip\n? - 48.85\n  - 2.35\n: Paris\n---\n",
        r#"Document
  Frontmatter format=Yaml
    Mapping
      Pair key=title
        Scalar String "Trip"
  MarkdownBlock
"#,
    );
    let last_kept = "Document
  Frontmatter format=Yaml
    Mapping
      Pair key=b
        Scalar Integer 2
  MarkdownBlock
";
    check("---\n? a: 1\n: v\nb: 2\n---\n", last_kept);
    check("---\n? {x: 1}\n: v\nb: 2\n---\n", last_kept);
    for source in [
        "---\n? [a, b]\n: v\n---\n",
        "---\n?\n  a: 1\n: v\n---\n",
        "---\n?\n  - a\n: v\n---\n",
        "---\n? - a\n: v\n---\n",
        "---\n? a: 1\n: v\n---\n",
        "---\n? !!seq [x]\n: v\n---\n",
        "---\n? [x]\n---\n",
    ] {
        check(source, EMPTY_YAML);
    }
}

#[test]
fn a_mapping_that_held_only_a_dropped_pair_is_an_empty_mapping() {
    check("---\n{x: 2}: 3\n---\n", EMPTY_YAML);
    check("---\n[x]: y\n---\n", EMPTY_YAML);
    check(
        "---\nc:\n  [x]: y\n---\n",
        "Document
  Frontmatter format=Yaml
    Mapping
      Pair key=c
        Mapping
  MarkdownBlock
",
    );
    check(
        "---\nitems:\n- {x: 1}: 12\n---\n",
        "Document
  Frontmatter format=Yaml
    Mapping
      Pair key=items
        Sequence
          Mapping
  MarkdownBlock
",
    );
    check(
        "---\nc:\n  [x]: y\n  d: 1\n---\n",
        "Document
  Frontmatter format=Yaml
    Mapping
      Pair key=c
        Mapping
          Pair key=d
            Scalar Integer 1
  MarkdownBlock
",
    );
}

#[test]
fn a_flow_pair_whose_key_is_a_list_or_a_mapping_is_dropped() {
    let mapping = "Document
  Frontmatter format=Yaml
    Mapping
      Pair key=a
        Mapping
          Pair key=b
            Scalar Integer 2
  MarkdownBlock
";
    check("---\na: {[x]: 1, b: 2}\n---\n", mapping);
    check("---\na: {? [x] : 1, b: 2}\n---\n", mapping);
    check("---\na: {!!map {x: 1}: 1, b: 2}\n---\n", mapping);
    check("---\na: {[x], b: 2}\n---\n", mapping);
    check(
        "---\na: [[x]: 1, c]\n---\n",
        r#"Document
  Frontmatter format=Yaml
    Mapping
      Pair key=a
        Sequence
          Mapping
          Scalar String "c"
  MarkdownBlock
"#,
    );
    let one_empty = "Document
  Frontmatter format=Yaml
    Mapping
      Pair key=a
        Sequence
          Mapping
  MarkdownBlock
";
    check("---\na: [? [x] : 1]\n---\n", one_empty);
    check("---\na: [{x: 1}:2]\n---\n", one_empty);
    check("---\na: [!!seq [x] : 1]\n---\n", one_empty);
    check(
        "---\nowner: {[a]: Ada}\n---\n",
        "Document
  Frontmatter format=Yaml
    Mapping
      Pair key=owner
        Mapping
  MarkdownBlock
",
    );
    check("---\n{[x]: 1}\n---\n", EMPTY_YAML);
}

#[test]
fn a_mapping_whose_every_pair_is_dropped_is_empty_at_its_first_pair() {
    let nested = "---\nc:\n  [x]: y\n---\n";
    check_spans(
        nested,
        &[
            ("Document", nested),
            ("Frontmatter format=Yaml", "---\nc:\n  [x]: y\n---"),
            ("Mapping", "c:\n  "),
            ("Pair key=c", "c:\n  "),
            ("Mapping", ""),
            ("MarkdownBlock", ""),
        ],
    );
    let entry = "---\nitems:\n- {x: 1}: 12\n---\n";
    check_spans(
        entry,
        &[
            ("Document", entry),
            ("Frontmatter format=Yaml", "---\nitems:\n- {x: 1}: 12\n---"),
            ("Mapping", "items:\n- "),
            ("Pair key=items", "items:\n- "),
            ("Sequence", "- "),
            ("Mapping", ""),
            ("MarkdownBlock", ""),
        ],
    );
    let root = "---\n# a comment\n  [x]: y\n---\n";
    let mapping = parse(root).unwrap().frontmatter.unwrap().mapping;
    let at = root.find('[').unwrap();
    assert_eq!(mapping.span, at..at);
}
