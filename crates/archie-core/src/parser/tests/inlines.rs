use super::check;

#[test]
fn a_wiki_link_has_a_target_and_may_have_an_alias() {
    check(
        "See [[grammar]], [[notes/parser|the parser notes]] and [[spec#Lists]].\n",
        r#"Document
  MarkdownBlock
    Section
      Heading
      Paragraph
        Line
          Text "See "
          Link form=Wiki target="grammar"
          Text ", "
          Link form=Wiki target="notes/parser"
            Text "the parser notes"
          Text " and "
          Link form=Wiki target="spec#Lists"
          Text "."
"#,
    );
}

#[test]
fn a_link_holds_its_text_as_children_and_its_destination() {
    check(
        "Read [the spec](docs/spec.md) or [`parse`](https://example.com/parse?x=1).\n",
        r#"Document
  MarkdownBlock
    Section
      Heading
      Paragraph
        Line
          Text "Read "
          Link form=Markdown target="docs/spec.md"
            Text "the spec"
          Text " or "
          Link form=Markdown target="https://example.com/parse?x=1"
            Code "parse"
          Text "."
"#,
    );
}

#[test]
fn a_code_span_keeps_what_is_inside_it_as_written() {
    check(
        "Use `[[not a link]]` and ``a ` tick`` here.\n",
        r#"Document
  MarkdownBlock
    Section
      Heading
      Paragraph
        Line
          Text "Use "
          Code "[[not a link]]"
          Text " and "
          Code "a ` tick"
          Text " here."
"#,
    );
}

#[test]
fn a_backslash_before_punctuation_makes_it_plain_text() {
    check(
        r"\[\[not a link\]\] and \* star, a path C:\temp
",
        r#"Document
  MarkdownBlock
    Section
      Heading
      Paragraph
        Line
          Text "[[not a link]] and * star, a path C:\\temp"
"#,
    );
}

#[test]
fn brackets_that_do_not_form_a_link_stay_text() {
    check(
        "[text] (gap), [text](has space), [[]], [[a]b]], [unclosed, an ![image](pic.png)\n",
        r#"Document
  MarkdownBlock
    Section
      Heading
      Paragraph
        Line
          Text "[text] (gap), [text](has space), [[]], [[a]b]], [unclosed, an "
          Image form=Markdown target="pic.png" alt="image"
"#,
    );
}

#[test]
fn each_line_of_a_paragraph_holds_the_inline_nodes_written_on_it() {
    check(
        "\
Start [[a]]
[b](c) end
",
        r#"Document
  MarkdownBlock
    Section
      Heading
      Paragraph
        Line
          Text "Start "
          Link form=Wiki target="a"
        Line
          Link form=Markdown target="c"
            Text "b"
          Text " end"
"#,
    );
}

#[test]
fn backticks_with_no_matching_run_stay_text() {
    check(
        "A lone ` tick.\n\n``two` ticks\n",
        r#"Document
  MarkdownBlock
    Section
      Heading
      Paragraph
        Line
          Text "A lone ` tick."
      Paragraph
        Line
          Text "``two` ticks"
"#,
    );
}

#[test]
fn text_outside_ascii_is_kept_and_spans_stay_on_character_boundaries() {
    check(
        "+++\ntitle = \"caf\u{e9}\"\n+++\ncaf\u{e9} [[na\u{ef}ve|\u{f1}]] `\u{2014}`\n",
        "\
Document
  Frontmatter format=Toml
    Mapping
      Pair key=title
        Scalar String \"caf\u{e9}\"
  MarkdownBlock
    Section
      Heading
      Paragraph
        Line
          Text \"caf\u{e9} \"
          Link form=Wiki target=\"na\u{ef}ve\"
            Text \"\u{f1}\"
          Text \" \"
          Code \"\u{2014}\"
",
    );
}

#[test]
fn an_inline_node_stays_on_one_line_and_link_text_holds_no_brackets() {
    check(
        "\
[text
more](dest) and [a [b] c](d)
",
        r#"Document
  MarkdownBlock
    Section
      Heading
      Paragraph
        Line
          Text "[text"
        Line
          Text "more](dest) and [a [b] c](d)"
"#,
    );
}

#[test]
fn an_image_is_an_embed_in_either_form_and_may_sit_inside_a_link() {
    check(
        "Hi! ![logo](logo.svg) ![[tree.png]] ![[note#Heading|the note]] [![build](badge.svg)](https://ci) ![not an image\n",
        r#"Document
  MarkdownBlock
    Section
      Heading
      Paragraph
        Line
          Text "Hi! "
          Image form=Markdown target="logo.svg" alt="logo"
          Text " "
          Image form=Wiki target="tree.png"
          Text " "
          Image form=Wiki target="note#Heading" alt="the note"
          Text " "
          Link form=Markdown target="https://ci"
            Image form=Markdown target="badge.svg" alt="build"
          Text " ![not an image"
"#,
    );
}

#[test]
fn emphasis_and_strong_are_read_from_their_delimiters() {
    check(
        "*emphasis* and **strong** are read.\n",
        r#"Document
  MarkdownBlock
    Section
      Heading
      Paragraph
        Line
          Emphasis
            Text "emphasis"
          Text " and "
          Strong
            Text "strong"
          Text " are read."
"#,
    );
}

#[test]
fn reference_links_autolinks_and_inline_html_are_read() {
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
          Link form=Markdown target="https://example.com"
            Text "label"
          Text " and "
          Link form=Markdown target="https://example.com"
            Text "https://example.com"
          Text " and "
          Html "<b>"
          Text "html"
          Html "</b>"
"#,
    );
}

#[test]
fn one_delimiter_is_emphasis_and_two_are_strong_with_either_character() {
    check(
        "*a* _b_ **c** __d__\n",
        r#"Document
  MarkdownBlock
    Section
      Heading
      Paragraph
        Line
          Emphasis
            Text "a"
          Text " "
          Emphasis
            Text "b"
          Text " "
          Strong
            Text "c"
          Text " "
          Strong
            Text "d"
"#,
    );
}

#[test]
fn three_delimiters_are_emphasis_around_strong() {
    check(
        "***c***\n",
        r#"Document
  MarkdownBlock
    Section
      Heading
      Paragraph
        Line
          Emphasis
            Strong
              Text "c"
"#,
    );
}

#[test]
fn an_underscore_inside_a_word_opens_no_emphasis() {
    check(
        "snake_case_word and _foo_bar_\n",
        r#"Document
  MarkdownBlock
    Section
      Heading
      Paragraph
        Line
          Text "snake_case_word and "
          Emphasis
            Text "foo_bar"
"#,
    );
}

#[test]
fn delimiter_runs_pair_by_the_rules_of_commonmark() {
    check(
        "\
**foo*

*foo**

*foo**bar**baz*

*(*foo*)*

foo-_(bar)_

***a** b*

**a*b*c**

a * not emphasis *
",
        r#"Document
  MarkdownBlock
    Section
      Heading
      Paragraph
        Line
          Text "*"
          Emphasis
            Text "foo"
      Paragraph
        Line
          Emphasis
            Text "foo"
          Text "*"
      Paragraph
        Line
          Emphasis
            Text "foo"
            Strong
              Text "bar"
            Text "baz"
      Paragraph
        Line
          Emphasis
            Text "("
            Emphasis
              Text "foo"
            Text ")"
      Paragraph
        Line
          Text "foo-"
          Emphasis
            Text "(bar)"
      Paragraph
        Line
          Emphasis
            Strong
              Text "a"
            Text " b"
      Paragraph
        Line
          Strong
            Text "a"
            Emphasis
              Text "b"
            Text "c"
      Paragraph
        Line
          Text "a * not emphasis *"
"#,
    );
}

#[test]
fn an_escaped_delimiter_is_text() {
    check(
        r"\*not emphasis\* and *\*starred\**
",
        r#"Document
  MarkdownBlock
    Section
      Heading
      Paragraph
        Line
          Text "*not emphasis* and "
          Emphasis
            Text "*starred*"
"#,
    );
}

#[test]
fn inline_html_is_an_html_node_as_written() {
    check(
        "a <span class=\"x\">b</span> c <br> d <!-- note --> <?php x ?> <!DOCTYPE html> <![CDATA[ y ]]> <a/>\n",
        r#"Document
  MarkdownBlock
    Section
      Heading
      Paragraph
        Line
          Text "a "
          Html "<span class=\"x\">"
          Text "b"
          Html "</span>"
          Text " c "
          Html "<br>"
          Text " d "
          Html "<!-- note -->"
          Text " "
          Html "<?php x ?>"
          Text " "
          Html "<!DOCTYPE html>"
          Text " "
          Html "<![CDATA[ y ]]>"
          Text " "
          Html "<a/>"
"#,
    );
}

#[test]
fn a_less_than_sign_that_starts_no_tag_is_text() {
    check(
        "1 < 2, <a b=>, <1a>, <not closed\n",
        r#"Document
  MarkdownBlock
    Section
      Heading
      Paragraph
        Line
          Text "1 < 2, <a b=>, <1a>, <not closed"
"#,
    );
}

#[test]
fn an_autolink_is_a_link_to_what_it_shows() {
    check(
        "<https://example.com> and <ada@example.com>\n",
        r#"Document
  MarkdownBlock
    Section
      Heading
      Paragraph
        Line
          Link form=Markdown target="https://example.com"
            Text "https://example.com"
          Text " and "
          Link form=Markdown target="mailto:ada@example.com"
            Text "ada@example.com"
"#,
    );
}

#[test]
fn a_backslash_or_two_spaces_before_another_line_end_their_line_with_a_line_break() {
    check(
        "first\\\nsecond  \nthird\n",
        r#"Document
  MarkdownBlock
    Section
      Heading
      Paragraph
        Line
          Text "first"
          LineBreak
        Line
          Text "second"
          LineBreak
        Line
          Text "third"
"#,
    );
}

#[test]
fn the_last_line_of_a_paragraph_ends_with_no_line_break() {
    check(
        "a\\\n\nb  \n\nc \\\nd\n",
        r#"Document
  MarkdownBlock
    Section
      Heading
      Paragraph
        Line
          Text "a\\"
      Paragraph
        Line
          Text "b"
      Paragraph
        Line
          Text "c "
          LineBreak
        Line
          Text "d"
"#,
    );
}

#[test]
fn one_space_or_an_escaped_backslash_at_the_end_of_a_line_is_no_line_break() {
    check(
        "a \nb\\ \nc\\\\\nd\n",
        r#"Document
  MarkdownBlock
    Section
      Heading
      Paragraph
        Line
          Text "a"
        Line
          Text "b\\"
        Line
          Text "c\\"
        Line
          Text "d"
"#,
    );
}

#[test]
fn a_line_break_inside_an_underlined_heading_stays_in_it() {
    check(
        "a\\\nb\n===\n",
        r#"Document
  MarkdownBlock
    Section
      Heading
        Text "a"
        LineBreak
        Text "b"
"#,
    );
}

#[test]
fn a_reference_link_takes_its_target_from_the_definition_of_its_label() {
    check(
        "\
[x][1] [1][] [1] and [X][Guide] [guide][] [GUIDE]

[1]: u
[guide]: /g
",
        r#"Document
  MarkdownBlock
    Section
      Heading
      Paragraph
        Line
          Link form=Markdown target="u"
            Text "x"
          Text " "
          Link form=Markdown target="u"
            Text "1"
          Text " "
          Link form=Markdown target="u"
            Text "1"
          Text " and "
          Link form=Markdown target="/g"
            Text "X"
          Text " "
          Link form=Markdown target="/g"
            Text "guide"
          Text " "
          Link form=Markdown target="/g"
            Text "GUIDE"
"#,
    );
}

#[test]
fn a_label_with_no_definition_stays_text() {
    check(
        "[the guide][nope], [nope][] and [nope]\n",
        r#"Document
  MarkdownBlock
    Section
      Heading
      Paragraph
        Line
          Text "[the guide][nope], [nope][] and [nope]"
"#,
    );
}

#[test]
fn brackets_followed_by_a_label_are_no_shortcut_and_an_inline_link_comes_first() {
    check(
        "\
[foo][bar][baz] and [foo](not a link)

[baz]: /url1
[foo]: /url2
",
        r#"Document
  MarkdownBlock
    Section
      Heading
      Paragraph
        Line
          Text "[foo]"
          Link form=Markdown target="/url1"
            Text "bar"
          Text " and "
          Link form=Markdown target="/url2"
            Text "foo"
          Text "(not a link)"
"#,
    );
}

#[test]
fn inline_nodes_nest_inside_links_emphasis_and_strong() {
    check(
        "[*the* guide](x) and **[a](x)** and *[b*](x)\n",
        r#"Document
  MarkdownBlock
    Section
      Heading
      Paragraph
        Line
          Link form=Markdown target="x"
            Emphasis
              Text "the"
            Text " guide"
          Text " and "
          Strong
            Link form=Markdown target="x"
              Text "a"
          Text " and *"
          Link form=Markdown target="x"
            Text "b*"
"#,
    );
}

#[test]
fn a_code_span_inside_link_text_may_hold_a_bracket() {
    check(
        "[a `]` b](x)\n",
        r#"Document
  MarkdownBlock
    Section
      Heading
      Paragraph
        Line
          Link form=Markdown target="x"
            Text "a "
            Code "]"
            Text " b"
"#,
    );
}

#[test]
fn a_paragraph_holds_strong_links_line_breaks_and_html_together() {
    check(
        "Read **[the guide][1]**,\\\nthen <span>stop</span>.\n\n[1]: https://example.com/guide\n",
        r#"Document
  MarkdownBlock
    Section
      Heading
      Paragraph
        Line
          Text "Read "
          Strong
            Link form=Markdown target="https://example.com/guide"
              Text "the guide"
          Text ","
          LineBreak
        Line
          Text "then "
          Html "<span>"
          Text "stop"
          Html "</span>"
          Text "."
"#,
    );
}

#[test]
fn labels_match_by_unicode_case_folding_and_collapsed_whitespace() {
    check(
        "[\u{1E9E}] and [Foo Bar]\n\n[SS]: /url\n[foo   bar]: /u\n",
        "\
Document
  MarkdownBlock
    Section
      Heading
      Paragraph
        Line
          Link form=Markdown target=\"/url\"
            Text \"\u{1E9E}\"
          Text \" and \"
          Link form=Markdown target=\"/u\"
            Text \"Foo Bar\"
",
    );
}

#[test]
fn a_destination_is_held_with_its_escapes_and_references_resolved() {
    check(
        "\
[a](/u\\_x) [b] [c](/u&amp;v) [d](</my url>) [e](/&#35;&#x41;&#0;&bogus;)

[b]: /u\\_x
",
        "\
Document
  MarkdownBlock
    Section
      Heading
      Paragraph
        Line
          Link form=Markdown target=\"/u_x\"
            Text \"a\"
          Text \" \"
          Link form=Markdown target=\"/u_x\"
            Text \"b\"
          Text \" \"
          Link form=Markdown target=\"/u&v\"
            Text \"c\"
          Text \" \"
          Link form=Markdown target=\"/my url\"
            Text \"d\"
          Text \" \"
          Link form=Markdown target=\"/#A\u{FFFD}&bogus;\"
            Text \"e\"
",
    );
}

#[test]
fn a_link_or_an_image_keeps_its_title_in_any_of_its_forms() {
    check(
        "\
[a](/u \"T\") [b](/u 'T') [c](/u (T)) [d] [e](/u \"T&amp;\\\"q\") [f](/u) ![g](/i \"T\")

[d]: /u \"T\"
",
        r#"Document
  MarkdownBlock
    Section
      Heading
      Paragraph
        Line
          Link form=Markdown target="/u" title="T"
            Text "a"
          Text " "
          Link form=Markdown target="/u" title="T"
            Text "b"
          Text " "
          Link form=Markdown target="/u" title="T"
            Text "c"
          Text " "
          Link form=Markdown target="/u" title="T"
            Text "d"
          Text " "
          Link form=Markdown target="/u" title="T&\"q"
            Text "e"
          Text " "
          Link form=Markdown target="/u"
            Text "f"
          Text " "
          Image form=Markdown target="/i" title="T" alt="g"
"#,
    );
}

#[test]
fn a_reference_link_with_a_title_and_an_escaped_destination() {
    check(
        "[\u{1E9E}][] and [b](/u\\_x \"T\")\n\n[SS]: /url\n",
        "\
Document
  MarkdownBlock
    Section
      Heading
      Paragraph
        Line
          Link form=Markdown target=\"/url\"
            Text \"\u{1E9E}\"
          Text \" and \"
          Link form=Markdown target=\"/u_x\" title=\"T\"
            Text \"b\"
",
    );
}

#[test]
fn flanking_counts_unicode_punctuation_and_whitespace() {
    check(
        "a*\u{201C}foo\u{201D}* and a*\"foo\"* and *\u{A0}a* and *\u{A3}x\u{A3}*\n",
        "\
Document
  MarkdownBlock
    Section
      Heading
      Paragraph
        Line
          Text \"a*\u{201C}foo\u{201D}* and a*\\\"foo\\\"* and *\\u{a0}a* and \"
          Emphasis
            Text \"\u{A3}x\u{A3}\"
",
    );
}
