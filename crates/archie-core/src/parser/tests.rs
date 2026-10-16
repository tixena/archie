//! Each sample is a markdown source and the tree it parses into, written as
//! an outline: one node per line, indented under its parent.

mod blocks;
mod documents;
mod errors;
mod example;
mod extension;
mod frontmatter;
mod inlines;
mod spans;
mod tables;
mod toml;
mod unrecognized;
mod yaml;

use core::ops::Range;

use crate::parser::ast::{
    Block, Date, DateTime, Document, Inline, Mapping, MarkdownBlock, ScalarValue, Section, Time,
    Value,
};
use crate::parser::extension::Extension;
use crate::parser::parse;

/// One node of a parsed tree: how deep it sits, how it reads, what it covers.
/// `merged` marks a pair whose key is written more than once.
struct Row {
    depth: usize,
    label: String,
    merged: bool,
    span: Range<usize>,
}

#[derive(Default)]
struct Rows {
    rows: Vec<Row>,
}

impl Rows {
    fn block<E>(&mut self, depth: usize, block: &Block<E>)
    where
        E: Extension,
    {
        match block {
            Block::CodeBlock(code_block) => {
                let claimed = if code_block.kind.0.is_some() {
                    " claimed"
                } else {
                    ""
                };
                let label = format!(
                    "CodeBlock info={:?} value={:?}{claimed}",
                    code_block.info, code_block.value
                );
                self.push(depth, label, &code_block.span);
            }
            Block::List(list) => {
                let start = list
                    .start
                    .map_or_else(String::new, |start| format!(" start={start}"));
                let loose = if list.loose { " loose" } else { "" };
                self.push(depth, format!("List{start}{loose}"), &list.span);
                for item in &list.items {
                    self.push(depth + 1, "Item".to_owned(), &item.span);
                    self.markdown_block(depth + 2, &item.body);
                }
            }
            Block::Paragraph(paragraph) => {
                self.push(depth, "Paragraph".to_owned(), &paragraph.span);
                for line in &paragraph.lines {
                    self.push(depth + 1, "Line".to_owned(), &line.span);
                    self.inlines(depth + 2, &line.children);
                }
            }
            Block::Table(table) => {
                let alignments: Vec<String> = table
                    .alignments
                    .iter()
                    .map(|alignment| format!("{alignment:?}"))
                    .collect();
                let label = format!(
                    "Table format={:?} alignments={}",
                    table.format,
                    alignments.join(",")
                );
                self.push(depth, label, &table.span);
                let head = table.head.iter().map(|row| ("TableRow head", row));
                let rows = table.rows.iter().map(|row| ("TableRow", row));
                for (name, row) in head.chain(rows) {
                    self.push(depth + 1, name.to_owned(), &row.span);
                    for cell in &row.cells {
                        self.push(depth + 2, "TableCell".to_owned(), &cell.span);
                        self.markdown_block(depth + 3, &cell.body);
                    }
                }
            }
        }
    }

    fn document<E>(&mut self, document: &Document<E>)
    where
        E: Extension,
    {
        self.push(0, "Document".to_owned(), &document.span);
        if let Some(frontmatter) = &document.frontmatter {
            let label = format!("Frontmatter format={:?}", frontmatter.format);
            self.push(1, label, &frontmatter.span);
            self.mapping(2, &frontmatter.mapping);
        }
        self.markdown_block(1, &document.body);
    }

    fn inlines<E>(&mut self, depth: usize, inlines: &[Inline<E>])
    where
        E: Extension,
    {
        for inline in inlines {
            match inline {
                Inline::Code(code) => {
                    self.push(depth, format!("Code {:?}", code.value), &code.span);
                }
                Inline::Image(image) => {
                    let alt = if image.alt.is_empty() {
                        String::new()
                    } else {
                        format!(" alt={:?}", image.alt)
                    };
                    let label =
                        format!("Image form={:?} target={:?}{alt}", image.form, image.target);
                    self.push(depth, label, &image.span);
                }
                Inline::Link(link) => {
                    let label = format!("Link form={:?} target={:?}", link.form, link.target);
                    self.push(depth, label, &link.span);
                    self.inlines(depth + 1, &link.children);
                }
                Inline::Text(text) => {
                    self.push(depth, format!("Text {:?}", text.value), &text.span);
                }
            }
        }
    }

    fn mapping(&mut self, depth: usize, mapping: &Mapping) {
        self.push(depth, "Mapping".to_owned(), &mapping.span);
        for (key, pair) in &mapping.pairs {
            let merged = !pair.repeats.is_empty();
            let label = if merged {
                format!("Pair key={key} repeats={}", pair.repeats.len())
            } else {
                format!("Pair key={key}")
            };
            self.rows.push(Row {
                depth: depth + 1,
                label,
                merged,
                span: pair.span.clone(),
            });
            self.value(depth + 2, &pair.value);
        }
    }

    fn markdown_block<E>(&mut self, depth: usize, block: &MarkdownBlock<E>)
    where
        E: Extension,
    {
        self.push(depth, "MarkdownBlock".to_owned(), &block.span);
        for section in &block.sections {
            self.section(depth + 1, section);
        }
    }

    fn push(&mut self, depth: usize, label: String, span: &Range<usize>) {
        self.rows.push(Row {
            depth,
            label,
            merged: false,
            span: span.clone(),
        });
    }

    fn section<E>(&mut self, depth: usize, section: &Section<E>)
    where
        E: Extension,
    {
        self.push(depth, "Section".to_owned(), &section.span);
        self.push(depth + 1, "Heading".to_owned(), &section.heading.span);
        self.inlines(depth + 2, &section.heading.children);
        for block in &section.blocks {
            self.block(depth + 1, block);
        }
        for subsection in &section.subsections {
            self.section(depth + 1, subsection);
        }
    }

    fn value(&mut self, depth: usize, value: &Value) {
        match value {
            Value::Mapping(mapping) => self.mapping(depth, mapping),
            Value::Scalar(scalar) => {
                let label = format!("Scalar {}", scalar_label(&scalar.value));
                self.push(depth, label, &scalar.span);
            }
            Value::Sequence(sequence) => {
                self.push(depth, "Sequence".to_owned(), &sequence.span);
                for item in &sequence.items {
                    self.value(depth + 1, item);
                }
            }
        }
    }
}

/// Parses `source` and checks the tree against `expected`, an outline.
fn check(source: &str, expected: &str) {
    let found = rows(source);
    assert_nested(source, &found);
    let actual = outline(&found);
    assert!(
        actual == expected,
        "the tree differs.\n--- expected\n{expected}\n--- actual\n{actual}"
    );
}

/// Parses `source` and checks, node by node, the source text each span
/// covers.
fn check_spans(source: &str, expected: &[(&str, &str)]) {
    let found = rows(source);
    let actual: Vec<(&str, &str)> = found
        .iter()
        .map(|row| (row.label.as_str(), &source[row.span.clone()]))
        .collect();
    assert_eq!(actual, expected);
}

/// One line per node, indented two spaces under its parent.
fn outline(rows: &[Row]) -> String {
    let mut text = String::new();
    for row in rows {
        text.push_str(&"  ".repeat(row.depth));
        text.push_str(&row.label);
        text.push('\n');
    }
    text
}

fn rows(source: &str) -> Vec<Row> {
    let document = parse(source).unwrap();
    assert_eq!(document.span, 0..source.len());
    let mut rows = Rows::default();
    rows.document(&document);
    rows.rows
}

/// Every span lies inside its parent's span and after its previous sibling.
/// Under a pair whose key is written more than once, the values come from
/// several places, so each only lies inside the frontmatter.
fn assert_nested(source: &str, rows: &[Row]) {
    let mut open: Vec<Range<usize>> = Vec::new();
    let mut merged: Option<(usize, Range<usize>)> = None;
    for row in rows {
        let span = &row.span;
        let label = &row.label;
        assert!(
            span.start <= span.end
                && source.is_char_boundary(span.start)
                && source.is_char_boundary(span.end),
            "{label} has a bad span"
        );
        if let Some((depth, frontmatter)) = &merged {
            if row.depth > *depth {
                assert!(
                    frontmatter.start <= span.start && span.end <= frontmatter.end,
                    "{label} is outside the frontmatter"
                );
                continue;
            }
            merged = None;
        }
        open.truncate(row.depth + 1);
        if let Some(parent) = row.depth.checked_sub(1).and_then(|depth| open.get(depth)) {
            assert!(
                parent.start <= span.start && span.end <= parent.end,
                "{label} is outside its parent"
            );
        }
        if let Some(sibling) = open.get(row.depth) {
            assert!(
                sibling.end <= span.start,
                "{label} overlaps the node before it"
            );
        }
        open.truncate(row.depth);
        open.push(span.clone());
        if row.merged {
            merged = open
                .get(1)
                .map(|frontmatter| (row.depth, frontmatter.clone()));
        }
    }
}

fn scalar_label(value: &ScalarValue) -> String {
    match value {
        ScalarValue::Boolean(boolean) => format!("Boolean {boolean}"),
        ScalarValue::Date(date) => format!("Date {}", date_label(*date)),
        ScalarValue::DateTime(date_time) => format!("DateTime {}", date_time_label(*date_time)),
        ScalarValue::Float(float) => format!("Float {float}"),
        ScalarValue::Integer(integer) => format!("Integer {integer}"),
        ScalarValue::Null => "Null".to_owned(),
        ScalarValue::String(string) => format!("String {string:?}"),
        ScalarValue::Time(time) => format!("Time {}", time_label(*time)),
    }
}

fn date_label(date: Date) -> String {
    format!("{:04}-{:02}-{:02}", date.year, date.month, date.day)
}

fn time_label(time: Time) -> String {
    let clock = format!("{:02}:{:02}:{:02}", time.hour, time.minute, time.second);
    if time.nanosecond == 0 {
        clock
    } else {
        format!("{clock}.{:09}", time.nanosecond)
    }
}

fn date_time_label(date_time: DateTime) -> String {
    let offset = match date_time.offset {
        None => String::new(),
        Some(0) => "Z".to_owned(),
        Some(minutes) => {
            let sign = if minutes < 0 { '-' } else { '+' };
            let absolute = minutes.unsigned_abs();
            format!("{sign}{:02}:{:02}", absolute / 60, absolute % 60)
        }
    };
    format!(
        "{}T{}{offset}",
        date_label(date_time.date),
        time_label(date_time.time)
    )
}
