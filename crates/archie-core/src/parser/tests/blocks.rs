use super::check;

#[test]
fn adjacent_lines_are_one_paragraph_and_a_blank_line_ends_it() {
    check(
        "\
First line
second line of the same paragraph.

A second paragraph.
",
        r#"Document
  Body
    Paragraph
      Text "First line\nsecond line of the same paragraph."
    Paragraph
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
  Body
    Paragraph
      Text "Intro before any heading."
    Section level=1
      Heading
        Text "Alpha"
      Paragraph
        Text "Text in alpha."
      Section level=2
        Heading
          Text "Beta"
        Paragraph
          Text "Text in beta."
        Section level=3
          Heading
            Text "Gamma"
      Section level=2
        Heading
          Text "Delta"
    Section level=1
      Heading
        Text "Epsilon"
"#,
    );
}

#[test]
fn a_skipped_level_still_nests_under_the_nearest_higher_heading() {
    check(
        "\
## Starts at two

# One

### Three

## Two
",
        r#"Document
  Body
    Section level=2
      Heading
        Text "Starts at two"
    Section level=1
      Heading
        Text "One"
      Section level=3
        Heading
          Text "Three"
      Section level=2
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
  Body
    Paragraph
      Text \"#NoSpace is a paragraph\"
    Paragraph
      Text \"####### Seven hashes is a paragraph\"
    Section level=1
      Heading
      Section level=2
        Heading
          Text \"See \"
          Link form=Wiki target=\"grammar\"
          Text \" and \"
          Code \"parse\"
",
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
  Body
    List
      Item
        Paragraph
          Text "one"
      Item
        Paragraph
          Text "two\ncontinues two"
      Item
        Paragraph
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
  Body
    List
      Item
        Paragraph
          Text "parent"
        List
          Item
            Paragraph
              Text "child"
            List
              Item
                Paragraph
                  Text "grandchild"
          Item
            Paragraph
              Text "second child"
        Paragraph
          Text "A second paragraph of parent."
      Item
        Paragraph
          Text "sibling"
"#,
    );
}

#[test]
fn blank_lines_and_other_bullets_keep_one_list_and_an_item_may_be_empty() {
    check(
        "\
* star

+ plus

-
- after the empty item
",
        r#"Document
  Body
    List
      Item
        Paragraph
          Text "star"
      Item
        Paragraph
          Text "plus"
      Item
      Item
        Paragraph
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
  Body
    List
      Item
        Paragraph
          Text "item"
    Paragraph
      Text "continues at column zero"
"#,
    );
}

#[test]
fn a_bullet_after_a_bullet_on_one_line_nests() {
    check(
        "- - inner\n",
        r#"Document
  Body
    List
      Item
        List
          Item
            Paragraph
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
  Body
    CodeBlock info="rust" value="fn main() {\n    // # not a heading\n    - not a list\n}"
    Paragraph
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
  Body
    List
      Item
        Paragraph
          Text "step"
        CodeBlock info="text" value="```\ninner fence\n\nstays code"
      Item
        Paragraph
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
  Body
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
  Body
    List
      Item
        Paragraph
          Text "item"
        CodeBlock info="" value="code"
      Item
        Paragraph
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
  Body
    Paragraph
      Text "Text line"
    List
      Item
        Paragraph
          Text "item"
    Section level=1
      Heading
        Text "Heading"
      Paragraph
        Text "text under the heading"
"#,
    );
}

#[test]
fn an_empty_file_is_a_document_with_an_empty_body() {
    check("", "Document\n  Body\n");
    check("\n\n", "Document\n  Body\n");
}

#[test]
fn the_item_text_column_is_two_past_the_bullet_however_it_is_spaced() {
    check(
        "\
-   wide
  - inner
",
        r#"Document
  Body
    List
      Item
        Paragraph
          Text "wide"
        List
          Item
            Paragraph
              Text "inner"
"#,
    );
}

#[test]
fn indentation_outside_a_list_means_nothing_and_a_tab_is_text() {
    check(
        "    four spaces\n\n\t- tab\n",
        r#"Document
  Body
    Paragraph
      Text "four spaces"
    Paragraph
      Text "\t- tab"
"#,
    );
}

#[test]
fn closing_hashes_of_a_heading_are_kept_as_text() {
    check(
        "# Title ##\n",
        r#"Document
  Body
    Section level=1
      Heading
        Text "Title ##"
"#,
    );
}

#[test]
fn a_fence_of_tildes_is_text() {
    check(
        "\
~~~
code
~~~
",
        r#"Document
  Body
    Paragraph
      Text "~~~\ncode\n~~~"
"#,
    );
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
  Body
    Paragraph
      Text "Intro."
    CodeBlock info="remargin" value="---\nid: abc\nauthor: ada\ntype: human\n---\nLooks good."
    Paragraph
      Text "More text."
"#,
    );
}
