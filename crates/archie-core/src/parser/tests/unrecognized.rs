use super::check;

#[test]
fn what_the_parser_does_not_recognize_stays_as_paragraph_text() {
    check(
        "\
> quoted line
> - not a list

*emphasis* and **strong** stay as written.

- - -

Setext heading
==============
",
        r#"Document
  MarkdownBlock
    Section
      Heading
      Paragraph
        Line
          Text "> quoted line"
        Line
          Text "> - not a list"
      Paragraph
        Line
          Text "*emphasis* and **strong** stay as written."
      Paragraph
        Line
          Text "- - -"
      Paragraph
        Line
          Text "Setext heading"
        Line
          Text "=============="
"#,
    );
}

#[test]
fn a_heading_or_a_fence_is_not_recognized_inside_a_block_quote() {
    check(
        "\
> # quoted heading
> ```
> quoted code
",
        r#"Document
  MarkdownBlock
    Section
      Heading
      Paragraph
        Line
          Text "> # quoted heading"
        Line
          Text "> ```"
        Line
          Text "> quoted code"
"#,
    );
}

#[test]
fn reference_links_autolinks_and_html_stay_text() {
    check(
        "\
[label][ref] and <https://example.com> and <b>html</b>

[ref]: https://example.com
",
        r#"Document
  MarkdownBlock
    Section
      Heading
      Paragraph
        Line
          Text "[label][ref] and <https://example.com> and <b>html</b>"
      Paragraph
        Line
          Text "[ref]: https://example.com"
"#,
    );
}
