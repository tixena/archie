//! The document tree a markdown file parses into.
//!
//! Every node carries `span`, the byte range it covers in the source file.
//! The frontmatter nodes hold values, whichever syntax they were written in.

use core::convert::Infallible;
use core::fmt::Debug;
use core::ops::Range;

use indexmap::IndexMap;

use crate::parser::error::ParseError;

/// What a consumer of archie adds to the tree. The consumer owns each type
/// it declares here, and archie asks for it while parsing.
pub trait Extension {
    /// The subtype a claimed fenced code block carries.
    type CodeBlock: Debug + Clone + PartialEq;

    /// The subtype of `block`, or `None` to leave it a plain code block. An
    /// error fails the whole parse.
    fn code_block(block: &FencedCode<'_>) -> Option<Result<Self::CodeBlock, ParseError>>;
}

/// archie's own extension: it claims nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Plain;

impl Extension for Plain {
    type CodeBlock = Infallible;

    fn code_block(_block: &FencedCode<'_>) -> Option<Result<Infallible, ParseError>> {
        None
    }
}

/// A fenced code block as an extension sees it. `content` is the range of
/// the lines between the fences, `value` those lines as the block holds them.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct FencedCode<'text> {
    pub content: Range<usize>,
    pub info: &'text str,
    pub source: &'text str,
    pub value: &'text str,
}

/// A whole markdown file.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Document<E = Plain>
where
    E: Extension,
{
    pub body: MarkdownBlock<E>,
    pub frontmatter: Option<Frontmatter>,
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
/// end of the value, at the key's first occurrence. `repeats` holds the span
/// of each later occurrence of the same key, in file order.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Pair {
    pub repeats: Vec<Range<usize>>,
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
/// content, a table cell's content.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct MarkdownBlock<E = Plain>
where
    E: Extension,
{
    pub sections: Vec<Section<E>>,
    pub span: Range<usize>,
}

/// A node that stands on its own lines.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Block<E = Plain>
where
    E: Extension,
{
    CodeBlock(CodeBlock<E>),
    List(List<E>),
    Paragraph(Paragraph),
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
pub struct Section<E = Plain>
where
    E: Extension,
{
    pub blocks: Vec<Block<E>>,
    pub heading: Heading,
    pub span: Range<usize>,
    pub subsections: Vec<Self>,
}

/// A heading line, such as `# Ship parser`. An empty heading has no children
/// and an empty span at the start of its section.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Heading {
    pub children: Vec<Inline>,
    pub span: Range<usize>,
}

/// A set of lines, ended by a blank line.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Paragraph {
    pub lines: Vec<Line>,
    pub span: Range<usize>,
}

/// One line of a paragraph and the inline nodes written on it.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Line {
    pub children: Vec<Inline>,
    pub span: Range<usize>,
}

/// A bulleted list.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct List<E = Plain>
where
    E: Extension,
{
    pub items: Vec<Item<E>>,
    pub span: Range<usize>,
}

/// One entry of a list.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Item<E = Plain>
where
    E: Extension,
{
    pub body: MarkdownBlock<E>,
    pub span: Range<usize>,
}

/// Lines between two fences of backticks. `info` is the text after the
/// opening fence; `value` is the lines between the fences.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct CodeBlock<E = Plain>
where
    E: Extension,
{
    pub info: String,
    /// The subtype an extension gave the block; `None` when none claims it.
    pub kind: Option<E::CodeBlock>,
    pub span: Range<usize>,
    pub value: String,
}

/// A table read from pipe, grid or simple table syntax. `head` holds the
/// header rows, `rows` the rest.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Table<E = Plain>
where
    E: Extension,
{
    pub alignments: Vec<Alignment>,
    pub format: TableFormat,
    pub head: Vec<TableRow<E>>,
    pub rows: Vec<TableRow<E>>,
    pub span: Range<usize>,
}

/// One row of a table.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct TableRow<E = Plain>
where
    E: Extension,
{
    pub cells: Vec<TableCell<E>>,
    pub span: Range<usize>,
}

/// One cell of a table row.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct TableCell<E = Plain>
where
    E: Extension,
{
    pub body: MarkdownBlock<E>,
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
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Inline {
    Code(Code),
    Image(Image),
    Link(Link),
    Text(Text),
}

impl Inline {
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
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Text {
    pub span: Range<usize>,
    pub value: String,
}

/// A link: `[text](target)`, `[[target]]` or `[[target|text]]`. `children`
/// are the text shown, and are empty for `[[target]]`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Link {
    pub children: Vec<Inline>,
    pub form: LinkForm,
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
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Image {
    pub alt: String,
    pub form: LinkForm,
    pub span: Range<usize>,
    pub target: String,
}

/// Text between backticks, such as `` `archie check` ``.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Code {
    pub span: Range<usize>,
    pub value: String,
}
