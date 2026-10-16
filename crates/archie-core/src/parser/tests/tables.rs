use super::{check, check_spans};

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
  Body
    Table format=Pipe alignments=Left,Center,Right
      TableRow head
        TableCell
          Paragraph
            Text "Name"
        TableCell
          Paragraph
            Text "Role"
        TableCell
          Paragraph
            Text "Notes"
      TableRow
        TableCell
          Paragraph
            Text "Ada"
        TableCell
          Paragraph
            Text "host"
        TableCell
          Paragraph
            Text "uses "
            Code "parse"
            Text " and a | pipe"
      TableRow
        TableCell
          Paragraph
            Text "Grace"
        TableCell
        TableCell
          Paragraph
            Link form=Wiki target="review"
    Paragraph
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
  Body
    Table format=Pipe alignments=Default,Default
      TableRow head
        TableCell
          Paragraph
            Text "a"
        TableCell
          Paragraph
            Text "b"
      TableRow
        TableCell
          Paragraph
            Text "1"
        TableCell
          Paragraph
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
  Body
    Paragraph
      Text "| a | b |\n|---|---|\n| 1 | 2 | 3 |"
    Paragraph
      Text "| a | b |\n|---|---|---|"
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
  Body
    List
      Item
        Paragraph
          Text "Results:"
        Table format=Pipe alignments=Default,Right
          TableRow head
            TableCell
              Paragraph
                Text "x"
            TableCell
              Paragraph
                Text "y"
          TableRow
            TableCell
              Paragraph
                Text "1"
            TableCell
              Paragraph
                Text "2"
      Item
        Paragraph
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
  Body
    Table format=Grid alignments=Left,Center
      TableRow head
        TableCell
          Paragraph
            Text "Name"
        TableCell
          Paragraph
            Text "Role"
      TableRow
        TableCell
          Paragraph
            Text "Ada"
        TableCell
          Paragraph
            Text "host"
      TableRow
        TableCell
          Paragraph
            Text "Grace"
        TableCell
          Paragraph
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
  Body
    Paragraph
      Text "+-----+-----+\n| a  | b    |\n+-----+-----+"
    Paragraph
      Text "+-----+-----+\n| a   | b   |\n| c   | d   |\n+-----+-----+"
"#,
    );
}

#[test]
fn a_simple_table_takes_each_alignment_from_where_its_header_sits() {
    check(
        "  Right     Left     Center     Default
-------     ------ ----------   -------
     12     12        12            12
    123     123       123          123
      1     1          1             1
",
        r#"Document
  Body
    Table format=Simple alignments=Right,Left,Center,Default
      TableRow head
        TableCell
          Paragraph
            Text "Right"
        TableCell
          Paragraph
            Text "Left"
        TableCell
          Paragraph
            Text "Center"
        TableCell
          Paragraph
            Text "Default"
      TableRow
        TableCell
          Paragraph
            Text "12"
        TableCell
          Paragraph
            Text "12"
        TableCell
          Paragraph
            Text "12"
        TableCell
          Paragraph
            Text "12"
      TableRow
        TableCell
          Paragraph
            Text "123"
        TableCell
          Paragraph
            Text "123"
        TableCell
          Paragraph
            Text "123"
        TableCell
          Paragraph
            Text "123"
      TableRow
        TableCell
          Paragraph
            Text "1"
        TableCell
          Paragraph
            Text "1"
        TableCell
          Paragraph
            Text "1"
        TableCell
          Paragraph
            Text "1"
"#,
    );
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
  Body
    Paragraph
      Text "Name    Role\n----    ----\nAda     host\nGrace Hopper reviews"
    Paragraph
      Text "Name    Role\n----    ----"
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
            ("Body", whole),
            (
                "Table format=Pipe alignments=Default,Default",
                "| a | b |\n|---|---|\n| 1 |   |",
            ),
            ("TableRow head", "| a | b |"),
            ("TableCell", "a"),
            ("Paragraph", "a"),
            (r#"Text "a""#, "a"),
            ("TableCell", "b"),
            ("Paragraph", "b"),
            (r#"Text "b""#, "b"),
            ("TableRow", "| 1 |   |"),
            ("TableCell", "1"),
            ("Paragraph", "1"),
            (r#"Text "1""#, "1"),
            ("TableCell", ""),
        ],
    );
}
