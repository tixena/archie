use core::iter;

use super::{check, on_two_megabytes};
use crate::parser::ast::{Block, Document, Inline, MarkdownBlock, Section};
use crate::parser::block;
use crate::parser::extension::Markdown;
use crate::parser::inline::INLINE_CAP;
use crate::parser::lexer::BLOCK_CAP;
use crate::parser::{from_json, parse, to_json};
use crate::writer::{WriteOptions, write};

/// A container that `headed_containers` nests.
#[derive(Clone, Copy)]
enum Container {
    Grid,
    List,
    Markdown,
}

#[test]
fn a_paragraph_is_a_set_of_lines_and_a_blank_line_ends_it() {
    check(
        "\
First line
second line of the same paragraph.

A second paragraph.
",
        r#"Document
  MarkdownBlock
    Section
      Heading
      Paragraph
        Line
          Text "First line"
        Line
          Text "second line of the same paragraph."
      Paragraph
        Line
          Text "A second paragraph."
"#,
    );
}

#[test]
fn a_heading_owns_what_follows_until_a_heading_of_its_level_or_higher() {
    check(
        "\
Intro before any heading.

# Alpha

Text in alpha.

## Beta

Text in beta.

### Gamma

## Delta

# Epsilon
",
        r#"Document
  MarkdownBlock
    Section
      Heading
      Paragraph
        Line
          Text "Intro before any heading."
    Section
      Heading
        Text "Alpha"
      Paragraph
        Line
          Text "Text in alpha."
      Section
        Heading
          Text "Beta"
        Paragraph
          Line
            Text "Text in beta."
        Section
          Heading
            Text "Gamma"
      Section
        Heading
          Text "Delta"
    Section
      Heading
        Text "Epsilon"
"#,
    );
}

#[test]
fn a_deeper_heading_opens_a_section_inside_the_current_one() {
    check(
        "\
# A
text

## D
text in D

# E
",
        r#"Document
  MarkdownBlock
    Section
      Heading
        Text "A"
      Paragraph
        Line
          Text "text"
      Section
        Heading
          Text "D"
        Paragraph
          Line
            Text "text in D"
    Section
      Heading
        Text "E"
"#,
    );
}

#[test]
fn a_skipped_level_is_a_section_with_an_empty_heading() {
    let tree = r#"Document
  MarkdownBlock
    Section
      Heading
        Text "Hi H1"
      Section
        Heading
        Section
          Heading
            Text "This is H3"
"#;
    check("# Hi H1\n### This is H3\n", tree);
    check("# Hi H1\n##\n### This is H3\n", tree);
}

#[test]
fn text_before_a_deeper_first_heading_holds_that_heading_as_a_subsection() {
    check(
        "intro\n\n## B\n",
        r#"Document
  MarkdownBlock
    Section
      Heading
      Paragraph
        Line
          Text "intro"
      Section
        Heading
          Text "B"
"#,
    );
}

#[test]
fn a_body_whose_first_heading_is_deeper_starts_with_an_empty_heading() {
    check(
        "\
## Starts at two

# One

### Three

## Two
",
        r#"Document
  MarkdownBlock
    Section
      Heading
      Section
        Heading
          Text "Starts at two"
    Section
      Heading
        Text "One"
      Section
        Heading
        Section
          Heading
            Text "Three"
      Section
        Heading
          Text "Two"
"#,
    );
}

#[test]
fn a_heading_needs_one_to_six_hashes_and_a_space() {
    check(
        "\
#NoSpace is a paragraph

####### Seven hashes is a paragraph

#

## See [[grammar]] and `parse`
",
        "\
Document
  MarkdownBlock
    Section
      Heading
      Paragraph
        Line
          Text \"#NoSpace is a paragraph\"
      Paragraph
        Line
          Text \"####### Seven hashes is a paragraph\"
    Section
      Heading
      Section
        Heading
          Text \"See \"
          Link form=Wiki target=\"grammar\"
          Text \" and \"
          Code \"parse\"
",
    );
}

#[test]
fn a_list_item_holds_sections_whose_levels_count_from_the_item() {
    check(
        "\
- # B
  in B
  # C
  in C
",
        r#"Document
  MarkdownBlock
    Section
      Heading
      List
        Item
          MarkdownBlock
            Section
              Heading
                Text "B"
              Paragraph
                Line
                  Text "in B"
            Section
              Heading
                Text "C"
              Paragraph
                Line
                  Text "in C"
"#,
    );
}

#[test]
fn a_line_indented_to_the_item_text_continues_the_item() {
    check(
        "\
- one
- two
  continues two
- three
",
        r#"Document
  MarkdownBlock
    Section
      Heading
      List
        Item
          MarkdownBlock
            Section
              Heading
              Paragraph
                Line
                  Text "one"
        Item
          MarkdownBlock
            Section
              Heading
              Paragraph
                Line
                  Text "two"
                Line
                  Text "continues two"
        Item
          MarkdownBlock
            Section
              Heading
              Paragraph
                Line
                  Text "three"
"#,
    );
}

#[test]
fn lists_nest_by_indentation_and_an_item_holds_several_blocks() {
    check(
        "\
- parent
  - child
    - grandchild
  - second child

  A second paragraph of parent.
- sibling
",
        r#"Document
  MarkdownBlock
    Section
      Heading
      List loose
        Item
          MarkdownBlock
            Section
              Heading
              Paragraph
                Line
                  Text "parent"
              List
                Item
                  MarkdownBlock
                    Section
                      Heading
                      Paragraph
                        Line
                          Text "child"
                      List
                        Item
                          MarkdownBlock
                            Section
                              Heading
                              Paragraph
                                Line
                                  Text "grandchild"
                Item
                  MarkdownBlock
                    Section
                      Heading
                      Paragraph
                        Line
                          Text "second child"
              Paragraph
                Line
                  Text "A second paragraph of parent."
        Item
          MarkdownBlock
            Section
              Heading
              Paragraph
                Line
                  Text "sibling"
"#,
    );
}

#[test]
fn another_bullet_character_starts_a_new_list_and_an_item_may_be_empty() {
    check(
        "\
* star

+ plus

-
- after the empty item
",
        r#"Document
  MarkdownBlock
    Section
      Heading
      List
        Item
          MarkdownBlock
            Section
              Heading
              Paragraph
                Line
                  Text "star"
      List
        Item
          MarkdownBlock
            Section
              Heading
              Paragraph
                Line
                  Text "plus"
      List
        Item
          MarkdownBlock
        Item
          MarkdownBlock
            Section
              Heading
              Paragraph
                Line
                  Text "after the empty item"
"#,
    );
}

#[test]
fn a_line_left_of_the_item_text_ends_the_list() {
    check(
        "\
- item
continues at column zero
",
        r#"Document
  MarkdownBlock
    Section
      Heading
      List
        Item
          MarkdownBlock
            Section
              Heading
              Paragraph
                Line
                  Text "item"
      Paragraph
        Line
          Text "continues at column zero"
"#,
    );
}

#[test]
fn a_bullet_after_a_bullet_on_one_line_nests() {
    check(
        "- - inner\n",
        r#"Document
  MarkdownBlock
    Section
      Heading
      List
        Item
          MarkdownBlock
            Section
              Heading
              List
                Item
                  MarkdownBlock
                    Section
                      Heading
                      Paragraph
                        Line
                          Text "inner"
"#,
    );
}

#[test]
fn a_fenced_code_block_keeps_its_lines_as_they_are() {
    check(
        "\
```rust
fn main() {
    // # not a heading
    - not a list
}
```

After the code.
",
        r#"Document
  MarkdownBlock
    Section
      Heading
      CodeBlock info="rust" value="fn main() {\n    // # not a heading\n    - not a list\n}"
      Paragraph
        Line
          Text "After the code."
"#,
    );
}

#[test]
fn a_fence_inside_an_item_closes_only_on_a_fence_at_least_as_long() {
    check(
        "\
- step

  ````text
  ```
  inner fence

  stays code
  ````
- next
",
        r#"Document
  MarkdownBlock
    Section
      Heading
      List loose
        Item
          MarkdownBlock
            Section
              Heading
              Paragraph
                Line
                  Text "step"
              CodeBlock info="text" value="```\ninner fence\n\nstays code"
        Item
          MarkdownBlock
            Section
              Heading
              Paragraph
                Line
                  Text "next"
"#,
    );
}

#[test]
fn an_unclosed_fence_runs_to_the_end_of_the_file() {
    check(
        "\
```
never closed
# still code
",
        r#"Document
  MarkdownBlock
    Section
      Heading
      CodeBlock info="" value="never closed\n# still code"
"#,
    );
}

#[test]
fn an_unclosed_fence_inside_an_item_ends_with_the_item() {
    check(
        "\
- item
  ```
  code
- next
",
        r#"Document
  MarkdownBlock
    Section
      Heading
      List
        Item
          MarkdownBlock
            Section
              Heading
              Paragraph
                Line
                  Text "item"
              CodeBlock info="" value="code"
        Item
          MarkdownBlock
            Section
              Heading
              Paragraph
                Line
                  Text "next"
"#,
    );
}

#[test]
fn a_list_or_a_heading_ends_a_paragraph_without_a_blank_line() {
    check(
        "\
Text line
- item
# Heading
text under the heading
",
        r#"Document
  MarkdownBlock
    Section
      Heading
      Paragraph
        Line
          Text "Text line"
      List
        Item
          MarkdownBlock
            Section
              Heading
              Paragraph
                Line
                  Text "item"
    Section
      Heading
        Text "Heading"
      Paragraph
        Line
          Text "text under the heading"
"#,
    );
}

#[test]
fn an_empty_file_is_a_document_with_an_empty_body() {
    check("", "Document\n  MarkdownBlock\n");
    check("\n\n", "Document\n  MarkdownBlock\n");
}

#[test]
fn the_item_text_starts_at_its_first_word() {
    check(
        "\
-   wide
  - inner
1.  one
    continues one
",
        r#"Document
  MarkdownBlock
    Section
      Heading
      List
        Item
          MarkdownBlock
            Section
              Heading
              Paragraph
                Line
                  Text "wide"
        Item
          MarkdownBlock
            Section
              Heading
              Paragraph
                Line
                  Text "inner"
      List start=1
        Item
          MarkdownBlock
            Section
              Heading
              Paragraph
                Line
                  Text "one"
                Line
                  Text "continues one"
"#,
    );
}

#[test]
fn four_spaces_of_indentation_are_a_code_block_and_a_tab_is_text() {
    check(
        "    four spaces\n\n\t- tab\n",
        r#"Document
  MarkdownBlock
    Section
      Heading
      CodeBlock info="" value="four spaces"
      Paragraph
        Line
          Text "\t- tab"
"#,
    );
}

#[test]
fn a_new_bullet_character_or_delimiter_starts_a_new_list() {
    check(
        "\
- one
* two
+ three

1. a
1) b
",
        r#"Document
  MarkdownBlock
    Section
      Heading
      List
        Item
          MarkdownBlock
            Section
              Heading
              Paragraph
                Line
                  Text "one"
      List
        Item
          MarkdownBlock
            Section
              Heading
              Paragraph
                Line
                  Text "two"
      List
        Item
          MarkdownBlock
            Section
              Heading
              Paragraph
                Line
                  Text "three"
      List start=1
        Item
          MarkdownBlock
            Section
              Heading
              Paragraph
                Line
                  Text "a"
      List start=1
        Item
          MarkdownBlock
            Section
              Heading
              Paragraph
                Line
                  Text "b"
"#,
    );
}

#[test]
fn an_ordered_list_keeps_its_first_number() {
    check(
        "\
3. third
4. fourth
",
        r#"Document
  MarkdownBlock
    Section
      Heading
      List start=3
        Item
          MarkdownBlock
            Section
              Heading
              Paragraph
                Line
                  Text "third"
        Item
          MarkdownBlock
            Section
              Heading
              Paragraph
                Line
                  Text "fourth"
"#,
    );
}

#[test]
fn an_ordered_marker_has_one_to_nine_digits() {
    check(
        "\
123456789) nine digits

1234567890) ten digits
",
        r#"Document
  MarkdownBlock
    Section
      Heading
      List start=123456789
        Item
          MarkdownBlock
            Section
              Heading
              Paragraph
                Line
                  Text "nine digits"
      Paragraph
        Line
          Text "1234567890) ten digits"
"#,
    );
}

#[test]
fn an_ordered_list_interrupts_a_paragraph_only_when_it_starts_at_one() {
    check(
        "\
text
2. two

text
1. one
",
        r#"Document
  MarkdownBlock
    Section
      Heading
      Paragraph
        Line
          Text "text"
        Line
          Text "2. two"
      Paragraph
        Line
          Text "text"
      List start=1
        Item
          MarkdownBlock
            Section
              Heading
              Paragraph
                Line
                  Text "one"
"#,
    );
}

#[test]
fn a_blank_line_between_items_or_between_two_blocks_of_an_item_makes_a_list_loose() {
    check(
        "\
- a

- b

# Next

- a
- b

  second block of b
- c

# Tight

- a
- b
",
        r#"Document
  MarkdownBlock
    Section
      Heading
      List loose
        Item
          MarkdownBlock
            Section
              Heading
              Paragraph
                Line
                  Text "a"
        Item
          MarkdownBlock
            Section
              Heading
              Paragraph
                Line
                  Text "b"
    Section
      Heading
        Text "Next"
      List loose
        Item
          MarkdownBlock
            Section
              Heading
              Paragraph
                Line
                  Text "a"
        Item
          MarkdownBlock
            Section
              Heading
              Paragraph
                Line
                  Text "b"
              Paragraph
                Line
                  Text "second block of b"
        Item
          MarkdownBlock
            Section
              Heading
              Paragraph
                Line
                  Text "c"
    Section
      Heading
        Text "Tight"
      List
        Item
          MarkdownBlock
            Section
              Heading
              Paragraph
                Line
                  Text "a"
        Item
          MarkdownBlock
            Section
              Heading
              Paragraph
                Line
                  Text "b"
"#,
    );
}

#[test]
fn a_blank_line_inside_a_nested_list_makes_only_the_list_it_separates_loose() {
    check(
        "\
- a
  - b

    c
- d
",
        r#"Document
  MarkdownBlock
    Section
      Heading
      List
        Item
          MarkdownBlock
            Section
              Heading
              Paragraph
                Line
                  Text "a"
              List loose
                Item
                  MarkdownBlock
                    Section
                      Heading
                      Paragraph
                        Line
                          Text "b"
                      Paragraph
                        Line
                          Text "c"
        Item
          MarkdownBlock
            Section
              Heading
              Paragraph
                Line
                  Text "d"
"#,
    );
}

#[test]
fn a_blank_line_closing_a_nested_list_separates_the_items_around_it() {
    check(
        "\
- a
  - b

- c
",
        r#"Document
  MarkdownBlock
    Section
      Heading
      List loose
        Item
          MarkdownBlock
            Section
              Heading
              Paragraph
                Line
                  Text "a"
              List
                Item
                  MarkdownBlock
                    Section
                      Heading
                      Paragraph
                        Line
                          Text "b"
        Item
          MarkdownBlock
            Section
              Heading
              Paragraph
                Line
                  Text "c"
"#,
    );
}

#[test]
fn text_indented_four_columns_past_the_item_text_is_a_code_block() {
    check(
        "\
1. first
2. second

- item

      code here
",
        r#"Document
  MarkdownBlock
    Section
      Heading
      List start=1
        Item
          MarkdownBlock
            Section
              Heading
              Paragraph
                Line
                  Text "first"
        Item
          MarkdownBlock
            Section
              Heading
              Paragraph
                Line
                  Text "second"
      List loose
        Item
          MarkdownBlock
            Section
              Heading
              Paragraph
                Line
                  Text "item"
              CodeBlock info="" value="code here"
"#,
    );
}

#[test]
fn a_marker_followed_by_five_spaces_starts_its_text_one_space_after_it() {
    check(
        "\
-     code
-      indented code
1.     indented code

   paragraph

       more code
",
        r#"Document
  MarkdownBlock
    Section
      Heading
      List
        Item
          MarkdownBlock
            Section
              Heading
              CodeBlock info="" value="code"
        Item
          MarkdownBlock
            Section
              Heading
              CodeBlock info="" value=" indented code"
      List start=1 loose
        Item
          MarkdownBlock
            Section
              Heading
              CodeBlock info="" value="indented code"
              Paragraph
                Line
                  Text "paragraph"
              CodeBlock info="" value="more code"
"#,
    );
}

#[test]
fn an_indented_code_block_keeps_the_blank_lines_between_its_lines() {
    check(
        "text\n\n    code\n\n      indented\n    \n\n# Heading\n    after the heading\n",
        r#"Document
  MarkdownBlock
    Section
      Heading
      Paragraph
        Line
          Text "text"
      CodeBlock info="" value="code\n\n  indented"
    Section
      Heading
        Text "Heading"
      CodeBlock info="" value="after the heading"
"#,
    );
}

#[test]
fn an_indented_line_after_paragraph_text_continues_the_paragraph() {
    check(
        "\
Some text
    more text
    # not a heading
    - not an item
",
        r##"Document
  MarkdownBlock
    Section
      Heading
      Paragraph
        Line
          Text "Some text"
        Line
          Text "more text"
        Line
          Text "# not a heading"
        Line
          Text "- not an item"
"##,
    );
}

#[test]
fn closing_hashes_of_a_heading_are_kept_as_text() {
    check(
        "# Title ##\n",
        r#"Document
  MarkdownBlock
    Section
      Heading
        Text "Title ##"
"#,
    );
}

#[test]
fn a_fence_of_tildes_is_a_code_block() {
    for (source, info, value) in [
        ("~~~\ncode\n~~~\n", "", "code"),
        ("~~~python\nprint(1)\n~~~\n", "python", "print(1)"),
        ("~~~ a`b\nx\n~~~\n", "a`b", "x"),
    ] {
        check(source, &code_block_outline(info, value));
    }
}

#[test]
fn a_fence_closes_only_on_its_own_character_at_least_as_long() {
    for (source, value) in [
        ("~~~\n```\n~~~\n", "```"),
        ("~~~~\nx\n~~~\n~~~~\n", "x\n~~~"),
        ("```\nx\n~~~\n", "x\n~~~"),
    ] {
        check(source, &code_block_outline("", value));
    }
}

/// The outline of a document holding only a code block.
fn code_block_outline(info: &str, value: &str) -> String {
    format!(
        "\
Document
  MarkdownBlock
    Section
      Heading
      CodeBlock info={info:?} value={value:?}
"
    )
}

#[test]
fn a_fence_of_tildes_inside_an_item_is_a_code_block_as_one_of_backticks_is() {
    for fence in ["```", "~~~"] {
        check(
            &format!("- step\n\n  {fence}text\n  code\n  {fence}\n- next\n"),
            r#"Document
  MarkdownBlock
    Section
      Heading
      List loose
        Item
          MarkdownBlock
            Section
              Heading
              Paragraph
                Line
                  Text "step"
              CodeBlock info="text" value="code"
        Item
          MarkdownBlock
            Section
              Heading
              Paragraph
                Line
                  Text "next"
"#,
        );
    }
}

#[test]
fn a_comment_block_written_by_remargin_is_a_code_block() {
    check(
        "\
Intro.

```remargin
---
id: abc
author: ada
type: human
---
Looks good.
```

More text.
",
        r#"Document
  MarkdownBlock
    Section
      Heading
      Paragraph
        Line
          Text "Intro."
      CodeBlock info="remargin" value="---\nid: abc\nauthor: ada\ntype: human\n---\nLooks good."
      Paragraph
        Line
          Text "More text."
"#,
    );
}

#[test]
fn a_markdown_block_inside_a_nested_item_keeps_its_lines_from_the_fence_column() {
    check(
        "- - ```markdown\n    # A\n    ```\n",
        r##"Document
  MarkdownBlock
    Section
      Heading
      List
        Item
          MarkdownBlock
            Section
              Heading
              List
                Item
                  MarkdownBlock
                    Section
                      Heading
                      CodeBlock info="markdown" value="# A" claimed
"##,
    );
}

#[test]
fn an_empty_item_does_not_interrupt_a_paragraph() {
    for marker in ["*", "1."] {
        check(
            &format!("text\n{marker}\nmore\n"),
            &format!(
                r#"Document
  MarkdownBlock
    Section
      Heading
      Paragraph
        Line
          Text "text"
        Line
          Text "{marker}"
        Line
          Text "more"
"#
            ),
        );
    }
}

#[test]
fn an_empty_item_ends_at_a_blank_line_after_its_marker() {
    check(
        "-\n\n  foo\n",
        r#"Document
  MarkdownBlock
    Section
      Heading
      List
        Item
          MarkdownBlock
      Paragraph
        Line
          Text "foo"
"#,
    );
    check(
        "1.\n\n   foo\n",
        r#"Document
  MarkdownBlock
    Section
      Heading
      List start=1
        Item
          MarkdownBlock
      Paragraph
        Line
          Text "foo"
"#,
    );
    check(
        "- -\n\n  foo\n",
        r#"Document
  MarkdownBlock
    Section
      Heading
      List loose
        Item
          MarkdownBlock
            Section
              Heading
              List
                Item
                  MarkdownBlock
              Paragraph
                Line
                  Text "foo"
"#,
    );
}

#[test]
fn an_empty_item_holds_the_text_on_the_line_right_after_its_marker() {
    check(
        "-\n  foo\n",
        r#"Document
  MarkdownBlock
    Section
      Heading
      List
        Item
          MarkdownBlock
            Section
              Heading
              Paragraph
                Line
                  Text "foo"
"#,
    );
}

#[test]
fn a_block_quote_holds_its_lines_as_text_without_their_marks() {
    check(
        "\
> # not a heading
> - not a list
>no space
>
>  two spaces
",
        r##"Document
  MarkdownBlock
    Section
      Heading
      BlockQuote "# not a heading\n- not a list\nno space\n\n two spaces"
"##,
    );
}

#[test]
fn a_line_without_a_mark_ends_a_block_quote() {
    check(
        "\
> a
b

> c

> d
",
        r#"Document
  MarkdownBlock
    Section
      Heading
      BlockQuote "a"
      Paragraph
        Line
          Text "b"
      BlockQuote "c"
      BlockQuote "d"
"#,
    );
}

#[test]
fn three_or_more_dashes_stars_or_underscores_are_a_thematic_break() {
    check(
        "\
text

---

***

___

- - -

* * *

  _ _ _ _
",
        r#"Document
  MarkdownBlock
    Section
      Heading
      Paragraph
        Line
          Text "text"
      ThematicBreak
      ThematicBreak
      ThematicBreak
      ThematicBreak
      ThematicBreak
      ThematicBreak
"#,
    );
}

#[test]
fn dashes_on_the_first_line_open_yaml_frontmatter_and_not_a_break() {
    check(
        "---\na: 1\n---\n",
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
fn an_underlined_line_is_a_heading_that_opens_a_section() {
    check(
        "\
Title
=====
text

Sub
---
more

# Next
",
        r#"Document
  MarkdownBlock
    Section
      Heading
        Text "Title"
      Paragraph
        Line
          Text "text"
      Section
        Heading
          Text "Sub"
        Paragraph
          Line
            Text "more"
    Section
      Heading
        Text "Next"
"#,
    );
}

#[test]
fn the_lines_of_an_underlined_heading_join_with_one_space() {
    check(
        "first\n  second `code`\n===\n",
        r#"Document
  MarkdownBlock
    Section
      Heading
        Text "first second "
        Code "code"
"#,
    );
}

#[test]
fn only_equals_signs_and_dashes_underline_paragraph_text() {
    check(
        "\
T
***

text
-
more

a
- - -

b
= =
",
        r#"Document
  MarkdownBlock
    Section
      Heading
      Paragraph
        Line
          Text "T"
      ThematicBreak
      Section
        Heading
          Text "text"
        Paragraph
          Line
            Text "more"
        Paragraph
          Line
            Text "a"
        ThematicBreak
        Paragraph
          Line
            Text "b"
          Line
            Text "= ="
"#,
    );
}

#[test]
fn an_html_block_holds_its_lines_as_written() {
    check(
        "<details>\n<summary>x</summary>\n</details>\n",
        r#"Document
  MarkdownBlock
    Section
      Heading
      Html "<details>\n<summary>x</summary>\n</details>"
"#,
    );
}

#[test]
fn each_kind_of_html_block_ends_where_commonmark_ends_it() {
    check(
        r#"<pre>

*a*
</pre> after
text

<!-- a

b --> c
<?php

?>
<!DOCTYPE html>
<![CDATA[
x
]]>
<div class="a">
# not a heading

<a href="x">
y
"#,
        r#"Document
  MarkdownBlock
    Section
      Heading
      Html "<pre>\n\n*a*\n</pre> after"
      Paragraph
        Line
          Text "text"
      Html "<!-- a\n\nb --> c"
      Html "<?php\n\n?>"
      Html "<!DOCTYPE html>"
      Html "<![CDATA[\nx\n]]>"
      Html "<div class=\"a\">\n# not a heading"
      Html "<a href=\"x\">\ny"
"#,
    );
}

#[test]
fn a_lone_tag_of_any_name_does_not_interrupt_a_paragraph() {
    check(
        "\
a
<div>
b

c
<span>
d
",
        r#"Document
  MarkdownBlock
    Section
      Heading
      Paragraph
        Line
          Text "a"
      Html "<div>\nb"
      Paragraph
        Line
          Text "c"
        Line
          Html "<span>"
        Line
          Text "d"
"#,
    );
}

#[test]
fn a_line_holding_one_whole_tag_starts_an_html_block() {
    check(
        "\
<a b='c' d=e f>

</a >

<a/ >

<a b=c=d>
",
        r#"Document
  MarkdownBlock
    Section
      Heading
      Html "<a b='c' d=e f>"
      Html "</a >"
      Paragraph
        Line
          Text "<a/ >"
      Paragraph
        Line
          Text "<a b=c=d>"
"#,
    );
}

#[test]
fn a_link_definition_gives_no_node_and_the_parser_keeps_its_target() {
    let source = "\
[1]: https://example.com/guide
[Guide  Two]: <https://example.com/guide> \"Title\"
[guide two]: /later
[a]:
/next-line
  'title'
";
    check(source, "Document\n  MarkdownBlock\n");
    let (_, definitions, _) = block::parse::<Markdown>(source, 0);
    let mut found: Vec<(&str, &str, Option<&str>)> = definitions
        .iter()
        .map(|(label, target)| {
            (
                label.as_str(),
                target.destination.as_str(),
                target.title.as_deref(),
            )
        })
        .collect();
    found.sort_unstable();
    assert_eq!(
        found,
        [
            ("1", "https://example.com/guide", None),
            ("a", "/next-line", Some("title")),
            ("guide two", "https://example.com/guide", Some("Title")),
        ]
    );
}

#[test]
fn a_definition_does_not_interrupt_a_paragraph_and_nothing_may_follow_it() {
    let source = "\
text
[a]: /u

[b]: /u \"t\" more

[c]: /u
\"t\" more
";
    check(
        source,
        r#"Document
  MarkdownBlock
    Section
      Heading
      Paragraph
        Line
          Text "text"
        Line
          Text "[a]: /u"
      Paragraph
        Line
          Text "[b]: /u \"t\" more"
      Paragraph
        Line
          Text "\"t\" more"
"#,
    );
    let (_, definitions, _) = block::parse::<Markdown>(source, 0);
    assert_eq!(definitions.len(), 1);
    assert_eq!(
        definitions
            .get("c")
            .map(|target| target.destination.as_str()),
        Some("/u")
    );
}

#[test]
fn inside_a_list_item_each_construct_counts_columns_from_the_item_text() {
    let source = "\
- > q
  > r
- a
  ---
- <div>
  x
  </div>
- [x]: /u
-    ***
";
    check(
        source,
        r#"Document
  MarkdownBlock
    Section
      Heading
      List
        Item
          MarkdownBlock
            Section
              Heading
              BlockQuote "q\nr"
        Item
          MarkdownBlock
            Section
              Heading
              Section
                Heading
                  Text "a"
        Item
          MarkdownBlock
            Section
              Heading
              Html "<div>\nx\n</div>"
        Item
          MarkdownBlock
        Item
          MarkdownBlock
            Section
              Heading
              ThematicBreak
"#,
    );
    let (_, definitions, _) = block::parse::<Markdown>(source, 0);
    assert_eq!(
        definitions
            .get("x")
            .map(|target| target.destination.as_str()),
        Some("/u")
    );
}

#[test]
fn a_quoted_line_left_of_the_item_text_ends_the_item() {
    check(
        "- > q\n> r\n",
        r#"Document
  MarkdownBlock
    Section
      Heading
      List
        Item
          MarkdownBlock
            Section
              Heading
              BlockQuote "q"
      BlockQuote "r"
"#,
    );
}

#[test]
fn a_block_quote_an_underlined_heading_a_break_and_html_read_together() {
    check(
        "> # quoted\n\nTitle\n=====\n\n***\n\n<div>\nraw\n</div>\n",
        r##"Document
  MarkdownBlock
    Section
      Heading
      BlockQuote "# quoted"
    Section
      Heading
        Text "Title"
      ThematicBreak
      Html "<div>\nraw\n</div>"
"##,
    );
}

#[test]
fn a_no_break_space_at_a_line_end_is_text_and_only_spaces_and_tabs_make_a_line_blank() {
    check(
        "a\u{a0}\nb\n",
        r#"Document
  MarkdownBlock
    Section
      Heading
      Paragraph
        Line
          Text "a\u{a0}"
        Line
          Text "b"
"#,
    );
    check(
        "a\n\u{a0}\nb\n",
        r#"Document
  MarkdownBlock
    Section
      Heading
      Paragraph
        Line
          Text "a"
        Line
          Text "\u{a0}"
        Line
          Text "b"
"#,
    );
    check(
        "a\n \t\nb\n",
        r#"Document
  MarkdownBlock
    Section
      Heading
      Paragraph
        Line
          Text "a"
      Paragraph
        Line
          Text "b"
"#,
    );
}

#[test]
fn a_heading_keeps_a_no_break_space_at_its_end() {
    check(
        "# a\u{a0}\n",
        r#"Document
  MarkdownBlock
    Section
      Heading
        Text "a\u{a0}"
"#,
    );
}

#[test]
fn a_definition_title_over_a_line_end_keeps_the_spaces_before_it() {
    check(
        "[x]: /u \"t  \nx\"\n\n[a][x]\n",
        r#"Document
  MarkdownBlock
    Section
      Heading
      Paragraph
        Line
          Link form=Markdown target="/u" title="t  \nx"
            Text "a"
"#,
    );
    check(
        "[x]: /u  \n\"t\"\n\n[a][x]\n",
        r#"Document
  MarkdownBlock
    Section
      Heading
      Paragraph
        Line
          Link form=Markdown target="/u" title="t"
            Text "a"
"#,
    );
}

/// How deep lists of one item nest from `body` down, and the body of the
/// innermost item.
fn list_depth(body: &MarkdownBlock) -> (usize, &MarkdownBlock) {
    let mut inner = body;
    let mut depth = 0_usize;
    while let [section] = inner.sections.as_slice()
        && let [Block::List(list)] = section.blocks.as_slice()
        && let [item] = list.items.as_slice()
    {
        depth += 1;
        inner = &item.body;
    }
    (depth, inner)
}

/// Checks that `document` holds lists nested `BLOCK_CAP` deep, the
/// innermost item holding one line of the text `rest`.
fn assert_block_capped(document: &Document, rest: &str) {
    let (depth, innermost) = list_depth(&document.body);
    assert_eq!(depth, BLOCK_CAP);
    let lines = match innermost.sections.as_slice() {
        [section] => match section.blocks.as_slice() {
            [Block::Paragraph(paragraph)] => paragraph.lines.as_slice(),
            _ => &[],
        },
        _ => &[],
    };
    assert!(matches!(lines, [line]
        if matches!(line.children.as_slice(), [Inline::Text(text)] if text.value == rest)));
}

#[test]
fn lists_nest_the_block_cap_deep_and_the_markers_past_it_stay_text() {
    on_two_megabytes(|| {
        let document = parse(&format!("{}a\n", "- ".repeat(10_000))).unwrap();
        let rest = format!("{}a", "- ".repeat(10_000 - BLOCK_CAP));
        assert_block_capped(&document, &rest);
        let written = write(&document, &WriteOptions::default()).unwrap();
        assert_eq!(written, format!("{}\\{rest}\n", "- ".repeat(BLOCK_CAP)));
        assert_block_capped(&parse(&written).unwrap(), &rest);
        let copy = document.clone();
        assert_eq!(copy, document);
        let printed = format!("{document:?}");
        assert_eq!(printed.matches("List(").count(), BLOCK_CAP);
        drop(copy);
        drop(document);
    });
}

#[test]
fn lists_at_the_block_cap_read_back_from_their_written_text_to_the_same_tree() {
    on_two_megabytes(|| {
        let source = format!("{}a\n", "- ".repeat(BLOCK_CAP));
        let document = parse(&source).unwrap();
        assert_eq!(list_depth(&document.body).0, BLOCK_CAP);
        let written = write(&document, &WriteOptions::default()).unwrap();
        assert_eq!(written, source);
        assert_eq!(parse(&written).unwrap(), document);
    });
}

#[test]
fn an_item_past_the_block_cap_on_its_own_line_continues_the_innermost_paragraph() {
    on_two_megabytes(|| {
        let mut source = String::new();
        for depth in 0..=BLOCK_CAP {
            source.push_str(&"  ".repeat(depth));
            source.push_str("- a\n");
        }
        let document = parse(&source).unwrap();
        let mut blocks = document.body.sections[0].blocks.as_slice();
        let mut depth = 0_usize;
        while let Some(Block::List(list)) = blocks.last()
            && let [item] = list.items.as_slice()
            && let [section] = item.body.sections.as_slice()
        {
            depth += 1;
            blocks = &section.blocks;
        }
        assert_eq!(depth, BLOCK_CAP);
        let lines = match blocks {
            [Block::Paragraph(paragraph)] => paragraph.lines.as_slice(),
            _ => &[],
        };
        let values: Vec<&str> = lines
            .iter()
            .map(|line| match line.children.as_slice() {
                [Inline::Text(text)] => text.value.as_str(),
                _ => "",
            })
            .collect();
        assert_eq!(values, ["a", "- a"]);
    });
}

/// The body of the one container `body` holds, when it holds nothing else: a
/// list of one item, a claimed markdown block, or a table of one cell.
fn only_container(body: &MarkdownBlock) -> Option<&MarkdownBlock> {
    let [section] = body.sections.as_slice() else {
        return None;
    };
    match section.blocks.as_slice() {
        [Block::CodeBlock(code)] => code.kind.0.as_deref(),
        [Block::List(list)] => match list.items.as_slice() {
            [item] => Some(&item.body),
            _ => None,
        },
        [Block::Table(table)] => match (table.head.as_slice(), table.rows.as_slice()) {
            ([], [row]) => match row.cells.as_slice() {
                [cell] => Some(&cell.body),
                _ => None,
            },
            _ => None,
        },
        _ => None,
    }
}

/// How many containers nest from `body` down, each the only block of the one
/// before, and the body of the innermost.
fn innermost(body: &MarkdownBlock) -> (usize, &MarkdownBlock) {
    let mut inner = body;
    let mut depth = 0_usize;
    while let Some(next) = only_container(inner) {
        depth += 1;
        inner = next;
    }
    (depth, inner)
}

/// How deep list items, claimed markdown blocks and table cells nest
/// anywhere in `body`.
fn container_depth(body: &MarkdownBlock) -> usize {
    let mut deepest = 0;
    let mut bodies = vec![(body, 0_usize)];
    while let Some((block, depth)) = bodies.pop() {
        deepest = deepest.max(depth);
        let mut sections: Vec<&Section> = block.sections.iter().collect();
        while let Some(section) = sections.pop() {
            sections.extend(&section.subsections);
            for each in &section.blocks {
                match each {
                    Block::CodeBlock(code) => {
                        bodies.extend(code.kind.0.as_deref().map(|inner| (inner, depth + 1)));
                    }
                    Block::List(list) => {
                        bodies.extend(list.items.iter().map(|item| (&item.body, depth + 1)));
                    }
                    Block::Table(table) => bodies.extend(
                        table
                            .head
                            .iter()
                            .chain(&table.rows)
                            .flat_map(|row| &row.cells)
                            .map(|cell| (&cell.body, depth + 1)),
                    ),
                    Block::BlockQuote(_)
                    | Block::Html(_)
                    | Block::Paragraph(_)
                    | Block::ThematicBreak(_) => {}
                }
            }
        }
    }
    deepest
}

/// `levels` markdown blocks around `a`, each inside the one before: the
/// innermost fence is three backticks, and each one around it one longer.
fn nested_markdown_blocks(levels: usize) -> String {
    let fences: Vec<String> = (3..levels + 3).map(|length| "`".repeat(length)).collect();
    let mut source = String::new();
    for fence in fences.iter().rev() {
        source.push_str(fence);
        source.push_str("markdown\n");
    }
    source.push_str("a\n");
    for fence in &fences {
        source.push_str(fence);
        source.push('\n');
    }
    source
}

/// `levels` grid tables around `a`, each in the single cell of the one
/// before.
fn nested_grid_tables(levels: usize) -> String {
    let line =
        |depth: usize, text: &str| format!("{}{text}{}\n", "| ".repeat(depth), " |".repeat(depth));
    let borders: Vec<String> = (0..levels)
        .map(|depth| {
            line(
                depth,
                &format!("+{}+", "-".repeat(4 * (levels - depth) - 1)),
            )
        })
        .collect();
    let mut source = borders.concat();
    source.push_str(&line(levels, "a"));
    for border in borders.iter().rev() {
        source.push_str(border);
    }
    source
}

/// `levels` containers around `a`, each inside the one before: a list item,
/// a markdown block and a grid table in turn, from the outside in.
fn mixed_containers(levels: usize) -> String {
    let mut lines = vec!["a".to_owned()];
    let mut fence = 2_usize;
    for level in (0..levels).rev() {
        lines = match level % 3 {
            0 => lines
                .iter()
                .enumerate()
                .map(|(index, line)| {
                    if index == 0 {
                        format!("- {line}")
                    } else {
                        format!("  {line}")
                    }
                })
                .collect(),
            1 => {
                fence += 1;
                let mark = "`".repeat(fence);
                iter::once(format!("{mark}markdown"))
                    .chain(lines)
                    .chain(iter::once(mark))
                    .collect()
            }
            _ => {
                let width = lines.iter().map(String::len).max().unwrap_or_default();
                let border = format!("+{}+", "-".repeat(width + 2));
                iter::once(border.clone())
                    .chain(lines.iter().map(|line| format!("| {line:<width$} |")))
                    .chain(iter::once(border))
                    .collect()
            }
        };
    }
    let mut source = lines.join("\n");
    source.push('\n');
    source
}

#[test]
fn markdown_blocks_nest_the_block_cap_deep_and_the_block_past_it_stays_a_code_block() {
    on_two_megabytes(|| {
        let source = nested_markdown_blocks(1000);
        let document = parse(&source).unwrap();
        let (depth, inner) = innermost(&document.body);
        assert_eq!(depth, BLOCK_CAP);
        let rest = nested_markdown_blocks(1000 - BLOCK_CAP - 1);
        let plain = match inner.sections.as_slice() {
            [section] => match section.blocks.as_slice() {
                [Block::CodeBlock(code)] => Some(code),
                _ => None,
            },
            _ => None,
        };
        assert!(plain.is_some_and(|code| code.kind.0.is_none()
            && code.info == "markdown"
            && rest.strip_suffix('\n') == Some(code.value.as_str())));
        let written = write(&document, &WriteOptions::default()).unwrap();
        assert_eq!(written, source);
        let copy = document.clone();
        assert_eq!(copy, document);
        let printed = format!("{document:?}");
        assert_eq!(printed.matches("Kind(Some(").count(), BLOCK_CAP);
        drop(copy);
        drop(document);
    });
}

#[test]
fn grid_tables_nest_the_block_cap_deep_and_the_tables_past_it_stay_text() {
    on_two_megabytes(|| {
        let source = nested_grid_tables(1000);
        let document = parse(&source).unwrap();
        let (depth, inner) = innermost(&document.body);
        assert_eq!(depth, BLOCK_CAP);
        let lines = match inner.sections.as_slice() {
            [section] => match section.blocks.as_slice() {
                [Block::Paragraph(paragraph)] => paragraph.lines.as_slice(),
                _ => &[],
            },
            _ => &[],
        };
        let texts: Vec<&str> = lines
            .iter()
            .map(|line| match line.children.as_slice() {
                [Inline::Text(text)] => text.value.as_str(),
                _ => "",
            })
            .collect();
        let rest = nested_grid_tables(1000 - BLOCK_CAP);
        assert_eq!(texts, rest.lines().collect::<Vec<_>>());
        let written = write(&document, &WriteOptions::default()).unwrap();
        assert_eq!(written.lines().count(), source.lines().count());
        let copy = document.clone();
        assert_eq!(copy, document);
        let printed = format!("{document:?}");
        assert_eq!(printed.matches("Table {").count(), BLOCK_CAP);
        drop(copy);
        drop(document);
    });
}

#[test]
fn lists_markdown_blocks_and_grid_tables_in_turn_nest_the_block_cap_deep_together() {
    on_two_megabytes(|| {
        let document = parse(&mixed_containers(1000)).unwrap();
        assert_eq!(innermost(&document.body).0, BLOCK_CAP);
        assert_eq!(container_depth(&document.body), BLOCK_CAP);
    });
}

/// `BLOCK_CAP` containers around emphasis `INLINE_CAP` deep, each inside the
/// one before and each holding the headings `# h` to `###### h` before it:
/// those of `cycle` in turn, from the outside in.
fn headed_containers(cycle: &[Container]) -> String {
    let marks = "*".repeat(2 * INLINE_CAP);
    let mut lines = vec![format!("{marks}a{marks}")];
    let mut fence = 2_usize;
    for level in (0..BLOCK_CAP).rev() {
        let headed = (1..=6)
            .map(|count| format!("{} h", "#".repeat(count)))
            .chain(lines);
        lines = match cycle[level % cycle.len()] {
            Container::Grid => {
                let rows: Vec<String> = headed.collect();
                let width = rows.iter().map(String::len).max().unwrap_or_default();
                let border = format!("+{}+", "-".repeat(width + 2));
                iter::once(border.clone())
                    .chain(rows.iter().map(|line| format!("| {line:<width$} |")))
                    .chain(iter::once(border))
                    .collect()
            }
            Container::List => headed
                .enumerate()
                .map(|(index, line)| {
                    if index == 0 {
                        format!("- {line}")
                    } else {
                        format!("  {line}")
                    }
                })
                .collect(),
            Container::Markdown => {
                fence += 1;
                let mark = "`".repeat(fence);
                iter::once(format!("{mark}markdown"))
                    .chain(headed)
                    .chain(iter::once(mark))
                    .collect()
            }
        };
    }
    let mut source = lines.join("\n");
    source.push('\n');
    source
}

#[test]
fn containers_that_hold_headings_of_every_level_nest_the_block_cap_deep() {
    on_two_megabytes(|| {
        let shapes: [&[Container]; 4] = [
            &[Container::List],
            &[Container::Markdown],
            &[Container::Grid],
            &[Container::List, Container::Markdown, Container::Grid],
        ];
        for cycle in shapes {
            let document = parse(&headed_containers(cycle)).unwrap();
            assert_eq!(container_depth(&document.body), BLOCK_CAP);
            let written = write(&document, &WriteOptions::default()).unwrap();
            assert_eq!(written.matches("###### h").count(), BLOCK_CAP);
            let copy = document.clone();
            assert_eq!(copy, document);
            let printed = format!("{document:?}");
            assert_eq!(printed.matches("Section {").count(), 1 + 6 * BLOCK_CAP);
            drop(copy);
            let json = to_json(&document).unwrap();
            let back: Document = from_json(&json).unwrap();
            assert_eq!(to_json(&back).unwrap(), json);
            drop(back);
            drop(document);
        }
    });
}
