use super::check;

#[test]
fn a_file_with_no_delimiter_on_its_first_line_has_no_frontmatter() {
    check(
        "Just text.\n",
        r#"Document
  MarkdownBlock
    Section
      Heading
      Paragraph
        Line
          Text "Just text."
"#,
    );
}

#[test]
fn an_opening_delimiter_with_no_closing_one_is_body_text() {
    check(
        "\
---
title: x

No closing line.
",
        r#"Document
  MarkdownBlock
    Section
      Heading
      Paragraph
        Line
          Text "---"
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
      Heading
      Paragraph
        Line
          Text "Body."
"#,
    );
}

#[test]
fn a_delimiter_after_the_first_line_is_body_text() {
    check(
        "\
Text.

---

More.
",
        r#"Document
  MarkdownBlock
    Section
      Heading
      Paragraph
        Line
          Text "Text."
      Paragraph
        Line
          Text "---"
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
              Heading
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
      Heading
      Paragraph
        Line
          Text "---"
        Line
          Text "a: 1"
        Line
          Text "---"
"#,
    );
}
