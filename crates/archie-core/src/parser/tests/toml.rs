use super::check;

#[test]
fn every_scalar_type_is_read_from_how_the_value_is_written() {
    check(
        r#"+++
title = "Ship the parser"
path = 'C:\notes\parser'
priority = 2
big = 1_000_000
mask = 0xff
weight = 0.5
ratio = 1e-3
done = false
due = 2026-10-10
at = 07:32:00
start = 2026-10-10T07:32:00
sent = 2026-10-10T07:32:00.250-04:00
seen = 2026-10-10 09:00:00Z
+++
"#,
        r#"Document
  Frontmatter format=Toml
    Mapping
      Pair key=title
        Scalar String "Ship the parser"
      Pair key=path
        Scalar String "C:\\notes\\parser"
      Pair key=priority
        Scalar Integer 2
      Pair key=big
        Scalar Integer 1000000
      Pair key=mask
        Scalar Integer 255
      Pair key=weight
        Scalar Float 0.5
      Pair key=ratio
        Scalar Float 0.001
      Pair key=done
        Scalar Boolean false
      Pair key=due
        Scalar Date 2026-10-10
      Pair key=at
        Scalar Time 07:32:00
      Pair key=start
        Scalar DateTime 2026-10-10T07:32:00
      Pair key=sent
        Scalar DateTime 2026-10-10T07:32:00.250000000-04:00
      Pair key=seen
        Scalar DateTime 2026-10-10T09:00:00Z
  Body
"#,
    );
}

#[test]
fn arrays_inline_tables_comments_and_quoted_keys() {
    check(
        r#"+++
# what this note is about
tags = ["parser", "design"]   # a list
matrix = [
  [1, 2],
  [3, 4],
]
owner = { name = "Ada", admin = true }
"quoted key" = "x"
empty = []
+++
Body text.
"#,
        r#"Document
  Frontmatter format=Toml
    Mapping
      Pair key=tags
        Sequence
          Scalar String "parser"
          Scalar String "design"
      Pair key=matrix
        Sequence
          Sequence
            Scalar Integer 1
            Scalar Integer 2
          Sequence
            Scalar Integer 3
            Scalar Integer 4
      Pair key=owner
        Mapping
          Pair key=name
            Scalar String "Ada"
          Pair key=admin
            Scalar Boolean true
      Pair key=quoted key
        Scalar String "x"
      Pair key=empty
        Sequence
  Body
    Paragraph
      Text "Body text."
"#,
    );
}

#[test]
fn a_basic_string_reads_its_escapes_and_a_literal_string_has_none() {
    check(
        r#"+++
basic = "tab\there \"quoted\" \u0041"
literal = 'tab\there'
+++
"#,
        r#"Document
  Frontmatter format=Toml
    Mapping
      Pair key=basic
        Scalar String "tab\there \"quoted\" A"
      Pair key=literal
        Scalar String "tab\\there"
  Body
"#,
    );
}

#[test]
fn signed_numbers_other_bases_and_infinity() {
    check(
        "\
+++
negative = -17
positive = +5
binary = 0b101
octal = 0o17
exponent = 5e2
infinite = -inf
+++
",
        "\
Document
  Frontmatter format=Toml
    Mapping
      Pair key=negative
        Scalar Integer -17
      Pair key=positive
        Scalar Integer 5
      Pair key=binary
        Scalar Integer 5
      Pair key=octal
        Scalar Integer 15
      Pair key=exponent
        Scalar Float 500
      Pair key=infinite
        Scalar Float -inf
  Body
",
    );
}

#[test]
fn numbers_are_read_more_loosely_than_toml_allows() {
    check(
        "\
+++
leading_zeros = 007
doubled_underscore = 1__0
+++
",
        "\
Document
  Frontmatter format=Toml
    Mapping
      Pair key=leading_zeros
        Scalar Integer 7
      Pair key=doubled_underscore
        Scalar Integer 10
  Body
",
    );
}
