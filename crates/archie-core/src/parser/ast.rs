//! The document tree a markdown file parses into.
//!
//! Every node carries `span`, the byte range it covers in the source file,
//! and `kind`, the type an extension gave it. The frontmatter nodes hold
//! values, whichever syntax they were written in.

use core::ops::Range;

use chrono::{DateTime, FixedOffset, NaiveDate, NaiveDateTime, NaiveTime};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

use crate::parser::extension::{Extension, Markdown, markdown_schema};

/// The type an extension gave a node, or `None`. Boxed, because a markdown
/// block can hold a markdown block.
///
/// JSON leaves it out: the claim pass builds it again from the plain node
/// under it. Editing it leaves that plain node as it was, so a store writes
/// each claimed node back into its plain node before it writes the tree.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Kind<E>(pub Option<Box<E::Node<E>>>)
where
    E: Extension;

impl<E> Default for Kind<E>
where
    E: Extension,
{
    fn default() -> Self {
        Self(None)
    }
}

/// A whole markdown file.
#[model_schema(default_types(E = Markdown))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(bound = "")]
#[non_exhaustive]
pub struct Document<E = Markdown>
where
    E: Extension,
{
    pub body: MarkdownBlock<E>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frontmatter: Option<Frontmatter>,
    #[serde(skip)]
    pub kind: Kind<E>,
    pub span: Range<usize>,
}

/// The metadata block at the top of a file: TOML between `+++` lines or
/// YAML between `---` lines.
#[model_schema()]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Frontmatter {
    pub format: FrontmatterFormat,
    pub mapping: Mapping,
    pub span: Range<usize>,
}

/// The syntax a frontmatter block is written in.
#[model_schema()]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum FrontmatterFormat {
    Toml,
    #[default]
    Yaml,
}

/// Keys and their values, in the order they were written. A key appears once.
#[model_schema()]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Mapping {
    pub pairs: IndexMap<String, Pair>,
    pub span: Range<usize>,
}

/// The value under one key of a mapping. Its span runs from the key to the
/// end of the value, at the key's first occurrence. `repeats` holds the span
/// of each later occurrence of the same key, in file order.
#[model_schema()]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Pair {
    pub repeats: Vec<Range<usize>>,
    pub span: Range<usize>,
    pub value: Value,
}

/// What a key maps to, or what a sequence holds.
#[model_schema()]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
#[model_schema()]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Sequence {
    pub items: Vec<Value>,
    pub span: Range<usize>,
}

/// A single value, such as `2` or `"task"`.
#[model_schema()]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Scalar {
    pub span: Range<usize>,
    pub value: ScalarValue,
}

/// The typed content of a scalar.
#[model_schema()]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ScalarValue {
    Boolean(bool),
    Date(NaiveDate),
    /// A date and time with its offset from UTC.
    DateTime(DateTime<FixedOffset>),
    /// A finite float.
    Float(f64),
    Integer(i64),
    /// A date and time with no offset.
    LocalDateTime(NaiveDateTime),
    NonFinite(NonFinite),
    /// YAML only: TOML has no null.
    Null,
    String(String),
    Time(NaiveTime),
}

/// A float that is not a finite number: `.inf`, `-.inf` and `.nan` in
/// YAML; `inf`, `-inf` and `nan` in TOML.
#[model_schema()]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum NonFinite {
    Infinity,
    Nan,
    NegativeInfinity,
}

impl NonFinite {
    /// The non-finite value `float` holds, or `None` when it is finite.
    #[must_use]
    pub const fn of(float: f64) -> Option<Self> {
        if float.is_nan() {
            Some(Self::Nan)
        } else if float.is_finite() {
            None
        } else if float.is_sign_negative() {
            Some(Self::NegativeInfinity)
        } else {
            Some(Self::Infinity)
        }
    }
}

/// A stretch of markdown read by the same rules: the body, a list item's
/// content, a table cell's content, a markdown code block's content.
#[model_schema(default_types(E = Markdown))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(bound = "")]
#[non_exhaustive]
pub struct MarkdownBlock<E = Markdown>
where
    E: Extension,
{
    #[serde(skip)]
    pub kind: Kind<E>,
    pub sections: Vec<Section<E>>,
    pub span: Range<usize>,
}

/// A node that stands on its own lines.
#[model_schema(default_types(E = Markdown))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(bound = "")]
#[non_exhaustive]
pub enum Block<E = Markdown>
where
    E: Extension,
{
    BlockQuote(BlockQuote<E>),
    CodeBlock(CodeBlock<E>),
    Html(Html<E>),
    List(List<E>),
    Paragraph(Paragraph<E>),
    Table(Table<E>),
    ThematicBreak(ThematicBreak<E>),
}

impl<E> Block<E>
where
    E: Extension,
{
    /// The byte range the block covers.
    #[must_use]
    pub const fn span(&self) -> &Range<usize> {
        match self {
            Self::BlockQuote(block_quote) => &block_quote.span,
            Self::CodeBlock(code_block) => &code_block.span,
            Self::Html(html) => &html.span,
            Self::List(list) => &list.span,
            Self::Paragraph(paragraph) => &paragraph.span,
            Self::Table(table) => &table.span,
            Self::ThematicBreak(thematic_break) => &thematic_break.span,
        }
    }
}

/// Quoted text, without its `>` marks. Nothing inside it is read.
#[model_schema(default_types(E = Markdown))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(bound = "")]
#[non_exhaustive]
pub struct BlockQuote<E = Markdown>
where
    E: Extension,
{
    #[serde(skip)]
    pub kind: Kind<E>,
    pub span: Range<usize>,
    pub value: String,
}

/// Raw HTML, as written: a block, or an inline node such as `<span>`. A
/// block's `continues` is always `Continues::Neither`.
#[model_schema(default_types(E = Markdown))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(bound = "")]
#[non_exhaustive]
pub struct Html<E = Markdown>
where
    E: Extension,
{
    pub continues: Continues,
    #[serde(skip)]
    pub kind: Kind<E>,
    pub span: Range<usize>,
    pub value: String,
}

/// A line of three or more `-`, `*` or `_`, such as `---`.
#[model_schema(default_types(E = Markdown))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(bound = "")]
#[non_exhaustive]
pub struct ThematicBreak<E = Markdown>
where
    E: Extension,
{
    #[serde(skip)]
    pub kind: Kind<E>,
    pub span: Range<usize>,
}

/// A heading, the blocks under it, then its subsections. Its level is its
/// depth in the `MarkdownBlock`.
///
/// `heading` is `None` when no heading line was written, and a heading line
/// with no text is a heading with no children.
#[model_schema(default_types(E = Markdown))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(bound = "")]
#[non_exhaustive]
pub struct Section<E = Markdown>
where
    E: Extension,
{
    pub blocks: Vec<Block<E>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub heading: Option<Heading<E>>,
    #[serde(skip)]
    pub kind: Kind<E>,
    pub span: Range<usize>,
    pub subsections: Vec<Self>,
}

/// A heading line, such as `# Ship parser`. An empty heading has no children
/// and an empty span at the start of its section.
#[model_schema(default_types(E = Markdown))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(bound = "")]
#[non_exhaustive]
pub struct Heading<E = Markdown>
where
    E: Extension,
{
    pub children: Vec<Inline<E>>,
    #[serde(skip)]
    pub kind: Kind<E>,
    pub span: Range<usize>,
}

/// A set of lines, ended by a blank line.
#[model_schema(default_types(E = Markdown))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(bound = "")]
#[non_exhaustive]
pub struct Paragraph<E = Markdown>
where
    E: Extension,
{
    #[serde(skip)]
    pub kind: Kind<E>,
    pub lines: Vec<Line<E>>,
    pub span: Range<usize>,
}

/// One line of a paragraph and the inline nodes written on it.
#[model_schema(default_types(E = Markdown))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(bound = "")]
#[non_exhaustive]
pub struct Line<E = Markdown>
where
    E: Extension,
{
    pub children: Vec<Inline<E>>,
    #[serde(skip)]
    pub kind: Kind<E>,
    pub span: Range<usize>,
}

/// A bullet or ordered list.
///
/// Its items have the same bullet character, or the same delimiter after
/// their number. `start` is an ordered list's first number, `None` for a
/// bullet list. `loose` is set when a blank line separates two items, or two
/// blocks of one item.
#[model_schema(default_types(E = Markdown))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(bound = "")]
#[non_exhaustive]
pub struct List<E = Markdown>
where
    E: Extension,
{
    pub items: Vec<Item<E>>,
    #[serde(skip)]
    pub kind: Kind<E>,
    pub loose: bool,
    pub span: Range<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start: Option<u64>,
}

/// One entry of a list.
#[model_schema(default_types(E = Markdown))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(bound = "")]
#[non_exhaustive]
pub struct Item<E = Markdown>
where
    E: Extension,
{
    pub body: MarkdownBlock<E>,
    #[serde(skip)]
    pub kind: Kind<E>,
    pub span: Range<usize>,
}

/// Lines between two fences of backticks or of tildes, or indented four
/// columns past their parent's.
///
/// `content` is the range of those lines in the source, and `value` the lines
/// as the block holds them. The info string, the text after the opening
/// fence, is split into `language`, its first word, `None` when there is no
/// info string, and `args`, the rest.
#[model_schema(default_types(E = Markdown))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(bound = "")]
#[non_exhaustive]
pub struct CodeBlock<E = Markdown>
where
    E: Extension,
{
    pub args: Args,
    pub content: Range<usize>,
    #[serde(skip)]
    pub kind: Kind<E>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    pub span: Range<usize>,
    pub value: String,
}

/// The rest of a fenced code block's info string, after its language.
#[model_schema()]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Args {
    /// The rest as written, when it is not the inside of a TOML inline table.
    Text(String),
    /// The rest read as the inside of a TOML inline table; empty when the
    /// fence has nothing after its language.
    Toml(Mapping),
}

/// A table read from pipe, grid or simple table syntax. `head` holds the
/// header rows, `rows` the rest.
#[model_schema(default_types(E = Markdown))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(bound = "")]
#[non_exhaustive]
pub struct Table<E = Markdown>
where
    E: Extension,
{
    pub alignments: Vec<Alignment>,
    pub format: TableFormat,
    pub head: Vec<TableRow<E>>,
    #[serde(skip)]
    pub kind: Kind<E>,
    pub rows: Vec<TableRow<E>>,
    pub span: Range<usize>,
}

/// One row of a table.
#[model_schema(default_types(E = Markdown))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(bound = "")]
#[non_exhaustive]
pub struct TableRow<E = Markdown>
where
    E: Extension,
{
    pub cells: Vec<TableCell<E>>,
    #[serde(skip)]
    pub kind: Kind<E>,
    pub span: Range<usize>,
}

/// One cell of a table row.
#[model_schema(default_types(E = Markdown))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(bound = "")]
#[non_exhaustive]
pub struct TableCell<E = Markdown>
where
    E: Extension,
{
    pub body: MarkdownBlock<E>,
    #[serde(skip)]
    pub kind: Kind<E>,
    pub span: Range<usize>,
}

/// How a column's cells are aligned.
#[model_schema()]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Alignment {
    Center,
    Default,
    Left,
    Right,
}

/// The syntax a table is written in.
#[model_schema()]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
#[model_schema(default_types(E = Markdown))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(bound = "")]
#[non_exhaustive]
pub enum Inline<E = Markdown>
where
    E: Extension,
{
    Code(Code<E>),
    Emphasis(Emphasis<E>),
    Html(Html<E>),
    Image(Image<E>),
    LineBreak(LineBreak<E>),
    Link(Link<E>),
    Strong(Strong<E>),
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
            Self::Emphasis(emphasis) => &emphasis.span,
            Self::Html(html) => &html.span,
            Self::Image(image) => &image.span,
            Self::LineBreak(line_break) => &line_break.span,
            Self::Link(link) => &link.span,
            Self::Strong(strong) => &strong.span,
            Self::Text(text) => &text.span,
        }
    }
}

/// Whether an inline node goes on from the line before or into the line
/// after. A node written across lines of a paragraph is cut into one piece
/// per line.
#[model_schema()]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Continues {
    /// A middle piece: it goes on from the line before and into the line after.
    Both,
    /// The node is whole on its line.
    #[default]
    Neither,
    /// The first piece: it goes on into the line after.
    Next,
    /// The last piece: it goes on from the line before.
    Previous,
}

/// Plain text.
#[model_schema(default_types(E = Markdown))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(bound = "")]
#[non_exhaustive]
pub struct Text<E = Markdown>
where
    E: Extension,
{
    #[serde(skip)]
    pub kind: Kind<E>,
    pub span: Range<usize>,
    pub value: String,
}

/// Text between `*` or `_`, such as `*a*`.
#[model_schema(default_types(E = Markdown))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(bound = "")]
#[non_exhaustive]
pub struct Emphasis<E = Markdown>
where
    E: Extension,
{
    pub children: Vec<Inline<E>>,
    pub continues: Continues,
    #[serde(skip)]
    pub kind: Kind<E>,
    pub span: Range<usize>,
}

/// Text between `**` or `__`, such as `**b**`.
#[model_schema(default_types(E = Markdown))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(bound = "")]
#[non_exhaustive]
pub struct Strong<E = Markdown>
where
    E: Extension,
{
    pub children: Vec<Inline<E>>,
    pub continues: Continues,
    #[serde(skip)]
    pub kind: Kind<E>,
    pub span: Range<usize>,
}

/// A hard line break: the two or more spaces, or the backslash, that end a
/// line followed by another line of its paragraph.
#[model_schema(default_types(E = Markdown))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(bound = "")]
#[non_exhaustive]
pub struct LineBreak<E = Markdown>
where
    E: Extension,
{
    #[serde(skip)]
    pub kind: Kind<E>,
    pub span: Range<usize>,
}

/// A link: `[text](target "title")`, a reference link, an autolink,
/// `[[target]]` or `[[target|text]]`.
///
/// `children` are the text shown, and are empty for `[[target]]`. A
/// reference link takes its target and title from its label's definition.
/// `target` and `title` hold their escapes and references resolved.
#[model_schema(default_types(E = Markdown))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(bound = "")]
#[non_exhaustive]
pub struct Link<E = Markdown>
where
    E: Extension,
{
    pub children: Vec<Inline<E>>,
    pub continues: Continues,
    pub form: LinkForm,
    #[serde(skip)]
    pub kind: Kind<E>,
    pub span: Range<usize>,
    pub target: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

/// How a link or an image is written.
#[model_schema()]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum LinkForm {
    /// `[text](target)`, or `![alt](target)` for an image.
    Markdown,
    /// `[[target|text]]`, or `![[target|alt]]` for an image.
    Wiki,
}

/// An embed: `![alt](target "title")`, `![[target]]` or `![[target|alt]]`.
///
/// `alt` is empty when none is written. `target` and `title` hold their
/// escapes and references resolved.
#[model_schema(default_types(E = Markdown))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(bound = "")]
#[non_exhaustive]
pub struct Image<E = Markdown>
where
    E: Extension,
{
    pub alt: String,
    pub continues: Continues,
    pub form: LinkForm,
    #[serde(skip)]
    pub kind: Kind<E>,
    pub span: Range<usize>,
    pub target: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

/// Text between backticks, such as `` `archie check` ``.
#[model_schema(default_types(E = Markdown))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(bound = "")]
#[non_exhaustive]
pub struct Code<E = Markdown>
where
    E: Extension,
{
    pub continues: Continues,
    #[serde(skip)]
    pub kind: Kind<E>,
    pub span: Range<usize>,
    pub value: String,
}
