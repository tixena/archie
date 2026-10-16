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

use chrono::{
    DateTime, Datelike as _, FixedOffset, NaiveDate, NaiveDateTime, NaiveTime, Timelike as _,
};

use crate::parser::ast::{
    Block, Document, Inline, Mapping, MarkdownBlock, ScalarValue, Section, Value,
};
use crate::parser::extension::Extension;
use crate::parser::parse;

/// One node of a parsed tree: how deep it sits, how it reads, what it covers.
/// `merged` marks a pair whose key is written more than once, and `spread` a
/// cell of a table row written over several lines.
struct Row {
    depth: usize,
    label: String,
    merged: bool,
    span: Range<usize>,
    spread: bool,
}

#[derive(Default)]
struct Rows<'src> {
    rows: Vec<Row>,
    /// The text the tree was parsed from, or empty for an outline alone.
    source: &'src str,
}

impl Rows<'_> {
    fn block<E>(&mut self, depth: usize, block: &Block<E>)
    where
        E: Extension,
    {
        match block {
            Block::BlockQuote(block_quote) => {
                let label = format!("BlockQuote {:?}", block_quote.value);
                self.push(depth, label, &block_quote.span);
            }
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
            Block::Html(html) => {
                self.push(depth, format!("Html {:?}", html.value), &html.span);
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
                    let spread = self
                        .source
                        .get(row.span.clone())
                        .is_some_and(|text| text.contains('\n'));
                    for cell in &row.cells {
                        self.rows.push(Row {
                            depth: depth + 2,
                            label: "TableCell".to_owned(),
                            merged: false,
                            span: cell.span.clone(),
                            spread,
                        });
                        self.markdown_block(depth + 3, &cell.body);
                    }
                }
            }
            Block::ThematicBreak(thematic_break) => {
                self.push(depth, "ThematicBreak".to_owned(), &thematic_break.span);
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
                Inline::Emphasis(emphasis) => {
                    self.push(depth, "Emphasis".to_owned(), &emphasis.span);
                    self.inlines(depth + 1, &emphasis.children);
                }
                Inline::Html(html) => {
                    self.push(depth, format!("Html {:?}", html.value), &html.span);
                }
                Inline::Image(image) => {
                    let alt = if image.alt.is_empty() {
                        String::new()
                    } else {
                        format!(" alt={:?}", image.alt)
                    };
                    let label = format!(
                        "Image form={:?} target={:?}{}{alt}",
                        image.form,
                        image.target,
                        title_label(image.title.as_deref())
                    );
                    self.push(depth, label, &image.span);
                }
                Inline::LineBreak(line_break) => {
                    self.push(depth, "LineBreak".to_owned(), &line_break.span);
                }
                Inline::Link(link) => {
                    let label = format!(
                        "Link form={:?} target={:?}{}",
                        link.form,
                        link.target,
                        title_label(link.title.as_deref())
                    );
                    self.push(depth, label, &link.span);
                    self.inlines(depth + 1, &link.children);
                }
                Inline::Strong(strong) => {
                    self.push(depth, "Strong".to_owned(), &strong.span);
                    self.inlines(depth + 1, &strong.children);
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
                spread: false,
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
            spread: false,
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
    let mut rows = Rows {
        rows: Vec::new(),
        source,
    };
    rows.document(&document);
    rows.rows
}

/// Every span lies inside its parent's span and after its previous sibling.
/// Under a pair whose key is written more than once, the values come from
/// several places, so each only lies inside the frontmatter. The cells of a
/// row written over several lines interleave, so each only lies inside its
/// row.
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
                row.spread || sibling.end <= span.start,
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

/// ` title="..."` for a link or an image with a title, else nothing.
fn title_label(title: Option<&str>) -> String {
    title.map_or_else(String::new, |written| format!(" title={written:?}"))
}

fn scalar_label(value: &ScalarValue) -> String {
    match value {
        ScalarValue::Boolean(boolean) => format!("Boolean {boolean}"),
        ScalarValue::Date(date) => format!("Date {}", date_label(*date)),
        ScalarValue::DateTime(date_time) => format!("DateTime {}", date_time_label(date_time)),
        ScalarValue::Float(float) => format!("Float {float}"),
        ScalarValue::Integer(integer) => format!("Integer {integer}"),
        ScalarValue::LocalDateTime(local) => format!("LocalDateTime {}", local_label(*local)),
        ScalarValue::Null => "Null".to_owned(),
        ScalarValue::String(string) => format!("String {string:?}"),
        ScalarValue::Time(time) => format!("Time {}", time_label(*time)),
    }
}

fn date_label(date: NaiveDate) -> String {
    format!("{:04}-{:02}-{:02}", date.year(), date.month(), date.day())
}

/// chrono holds a leap second as second 59 with a second or more of
/// nanoseconds; it prints as second 60.
fn time_label(time: NaiveTime) -> String {
    let leap = time.nanosecond() / 1_000_000_000;
    let nanosecond = time.nanosecond() % 1_000_000_000;
    let clock = format!(
        "{:02}:{:02}:{:02}",
        time.hour(),
        time.minute(),
        time.second() + leap
    );
    if nanosecond == 0 {
        clock
    } else {
        format!("{clock}.{nanosecond:09}")
    }
}

fn local_label(local: NaiveDateTime) -> String {
    format!("{}T{}", date_label(local.date()), time_label(local.time()))
}

fn date_time_label(date_time: &DateTime<FixedOffset>) -> String {
    let seconds = date_time.offset().local_minus_utc();
    let offset = if seconds == 0_i32 {
        "Z".to_owned()
    } else {
        let sign = if seconds < 0_i32 { '-' } else { '+' };
        let minutes = seconds.unsigned_abs() / 60;
        format!("{sign}{:02}:{:02}", minutes / 60, minutes % 60)
    };
    format!("{}{offset}", local_label(date_time.naive_local()))
}
