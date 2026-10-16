use super::check;

#[test]
fn a_wiki_link_has_a_target_and_may_have_an_alias() {
    check(
        "See [[grammar]], [[notes/parser|the parser notes]] and [[spec#Lists]].\n",
        r#"Document
  Body
    Paragraph
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
  Body
    Paragraph
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
  Body
    Paragraph
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
  Body
    Paragraph
      Text "[[not a link]] and * star, a path C:\\temp"
"#,
    );
}

#[test]
fn brackets_that_do_not_form_a_link_stay_text() {
    check(
        "[text] (gap), [text](has space), [[]], [[a]b]], [unclosed, an ![image](pic.png)\n",
        r#"Document
  Body
    Paragraph
      Text "[text] (gap), [text](has space), [[]], [[a]b]], [unclosed, an "
      Image form=Markdown target="pic.png" alt="image"
"#,
    );
}

#[test]
fn a_line_break_inside_a_paragraph_is_a_newline_in_the_text() {
    check(
        "\
Start [[a]]
[b](c) end
",
        r#"Document
  Body
    Paragraph
      Text "Start "
      Link form=Wiki target="a"
      Text "\n"
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
  Body
    Paragraph
      Text "A lone ` tick."
    Paragraph
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
  Body
    Paragraph
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
  Body
    Paragraph
      Text "[text\nmore](dest) and [a [b] c](d)"
"#,
    );
}

#[test]
fn an_image_is_an_embed_in_either_form_and_may_sit_inside_a_link() {
    check(
        "Hi! ![logo](logo.svg) ![[tree.png]] ![[note#Heading|the note]] [![build](badge.svg)](https://ci) ![not an image\n",
        r#"Document
  Body
    Paragraph
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
