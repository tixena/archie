//! The document tree a markdown file parses into.
//!
//! Every node carries `span`, the byte range it covers in the source file,
//! and `kind`, the type an extension gave it. The frontmatter nodes hold
//! values, whichever syntax they were written in.

use core::ops::Range;

use indexmap::IndexMap;

use crate::parser::extension::{Extension, Markdown};

/// The type an extension gave a node, or `None`. Boxed, because a markdown
/// block can hold a markdown block.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Kind<E>(pub Option<Box<E::Node<E>>>)
where
    E: Extension;

/// A whole markdown file.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Document<E = Markdown>
where
    E: Extension,
{
    pub body: MarkdownBlock<E>,
    pub frontmatter: Option<Frontmatter>,
    pub kind: Kind<E>,
    pub span: Range<usize>,
}

/// The metadata block at the top of a file: TOML between `+++` lines or
/// YAML between `---` lines.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Frontmatter {
    pub format: FrontmatterFormat,
    pub mapping: Mapping,
    pub span: Range<usize>,
}

/// The syntax a frontmatter block is written in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum FrontmatterFormat {
    Toml,
    Yaml,
}

/// Keys and their values, in the order they were written. A key appears once.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Mapping {
    pub pairs: IndexMap<String, Pair>,
    pub span: Range<usize>,
}

/// The value under one key of a mapping. Its span runs from the key to the
/// end of the value.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Pair {
    pub span: Range<usize>,
    pub value: Value,
}

/// What a key maps to, or what a sequence holds.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Value {
    Mapping(Mapping),
    Scalar(Scalar),
    Sequence(Sequence),
}

impl Value {
    /// The byte range the value covers.
    #[must_use]
    pub const fn span(&self) -> &Range<usize> {
        match self {
            Self::Mapping(mapping) => &mapping.span,
            Self::Scalar(scalar) => &scalar.span,
            Self::Sequence(sequence) => &sequence.span,
        }
    }
}

/// An ordered list of values, such as `["parser", "design"]`.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Sequence {
    pub items: Vec<Value>,
    pub span: Range<usize>,
}

/// A single value, such as `2` or `"task"`.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Scalar {
    pub span: Range<usize>,
    pub value: ScalarValue,
}

/// The typed content of a scalar.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum ScalarValue {
    Boolean(bool),
    Date(Date),
    DateTime(DateTime),
    Float(f64),
    Integer(i64),
    /// YAML only: TOML has no null.
    Null,
    String(String),
    Time(Time),
}

/// A calendar date, such as `2026-10-10`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Date {
    pub day: u8,
    pub month: u8,
    pub year: u16,
}

/// A time of day, such as `07:32:00`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Time {
    pub hour: u8,
    pub minute: u8,
    pub nanosecond: u32,
    pub second: u8,
}

/// A date with a time, such as `2026-10-10T07:32:00-04:00`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct DateTime {
    pub date: Date,
    /// Minutes east of UTC; `None` when the value names no offset.
    pub offset: Option<i16>,
    pub time: Time,
}

/// A stretch of markdown read by the same rules: the body, a list item's
/// content, a table cell's content, a markdown code block's content.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct MarkdownBlock<E = Markdown>
where
    E: Extension,
{
    pub kind: Kind<E>,
    pub sections: Vec<Section<E>>,
    pub span: Range<usize>,
}

/// A node that stands on its own lines.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Block<E = Markdown>
where
    E: Extension,
{
    CodeBlock(CodeBlock<E>),
    List(List<E>),
    Paragraph(Paragraph<E>),
    Table(Table<E>),
}

impl<E> Block<E>
where
    E: Extension,
{
    /// The byte range the block covers.
    #[must_use]
    pub const fn span(&self) -> &Range<usize> {
        match self {
            Self::CodeBlock(code_block) => &code_block.span,
            Self::List(list) => &list.span,
            Self::Paragraph(paragraph) => &paragraph.span,
            Self::Table(table) => &table.span,
        }
    }
}

/// A heading, the blocks under it, then its subsections. Its level is its
/// depth in the `MarkdownBlock`. An empty heading has no children.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Section<E = Markdown>
where
    E: Extension,
{
    pub blocks: Vec<Block<E>>,
    pub heading: Heading<E>,
    pub kind: Kind<E>,
    pub span: Range<usize>,
    pub subsections: Vec<Self>,
}

/// A heading line, such as `# Ship parser`. An empty heading has no children
/// and an empty span at the start of its section.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Heading<E = Markdown>
where
    E: Extension,
{
    pub children: Vec<Inline<E>>,
    pub kind: Kind<E>,
    pub span: Range<usize>,
}

/// A set of lines, ended by a blank line.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Paragraph<E = Markdown>
where
    E: Extension,
{
    pub kind: Kind<E>,
    pub lines: Vec<Line<E>>,
    pub span: Range<usize>,
}

/// One line of a paragraph and the inline nodes written on it.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Line<E = Markdown>
where
    E: Extension,
{
    pub children: Vec<Inline<E>>,
    pub kind: Kind<E>,
    pub span: Range<usize>,
}

/// A bulleted list.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct List<E = Markdown>
where
    E: Extension,
{
    pub items: Vec<Item<E>>,
    pub kind: Kind<E>,
    pub span: Range<usize>,
}

/// One entry of a list.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Item<E = Markdown>
where
    E: Extension,
{
    pub body: MarkdownBlock<E>,
    pub kind: Kind<E>,
    pub span: Range<usize>,
}

/// Lines between two fences of backticks. `content` is the range of those
/// lines in the source, `info` the text after the opening fence, and `value`
/// the lines as the block holds them.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct CodeBlock<E = Markdown>
where
    E: Extension,
{
    pub content: Range<usize>,
    pub info: String,
    pub kind: Kind<E>,
    pub span: Range<usize>,
    pub value: String,
}

/// A table read from pipe, grid or simple table syntax. `head` holds the
/// header rows, `rows` the rest.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Table<E = Markdown>
where
    E: Extension,
{
    pub alignments: Vec<Alignment>,
    pub format: TableFormat,
    pub head: Vec<TableRow<E>>,
    pub kind: Kind<E>,
    pub rows: Vec<TableRow<E>>,
    pub span: Range<usize>,
}

/// One row of a table.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct TableRow<E = Markdown>
where
    E: Extension,
{
    pub cells: Vec<TableCell<E>>,
    pub kind: Kind<E>,
    pub span: Range<usize>,
}

/// One cell of a table row.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct TableCell<E = Markdown>
where
    E: Extension,
{
    pub body: MarkdownBlock<E>,
    pub kind: Kind<E>,
    pub span: Range<usize>,
}

/// How a column's cells are aligned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Alignment {
    Center,
    Default,
    Left,
    Right,
}

/// The syntax a table is written in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum TableFormat {
    /// `+---+` borders and `|` walls.
    Grid,
    /// `| a | b |` rows under a `|---|---|` delimiter row.
    Pipe,
    /// Columns marked by groups of dashes under a header line.
    Simple,
}

/// A node inside a line of text.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Inline<E = Markdown>
where
    E: Extension,
{
    Code(Code<E>),
    Image(Image<E>),
    Link(Link<E>),
    Text(Text<E>),
}

impl<E> Inline<E>
where
    E: Extension,
{
    /// The byte range the node covers.
    #[must_use]
    pub const fn span(&self) -> &Range<usize> {
        match self {
            Self::Code(code) => &code.span,
            Self::Image(image) => &image.span,
            Self::Link(link) => &link.span,
            Self::Text(text) => &text.span,
        }
    }
}

/// Plain text.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Text<E = Markdown>
where
    E: Extension,
{
    pub kind: Kind<E>,
    pub span: Range<usize>,
    pub value: String,
}

/// A link: `[text](target)`, `[[target]]` or `[[target|text]]`. `children`
/// are the text shown, and are empty for `[[target]]`.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Link<E = Markdown>
where
    E: Extension,
{
    pub children: Vec<Inline<E>>,
    pub form: LinkForm,
    pub kind: Kind<E>,
    pub span: Range<usize>,
    pub target: String,
}

/// How a link or an image is written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum LinkForm {
    /// `[text](target)`, or `![alt](target)` for an image.
    Markdown,
    /// `[[target|text]]`, or `![[target|alt]]` for an image.
    Wiki,
}

/// An embed: `![alt](target)`, `![[target]]` or `![[target|alt]]`. `alt` is
/// empty when none is written.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Image<E = Markdown>
where
    E: Extension,
{
    pub alt: String,
    pub form: LinkForm,
    pub kind: Kind<E>,
    pub span: Range<usize>,
    pub target: String,
}

/// Text between backticks, such as `` `archie check` ``.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Code<E = Markdown>
where
    E: Extension,
{
    pub kind: Kind<E>,
    pub span: Range<usize>,
    pub value: String,
}
