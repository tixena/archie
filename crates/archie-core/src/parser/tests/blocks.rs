use super::check;

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
fn a_fence_of_tildes_is_text() {
    check(
        "\
~~~
code
~~~
",
        r#"Document
  MarkdownBlock
    Section
      Heading
      Paragraph
        Line
          Text "~~~"
        Line
          Text "code"
        Line
          Text "~~~"
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
