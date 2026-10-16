use super::{check, check_spans};

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
fn a_pipe_table_with_a_row_or_a_delimiter_of_the_wrong_width_is_text() {
    check(
        "\
| a | b |
|---|---|
| 1 | 2 | 3 |

| a | b |
|---|---|---|
",
        r#"Document
  MarkdownBlock
    Section
      Heading
      Paragraph
        Line
          Text "| a | b |"
        Line
          Text "|---|---|"
        Line
          Text "| 1 | 2 | 3 |"
      Paragraph
        Line
          Text "| a | b |"
        Line
          Text "|---|---|---|"
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
fn a_grid_table_with_a_misplaced_wall_or_a_row_over_two_lines_is_text() {
    check(
        "\
+-----+-----+
| a  | b    |
+-----+-----+

+-----+-----+
| a   | b   |
| c   | d   |
+-----+-----+
",
        r#"Document
  MarkdownBlock
    Section
      Heading
      Paragraph
        Line
          Text "+-----+-----+"
        Line
          Text "| a  | b    |"
        Line
          Text "+-----+-----+"
      Paragraph
        Line
          Text "+-----+-----+"
        Line
          Text "| a   | b   |"
        Line
          Text "| c   | d   |"
        Line
          Text "+-----+-----+"
"#,
    );
}

#[test]
fn a_simple_table_takes_each_alignment_from_where_its_header_sits() {
    check(SIMPLE_TABLE, SIMPLE_TABLE_TREE);
}

#[test]
fn a_simple_table_with_a_word_across_a_column_edge_or_no_rows_is_text() {
    check(
        "\
Name    Role
----    ----
Ada     host
Grace Hopper reviews

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
        Line
          Text "----    ----"
        Line
          Text "Ada     host"
        Line
          Text "Grace Hopper reviews"
      Paragraph
        Line
          Text "Name    Role"
        Line
          Text "----    ----"
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
