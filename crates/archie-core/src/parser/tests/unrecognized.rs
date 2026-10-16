use super::check;

#[test]
fn emphasis_and_strong_stay_as_written() {
    check(
        "*emphasis* and **strong** stay as written.\n",
        r#"Document
  MarkdownBlock
    Section
      Heading
      Paragraph
        Line
          Text "*emphasis* and **strong** stay as written."
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
        r##"Document
  MarkdownBlock
    Section
      Heading
      BlockQuote "# quoted heading\n```\nquoted code"
"##,
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
"#,
    );
}
