use super::{check, check_spans};
use crate::parser::ast::Block;
use crate::parser::parse;

/// A simple table whose columns show the four alignments.
const SIMPLE_TABLE: &str = "  Right     Left     Center     Default
-------     ------ ----------   -------
     12     12        12            12
    123     123       123          123
      1     1          1             1
";

const SIMPLE_TABLE_TREE: &str = r#"Document
  MarkdownBlock
    Section
      Heading
      Table format=Simple alignments=Right,Left,Center,Default
        TableRow head
          TableCell
            MarkdownBlock
              Section
                Heading
                Paragraph
                  Line
                    Text "Right"
          TableCell
            MarkdownBlock
              Section
                Heading
                Paragraph
                  Line
                    Text "Left"
          TableCell
            MarkdownBlock
              Section
                Heading
                Paragraph
                  Line
                    Text "Center"
          TableCell
            MarkdownBlock
              Section
                Heading
                Paragraph
                  Line
                    Text "Default"
        TableRow
          TableCell
            MarkdownBlock
              Section
                Heading
                Paragraph
                  Line
                    Text "12"
          TableCell
            MarkdownBlock
              Section
                Heading
                Paragraph
                  Line
                    Text "12"
          TableCell
            MarkdownBlock
              Section
                Heading
                Paragraph
                  Line
                    Text "12"
          TableCell
            MarkdownBlock
              Section
                Heading
                Paragraph
                  Line
                    Text "12"
        TableRow
          TableCell
            MarkdownBlock
              Section
                Heading
                Paragraph
                  Line
                    Text "123"
          TableCell
            MarkdownBlock
              Section
                Heading
                Paragraph
                  Line
                    Text "123"
          TableCell
            MarkdownBlock
              Section
                Heading
                Paragraph
                  Line
                    Text "123"
          TableCell
            MarkdownBlock
              Section
                Heading
                Paragraph
                  Line
                    Text "123"
        TableRow
          TableCell
            MarkdownBlock
              Section
                Heading
                Paragraph
                  Line
                    Text "1"
          TableCell
            MarkdownBlock
              Section
                Heading
                Paragraph
                  Line
                    Text "1"
          TableCell
            MarkdownBlock
              Section
                Heading
                Paragraph
                  Line
                    Text "1"
          TableCell
            MarkdownBlock
              Section
                Heading
                Paragraph
                  Line
                    Text "1"
"#;

#[test]
fn a_pipe_table_reads_alignments_escaped_pipes_and_empty_cells() {
    check(
        r"| Name | Role | Notes |
|:-----|:----:|------:|
| Ada | host | uses `parse` and a \| pipe |
| Grace |  | [[review]] |
After the table.
",
        r#"Document
  MarkdownBlock
    Section
      Heading
      Table format=Pipe alignments=Left,Center,Right
        TableRow head
          TableCell
            MarkdownBlock
              Section
                Heading
                Paragraph
                  Line
                    Text "Name"
          TableCell
            MarkdownBlock
              Section
                Heading
                Paragraph
                  Line
                    Text "Role"
          TableCell
            MarkdownBlock
              Section
                Heading
                Paragraph
                  Line
                    Text "Notes"
        TableRow
          TableCell
            MarkdownBlock
              Section
                Heading
                Paragraph
                  Line
                    Text "Ada"
          TableCell
            MarkdownBlock
              Section
                Heading
                Paragraph
                  Line
                    Text "host"
          TableCell
            MarkdownBlock
              Section
                Heading
                Paragraph
                  Line
                    Text "uses "
                    Code "parse"
                    Text " and a | pipe"
        TableRow
          TableCell
            MarkdownBlock
              Section
                Heading
                Paragraph
                  Line
                    Text "Grace"
          TableCell
            MarkdownBlock
          TableCell
            MarkdownBlock
              Section
                Heading
                Paragraph
                  Line
                    Link form=Wiki target="review"
      Paragraph
        Line
          Text "After the table."
"#,
    );
}

#[test]
fn a_pipe_table_may_leave_out_its_outer_pipes() {
    check(
        "\
a | b
--|--
1 | 2
",
        r#"Document
  MarkdownBlock
    Section
      Heading
      Table format=Pipe alignments=Default,Default
        TableRow head
          TableCell
            MarkdownBlock
              Section
                Heading
                Paragraph
                  Line
                    Text "a"
          TableCell
            MarkdownBlock
              Section
                Heading
                Paragraph
                  Line
                    Text "b"
        TableRow
          TableCell
            MarkdownBlock
              Section
                Heading
                Paragraph
                  Line
                    Text "1"
          TableCell
            MarkdownBlock
              Section
                Heading
                Paragraph
                  Line
                    Text "2"
"#,
    );
}

#[test]
fn a_pipe_row_of_another_width_widens_the_table_or_gets_empty_cells() {
    check(
        "\
| a | b |
|---|---|
| 1 | 2 | 3 |
| 4 |
",
        r#"Document
  MarkdownBlock
    Section
      Heading
      Table format=Pipe alignments=Default,Default,Default
        TableRow head
          TableCell
            MarkdownBlock
              Section
                Heading
                Paragraph
                  Line
                    Text "a"
          TableCell
            MarkdownBlock
              Section
                Heading
                Paragraph
                  Line
                    Text "b"
          TableCell
            MarkdownBlock
        TableRow
          TableCell
            MarkdownBlock
              Section
                Heading
                Paragraph
                  Line
                    Text "1"
          TableCell
            MarkdownBlock
              Section
                Heading
                Paragraph
                  Line
                    Text "2"
          TableCell
            MarkdownBlock
              Section
                Heading
                Paragraph
                  Line
                    Text "3"
        TableRow
          TableCell
            MarkdownBlock
              Section
                Heading
                Paragraph
                  Line
                    Text "4"
          TableCell
            MarkdownBlock
          TableCell
            MarkdownBlock
"#,
    );
}

#[test]
fn a_dash_line_of_another_width_still_makes_a_table() {
    check(
        "\
| a | b |
|---|---|---|
| 1 | 2 |
",
        r#"Document
  MarkdownBlock
    Section
      Heading
      Table format=Pipe alignments=Default,Default,Default
        TableRow head
          TableCell
            MarkdownBlock
              Section
                Heading
                Paragraph
                  Line
                    Text "a"
          TableCell
            MarkdownBlock
              Section
                Heading
                Paragraph
                  Line
                    Text "b"
          TableCell
            MarkdownBlock
        TableRow
          TableCell
            MarkdownBlock
              Section
                Heading
                Paragraph
                  Line
                    Text "1"
          TableCell
            MarkdownBlock
              Section
                Heading
                Paragraph
                  Line
                    Text "2"
          TableCell
            MarkdownBlock
"#,
    );
}

#[test]
fn a_column_the_dash_line_does_not_reach_has_the_default_alignment() {
    check(
        "\
| a |
|:-:|
| 1 | 2 |
",
        r#"Document
  MarkdownBlock
    Section
      Heading
      Table format=Pipe alignments=Center,Default
        TableRow head
          TableCell
            MarkdownBlock
              Section
                Heading
                Paragraph
                  Line
                    Text "a"
          TableCell
            MarkdownBlock
        TableRow
          TableCell
            MarkdownBlock
              Section
                Heading
                Paragraph
                  Line
                    Text "1"
          TableCell
            MarkdownBlock
              Section
                Heading
                Paragraph
                  Line
                    Text "2"
"#,
    );
}

#[test]
fn a_pipe_cell_holds_a_markdown_block_with_its_headings() {
    check(
        "\
| x |
|---|
| # Foo |
",
        r#"Document
  MarkdownBlock
    Section
      Heading
      Table format=Pipe alignments=Default
        TableRow head
          TableCell
            MarkdownBlock
              Section
                Heading
                Paragraph
                  Line
                    Text "x"
        TableRow
          TableCell
            MarkdownBlock
              Section
                Heading
                  Text "Foo"
"#,
    );
}

#[test]
fn a_line_with_no_pipe_or_a_header_with_no_dash_line_under_it_is_text() {
    check(
        "\
a | b
plain

| a | b |
| 1 | 2 |
",
        r#"Document
  MarkdownBlock
    Section
      Heading
      Paragraph
        Line
          Text "a | b"
        Line
          Text "plain"
      Paragraph
        Line
          Text "| a | b |"
        Line
          Text "| 1 | 2 |"
"#,
    );
}

#[test]
fn a_table_may_sit_inside_a_list_item() {
    check(
        "\
- Results:

  | x | y |
  |---|--:|
  | 1 | 2 |
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
                  Text "Results:"
              Table format=Pipe alignments=Default,Right
                TableRow head
                  TableCell
                    MarkdownBlock
                      Section
                        Heading
                        Paragraph
                          Line
                            Text "x"
                  TableCell
                    MarkdownBlock
                      Section
                        Heading
                        Paragraph
                          Line
                            Text "y"
                TableRow
                  TableCell
                    MarkdownBlock
                      Section
                        Heading
                        Paragraph
                          Line
                            Text "1"
                  TableCell
                    MarkdownBlock
                      Section
                        Heading
                        Paragraph
                          Line
                            Text "2"
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
fn a_grid_table_reads_its_header_and_the_alignments_of_its_separator() {
    check(
        "\
+-------+-------+
| Name  | Role  |
+:======+:=====:+
| Ada   | host  |
+-------+-------+
| Grace | review|
+-------+-------+
",
        r#"Document
  MarkdownBlock
    Section
      Heading
      Table format=Grid alignments=Left,Center
        TableRow head
          TableCell
            MarkdownBlock
              Section
                Heading
                Paragraph
                  Line
                    Text "Name"
          TableCell
            MarkdownBlock
              Section
                Heading
                Paragraph
                  Line
                    Text "Role"
        TableRow
          TableCell
            MarkdownBlock
              Section
                Heading
                Paragraph
                  Line
                    Text "Ada"
          TableCell
            MarkdownBlock
              Section
                Heading
                Paragraph
                  Line
                    Text "host"
        TableRow
          TableCell
            MarkdownBlock
              Section
                Heading
                Paragraph
                  Line
                    Text "Grace"
          TableCell
            MarkdownBlock
              Section
                Heading
                Paragraph
                  Line
                    Text "review"
"#,
    );
}

#[test]
fn a_grid_row_may_run_over_several_lines() {
    check(
        "\
+-----+-----+
| a   | b   |
| c   | d   |
+-----+-----+
",
        r#"Document
  MarkdownBlock
    Section
      Heading
      Table format=Grid alignments=Default,Default
        TableRow
          TableCell
            MarkdownBlock
              Section
                Heading
                Paragraph
                  Line
                    Text "a"
                  Line
                    Text "c"
          TableCell
            MarkdownBlock
              Section
                Heading
                Paragraph
                  Line
                    Text "b"
                  Line
                    Text "d"
"#,
    );
}

#[test]
fn a_grid_wall_out_of_line_leaves_every_character_in_a_cell() {
    check(
        "\
+-----+-----+
| a  | b    |
+-----+-----+

+-----+-----+
| a   | b   |
| c  | d    |
+-----+-----+
",
        r#"Document
  MarkdownBlock
    Section
      Heading
      Table format=Grid alignments=Default,Default
        TableRow
          TableCell
            MarkdownBlock
              Section
                Heading
                Paragraph
                  Line
                    Text "a  |"
          TableCell
            MarkdownBlock
              Section
                Heading
                Paragraph
                  Line
                    Text "b"
      Table format=Grid alignments=Default,Default
        TableRow
          TableCell
            MarkdownBlock
              Section
                Heading
                Paragraph
                  Line
                    Text "a"
                  Line
                    Text "c  |"
          TableCell
            MarkdownBlock
              Section
                Heading
                Paragraph
                  Line
                    Text "b"
                  Line
                    Text "d"
"#,
    );
}

#[test]
fn a_grid_cell_holds_a_markdown_block_over_its_lines() {
    check(
        "\
+-------+-----+
| - a   | x   |
| - b   |     |
|   - c |     |
+-------+-----+
",
        r#"Document
  MarkdownBlock
    Section
      Heading
      Table format=Grid alignments=Default,Default
        TableRow
          TableCell
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
                  Item
                    MarkdownBlock
                      Section
                        Heading
                        Paragraph
                          Line
                            Text "b"
                        List
                          Item
                            MarkdownBlock
                              Section
                                Heading
                                Paragraph
                                  Line
                                    Text "c"
          TableCell
            MarkdownBlock
              Section
                Heading
                Paragraph
                  Line
                    Text "x"
"#,
    );
}

#[test]
fn a_simple_table_takes_each_alignment_from_where_its_header_sits() {
    check(SIMPLE_TABLE, SIMPLE_TABLE_TREE);
}

#[test]
fn a_simple_row_that_does_not_fit_its_columns_keeps_every_word_in_a_cell() {
    check(
        "  Name    Role
  ----    ----
  Ada     host
  Grace Hopper reviews
X Linus   maintains
",
        r#"Document
  MarkdownBlock
    Section
      Heading
      Table format=Simple alignments=Default,Default
        TableRow head
          TableCell
            MarkdownBlock
              Section
                Heading
                Paragraph
                  Line
                    Text "Name"
          TableCell
            MarkdownBlock
              Section
                Heading
                Paragraph
                  Line
                    Text "Role"
        TableRow
          TableCell
            MarkdownBlock
              Section
                Heading
                Paragraph
                  Line
                    Text "Ada"
          TableCell
            MarkdownBlock
              Section
                Heading
                Paragraph
                  Line
                    Text "host"
        TableRow
          TableCell
            MarkdownBlock
              Section
                Heading
                Paragraph
                  Line
                    Text "Grace Hopper"
          TableCell
            MarkdownBlock
              Section
                Heading
                Paragraph
                  Line
                    Text "reviews"
        TableRow
          TableCell
            MarkdownBlock
              Section
                Heading
                Paragraph
                  Line
                    Text "X Linus"
          TableCell
            MarkdownBlock
              Section
                Heading
                Paragraph
                  Line
                    Text "maintains"
"#,
    );
}

#[test]
fn a_simple_table_with_no_rows_is_not_a_table() {
    check(
        "\
Name    Role
----    ----
",
        r#"Document
  MarkdownBlock
    Section
      Heading
      Paragraph
        Line
          Text "Name    Role"
      ThematicBreak
"#,
    );
}

#[test]
fn a_table_spans_from_its_first_line_to_its_last_and_a_cell_spans_its_text() {
    let whole = "| a | b |\n|---|---|\n| 1 |   |\n";
    check_spans(
        whole,
        &[
            ("Document", whole),
            ("MarkdownBlock", whole),
            ("Section", "| a | b |\n|---|---|\n| 1 |   |"),
            ("Heading", ""),
            (
                "Table format=Pipe alignments=Default,Default",
                "| a | b |\n|---|---|\n| 1 |   |",
            ),
            ("TableRow head", "| a | b |"),
            ("TableCell", "a"),
            ("MarkdownBlock", "a"),
            ("Section", "a"),
            ("Heading", ""),
            ("Paragraph", "a"),
            ("Line", "a"),
            (r#"Text "a""#, "a"),
            ("TableCell", "b"),
            ("MarkdownBlock", "b"),
            ("Section", "b"),
            ("Heading", ""),
            ("Paragraph", "b"),
            ("Line", "b"),
            (r#"Text "b""#, "b"),
            ("TableRow", "| 1 |   |"),
            ("TableCell", "1"),
            ("MarkdownBlock", "1"),
            ("Section", "1"),
            ("Heading", ""),
            ("Paragraph", "1"),
            ("Line", "1"),
            (r#"Text "1""#, "1"),
            ("TableCell", ""),
            ("MarkdownBlock", ""),
        ],
    );
}

#[test]
fn an_added_cell_is_empty_at_the_end_of_its_row() {
    let whole = "| a |\n|---|\n| 1 | 2 |\n";
    let table = "| a |\n|---|\n| 1 | 2 |";
    check_spans(
        whole,
        &[
            ("Document", whole),
            ("MarkdownBlock", whole),
            ("Section", table),
            ("Heading", ""),
            ("Table format=Pipe alignments=Default,Default", table),
            ("TableRow head", "| a |"),
            ("TableCell", "a"),
            ("MarkdownBlock", "a"),
            ("Section", "a"),
            ("Heading", ""),
            ("Paragraph", "a"),
            ("Line", "a"),
            (r#"Text "a""#, "a"),
            ("TableCell", ""),
            ("MarkdownBlock", ""),
            ("TableRow", "| 1 | 2 |"),
            ("TableCell", "1"),
            ("MarkdownBlock", "1"),
            ("Section", "1"),
            ("Heading", ""),
            ("Paragraph", "1"),
            ("Line", "1"),
            (r#"Text "1""#, "1"),
            ("TableCell", "2"),
            ("MarkdownBlock", "2"),
            ("Section", "2"),
            ("Heading", ""),
            ("Paragraph", "2"),
            ("Line", "2"),
            (r#"Text "2""#, "2"),
        ],
    );
    let document = parse(whole).unwrap();
    let block = &document.body.sections[0].blocks[0];
    assert!(matches!(block, Block::Table(read) if read.head[0].cells[1].span == (5..5)));
}

#[test]
fn a_grid_cell_spans_from_its_first_line_to_its_last() {
    let whole = "+---+---+\n| a | b |\n| c | d |\n+---+---+\n";
    let table = "+---+---+\n| a | b |\n| c | d |\n+---+---+";
    check_spans(
        whole,
        &[
            ("Document", whole),
            ("MarkdownBlock", whole),
            ("Section", table),
            ("Heading", ""),
            ("Table format=Grid alignments=Default,Default", table),
            ("TableRow", "| a | b |\n| c | d |"),
            ("TableCell", "a | b |\n| c"),
            ("MarkdownBlock", "a | b |\n| c"),
            ("Section", "a | b |\n| c"),
            ("Heading", ""),
            ("Paragraph", "a | b |\n| c"),
            ("Line", "a"),
            (r#"Text "a""#, "a"),
            ("Line", "c"),
            (r#"Text "c""#, "c"),
            ("TableCell", "b |\n| c | d"),
            ("MarkdownBlock", "b |\n| c | d"),
            ("Section", "b |\n| c | d"),
            ("Heading", ""),
            ("Paragraph", "b |\n| c | d"),
            ("Line", "b"),
            (r#"Text "b""#, "b"),
            ("Line", "d"),
            (r#"Text "d""#, "d"),
        ],
    );
}
