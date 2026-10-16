use super::check;

#[test]
fn what_the_parser_does_not_recognize_stays_as_paragraph_text() {
    check(
        "\
> quoted line
> - not a list

1. ordered item
2. second

*emphasis* and **strong** stay as written.

- - -

Setext heading
==============
",
        r#"Document
  Body
    Paragraph
      Text "> quoted line\n> - not a list"
    Paragraph
      Text "1. ordered item\n2. second"
    Paragraph
      Text "*emphasis* and **strong** stay as written."
    Paragraph
      Text "- - -"
    Paragraph
      Text "Setext heading\n=============="
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
  Body
    Paragraph
      Text "> # quoted heading\n> ```\n> quoted code"
"#,
    );
}

#[test]
fn a_heading_marker_inside_a_list_item_is_text() {
    check(
        "- # not a heading\n",
        "\
Document
  Body
    List
      Item
        Paragraph
          Text \"# not a heading\"
",
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
  Body
    Paragraph
      Text "[label][ref] and <https://example.com> and <b>html</b>"
    Paragraph
      Text "[ref]: https://example.com"
"#,
    );
}
