use super::check;
use crate::parser::{read_toml, read_yaml};

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
  MarkdownBlock
    Section
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
  MarkdownBlock
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
  MarkdownBlock
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
  MarkdownBlock
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
  MarkdownBlock
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
