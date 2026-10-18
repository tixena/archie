//! The document tree a markdown file parses into.
//!
//! Every node carries `kind`, the type an extension gave it, and none holds
//! a span: a parse reports positions beside the document.

use chrono::{DateTime, FixedOffset, NaiveDate, NaiveDateTime, NaiveTime};
use indexmap::IndexMap;
use rkyv::rancor::Source;
use rkyv::ser::{Allocator, Writer};
use rkyv::validation::ArchiveContext;
use rkyv::with::{Map, Skip};
// tixschema takes every derive named `Deserialize` off a model whose read it
// writes, rkyv's with serde's.
use rkyv::Deserialize as RkyvDeserialize;
use serde::{Deserialize, Deserializer, Serialize};
use tixschema::model_schema;

use crate::parser::extension::{Extension, Markdown, markdown_schema};
use crate::parser::mirror::{
    self, AlignmentMirror, ArgsMirror, BlockMirror, ContinuesMirror, FrontmatterFormatMirror,
    InlineMirror, LinkFormMirror, MappingMirror, TableFormatMirror,
};

/// The stack left below which `grown` reads on a new segment. It holds the
/// `validate()` that tixschema runs over the subtree each grown read returns.
const RED_ZONE: usize = 3 * 1024 * 1024;

/// The size of each segment `grown` adds.
const GROWTH: usize = 4 * 1024 * 1024;

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

/// What `parse` builds: a document whose frontmatter reads, or one whose
/// frontmatter no entry of reads.
#[model_schema(default_types(E = Markdown), enable = ["decode_with", "mongodb"])]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(bound = "")]
#[serde(tag = "t", rename_all = "kebab-case")]
#[non_exhaustive]
pub enum RawDocument<E = Markdown>
where
    E: Extension,
{
    Invalid(InvalidDocument),
    Valid(Document<E>),
}

/// A `RawDocument` as rkyv archives it.
pub type ArchivedRawDocument<E = Markdown> = mirror::ArchivedRawDocumentMirror<E>;

/// A document whose frontmatter no entry of reads: its body, claimed by
/// archie's markdown extension alone, and the frontmatter as written.
#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(
    Debug, Clone, PartialEq, Serialize, Deserialize, rkyv::Archive, rkyv::Serialize, RkyvDeserialize,
)]
#[rkyv(attr(non_exhaustive))]
#[non_exhaustive]
pub struct InvalidDocument {
    #[serde(rename = "bd")]
    pub body: MarkdownBlock<Markdown>,
    #[serde(rename = "data")]
    pub frontmatter: InvalidFrontmatter,
}

/// A frontmatter no entry of reads: the reader's message, its syntax, and its
/// text as written, so that a write gives it back.
#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    rkyv::Archive,
    rkyv::Serialize,
    RkyvDeserialize,
)]
#[rkyv(attr(non_exhaustive))]
#[non_exhaustive]
pub struct InvalidFrontmatter {
    #[serde(rename = "er")]
    pub error: String,
    #[serde(rename = "fo")]
    #[rkyv(with = FrontmatterFormatMirror)]
    pub format: FrontmatterFormat,
    #[serde(rename = "tx")]
    pub text: String,
}

/// A markdown file whose frontmatter, if it has one, reads.
#[model_schema(default_types(E = Markdown), enable = ["decode_with", "mongodb"])]
#[derive(
    Debug, Clone, PartialEq, Serialize, Deserialize, rkyv::Archive, rkyv::Serialize, RkyvDeserialize,
)]
#[serde(bound = "")]
#[rkyv(attr(non_exhaustive))]
#[non_exhaustive]
pub struct Document<E = Markdown>
where
    E: Extension,
{
    #[serde(rename = "bd")]
    pub body: MarkdownBlock<E>,
    #[serde(rename = "data")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[rkyv(with = Map<MappingMirror>)]
    pub frontmatter: Option<Mapping>,
    #[serde(skip)]
    #[rkyv(with = Skip)]
    pub kind: Kind<E>,
}

/// The syntax a frontmatter block is written in.
#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum FrontmatterFormat {
    Toml,
    #[default]
    Yaml,
}

/// A `FrontmatterFormat` as rkyv archives it.
pub type ArchivedFrontmatterFormat = mirror::ArchivedFrontmatterFormatMirror;

/// Keys and their values, in the order they were written. A key appears once.
#[model_schema()]
pub type Mapping = IndexMap<String, Value>;

/// What a key maps to, or what a list holds.
#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "t", content = "v", rename_all = "kebab-case")]
#[non_exhaustive]
pub enum Value {
    Boolean(bool),
    Date(NaiveDate),
    /// A date and time with no offset, such as `1979-05-27T07:32:00`.
    DateTime(NaiveDateTime),
    /// A finite float.
    Float(f64),
    Integer(i64),
    #[serde(deserialize_with = "grown")]
    Mapping(Mapping),
    NonFinite(NonFinite),
    /// YAML only: TOML has no null.
    Null,
    /// A date and time with an offset, such as `1979-05-27T07:32:00+02:00`.
    OffsetDateTime(DateTime<FixedOffset>),
    #[serde(deserialize_with = "grown")]
    Sequence(Vec<Self>),
    String(String),
    Time(NaiveTime),
}

/// A `Value` as rkyv archives it.
pub type ArchivedValue = mirror::ArchivedValueMirror;

/// A float that is not a finite number: `.inf`, `-.inf` and `.nan` in
/// YAML; `inf`, `-inf` and `nan` in TOML.
#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
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

/// A `NonFinite` as rkyv archives it.
pub type ArchivedNonFinite = mirror::ArchivedNonFiniteMirror;

/// A stretch of markdown read by the same rules: the body, a list item's
/// content, a table cell's content, a markdown code block's content.
#[model_schema(default_types(E = Markdown), enable = ["decode_with", "mongodb"])]
#[derive(
    Debug, Clone, PartialEq, Serialize, Deserialize, rkyv::Archive, rkyv::Serialize, RkyvDeserialize,
)]
#[serde(bound = "")]
#[rkyv(attr(non_exhaustive))]
#[non_exhaustive]
pub struct MarkdownBlock<E = Markdown>
where
    E: Extension,
{
    #[serde(skip)]
    #[rkyv(with = Skip)]
    pub kind: Kind<E>,
    #[serde(rename = "se")]
    #[serde(deserialize_with = "grown")]
    pub sections: Vec<Section<E>>,
}

/// A node that stands on its own lines.
#[model_schema(default_types(E = Markdown), enable = ["decode_with", "mongodb"])]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(bound = "")]
#[serde(tag = "t", rename_all = "kebab-case")]
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

/// A `Block` as rkyv archives it.
pub type ArchivedBlock<E = Markdown> = mirror::ArchivedBlockMirror<E>;

/// Quoted text, without its `>` marks. Nothing inside it is read.
#[model_schema(default_types(E = Markdown), enable = ["decode_with", "mongodb"])]
#[derive(
    Debug, Clone, PartialEq, Serialize, Deserialize, rkyv::Archive, rkyv::Serialize, RkyvDeserialize,
)]
#[serde(bound = "")]
#[rkyv(attr(non_exhaustive))]
#[non_exhaustive]
pub struct BlockQuote<E = Markdown>
where
    E: Extension,
{
    #[serde(skip)]
    #[rkyv(with = Skip)]
    pub kind: Kind<E>,
    #[serde(rename = "v")]
    pub value: String,
}

/// Raw HTML, as written: a block, or an inline node such as `<span>`. A
/// block's `continues` is always `Continues::Neither`.
#[model_schema(default_types(E = Markdown), enable = ["decode_with", "mongodb"])]
#[derive(
    Debug, Clone, PartialEq, Serialize, Deserialize, rkyv::Archive, rkyv::Serialize, RkyvDeserialize,
)]
#[serde(bound = "")]
#[rkyv(attr(non_exhaustive))]
#[non_exhaustive]
pub struct Html<E = Markdown>
where
    E: Extension,
{
    #[serde(rename = "co")]
    #[rkyv(with = ContinuesMirror)]
    pub continues: Continues,
    #[serde(skip)]
    #[rkyv(with = Skip)]
    pub kind: Kind<E>,
    #[serde(rename = "v")]
    pub value: String,
}

/// A line of three or more `-`, `*` or `_`, such as `---`.
#[model_schema(default_types(E = Markdown), enable = ["decode_with", "mongodb"])]
#[derive(
    Debug, Clone, PartialEq, Serialize, Deserialize, rkyv::Archive, rkyv::Serialize, RkyvDeserialize,
)]
#[serde(bound = "")]
#[rkyv(attr(non_exhaustive))]
#[non_exhaustive]
pub struct ThematicBreak<E = Markdown>
where
    E: Extension,
{
    #[serde(skip)]
    #[rkyv(with = Skip)]
    pub kind: Kind<E>,
}

/// A heading, the blocks under it, then its subsections. Its level is its
/// depth in the `MarkdownBlock`.
///
/// `heading` is `None` when no heading line was written, and a heading line
/// with no text is a heading with no children.
#[model_schema(default_types(E = Markdown), enable = ["decode_with", "mongodb"])]
#[derive(
    Debug, Clone, PartialEq, Serialize, Deserialize, rkyv::Archive, rkyv::Serialize, RkyvDeserialize,
)]
#[serde(bound = "")]
#[rkyv(serialize_bounds(__S: Writer + Allocator, __S::Error: Source))]
#[rkyv(deserialize_bounds(__D::Error: Source))]
#[rkyv(bytecheck(bounds(__C: ArchiveContext, __C::Error: Source)))]
#[rkyv(attr(non_exhaustive))]
#[non_exhaustive]
pub struct Section<E = Markdown>
where
    E: Extension,
{
    #[serde(rename = "bl")]
    #[rkyv(omit_bounds, with = Map<BlockMirror<E>>)]
    pub blocks: Vec<Block<E>>,
    #[serde(rename = "hd")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub heading: Option<Heading<E>>,
    #[serde(skip)]
    #[rkyv(with = Skip)]
    pub kind: Kind<E>,
    #[serde(rename = "ss")]
    #[serde(deserialize_with = "grown")]
    #[rkyv(omit_bounds)]
    pub subsections: Vec<Self>,
}

/// A heading line, such as `# Ship parser`. An empty heading has no children.
#[model_schema(default_types(E = Markdown), enable = ["decode_with", "mongodb"])]
#[derive(
    Debug, Clone, PartialEq, Serialize, Deserialize, rkyv::Archive, rkyv::Serialize, RkyvDeserialize,
)]
#[serde(bound = "")]
#[rkyv(attr(non_exhaustive))]
#[non_exhaustive]
pub struct Heading<E = Markdown>
where
    E: Extension,
{
    #[serde(rename = "ch")]
    #[rkyv(with = Map<InlineMirror<E>>)]
    pub children: Vec<Inline<E>>,
    #[serde(skip)]
    #[rkyv(with = Skip)]
    pub kind: Kind<E>,
}

/// A set of lines, ended by a blank line.
#[model_schema(default_types(E = Markdown), enable = ["decode_with", "mongodb"])]
#[derive(
    Debug, Clone, PartialEq, Serialize, Deserialize, rkyv::Archive, rkyv::Serialize, RkyvDeserialize,
)]
#[serde(bound = "")]
#[rkyv(attr(non_exhaustive))]
#[non_exhaustive]
pub struct Paragraph<E = Markdown>
where
    E: Extension,
{
    #[serde(skip)]
    #[rkyv(with = Skip)]
    pub kind: Kind<E>,
    #[serde(rename = "li")]
    pub lines: Vec<Line<E>>,
}

/// One line of a paragraph and the inline nodes written on it.
#[model_schema(default_types(E = Markdown), enable = ["decode_with", "mongodb"])]
#[derive(
    Debug, Clone, PartialEq, Serialize, Deserialize, rkyv::Archive, rkyv::Serialize, RkyvDeserialize,
)]
#[serde(bound = "")]
#[rkyv(attr(non_exhaustive))]
#[non_exhaustive]
pub struct Line<E = Markdown>
where
    E: Extension,
{
    #[serde(rename = "ch")]
    #[rkyv(with = Map<InlineMirror<E>>)]
    pub children: Vec<Inline<E>>,
    #[serde(skip)]
    #[rkyv(with = Skip)]
    pub kind: Kind<E>,
}

/// A bullet or ordered list.
///
/// Its items have the same bullet character, or the same delimiter after
/// their number. `start` is an ordered list's first number, `None` for a
/// bullet list. `loose` is set when a blank line separates two items, or two
/// blocks of one item.
#[model_schema(default_types(E = Markdown), enable = ["decode_with", "mongodb"])]
#[derive(
    Debug, Clone, PartialEq, Serialize, Deserialize, rkyv::Archive, rkyv::Serialize, RkyvDeserialize,
)]
#[serde(bound = "")]
#[rkyv(attr(non_exhaustive))]
#[non_exhaustive]
pub struct List<E = Markdown>
where
    E: Extension,
{
    #[serde(rename = "it")]
    pub items: Vec<Item<E>>,
    #[serde(skip)]
    #[rkyv(with = Skip)]
    pub kind: Kind<E>,
    #[serde(rename = "lo")]
    pub loose: bool,
    #[serde(rename = "st")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start: Option<u64>,
}

/// One entry of a list.
#[model_schema(default_types(E = Markdown), enable = ["decode_with", "mongodb"])]
#[derive(
    Debug, Clone, PartialEq, Serialize, Deserialize, rkyv::Archive, rkyv::Serialize, RkyvDeserialize,
)]
#[serde(bound = "")]
#[rkyv(attr(non_exhaustive))]
#[non_exhaustive]
pub struct Item<E = Markdown>
where
    E: Extension,
{
    #[serde(rename = "bd")]
    pub body: MarkdownBlock<E>,
    #[serde(skip)]
    #[rkyv(with = Skip)]
    pub kind: Kind<E>,
}

/// Lines between two fences of backticks or of tildes, or indented four
/// columns past their parent's.
///
/// `value` holds the lines as the block holds them. The info string, the
/// text after the opening fence, is split into `language`, its first word,
/// `None` when there is no info string, and `args`, the rest.
#[model_schema(default_types(E = Markdown), enable = ["decode_with", "mongodb"])]
#[derive(
    Debug, Clone, PartialEq, Serialize, Deserialize, rkyv::Archive, rkyv::Serialize, RkyvDeserialize,
)]
#[serde(bound = "")]
#[rkyv(attr(non_exhaustive))]
#[non_exhaustive]
pub struct CodeBlock<E = Markdown>
where
    E: Extension,
{
    #[serde(rename = "ar")]
    #[rkyv(with = ArgsMirror)]
    pub args: Args,
    #[serde(skip)]
    #[rkyv(with = Skip)]
    pub kind: Kind<E>,
    #[serde(rename = "la")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    #[serde(rename = "v")]
    pub value: String,
}

/// The rest of a fenced code block's info string, after its language.
#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "t", content = "v", rename_all = "kebab-case")]
#[non_exhaustive]
pub enum Args {
    /// The rest as written, when it is not the inside of a TOML inline table.
    Text(String),
    /// The rest read as the inside of a TOML inline table; empty when the
    /// fence has nothing after its language.
    Toml(Mapping),
}

/// An `Args` as rkyv archives it.
pub type ArchivedArgs = mirror::ArchivedArgsMirror;

/// A table read from pipe, grid or simple table syntax. `head` holds the
/// header rows, `rows` the rest.
#[model_schema(default_types(E = Markdown), enable = ["decode_with", "mongodb"])]
#[derive(
    Debug, Clone, PartialEq, Serialize, Deserialize, rkyv::Archive, rkyv::Serialize, RkyvDeserialize,
)]
#[serde(bound = "")]
#[rkyv(attr(non_exhaustive))]
#[non_exhaustive]
pub struct Table<E = Markdown>
where
    E: Extension,
{
    #[serde(rename = "al")]
    #[rkyv(with = Map<AlignmentMirror>)]
    pub alignments: Vec<Alignment>,
    #[serde(rename = "fo")]
    #[rkyv(with = TableFormatMirror)]
    pub format: TableFormat,
    #[serde(rename = "he")]
    pub head: Vec<TableRow<E>>,
    #[serde(skip)]
    #[rkyv(with = Skip)]
    pub kind: Kind<E>,
    #[serde(rename = "ro")]
    pub rows: Vec<TableRow<E>>,
}

/// One row of a table.
#[model_schema(default_types(E = Markdown), enable = ["decode_with", "mongodb"])]
#[derive(
    Debug, Clone, PartialEq, Serialize, Deserialize, rkyv::Archive, rkyv::Serialize, RkyvDeserialize,
)]
#[serde(bound = "")]
#[rkyv(attr(non_exhaustive))]
#[non_exhaustive]
pub struct TableRow<E = Markdown>
where
    E: Extension,
{
    #[serde(rename = "ce")]
    pub cells: Vec<TableCell<E>>,
    #[serde(skip)]
    #[rkyv(with = Skip)]
    pub kind: Kind<E>,
}

/// One cell of a table row.
#[model_schema(default_types(E = Markdown), enable = ["decode_with", "mongodb"])]
#[derive(
    Debug, Clone, PartialEq, Serialize, Deserialize, rkyv::Archive, rkyv::Serialize, RkyvDeserialize,
)]
#[serde(bound = "")]
#[rkyv(attr(non_exhaustive))]
#[non_exhaustive]
pub struct TableCell<E = Markdown>
where
    E: Extension,
{
    #[serde(rename = "bd")]
    pub body: MarkdownBlock<E>,
    #[serde(skip)]
    #[rkyv(with = Skip)]
    pub kind: Kind<E>,
}

/// How a column's cells are aligned.
#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum Alignment {
    Center,
    Default,
    Left,
    Right,
}

/// An `Alignment` as rkyv archives it.
pub type ArchivedAlignment = mirror::ArchivedAlignmentMirror;

/// The syntax a table is written in.
#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum TableFormat {
    /// `+---+` borders and `|` walls.
    Grid,
    /// `| a | b |` rows under a `|---|---|` delimiter row.
    Pipe,
    /// Columns marked by groups of dashes under a header line.
    Simple,
}

/// A `TableFormat` as rkyv archives it.
pub type ArchivedTableFormat = mirror::ArchivedTableFormatMirror;

/// A node inside a line of text.
#[model_schema(default_types(E = Markdown), enable = ["decode_with", "mongodb"])]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(bound = "")]
#[serde(tag = "t", rename_all = "kebab-case")]
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

/// An `Inline` as rkyv archives it.
pub type ArchivedInline<E = Markdown> = mirror::ArchivedInlineMirror<E>;

/// Whether an inline node goes on from the line before or into the line
/// after. A node written across lines of a paragraph is cut into one piece
/// per line.
#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
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

/// A `Continues` as rkyv archives it.
pub type ArchivedContinues = mirror::ArchivedContinuesMirror;

/// Plain text.
#[model_schema(default_types(E = Markdown), enable = ["decode_with", "mongodb"])]
#[derive(
    Debug, Clone, PartialEq, Serialize, Deserialize, rkyv::Archive, rkyv::Serialize, RkyvDeserialize,
)]
#[serde(bound = "")]
#[rkyv(attr(non_exhaustive))]
#[non_exhaustive]
pub struct Text<E = Markdown>
where
    E: Extension,
{
    #[serde(skip)]
    #[rkyv(with = Skip)]
    pub kind: Kind<E>,
    #[serde(rename = "v")]
    pub value: String,
}

/// Text between `*` or `_`, such as `*a*`.
#[model_schema(default_types(E = Markdown), enable = ["decode_with", "mongodb"])]
#[derive(
    Debug, Clone, PartialEq, Serialize, Deserialize, rkyv::Archive, rkyv::Serialize, RkyvDeserialize,
)]
#[serde(bound = "")]
#[rkyv(serialize_bounds(__S: Writer + Allocator, __S::Error: Source))]
#[rkyv(deserialize_bounds(__D::Error: Source))]
#[rkyv(bytecheck(bounds(__C: ArchiveContext, __C::Error: Source)))]
#[rkyv(attr(non_exhaustive))]
#[non_exhaustive]
pub struct Emphasis<E = Markdown>
where
    E: Extension,
{
    #[serde(rename = "ch")]
    #[serde(deserialize_with = "grown")]
    #[rkyv(omit_bounds, with = Map<InlineMirror<E>>)]
    pub children: Vec<Inline<E>>,
    #[serde(rename = "co")]
    #[rkyv(with = ContinuesMirror)]
    pub continues: Continues,
    #[serde(skip)]
    #[rkyv(with = Skip)]
    pub kind: Kind<E>,
}

/// Text between `**` or `__`, such as `**b**`.
#[model_schema(default_types(E = Markdown), enable = ["decode_with", "mongodb"])]
#[derive(
    Debug, Clone, PartialEq, Serialize, Deserialize, rkyv::Archive, rkyv::Serialize, RkyvDeserialize,
)]
#[serde(bound = "")]
#[rkyv(serialize_bounds(__S: Writer + Allocator, __S::Error: Source))]
#[rkyv(deserialize_bounds(__D::Error: Source))]
#[rkyv(bytecheck(bounds(__C: ArchiveContext, __C::Error: Source)))]
#[rkyv(attr(non_exhaustive))]
#[non_exhaustive]
pub struct Strong<E = Markdown>
where
    E: Extension,
{
    #[serde(rename = "ch")]
    #[serde(deserialize_with = "grown")]
    #[rkyv(omit_bounds, with = Map<InlineMirror<E>>)]
    pub children: Vec<Inline<E>>,
    #[serde(rename = "co")]
    #[rkyv(with = ContinuesMirror)]
    pub continues: Continues,
    #[serde(skip)]
    #[rkyv(with = Skip)]
    pub kind: Kind<E>,
}

/// A hard line break: the two or more spaces, or the backslash, that end a
/// line followed by another line of its paragraph.
#[model_schema(default_types(E = Markdown), enable = ["decode_with", "mongodb"])]
#[derive(
    Debug, Clone, PartialEq, Serialize, Deserialize, rkyv::Archive, rkyv::Serialize, RkyvDeserialize,
)]
#[serde(bound = "")]
#[rkyv(attr(non_exhaustive))]
#[non_exhaustive]
pub struct LineBreak<E = Markdown>
where
    E: Extension,
{
    #[serde(skip)]
    #[rkyv(with = Skip)]
    pub kind: Kind<E>,
}

/// A link: `[text](target "title")`, a reference link, an autolink,
/// `[[target]]` or `[[target|text]]`.
///
/// `children` are the text shown, and are empty for `[[target]]`. A
/// reference link takes its target and title from its label's definition.
/// `target` and `title` hold their escapes and references resolved.
#[model_schema(default_types(E = Markdown), enable = ["decode_with", "mongodb"])]
#[derive(
    Debug, Clone, PartialEq, Serialize, Deserialize, rkyv::Archive, rkyv::Serialize, RkyvDeserialize,
)]
#[serde(bound = "")]
#[rkyv(serialize_bounds(__S: Writer + Allocator, __S::Error: Source))]
#[rkyv(deserialize_bounds(__D::Error: Source))]
#[rkyv(bytecheck(bounds(__C: ArchiveContext, __C::Error: Source)))]
#[rkyv(attr(non_exhaustive))]
#[non_exhaustive]
pub struct Link<E = Markdown>
where
    E: Extension,
{
    #[serde(rename = "ch")]
    #[serde(deserialize_with = "grown")]
    #[rkyv(omit_bounds, with = Map<InlineMirror<E>>)]
    pub children: Vec<Inline<E>>,
    #[serde(rename = "co")]
    #[rkyv(with = ContinuesMirror)]
    pub continues: Continues,
    #[serde(rename = "fr")]
    #[rkyv(with = LinkFormMirror)]
    pub form: LinkForm,
    #[serde(skip)]
    #[rkyv(with = Skip)]
    pub kind: Kind<E>,
    #[serde(rename = "tg")]
    pub target: String,
    #[serde(rename = "ti")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

/// How a link or an image is written.
#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum LinkForm {
    /// `[text](target)`, or `![alt](target)` for an image.
    Markdown,
    /// `[[target|text]]`, or `![[target|alt]]` for an image.
    Wiki,
}

/// A `LinkForm` as rkyv archives it.
pub type ArchivedLinkForm = mirror::ArchivedLinkFormMirror;

/// An embed: `![alt](target "title")`, `![[target]]` or `![[target|alt]]`.
///
/// `alt` is empty when none is written. `target` and `title` hold their
/// escapes and references resolved.
#[model_schema(default_types(E = Markdown), enable = ["decode_with", "mongodb"])]
#[derive(
    Debug, Clone, PartialEq, Serialize, Deserialize, rkyv::Archive, rkyv::Serialize, RkyvDeserialize,
)]
#[serde(bound = "")]
#[rkyv(attr(non_exhaustive))]
#[non_exhaustive]
pub struct Image<E = Markdown>
where
    E: Extension,
{
    #[serde(rename = "at")]
    pub alt: String,
    #[serde(rename = "co")]
    #[rkyv(with = ContinuesMirror)]
    pub continues: Continues,
    #[serde(rename = "fr")]
    #[rkyv(with = LinkFormMirror)]
    pub form: LinkForm,
    #[serde(skip)]
    #[rkyv(with = Skip)]
    pub kind: Kind<E>,
    #[serde(rename = "tg")]
    pub target: String,
    #[serde(rename = "ti")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

/// Text between backticks, such as `` `archie check` ``.
#[model_schema(default_types(E = Markdown), enable = ["decode_with", "mongodb"])]
#[derive(
    Debug, Clone, PartialEq, Serialize, Deserialize, rkyv::Archive, rkyv::Serialize, RkyvDeserialize,
)]
#[serde(bound = "")]
#[rkyv(attr(non_exhaustive))]
#[non_exhaustive]
pub struct Code<E = Markdown>
where
    E: Extension,
{
    #[serde(rename = "co")]
    #[rkyv(with = ContinuesMirror)]
    pub continues: Continues,
    #[serde(skip)]
    #[rkyv(with = Skip)]
    pub kind: Kind<E>,
    #[serde(rename = "v")]
    pub value: String,
}

/// Reads a field with the stack grown when little is left: serde reads an
/// internally tagged enum from a buffer that `serde_stacker` does not guard.
fn grown<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    stacker::maybe_grow(RED_ZONE, GROWTH, || T::deserialize(deserializer))
}
