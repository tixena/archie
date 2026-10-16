use super::check_spans;

#[test]
fn a_text_spans_the_indentation_between_its_lines() {
    let whole = "- a\n  b\n";
    check_spans(
        whole,
        &[
            ("Document", whole),
            ("Body", whole),
            ("List", "- a\n  b"),
            ("Item", "- a\n  b"),
            ("Paragraph", "a\n  b"),
            (r#"Text "a\nb""#, "a\n  b"),
        ],
    );
}

#[test]
fn a_section_runs_from_its_heading_to_the_end_of_its_last_block() {
    let whole = "# A\n\ntext\n\n## B\n\n# C\n";
    check_spans(
        whole,
        &[
            ("Document", whole),
            ("Body", whole),
            ("Section level=1", "# A\n\ntext\n\n## B"),
            ("Heading", "# A"),
            (r#"Text "A""#, "A"),
            ("Paragraph", "text"),
            (r#"Text "text""#, "text"),
            ("Section level=2", "## B"),
            ("Heading", "## B"),
            (r#"Text "B""#, "B"),
            ("Section level=1", "# C"),
            ("Heading", "# C"),
            (r#"Text "C""#, "C"),
        ],
    );
}

#[test]
fn a_code_block_spans_from_its_opening_fence_to_its_closing_fence() {
    let whole = "- x\n  ```sh\n  ls\n  ```\n";
    check_spans(
        whole,
        &[
            ("Document", whole),
            ("Body", whole),
            ("List", "- x\n  ```sh\n  ls\n  ```"),
            ("Item", "- x\n  ```sh\n  ls\n  ```"),
            ("Paragraph", "x"),
            (r#"Text "x""#, "x"),
            (r#"CodeBlock info="sh" value="ls""#, "```sh\n  ls\n  ```"),
        ],
    );
}

#[test]
fn inline_nodes_span_their_delimiters() {
    let whole = "a [b](c) `d` [[e|f]]\n";
    check_spans(
        whole,
        &[
            ("Document", whole),
            ("Body", whole),
            ("Paragraph", "a [b](c) `d` [[e|f]]"),
            (r#"Text "a ""#, "a "),
            (r#"Link form=Markdown target="c""#, "[b](c)"),
            (r#"Text "b""#, "b"),
            (r#"Text " ""#, " "),
            (r#"Code "d""#, "`d`"),
            (r#"Text " ""#, " "),
            (r#"Link form=Wiki target="e""#, "[[e|f]]"),
            (r#"Text "f""#, "f"),
        ],
    );
}

#[test]
fn yaml_frontmatter_spans() {
    let whole = "---\na: 1\nb:\n  - x\nc:\n---\nbody\n";
    check_spans(
        whole,
        &[
            ("Document", whole),
            ("Frontmatter format=Yaml", "---\na: 1\nb:\n  - x\nc:\n---"),
            ("Mapping", "a: 1\nb:\n  - x\nc:"),
            ("Pair key=a", "a: 1"),
            ("Scalar Integer 1", "1"),
            ("Pair key=b", "b:\n  - x"),
            ("Sequence", "- x"),
            (r#"Scalar String "x""#, "x"),
            ("Pair key=c", "c:"),
            ("Scalar Null", ""),
            ("Body", "body\n"),
            ("Paragraph", "body"),
            (r#"Text "body""#, "body"),
        ],
    );
}

#[test]
fn toml_frontmatter_spans() {
    let whole = "+++\nk = [1, \"s\"]\nt = { a = 2 }\n+++\n";
    check_spans(
        whole,
        &[
            ("Document", whole),
            (
                "Frontmatter format=Toml",
                "+++\nk = [1, \"s\"]\nt = { a = 2 }\n+++",
            ),
            ("Mapping", "k = [1, \"s\"]\nt = { a = 2 }"),
            ("Pair key=k", "k = [1, \"s\"]"),
            ("Sequence", "[1, \"s\"]"),
            ("Scalar Integer 1", "1"),
            (r#"Scalar String "s""#, "\"s\""),
            ("Pair key=t", "t = { a = 2 }"),
            ("Mapping", "{ a = 2 }"),
            ("Pair key=a", "a = 2"),
            ("Scalar Integer 2", "2"),
            ("Body", ""),
        ],
    );
}
