//! A document tree written back to markdown, in archie's own form: one
//! written form for each construct, whatever form the file used.

mod inline;
mod table;
mod toml;
mod yaml;

use core::fmt::{self, Write as _};

use chrono::{DateTime, FixedOffset, NaiveDateTime};

use crate::parser::ast::{
    Block, BlockQuote, CodeBlock, Document, FrontmatterFormat, Heading, Kind, Line, List,
    MarkdownBlock, Paragraph, Section,
};
use crate::parser::extension::{Construct, Extension};
use crate::writer::inline::{Near, Pieces, Segments};

/// The deepest heading level markdown has. A section deeper than this is
/// written inside its parent as a `markdown` block.
const DEEPEST: usize = 6;

/// The marker of a list's items: `-` for a bullet list and `.` after an
/// ordered list's numbers, or the other one, `*` or `)`, for a list right
/// after a list of its kind written with `-` or `.`.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Marker {
    First,
    Other,
}

/// Why a document could not be written: the frontmatter key whose value the
/// chosen syntax cannot hold.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct WriteError {
    pub key: String,
    pub message: String,
}

/// How a document is written.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct WriteOptions {
    /// The syntax the frontmatter is written in.
    pub frontmatter: FrontmatterFormat,
}

/// Builds the markdown text. Extensions write their nodes through it.
pub struct Writer {
    /// Whether a blank line is due before the next text written.
    blank: bool,
    /// The depth of the section being written, from 1.
    depth: usize,
    /// Whether the text written starts the file, with no frontmatter before
    /// it.
    file_start: bool,
    /// The marker the list written next takes.
    marker: Marker,
    out: String,
    /// Whether blocks follow each other with no blank line, as in the items
    /// of a tight list.
    tight: bool,
}

impl Writer {
    /// Whether the next text written is the first of the file.
    const fn at_file_start(&self) -> bool {
        self.file_start && self.out.is_empty()
    }

    fn block<T>(&mut self, block: &Block<T>)
    where
        T: Extension,
    {
        match block {
            Block::BlockQuote(quote) => self.node(&quote.kind, Construct::BlockQuote(quote)),
            Block::CodeBlock(code) => self.node(&code.kind, Construct::CodeBlock(code)),
            Block::Html(html) => self.node(&html.kind, Construct::Html(html)),
            Block::List(list) => self.node(&list.kind, Construct::List(list)),
            Block::Paragraph(paragraph) => {
                self.node(&paragraph.kind, Construct::Paragraph(paragraph));
            }
            Block::Table(table) => self.node(&table.kind, Construct::Table(table)),
            Block::ThematicBreak(thematic_break) => {
                self.node(
                    &thematic_break.kind,
                    Construct::ThematicBreak(thematic_break),
                );
            }
        }
    }

    /// `> ` before each line of the quoted text, `>` alone on an empty one.
    fn block_quote<T>(&mut self, quote: &BlockQuote<T>)
    where
        T: Extension,
    {
        for line in quote.value.split('\n') {
            if line.is_empty() {
                self.put(">\n");
            } else {
                self.put("> ");
                self.put(line);
                self.put("\n");
            }
        }
    }

    /// What `T` writes for `node`, which it claimed in place of `plain`.
    fn claimed<T>(&self, node: &T::Node<T>, plain: Construct<'_, T>) -> String
    where
        T: Extension,
    {
        let mut writer = Self {
            depth: self.depth,
            file_start: self.at_file_start(),
            tight: self.tight,
            ..Self::new()
        };
        T::write(node, plain, &mut writer);
        writer.out
    }

    /// Fenced with backticks, also when it was indented.
    fn code_block<T>(&mut self, code: &CodeBlock<T>)
    where
        T: Extension,
    {
        self.fenced(&code.info, |inner| {
            if !code.value.is_empty() {
                inner.put(&code.value);
                inner.put("\n");
            }
        });
    }

    /// Writes `construct` as archie writes it when no extension claims it.
    /// Its children are written as usual, each by the extension that
    /// claimed it, if one did.
    pub fn construct<T>(&mut self, construct: Construct<'_, T>)
    where
        T: Extension,
    {
        construct.write_plain(self);
    }

    /// Ends the line written last, if it is open.
    fn end_line(&mut self) {
        if !self.out.is_empty() && !self.out.ends_with('\n') {
            self.out.push('\n');
        }
    }

    /// Writes a fenced block: `info` after a fence of backticks one longer
    /// than the longest run of backticks in the content, and at least three,
    /// then the lines `content` writes, then the closing fence. The content
    /// is written with its levels counted from one.
    pub fn fenced<F>(&mut self, info: &str, content: F)
    where
        F: FnOnce(&mut Self),
    {
        let mut inner = Self::new();
        content(&mut inner);
        let text = inner.finish();
        let fence = "`".repeat(longest_run(&text).max(2) + 1);
        self.end_line();
        self.put(&fence);
        self.put(info);
        self.put("\n");
        self.put(&text);
        self.put(&fence);
        self.put("\n");
    }

    /// The text written, ending with one line ending unless it is empty.
    fn finish(mut self) -> String {
        self.end_line();
        self.out
    }

    /// `#` marks, one per level of depth, then the heading's text, a line
    /// break in it written `&#10;`. At depth 1 or 2, a heading that holds a
    /// line break is written over lines instead, each but the last ended by
    /// the break's `\`, then `===` or `---` on a line of its own.
    fn heading<T>(&mut self, heading: &Heading<T>)
    where
        T: Extension,
    {
        let underline = match self.depth {
            1 => "===",
            2 => "---",
            _ => "",
        };
        let mut segments = Segments::new(&Pieces::of(&heading.children));
        if !underline.is_empty() && inline::holds_line_break(&heading.children) {
            segments.cut_at_line_breaks();
            segments.nodes(self, &heading.children, false);
            for line in segments.lines() {
                self.put(&line);
                self.put("\n");
            }
            self.put(underline);
        } else {
            self.put(&"#".repeat(self.depth));
            if !heading.children.is_empty() {
                segments.line_breaks_as_references();
                segments.nodes(self, &heading.children, false);
                self.put(" ");
                self.put(&segments.take(Near::Edge, Near::Edge));
            }
        }
        self.put("\n");
    }

    /// Writes inline nodes on their own, with nothing known around them.
    fn inline<F>(&mut self, build: F)
    where
        F: FnOnce(&mut Segments, &Self),
    {
        let mut segments = Segments::new(&Pieces::default());
        build(&mut segments, self);
        let text = segments.take(Near::Unknown, Near::Unknown);
        self.put(&text);
    }

    /// The text of `line`, and whether it starts with text. `first` is set
    /// for the first line of a paragraph.
    fn line<T>(&self, line: &Line<T>, segments: &mut Segments, first: bool) -> (String, bool)
    where
        T: Extension,
    {
        line.kind.0.as_deref().map_or_else(
            || {
                segments.nodes(self, &line.children, false);
                segments.line(first)
            },
            |node| {
                let text = self.claimed(node, Construct::Line(line));
                (text.trim_end_matches('\n').to_owned(), false)
            },
        )
    }

    /// `-` and one space before each item of a bullet list, the number and
    /// `. ` before each of an ordered list, numbered up from its first
    /// number, or `*` and `)` in their place for the other marker. Each
    /// item's lines after the first are indented to its text.
    fn list<T>(&mut self, list: &List<T>)
    where
        T: Extension,
    {
        let (bullet, delimiter) = match self.marker {
            Marker::First => ("-", '.'),
            Marker::Other => ("*", ')'),
        };
        for (index, item) in list.items.iter().enumerate() {
            if index > 0 {
                self.separate_items(list.loose);
            }
            let marker = list.start.map_or_else(
                || bullet.to_owned(),
                |start| {
                    let number =
                        u64::try_from(index).map_or(start, |step| start.saturating_add(step));
                    format!("{number}{delimiter}")
                },
            );
            let mut inner = Self {
                tight: !list.loose,
                ..Self::new()
            };
            inner.node(&item.kind, Construct::Item(item));
            self.marked(&marker, &inner.finish());
        }
    }

    /// Writes `block` with its levels counted from one.
    pub fn markdown_block<T>(&mut self, block: &MarkdownBlock<T>)
    where
        T: Extension,
    {
        let mut inner = Self {
            file_start: self.at_file_start(),
            ..Self::new()
        };
        inner.markdown_node(block);
        self.end_line();
        self.put(&inner.finish());
    }

    fn markdown_node<T>(&mut self, block: &MarkdownBlock<T>)
    where
        T: Extension,
    {
        self.node(&block.kind, Construct::MarkdownBlock(block));
    }

    /// Writes `content`, the lines of an item, after `marker`, every line
    /// after the first indented to the item's text. A first line that starts
    /// with a space goes under the marker, so the item's text column stays
    /// where it is.
    fn marked(&mut self, marker: &str, content: &str) {
        let indent = " ".repeat(marker.len() + 1);
        let mut lines = content.strip_suffix('\n').unwrap_or(content).split('\n');
        self.put(marker);
        if let Some(first) = lines.next().filter(|first| !first.is_empty()) {
            if first.starts_with(' ') {
                self.put("\n");
                self.put(&indent);
            } else {
                self.put(" ");
            }
            self.put(first);
        }
        self.put("\n");
        for line in lines {
            if !line.is_empty() {
                self.put(&indent);
            }
            self.put(line);
            self.put("\n");
        }
    }

    const fn new() -> Self {
        Self {
            blank: false,
            depth: 1,
            file_start: false,
            marker: Marker::First,
            out: String::new(),
            tight: false,
        }
    }

    /// Writes a node that stands on its own lines: what the extension that
    /// claimed it writes, or else its plain form.
    fn node<T>(&mut self, kind: &Kind<T>, plain: Construct<'_, T>)
    where
        T: Extension,
    {
        self.separate();
        match kind.0.as_deref() {
            Some(node) => {
                let text = self.claimed(node, plain);
                self.put(&text);
            }
            None => self.construct(plain),
        }
        self.end_line();
    }

    /// Each line of the paragraph on a line of its own, written again with
    /// a backslash before every `|` of its text when its lines would read as
    /// a table.
    fn paragraph<T>(&mut self, paragraph: &Paragraph<T>)
    where
        T: Extension,
    {
        let mut pieces = Pieces::default();
        for line in paragraph.lines.iter().filter(|line| line.kind.0.is_none()) {
            pieces.add(&line.children);
        }
        let mut lines = self.paragraph_lines(paragraph, &pieces, false);
        if inline::reads_as_table(&lines) {
            lines = self.paragraph_lines(paragraph, &pieces, true);
        }
        for line in &lines {
            self.put(line);
            self.put("\n");
        }
    }

    /// The lines of `paragraph`, with a backslash before a first line that
    /// would open a TOML frontmatter, and before every `|` of the text when
    /// `pipes` is set.
    fn paragraph_lines<T>(
        &self,
        paragraph: &Paragraph<T>,
        pieces: &Pieces,
        pipes: bool,
    ) -> Vec<String>
    where
        T: Extension,
    {
        let mut segments = Segments::new(pieces);
        if pipes {
            segments.escape_pipes();
        }
        let mut lines = Vec::with_capacity(paragraph.lines.len());
        let mut text_led = false;
        for (index, line) in paragraph.lines.iter().enumerate() {
            let (text, led) = self.line(line, &mut segments, index == 0);
            if index == 0 {
                text_led = led;
            }
            lines.push(text);
        }
        if text_led {
            inline::guard_definition(&mut lines);
            if self.at_file_start()
                && let Some(first) = lines.first_mut().filter(|first| *first == "+++")
            {
                first.insert(0, '\\');
            }
        }
        lines
    }

    /// The heading, written when the section has one, then the blocks, then
    /// the subsections one level deeper. Past the deepest level, each
    /// subsection is a `markdown` block. A plain list right after a plain
    /// list of its kind takes the other marker than that list, so the two
    /// read back as two lists.
    fn plain_section<T>(&mut self, section: &Section<T>)
    where
        T: Extension,
    {
        if let Some(heading) = &section.heading {
            self.node(&heading.kind, Construct::Heading(heading));
        }
        let mut previous: Option<&List<T>> = None;
        for block in &section.blocks {
            let list = plain_list(block);
            let same_kind = previous
                .zip(list)
                .is_some_and(|(before, after)| before.start.is_some() == after.start.is_some());
            self.marker = if same_kind && self.marker == Marker::First {
                Marker::Other
            } else {
                Marker::First
            };
            self.block(block);
            previous = list;
        }
        let depth = self.depth;
        for subsection in &section.subsections {
            if depth < DEEPEST {
                self.depth = depth + 1;
                self.section(subsection);
                self.depth = depth;
            } else {
                self.separate();
                self.fenced("markdown", |inner| inner.section(subsection));
            }
        }
    }

    /// Adds `text` to what is written, as it is.
    pub fn push_str(&mut self, text: &str) {
        self.put(text);
    }

    /// Adds `text`, after the blank line that is due, if any.
    fn put(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }
        if self.blank {
            self.out.push('\n');
            self.blank = false;
        }
        self.out.push_str(text);
    }

    fn section<T>(&mut self, section: &Section<T>)
    where
        T: Extension,
    {
        self.node(&section.kind, Construct::Section(section));
    }

    /// Ends the line written last, and makes a blank line due before the
    /// next text, outside a tight list.
    fn separate(&mut self) {
        self.end_line();
        if !self.tight && !self.out.is_empty() && !self.out.ends_with("\n\n") {
            self.blank = true;
        }
    }

    /// Makes a blank line due between two items of a loose list.
    const fn separate_items(&mut self, loose: bool) {
        if loose {
            self.blank = true;
        }
    }
}

impl<T> Construct<'_, T>
where
    T: Extension,
{
    /// Writes the construct as archie writes it when no extension claims
    /// it, with `writer`.
    fn write_plain(self, writer: &mut Writer) {
        match self {
            Self::BlockQuote(quote) => writer.block_quote(quote),
            Self::Code(code) => writer.inline(|segments, _| segments.code(code)),
            Self::CodeBlock(code) => writer.code_block(code),
            Self::Document(document) => writer.markdown_node(&document.body),
            Self::Emphasis(emphasis) => {
                writer.inline(|segments, parent| segments.emphasis(parent, emphasis, false));
            }
            Self::Heading(heading) => writer.heading(heading),
            Self::Html(html) => writer.put(&html.value),
            Self::Image(image) => writer.inline(|segments, _| segments.image(image)),
            Self::Item(item) => writer.markdown_node(&item.body),
            Self::Line(line) => {
                let mut segments = Segments::new(&Pieces::of(&line.children));
                let (text, _) = writer.line(line, &mut segments, false);
                writer.put(&text);
            }
            Self::LineBreak(_) => writer.put("\\"),
            Self::Link(link) => writer.inline(|segments, parent| segments.link(parent, link)),
            Self::List(list) => writer.list(list),
            Self::MarkdownBlock(block) => {
                let mut inner = Writer {
                    file_start: writer.at_file_start(),
                    tight: writer.tight,
                    ..Writer::new()
                };
                for section in &block.sections {
                    inner.section(section);
                }
                writer.put(&inner.finish());
            }
            Self::Paragraph(paragraph) => writer.paragraph(paragraph),
            Self::Section(section) => writer.plain_section(section),
            Self::Strong(strong) => {
                writer.inline(|segments, parent| segments.strong(parent, strong, false));
            }
            Self::Table(table) => table::write(writer, table),
            Self::TableCell(cell) => writer.markdown_node(&cell.body),
            Self::TableRow(row) => writer.put(&table::pipe_row(row)),
            Self::Text(text) => writer.inline(|segments, _| segments.text(&text.value, false)),
            Self::ThematicBreak(_) => writer.put("***\n"),
        }
    }
}

/// Writes `document` in archie's form: its frontmatter, if it has one, in
/// the syntax `options` picks, then its body.
///
/// # Errors
///
/// Returns an error naming the key when TOML is picked and the frontmatter
/// holds a null, which TOML cannot hold.
pub fn write<E>(document: &Document<E>, options: &WriteOptions) -> Result<String, WriteError>
where
    E: Extension,
{
    let frontmatter = match (&document.frontmatter, options.frontmatter) {
        (None, _) => String::new(),
        (Some(frontmatter), FrontmatterFormat::Toml) => toml::frontmatter(&frontmatter.mapping)?,
        (Some(frontmatter), FrontmatterFormat::Yaml) => yaml::frontmatter(&frontmatter.mapping),
    };
    Ok(with_body(frontmatter, document))
}

/// Writes `document` in archie's form, its frontmatter in the syntax it was
/// read in, or in YAML when that syntax is TOML and the frontmatter holds a
/// null, which TOML cannot hold.
pub(crate) fn write_for_rebuild<E>(document: &Document<E>) -> String
where
    E: Extension,
{
    let frontmatter = document
        .frontmatter
        .as_ref()
        .map_or_else(String::new, |frontmatter| match frontmatter.format {
            FrontmatterFormat::Toml => toml::frontmatter(&frontmatter.mapping)
                .unwrap_or_else(|_| yaml::frontmatter(&frontmatter.mapping)),
            FrontmatterFormat::Yaml => yaml::frontmatter(&frontmatter.mapping),
        });
    with_body(frontmatter, document)
}

/// `frontmatter`, the written frontmatter of `document`, then its body.
fn with_body<E>(mut frontmatter: String, document: &Document<E>) -> String
where
    E: Extension,
{
    let mut writer = Writer {
        file_start: document.frontmatter.is_none(),
        ..Writer::new()
    };
    writer.node(&document.kind, Construct::Document(document));
    frontmatter.push_str(&writer.finish());
    frontmatter
}

/// A date and time with no offset, such as `2026-10-10T07:32:00`, with the
/// fraction of a second when it has one.
fn push_local_date_time(out: &mut String, date_time: &NaiveDateTime) {
    let _: fmt::Result = write!(out, "{}T{}", date_time.date(), date_time.time());
}

/// A date and time with its offset: `Z` for UTC, such as `+02:00` otherwise.
fn push_date_time(out: &mut String, date_time: &DateTime<FixedOffset>) {
    push_local_date_time(out, &date_time.naive_local());
    let offset = date_time.offset();
    if offset.local_minus_utc() == 0_i32 {
        out.push('Z');
    } else {
        let _: fmt::Result = write!(out, "{offset}");
    }
}

/// A finite float as the shortest text that reads back as it, with `.0`
/// when that text would read as an integer.
fn push_finite(out: &mut String, float: f64) {
    let start = out.len();
    let _: fmt::Result = write!(out, "{float}");
    if !out[start..].contains('.') {
        out.push_str(".0");
    }
}

/// `block` when it is a list no extension claimed.
const fn plain_list<T>(block: &Block<T>) -> Option<&List<T>>
where
    T: Extension,
{
    match block {
        Block::List(list) if list.kind.0.is_none() => Some(list),
        Block::BlockQuote(_)
        | Block::CodeBlock(_)
        | Block::Html(_)
        | Block::List(_)
        | Block::Paragraph(_)
        | Block::Table(_)
        | Block::ThematicBreak(_) => None,
    }
}

/// The length of the longest run of backticks in `text`.
fn longest_run(text: &str) -> usize {
    text.split(|character| character != '`')
        .map(str::len)
        .max()
        .unwrap_or_default()
}

#[cfg(test)]
mod tests;
