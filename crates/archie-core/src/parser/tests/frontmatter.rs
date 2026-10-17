use super::{check, check_spans, on_two_megabytes};
use crate::parser::frontmatter::VALUE_CAP;
use crate::parser::parse;
use crate::writer::{WriteOptions, write};

/// The outline of a YAML frontmatter that holds no pair, before an empty body.
const EMPTY_YAML: &str = "Document
  Frontmatter format=Yaml
    Mapping
  MarkdownBlock
";

/// The outline of a YAML frontmatter that holds no pair, before the heading
/// `A`.
const EMPTY_BEFORE_A: &str = r#"Document
  Frontmatter format=Yaml
    Mapping
  MarkdownBlock
    Section
      Heading
        Text "A"
"#;

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

#[test]
fn a_yaml_integer_too_large_for_64_bits_is_the_text_of_its_digits() {
    let source = "---\nn: 99999999999999999999\n---\n";
    check_spans(
        source,
        &[
            ("Document", source),
            (
                "Frontmatter format=Yaml",
                "---\nn: 99999999999999999999\n---",
            ),
            ("Mapping", "n: 99999999999999999999"),
            ("Pair key=n", "n: 99999999999999999999"),
            (
                "Scalar String \"99999999999999999999\"",
                "99999999999999999999",
            ),
            ("MarkdownBlock", ""),
        ],
    );
    check(
        "---\nn: [1, 99999999999999999999]\n---\n",
        r#"Document
  Frontmatter format=Yaml
    Mapping
      Pair key=n
        Sequence
          Scalar Integer 1
          Scalar String "99999999999999999999"
  MarkdownBlock
"#,
    );
}

#[test]
fn a_yaml_integer_too_large_for_64_bits_is_its_text_as_written_wherever_it_stands() {
    check(
        "\
---
hex: 0xFFFFFFFFFFFFFFFFFF
negative: -99999999999999999999
under:
  99999999999999999999
entries:
  - 99999999999999999999
flow: {n: 99999999999999999999}
---
",
        r#"Document
  Frontmatter format=Yaml
    Mapping
      Pair key=hex
        Scalar String "0xFFFFFFFFFFFFFFFFFF"
      Pair key=negative
        Scalar String "-99999999999999999999"
      Pair key=under
        Scalar String "99999999999999999999"
      Pair key=entries
        Sequence
          Scalar String "99999999999999999999"
      Pair key=flow
        Mapping
          Pair key=n
            Scalar String "99999999999999999999"
  MarkdownBlock
"#,
    );
}

#[test]
fn digits_too_large_for_64_bits_under_an_int_tag_are_text_and_under_a_float_tag_a_float() {
    check(
        "\
---
int: !!int 99999999999999999999
float: !!float 99999999999999999999
text: !!str 99999999999999999999
---
",
        r#"Document
  Frontmatter format=Yaml
    Mapping
      Pair key=int
        Scalar String "99999999999999999999"
      Pair key=float
        Scalar Float 100000000000000000000
      Pair key=text
        Scalar String "99999999999999999999"
  MarkdownBlock
"#,
    );
}

#[test]
fn a_toml_integer_too_large_for_64_bits_is_its_text_as_written() {
    check(
        "\
+++
n = 99999999999999999999
negative = -99999999999999999999
underscores = 9_999_999_999_999_999_999
hex = 0xFFFFFFFFFFFFFFFFFF
binary = 0b11111111111111111111111111111111111111111111111111111111111111111
list = [1, 99999999999999999999]
+++
",
        r#"Document
  Frontmatter format=Toml
    Mapping
      Pair key=n
        Scalar String "99999999999999999999"
      Pair key=negative
        Scalar String "-99999999999999999999"
      Pair key=underscores
        Scalar String "9_999_999_999_999_999_999"
      Pair key=hex
        Scalar String "0xFFFFFFFFFFFFFFFFFF"
      Pair key=binary
        Scalar String "0b11111111111111111111111111111111111111111111111111111111111111111"
      Pair key=list
        Sequence
          Scalar Integer 1
          Scalar String "99999999999999999999"
  MarkdownBlock
"#,
    );
}

#[test]
fn an_integer_that_fits_64_bits_stays_an_integer_in_yaml_and_toml() {
    let integers = |format: &str| {
        format!(
            "Document
  Frontmatter format={format}
    Mapping
      Pair key=largest
        Scalar Integer 9223372036854775807
      Pair key=smallest
        Scalar Integer -9223372036854775808
  MarkdownBlock
"
        )
    };
    check(
        "---\nlargest: 9223372036854775807\nsmallest: -9223372036854775808\n---\n",
        &integers("Yaml"),
    );
    check(
        "+++\nlargest = 9223372036854775807\nsmallest = -9223372036854775808\n+++\n",
        &integers("Toml"),
    );
}

/// The outline of a frontmatter in `format` that holds `b: 2` alone, before an
/// empty body.
fn only_b(format: &str) -> String {
    format!(
        "Document
  Frontmatter format={format}
    Mapping
      Pair key=b
        Scalar Integer 2
  MarkdownBlock
"
    )
}

#[test]
fn a_tag_archie_does_not_read_drops_the_innermost_key_that_holds_it() {
    for source in [
        "---\na: !point {x: 1}\nb: 2\n---\n",
        "---\n!point a: 1\nb: 2\n---\n",
        "---\ntags: [!point x, y]\nb: 2\n---\n",
        "---\norigin: !point 3,4\nb: 2\n---\n",
        "---\ndata: !!binary R0lG\nb: 2\n---\n",
        "---\norigin: !<tag:example.com,2026:point> 3,4\nb: 2\n---\n",
        "---\npoints: [!point 3]\nb: 2\n---\n",
        "---\n!point origin: 3\nb: 2\n---\n",
        "---\na: !point\n  x: 1\nb: 2\n---\n",
        "---\na:\n- !point x\n- y\nb: 2\n---\n",
        "---\n? !point k\n: v\nb: 2\n---\n",
        "---\n[!point x]: 1\nb: 2\n---\n",
        "---\na: !point \"x, ]\" # ]\n  [y\nb: 2\n---\n",
    ] {
        check(source, &only_b("Yaml"));
    }
    check(
        "---\nowner:\n  pin: !point 1\n  name: Ada\n---\n",
        r#"Document
  Frontmatter format=Yaml
    Mapping
      Pair key=owner
        Mapping
          Pair key=name
            Scalar String "Ada"
  MarkdownBlock
"#,
    );
}

#[test]
fn a_tag_archie_does_not_read_inside_a_flow_collection_drops_the_pair_that_holds_it() {
    let flow_b = "Document
  Frontmatter format=Yaml
    Mapping
      Pair key=a
        Mapping
          Pair key=b
            Scalar Integer 2
  MarkdownBlock
";
    check("---\na: {x: !point 1, b: 2}\n---\n", flow_b);
    check("---\na: {!point x: 1, b: 2}\n---\n", flow_b);
    check("---\na: {x: [!point {y: \"}\"}], b: 2}\n---\n", flow_b);
    check("---\n{x: !point [1, 2],\nb: 2}\n---\n", &only_b("Yaml"));
    let one_pair = r#"Document
  Frontmatter format=Yaml
    Mapping
      Pair key=a
        Sequence
          Mapping
          Scalar String "c"
  MarkdownBlock
"#;
    check("---\na: [!point x: 1, c]\n---\n", one_pair);
    check("---\na: [x: !point 1, c]\n---\n", one_pair);
    check("---\na: [[!point x]: 1, c]\n---\n", one_pair);
    check(
        "---\nitems:\n- a: !point 1\n  b: 2\n---\n",
        "Document
  Frontmatter format=Yaml
    Mapping
      Pair key=items
        Sequence
          Mapping
            Pair key=b
              Scalar Integer 2
  MarkdownBlock
",
    );
}

#[test]
fn a_tag_that_ends_its_line_over_a_mapping_whose_every_pair_is_dropped_holds_it_empty() {
    let empty_c = "Document
  Frontmatter format=Yaml
    Mapping
      Pair key=c
        Mapping
      Pair key=d
        Scalar Integer 1
  MarkdownBlock
";
    check("---\nc: !!map\n  a: !point x\nd: 1\n---\n", empty_c);
    check(
        "---\nc: !!map\n  [x]: y\n\n  # a comment\nd: 1\n---\n",
        empty_c,
    );
}

#[test]
fn a_frontmatter_with_a_dropped_tag_is_written_without_its_key() {
    let document = parse("---\ntitle: Notes\nshape: !point {x: 1, y: 2}\n---\n# Notes\n").unwrap();
    let pairs = &document.frontmatter.as_ref().unwrap().mapping.pairs;
    assert_eq!(pairs.keys().collect::<Vec<_>>(), ["title"]);
    assert_eq!(
        write(&document, &WriteOptions::default()).unwrap(),
        "---\ntitle: Notes\n---\n# Notes\n"
    );
}

/// `deep:` and a YAML list nested `depth` deep in each of its forms, beside
/// `b: 2`.
fn deep_yaml_lists(depth: usize) -> [String; 3] {
    let mut lines = String::new();
    for level in 0..depth {
        lines.push_str(&"  ".repeat(level));
        lines.push_str("-\n");
    }
    lines.push_str(&"  ".repeat(depth));
    lines.push_str("1\n");
    let flow = format!("{}1{}", "[".repeat(depth), "]".repeat(depth));
    [
        format!("---\ndeep:\n{lines}b: 2\n---\n"),
        format!("---\ndeep:\n  {}1\nb: 2\n---\n", "- ".repeat(depth)),
        format!("---\ndeep: {flow}\nb: 2\n---\n"),
    ]
}

#[test]
fn a_yaml_value_nested_past_the_value_cap_drops_the_key_that_holds_it() {
    on_two_megabytes(|| {
        for source in deep_yaml_lists(VALUE_CAP + 1) {
            check(&source, &only_b("Yaml"));
        }
        for source in deep_yaml_lists(VALUE_CAP) {
            let document = parse(&source).unwrap();
            let pairs = &document.frontmatter.as_ref().unwrap().mapping.pairs;
            assert_eq!(pairs.keys().collect::<Vec<_>>(), ["deep", "b"]);
        }
    });
}

#[test]
fn a_toml_value_nested_past_the_value_cap_drops_its_pair() {
    on_two_megabytes(|| {
        let past = VALUE_CAP + 1;
        let arrays = format!("{}1{}", "[".repeat(past), "]".repeat(past));
        let table = format!("{}{{x = 1}}{}", "[".repeat(past - 1), "]".repeat(past - 1));
        let quoted = format!(
            "[\n  # ] a comment\n  \"]\", '[', \"\"\"\n]\"\"\", {}1{}]",
            "[".repeat(past),
            "]".repeat(past)
        );
        for value in [arrays, table, quoted] {
            check(
                &format!("+++\ndeep = {value}\nb = 2\n+++\n"),
                &only_b("Toml"),
            );
        }
        let dotted = "a.".repeat(past);
        check(
            &format!("+++\n{dotted}x = 1\nb = 2\n+++\n"),
            &only_b("Toml"),
        );
    });
}

#[test]
fn a_toml_header_nested_past_the_value_cap_drops_its_table_and_its_pairs() {
    let source = format!(
        "+++\nb = 2\n[{}x]\nc = [\n[1],\n]\nd = \"\"\"\n[e]\n\"\"\"\n[t]\nf = 3\n+++\n",
        "a.".repeat(VALUE_CAP)
    );
    check(
        &source,
        "Document
  Frontmatter format=Toml
    Mapping
      Pair key=b
        Scalar Integer 2
      Pair key=t
        Mapping
          Pair key=f
            Scalar Integer 3
  MarkdownBlock
",
    );
}

#[test]
fn a_toml_table_keeps_its_span_to_its_last_pair_that_is_not_dropped() {
    on_two_megabytes(|| {
        let source = format!(
            "+++\n[t]\na = 1\nb = {}1{}\n+++\n",
            "[".repeat(VALUE_CAP + 1),
            "]".repeat(VALUE_CAP + 1)
        );
        let mapping = parse(&source).unwrap().frontmatter.unwrap().mapping;
        let pair = &mapping.pairs["t"];
        assert_eq!(&source[pair.span.clone()], "[t]\na = 1");
    });
}

#[test]
fn a_yaml_frontmatter_that_holds_one_value_or_one_list_is_empty() {
    for source in ["---\nabc\n---\n# A\n", "---\n- a\n- b\n---\n# A\n"] {
        check(source, EMPTY_BEFORE_A);
        let document = parse(source).unwrap();
        assert_eq!(
            write(&document, &WriteOptions::default()).unwrap(),
            "---\n---\n# A\n"
        );
    }
    for content in [
        "[a, b]",
        "hello",
        "\"a\n  b\"",
        "- a",
        "99999999999999999999",
        "- 99999999999999999999",
        "[99999999999999999999]",
        "abc # a comment\n\n# another",
        "  - a\n  - b",
        "!!seq\n- a",
    ] {
        check(&format!("---\n{content}\n---\n# A\n"), EMPTY_BEFORE_A);
    }
}

#[test]
fn a_yaml_frontmatter_that_holds_one_value_or_one_list_with_a_drop_is_empty() {
    on_two_megabytes(|| {
        // A list at the root opens no level of its own.
        let past = VALUE_CAP + 2;
        for content in [
            "!point x".to_owned(),
            "- !point x\n- y".to_owned(),
            "[!point x, y]".to_owned(),
            "!!seq\n- !point x".to_owned(),
            format!("{}1{}", "[".repeat(past), "]".repeat(past)),
            format!("{}1", "- ".repeat(past)),
        ] {
            check(&format!("---\n{content}\n---\n# A\n"), EMPTY_BEFORE_A);
        }
    });
}

#[test]
fn the_empty_mapping_of_a_frontmatter_that_holds_one_value_spans_the_value() {
    let list = "---\n- a\n- b\n---\n";
    check_spans(
        list,
        &[
            ("Document", list),
            ("Frontmatter format=Yaml", "---\n- a\n- b\n---"),
            ("Mapping", "- a\n- b"),
            ("MarkdownBlock", ""),
        ],
    );
    let dropped = "---\n- !point x\n- y\n\n---\n";
    check_spans(
        dropped,
        &[
            ("Document", dropped),
            ("Frontmatter format=Yaml", "---\n- !point x\n- y\n\n---"),
            ("Mapping", "- !point x\n- y"),
            ("MarkdownBlock", ""),
        ],
    );
}

#[test]
fn an_alias_reads_as_a_copy_of_the_scalar_its_anchor_names() {
    let source = "---\nauthor: &ada Ada\nreviewer: *ada\n---\n";
    check(
        source,
        r#"Document
  Frontmatter format=Yaml
    Mapping
      Pair key=author
        Scalar String "Ada"
      Pair key=reviewer
        Scalar String "Ada"
  MarkdownBlock
"#,
    );
    check_spans(
        source,
        &[
            ("Document", source),
            (
                "Frontmatter format=Yaml",
                "---\nauthor: &ada Ada\nreviewer: *ada\n---",
            ),
            ("Mapping", "author: &ada Ada\nreviewer: *ada"),
            ("Pair key=author", "author: &ada Ada"),
            ("Scalar String \"Ada\"", "Ada"),
            ("Pair key=reviewer", "reviewer: *ada"),
            ("Scalar String \"Ada\"", "*ada"),
            ("MarkdownBlock", ""),
        ],
    );
}

#[test]
fn an_alias_of_a_mapping_copies_every_value_in_it_each_spanning_the_alias() {
    let source = "---\nbase: &b\n  x: 1\n  y: [a, b]\ncopy: *b\n---\n";
    check(
        source,
        r#"Document
  Frontmatter format=Yaml
    Mapping
      Pair key=base
        Mapping
          Pair key=x
            Scalar Integer 1
          Pair key=y
            Sequence
              Scalar String "a"
              Scalar String "b"
      Pair key=copy
        Mapping
          Pair key=x
            Scalar Integer 1
          Pair key=y
            Sequence
              Scalar String "a"
              Scalar String "b"
  MarkdownBlock
"#,
    );
    let copy: Vec<(&str, &str)> = [
        "Mapping",
        "Pair key=x",
        "Scalar Integer 1",
        "Pair key=y",
        "Sequence",
        "Scalar String \"a\"",
        "Scalar String \"b\"",
    ]
    .into_iter()
    .map(|label| (label, "*b"))
    .collect();
    let mut expected = vec![
        ("Document", source),
        (
            "Frontmatter format=Yaml",
            "---\nbase: &b\n  x: 1\n  y: [a, b]\ncopy: *b\n---",
        ),
        ("Mapping", "base: &b\n  x: 1\n  y: [a, b]\ncopy: *b"),
        ("Pair key=base", "base: &b\n  x: 1\n  y: [a, b]"),
        ("Mapping", "x: 1\n  y: [a, b]"),
        ("Pair key=x", "x: 1"),
        ("Scalar Integer 1", "1"),
        ("Pair key=y", "y: [a, b]"),
        ("Sequence", "[a, b]"),
        ("Scalar String \"a\"", "a"),
        ("Scalar String \"b\"", "b"),
        ("Pair key=copy", "copy: *b"),
    ];
    expected.extend(copy);
    expected.push(("MarkdownBlock", ""));
    check_spans(source, &expected);
}

#[test]
fn an_anchor_and_its_alias_inside_flow_collections() {
    check(
        "---\ntags: [&t one, two, *t]\n---\n",
        r#"Document
  Frontmatter format=Yaml
    Mapping
      Pair key=tags
        Sequence
          Scalar String "one"
          Scalar String "two"
          Scalar String "one"
  MarkdownBlock
"#,
    );
    check(
        "---\na: {k: &v 1, l: *v}\n---\n",
        "Document
  Frontmatter format=Yaml
    Mapping
      Pair key=a
        Mapping
          Pair key=k
            Scalar Integer 1
          Pair key=l
            Scalar Integer 1
  MarkdownBlock
",
    );
}

/// The outline of a YAML frontmatter that holds `a` and `b`, each the
/// scalar labeled `scalar`, before an empty body.
fn a_and_b(scalar: &str) -> String {
    format!(
        "Document
  Frontmatter format=Yaml
    Mapping
      Pair key=a
        Scalar {scalar}
      Pair key=b
        Scalar {scalar}
  MarkdownBlock
"
    )
}

#[test]
fn a_tag_and_an_anchor_may_be_written_in_either_order() {
    for source in [
        "---\na: !!str &s 1\nb: *s\n---\n",
        "---\na: &s !!str 1\nb: *s\n---\n",
        "---\na: !!str\n  &s 1\nb: *s\n---\n",
        "---\na: &s\n  !!str 1\nb: *s\n---\n",
    ] {
        check(source, &a_and_b("String \"1\""));
    }
}

#[test]
fn an_anchor_on_an_empty_value_names_null() {
    check("---\na: &e\nb: *e\n---\n", &a_and_b("Null"));
    check(
        "---\nl:\n- &e\n- *e\n---\n",
        "Document
  Frontmatter format=Yaml
    Mapping
      Pair key=l
        Sequence
          Scalar Null
          Scalar Null
  MarkdownBlock
",
    );
}

#[test]
fn an_anchor_on_a_key_names_the_key_and_its_alias_writes_that_key_again() {
    check(
        "---\n&k a: 1\n*k : 2\n---\n",
        "Document
  Frontmatter format=Yaml
    Mapping
      Pair key=a repeats=1
        Sequence
          Scalar Integer 1
          Scalar Integer 2
  MarkdownBlock
",
    );
    check(
        "---\nk:\n  &a x: 1\n  y: *a\n---\n",
        r#"Document
  Frontmatter format=Yaml
    Mapping
      Pair key=k
        Mapping
          Pair key=x
            Scalar Integer 1
          Pair key=y
            Scalar String "x"
  MarkdownBlock
"#,
    );
}

#[test]
fn an_alias_as_a_key_is_the_text_of_the_scalar_its_anchor_names() {
    check(
        "---\nn: &n 0x1A\n? *n\n: v\n{*n : w}: x\nl: [*n : y]\n---\n",
        r#"Document
  Frontmatter format=Yaml
    Mapping
      Pair key=n
        Scalar Integer 26
      Pair key=0x1A
        Scalar String "v"
      Pair key=l
        Sequence
          Mapping
            Pair key=0x1A
              Scalar String "y"
  MarkdownBlock
"#,
    );
}

#[test]
fn an_alias_of_a_list_or_a_mapping_as_a_key_drops_its_pair() {
    for source in [
        "---\nm: &m {x: 1}\n*m : 2\nb: 2\n---\n",
        "---\nm: &m [1]\n? *m\n: v\nb: 2\n---\n",
    ] {
        let document = parse(source).unwrap();
        let pairs = &document.frontmatter.as_ref().unwrap().mapping.pairs;
        assert_eq!(pairs.keys().collect::<Vec<_>>(), ["m", "b"], "{source:?}");
    }
    check(
        "---\nm: &m [1]\nf: {*m : 1, b: 2}\n---\n",
        "Document
  Frontmatter format=Yaml
    Mapping
      Pair key=m
        Sequence
          Scalar Integer 1
      Pair key=f
        Mapping
          Pair key=b
            Scalar Integer 2
  MarkdownBlock
",
    );
}

#[test]
fn an_alias_copies_the_most_recent_anchor_of_its_name_before_it() {
    check(
        "---\na: &x 1\nb: &x 2\nc: *x\n---\n",
        "Document
  Frontmatter format=Yaml
    Mapping
      Pair key=a
        Scalar Integer 1
      Pair key=b
        Scalar Integer 2
      Pair key=c
        Scalar Integer 2
  MarkdownBlock
",
    );
    let inner = "Document
  Frontmatter format=Yaml
    Mapping
      Pair key=a
        Sequence
          Scalar Integer 1
          Scalar Integer 1
  MarkdownBlock
";
    check("---\na: &A [&A 1, *A]\n---\n", inner);
    let before = "Document
  Frontmatter format=Yaml
    Mapping
      Pair key=a
        Scalar Integer 0
      Pair key=b
        Sequence
          Scalar Integer 0
          Scalar Integer 1
  MarkdownBlock
";
    check("---\na: &x 0\nb: [*x, &x 1]\n---\n", before);
    check("---\na: &x 0\nb:\n  [*x, &x 1]\n---\n", before);
    check("---\na: &x 0\nb:\n- [*x, &x 1]\n---\n", &before.replace(
        "        Sequence\n          Scalar Integer 0\n          Scalar Integer 1\n",
        "        Sequence\n          Sequence\n            Scalar Integer 0\n            Scalar Integer 1\n",
    ));
}

#[test]
fn an_alias_inside_the_value_its_anchor_names_drops_the_innermost_key_that_holds_it() {
    check("---\na: &A [*A]\nb: 2\n---\n", &only_b("Yaml"));
    check(
        "---\nx: &A 1\na: &A [*A]\nb: 2\n---\n",
        "Document
  Frontmatter format=Yaml
    Mapping
      Pair key=x
        Scalar Integer 1
      Pair key=b
        Scalar Integer 2
  MarkdownBlock
",
    );
    let emptied = "Document
  Frontmatter format=Yaml
    Mapping
      Pair key=a
        Mapping
      Pair key=b
        Scalar Integer 2
  MarkdownBlock
";
    check("---\na: &A {x: [*A]}\nb: 2\n---\n", emptied);
    check("---\na: &A\n  x: *A\nb: 2\n---\n", emptied);
}

#[test]
fn an_alias_with_no_anchor_before_it_drops_the_key_that_holds_it() {
    check(
        "---\na: *B\nb: &B 1\n---\n",
        "Document
  Frontmatter format=Yaml
    Mapping
      Pair key=b
        Scalar Integer 1
  MarkdownBlock
",
    );
    check(
        "---\na: 1\nb: *a\n---\n",
        "Document
  Frontmatter format=Yaml
    Mapping
      Pair key=a
        Scalar Integer 1
  MarkdownBlock
",
    );
    check("---\nx: [*y, &y \"]\"]\nb: 2\n---\n", &only_b("Yaml"));
    check(
        "---\na: {x: [*y, &y \"]\"], b: 2}\n---\n",
        "Document
  Frontmatter format=Yaml
    Mapping
      Pair key=a
        Mapping
          Pair key=b
            Scalar Integer 2
  MarkdownBlock
",
    );
}

#[test]
fn anchors_on_values_list_entries_and_keys_are_read() {
    check(
        "---\nbase: &base 1\n---\n",
        "Document
  Frontmatter format=Yaml
    Mapping
      Pair key=base
        Scalar Integer 1
  MarkdownBlock
",
    );
    check(
        "---\nitems:\n  - &first 1\n  - [*first]\n---\n",
        "Document
  Frontmatter format=Yaml
    Mapping
      Pair key=items
        Sequence
          Scalar Integer 1
          Sequence
            Scalar Integer 1
  MarkdownBlock
",
    );
    check(
        "---\n&key a: 1\n---\n",
        "Document
  Frontmatter format=Yaml
    Mapping
      Pair key=a
        Scalar Integer 1
  MarkdownBlock
",
    );
}

#[test]
fn a_merge_key_is_a_key_like_any_other() {
    check(
        "---\nd: &d {a: 1}\ne:\n  <<: *d\n---\n",
        "Document
  Frontmatter format=Yaml
    Mapping
      Pair key=d
        Mapping
          Pair key=a
            Scalar Integer 1
      Pair key=e
        Mapping
          Pair key=<<
            Mapping
              Pair key=a
                Scalar Integer 1
  MarkdownBlock
",
    );
}

/// A frontmatter of ten anchors, each a list of ten aliases of the one
/// before, the first a list of ten scalars, then `b: 2` and `c: *a0`.
fn ten_anchors_of_ten_aliases() -> String {
    let mut lines = vec!["a0: &a0 [x, x, x, x, x, x, x, x, x, x]".to_owned()];
    for level in 1_usize..10 {
        let aliases = vec![format!("*a{}", level - 1); 10].join(", ");
        lines.push(format!("a{level}: &a{level} [{aliases}]"));
    }
    format!("---\n{}\nb: 2\nc: *a0\n---\n", lines.join("\n"))
}

#[test]
fn aliases_that_copy_more_than_the_cap_drop_the_key_that_passes_it_and_every_later_alias() {
    let document = parse(&ten_anchors_of_ten_aliases()).unwrap();
    let pairs = &document.frontmatter.as_ref().unwrap().mapping.pairs;
    // `a3` brings the copies to 12330 values, and the eighth alias in `a4`
    // passes 100000.
    assert_eq!(
        pairs.keys().collect::<Vec<_>>(),
        ["a0", "a1", "a2", "a3", "b"]
    );
}
