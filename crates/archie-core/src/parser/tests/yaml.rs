use core::ops::Range;

use super::{check, check_spans};
use crate::parser::{parse, read_toml, read_yaml};

#[test]
fn a_note_frontmatter_with_every_scalar_type_and_both_list_styles() {
    check(
        r#"---
type: agent-lesson
title: 'It''s "quoted"'
description: ''
date: 2026-09-26
created: 2026-09-26T17:08:20.162234780Z
priority: 2
weight: 0.5
done: false
remargin_last_activity: null
tags:
- design
- requirements
aliases: [Parser, "The parser"]
---
# Title
"#,
        r#"Document
  Frontmatter format=Yaml
    Mapping
      Pair key=type
        Scalar String "agent-lesson"
      Pair key=title
        Scalar String "It's \"quoted\""
      Pair key=description
        Scalar String ""
      Pair key=date
        Scalar Date 2026-09-26
      Pair key=created
        Scalar DateTime 2026-09-26T17:08:20.162234780Z
      Pair key=priority
        Scalar Integer 2
      Pair key=weight
        Scalar Float 0.5
      Pair key=done
        Scalar Boolean false
      Pair key=remargin_last_activity
        Scalar Null
      Pair key=tags
        Sequence
          Scalar String "design"
          Scalar String "requirements"
      Pair key=aliases
        Sequence
          Scalar String "Parser"
          Scalar String "The parser"
  Body
    Section level=1
      Heading
        Text "Title"
"#,
    );
}

#[test]
fn nested_mappings_indented_lists_comments_and_keys_with_no_value() {
    check(
        r#"---
# who owns it
owner:
  name: Ada
  contact:
    email: ada@example.com
"quoted key": yes   # a string: only true and false are booleans
steps:
  - first
  -
  - third
nothing:
---
"#,
        r#"Document
  Frontmatter format=Yaml
    Mapping
      Pair key=owner
        Mapping
          Pair key=name
            Scalar String "Ada"
          Pair key=contact
            Mapping
              Pair key=email
                Scalar String "ada@example.com"
      Pair key=quoted key
        Scalar String "yes"
      Pair key=steps
        Sequence
          Scalar String "first"
          Scalar Null
          Scalar String "third"
      Pair key=nothing
        Scalar Null
  Body
"#,
    );
}

#[test]
fn a_double_quoted_string_reads_its_escapes() {
    check(
        r#"---
text: "tab\there \"quoted\" A"
---
"#,
        r#"Document
  Frontmatter format=Yaml
    Mapping
      Pair key=text
        Scalar String "tab\there \"quoted\" A"
  Body
"#,
    );
}

#[test]
fn a_plain_scalar_is_typed_by_how_it_is_written() {
    check(
        "\
---
negative: -17
hex: 0x1F
exponent: 1e3
version: 1.2.3
time: 07:32:00
tilde: ~
truthy: True
leading_dot: .5
---
",
        r#"Document
  Frontmatter format=Yaml
    Mapping
      Pair key=negative
        Scalar Integer -17
      Pair key=hex
        Scalar Integer 31
      Pair key=exponent
        Scalar Float 1000
      Pair key=version
        Scalar String "1.2.3"
      Pair key=time
        Scalar String "07:32:00"
      Pair key=tilde
        Scalar Null
      Pair key=truthy
        Scalar Boolean true
      Pair key=leading_dot
        Scalar String ".5"
  Body
"#,
    );
}

#[test]
fn a_list_entry_may_hold_a_mapping_and_mappings_nest_lists_of_mappings() {
    check(
        "\
---
people:
  - name: Ada
    role: host
  - name: Grace
    tags: [review]
reactions:
  thumbs:
    - author: ada
      ts: 2026-10-01T10:00:00Z
---
",
        r#"Document
  Frontmatter format=Yaml
    Mapping
      Pair key=people
        Sequence
          Mapping
            Pair key=name
              Scalar String "Ada"
            Pair key=role
              Scalar String "host"
          Mapping
            Pair key=name
              Scalar String "Grace"
            Pair key=tags
              Sequence
                Scalar String "review"
      Pair key=reactions
        Mapping
          Pair key=thumbs
            Sequence
              Mapping
                Pair key=author
                  Scalar String "ada"
                Pair key=ts
                  Scalar DateTime 2026-10-01T10:00:00Z
  Body
"#,
    );
}

#[test]
fn the_readers_take_a_range_of_a_larger_text_and_keep_its_offsets() {
    let source = "intro\nname: Ada\nrole: host\nmore";
    let mapping = read_yaml(source, 6..27).unwrap();
    let keys: Vec<&str> = mapping.pairs.keys().map(String::as_str).collect();
    assert_eq!(keys, ["name", "role"]);
    assert_eq!(&source[mapping.span.clone()], "name: Ada\nrole: host");

    let toml = "x\nname = \"Ada\"\ny";
    let table = read_toml(toml, 2..15).unwrap();
    assert_eq!(&toml[table.pairs["name"].span.clone()], "name = \"Ada\"");
}

#[test]
fn a_key_written_twice_holds_a_list_of_both_values() {
    check(
        "---\ntitle: Draft\ntitle: Final\n---\n",
        r#"Document
  Frontmatter format=Yaml
    Mapping
      Pair key=title repeats=1
        Sequence
          Scalar String "Draft"
          Scalar String "Final"
  Body
"#,
    );
}

#[test]
fn a_list_written_again_adds_its_items_and_any_other_value_adds_itself() {
    check(
        "\
---
tags: draft
tags: [review, parser]
labels: [review, parser]
labels: draft
---
",
        r#"Document
  Frontmatter format=Yaml
    Mapping
      Pair key=tags repeats=1
        Sequence
          Scalar String "draft"
          Scalar String "review"
          Scalar String "parser"
      Pair key=labels repeats=1
        Sequence
          Scalar String "review"
          Scalar String "parser"
          Scalar String "draft"
  Body
"#,
    );
}

#[test]
fn a_mapping_written_again_is_one_item_of_the_list() {
    check(
        "\
---
owner:
  name: Ada
owner:
  role: host
host: Ada
host:
  name: Grace
  role: host
people: [ada, grace]
people:
  - name: Linus
---
",
        r#"Document
  Frontmatter format=Yaml
    Mapping
      Pair key=owner repeats=1
        Sequence
          Mapping
            Pair key=name
              Scalar String "Ada"
          Mapping
            Pair key=role
              Scalar String "host"
      Pair key=host repeats=1
        Sequence
          Scalar String "Ada"
          Mapping
            Pair key=name
              Scalar String "Grace"
            Pair key=role
              Scalar String "host"
      Pair key=people repeats=1
        Sequence
          Scalar String "ada"
          Scalar String "grace"
          Mapping
            Pair key=name
              Scalar String "Linus"
  Body
"#,
    );
}

#[test]
fn three_occurrences_give_one_list_in_file_order() {
    check(
        "---\na: 1\na: 2\na: 3\n---\n",
        "\
Document
  Frontmatter format=Yaml
    Mapping
      Pair key=a repeats=2
        Sequence
          Scalar Integer 1
          Scalar Integer 2
          Scalar Integer 3
  Body
",
    );
}

#[test]
fn a_key_written_twice_in_a_nested_mapping_holds_a_list() {
    check(
        "\
---
owner:
  name: Ada
  name: Grace
---
",
        r#"Document
  Frontmatter format=Yaml
    Mapping
      Pair key=owner
        Mapping
          Pair key=name repeats=1
            Sequence
              Scalar String "Ada"
              Scalar String "Grace"
  Body
"#,
    );
}

#[test]
fn a_key_written_again_keeps_its_first_place() {
    check(
        "---\ntitle: A\nother: 1\ntitle: B\n---\n",
        r#"Document
  Frontmatter format=Yaml
    Mapping
      Pair key=title repeats=1
        Sequence
          Scalar String "A"
          Scalar String "B"
      Pair key=other
        Scalar Integer 1
  Body
"#,
    );
}

#[test]
fn each_later_occurrence_of_a_key_is_recorded_from_its_key_to_its_value() {
    let source = "---\ntitle: Draft\nonce: 1\ntitle: Final\ntitle:\n  - x\n---\n";
    let document = parse(source).unwrap();
    let mapping = document.frontmatter.unwrap().mapping;
    let title = &mapping.pairs["title"];
    assert_eq!(&source[title.span.clone()], "title: Draft");
    let repeats: Vec<&str> = title
        .repeats
        .iter()
        .map(|span| &source[span.clone()])
        .collect();
    assert_eq!(repeats, ["title: Final", "title:\n  - x"]);
    assert_eq!(mapping.pairs["once"].repeats, Vec::<Range<usize>>::new());
    assert_eq!(
        &source[mapping.span],
        "title: Draft\nonce: 1\ntitle: Final\ntitle:\n  - x"
    );
}

#[test]
fn a_literal_block_scalar_keeps_its_lines_with_each_chomping_and_an_indentation_digit() {
    check(
        "\
---
clip: |
  Two lines
  of text.

strip: |-
  no break

keep: |+
  kept

indented: |2
    four spaces
  two spaces
empty: |
kept_empty: |+

last: 1
---
",
        r#"Document
  Frontmatter format=Yaml
    Mapping
      Pair key=clip
        Scalar String "Two lines\nof text.\n"
      Pair key=strip
        Scalar String "no break"
      Pair key=keep
        Scalar String "kept\n\n"
      Pair key=indented
        Scalar String "  four spaces\ntwo spaces\n"
      Pair key=empty
        Scalar String ""
      Pair key=kept_empty
        Scalar String "\n"
      Pair key=last
        Scalar Integer 1
  Body
"#,
    );
}

#[test]
fn a_folded_block_scalar_joins_its_lines_and_keeps_blank_and_more_indented_lines() {
    check(
        "\
---
text: >
  Folded into
  one line.

  A new paragraph,
    kept as written,
  and folded again.
---
",
        r#"Document
  Frontmatter format=Yaml
    Mapping
      Pair key=text
        Scalar String "Folded into one line.\nA new paragraph,\n  kept as written,\nand folded again.\n"
  Body
"#,
    );
}

#[test]
fn a_block_scalar_may_be_a_list_entry_or_a_value_inside_one() {
    check(
        "\
---
steps:
  - |
    first
  - name: x
    run: >-
      a
      b
---
",
        r#"Document
  Frontmatter format=Yaml
    Mapping
      Pair key=steps
        Sequence
          Scalar String "first\n"
          Mapping
            Pair key=name
              Scalar String "x"
            Pair key=run
              Scalar String "a b"
  Body
"#,
    );
}

#[test]
fn flow_mappings_and_flow_lists_nest_and_may_span_several_lines() {
    check(
        r#"---
owner: {name: Ada, tags: [x, y]}
people: [{name: Ada}, {name: Grace, role: host}]
nested: {a: {b: [1, {c: 2}]}}
spread: {name: Ada,
  role: host}
list: [
  a,  # first
  b,
]
empty: {}
bare: {key, other: }
quoted: {"a b": 1, 'c': "d"}
---
"#,
        r#"Document
  Frontmatter format=Yaml
    Mapping
      Pair key=owner
        Mapping
          Pair key=name
            Scalar String "Ada"
          Pair key=tags
            Sequence
              Scalar String "x"
              Scalar String "y"
      Pair key=people
        Sequence
          Mapping
            Pair key=name
              Scalar String "Ada"
          Mapping
            Pair key=name
              Scalar String "Grace"
            Pair key=role
              Scalar String "host"
      Pair key=nested
        Mapping
          Pair key=a
            Mapping
              Pair key=b
                Sequence
                  Scalar Integer 1
                  Mapping
                    Pair key=c
                      Scalar Integer 2
      Pair key=spread
        Mapping
          Pair key=name
            Scalar String "Ada"
          Pair key=role
            Scalar String "host"
      Pair key=list
        Sequence
          Scalar String "a"
          Scalar String "b"
      Pair key=empty
        Mapping
      Pair key=bare
        Mapping
          Pair key=key
            Scalar Null
          Pair key=other
            Scalar Null
      Pair key=quoted
        Mapping
          Pair key=a b
            Scalar Integer 1
          Pair key=c
            Scalar String "d"
  Body
"#,
    );
}

#[test]
fn a_tag_fixes_the_type_of_its_value() {
    check(
        r#"---
count: !!str 12
zip: !!str 01234
ratio: !!float 1
half: !!float 0.5
big: !!int "42"
done: !!bool true
nothing: !!null ~
when: !!timestamp 2026-10-10
at: !!timestamp 2026-10-10T07:32:00Z
verbatim: !<tag:yaml.org,2002:str> 12
listed: [!!str 1, !!int "2"]
!!str tagged key: !!str
---
"#,
        r#"Document
  Frontmatter format=Yaml
    Mapping
      Pair key=count
        Scalar String "12"
      Pair key=zip
        Scalar String "01234"
      Pair key=ratio
        Scalar Float 1
      Pair key=half
        Scalar Float 0.5
      Pair key=big
        Scalar Integer 42
      Pair key=done
        Scalar Boolean true
      Pair key=nothing
        Scalar Null
      Pair key=when
        Scalar Date 2026-10-10
      Pair key=at
        Scalar DateTime 2026-10-10T07:32:00Z
      Pair key=verbatim
        Scalar String "12"
      Pair key=listed
        Sequence
          Scalar String "1"
          Scalar Integer 2
      Pair key=tagged key
        Scalar String ""
  Body
"#,
    );
}

#[test]
fn a_collection_tag_checks_its_value_whether_it_is_written_inline_or_as_a_block() {
    check(
        "\
---
owner: !!map {name: Ada}
tags: !!seq [a]
block: !!map
  name: Grace
items: !!seq
  - b
text: !!str |
  line
after: 1
---
",
        r#"Document
  Frontmatter format=Yaml
    Mapping
      Pair key=owner
        Mapping
          Pair key=name
            Scalar String "Ada"
      Pair key=tags
        Sequence
          Scalar String "a"
      Pair key=block
        Mapping
          Pair key=name
            Scalar String "Grace"
      Pair key=items
        Sequence
          Scalar String "b"
      Pair key=text
        Scalar String "line\n"
      Pair key=after
        Scalar Integer 1
  Body
"#,
    );
}

#[test]
fn an_unquoted_date_is_a_date_only_when_that_day_exists() {
    check(
        "\
---
due: 2028-02-29
late: 2026-02-30
never: 2026-04-31
century: 1900-02-29
millennium: 2000-02-29
at: 2026-02-30T10:00:00Z
---
",
        r#"Document
  Frontmatter format=Yaml
    Mapping
      Pair key=due
        Scalar Date 2028-02-29
      Pair key=late
        Scalar String "2026-02-30"
      Pair key=never
        Scalar String "2026-04-31"
      Pair key=century
        Scalar String "1900-02-29"
      Pair key=millennium
        Scalar Date 2000-02-29
      Pair key=at
        Scalar String "2026-02-30T10:00:00Z"
  Body
"#,
    );
}

#[test]
fn a_frontmatter_with_a_block_scalar_a_flow_mapping_a_tag_and_a_date_that_does_not_exist() {
    check(
        "\
---
summary: |
  Two lines
  of text.
owner: {name: Ada, tags: [x, y]}
count: !!str 12
due: 2026-02-30
---
# Notes
",
        r#"Document
  Frontmatter format=Yaml
    Mapping
      Pair key=summary
        Scalar String "Two lines\nof text.\n"
      Pair key=owner
        Mapping
          Pair key=name
            Scalar String "Ada"
          Pair key=tags
            Sequence
              Scalar String "x"
              Scalar String "y"
      Pair key=count
        Scalar String "12"
      Pair key=due
        Scalar String "2026-02-30"
  Body
    Section level=1
      Heading
        Text "Notes"
"#,
    );
}

#[test]
fn a_flow_mapping_written_again_is_one_item_of_the_list() {
    check(
        "\
---
owner: {name: Ada}
owner: {role: host}
host: Ada
host: {name: Grace, role: host}
---
",
        r#"Document
  Frontmatter format=Yaml
    Mapping
      Pair key=owner repeats=1
        Sequence
          Mapping
            Pair key=name
              Scalar String "Ada"
          Mapping
            Pair key=role
              Scalar String "host"
      Pair key=host repeats=1
        Sequence
          Scalar String "Ada"
          Mapping
            Pair key=name
              Scalar String "Grace"
            Pair key=role
              Scalar String "host"
  Body
"#,
    );
}

#[test]
fn a_key_written_twice_in_a_flow_mapping_holds_a_list() {
    check(
        "---\nowner: {name: Ada, name: Grace}\n---\n",
        r#"Document
  Frontmatter format=Yaml
    Mapping
      Pair key=owner
        Mapping
          Pair key=name repeats=1
            Sequence
              Scalar String "Ada"
              Scalar String "Grace"
  Body
"#,
    );
}

#[test]
fn a_block_scalar_spans_from_its_indicator_to_its_last_text() {
    let whole = "---\ntext: |+\n  a\n  b\n\nflow: {k: [1]}\ntag: !!str x\n---\n";
    check_spans(
        whole,
        &[
            ("Document", whole),
            (
                "Frontmatter format=Yaml",
                "---\ntext: |+\n  a\n  b\n\nflow: {k: [1]}\ntag: !!str x\n---",
            ),
            (
                "Mapping",
                "text: |+\n  a\n  b\n\nflow: {k: [1]}\ntag: !!str x",
            ),
            ("Pair key=text", "text: |+\n  a\n  b"),
            (r#"Scalar String "a\nb\n\n""#, "|+\n  a\n  b"),
            ("Pair key=flow", "flow: {k: [1]}"),
            ("Mapping", "{k: [1]}"),
            ("Pair key=k", "k: [1]"),
            ("Sequence", "[1]"),
            ("Scalar Integer 1", "1"),
            ("Pair key=tag", "tag: !!str x"),
            (r#"Scalar String "x""#, "x"),
            ("Body", ""),
        ],
    );
}
