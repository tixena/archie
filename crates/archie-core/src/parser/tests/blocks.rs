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
      List
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
fn blank_lines_and_other_bullets_keep_one_list_and_an_item_may_be_empty() {
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
        Item
          MarkdownBlock
            Section
              Heading
              Paragraph
                Line
                  Text "plus"
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
      List
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
fn the_item_text_column_is_two_past_the_bullet_however_it_is_spaced() {
    check(
        "\
-   wide
  - inner
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
fn indentation_outside_a_list_means_nothing_and_a_tab_is_text() {
    check(
        "    four spaces\n\n\t- tab\n",
        r#"Document
  MarkdownBlock
    Section
      Heading
      Paragraph
        Line
          Text "four spaces"
      Paragraph
        Line
          Text "\t- tab"
"#,
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
