//! What a consumer of archie adds to the tree: its own node types, which it
//! gives to the nodes it claims.

use core::convert::Infallible;
use core::fmt::Debug;
use core::ops::Range;

use tixschema::model_schema;

use crate::parser::ast::{
    BlockQuote, Code, CodeBlock, Document, Emphasis, Heading, Html, Image, Item, Line, LineBreak,
    Link, List, MarkdownBlock, Paragraph, Section, Strong, Table, TableCell, TableRow, Text,
    ThematicBreak,
};
use crate::parser::error::ParseError;
use crate::parser::{block, claim};
use crate::writer::Writer;

/// A consumer's node types. archie builds every node first, children before
/// parents, and then offers it to the extension.
pub trait Extension: Clone + Debug + PartialEq + Sized {
    /// The node an extension builds. `T` is the full set of extensions, so a
    /// node can hold archie nodes typed with it.
    type Node<T>: Clone + Debug + PartialEq
    where
        T: Extension;

    /// `None` leaves the construct to archie. `Some(Err(..))` falls back to
    /// the plain node, or fails a strict parse. No construct that
    /// `BLOCK_CAP` or more list items, markdown blocks and table cells hold
    /// is offered: it stays the node archie built.
    fn claim<T>(
        construct: Construct<'_, T>,
        context: &Context<'_>,
    ) -> Option<Result<Self::Node<T>, ParseError>>
    where
        T: Extension;

    /// Writes a claimed node back. `plain` is the node archie built under it.
    fn write<T>(node: &Self::Node<T>, plain: Construct<'_, T>, writer: &mut Writer)
    where
        T: Extension;
}

/// A node archie offers to the extensions, typed with the full set `T`.
#[derive(Clone)]
#[non_exhaustive]
pub enum Construct<'node, T>
where
    T: Extension,
{
    BlockQuote(&'node BlockQuote<T>),
    Code(&'node Code<T>),
    CodeBlock(&'node CodeBlock<T>),
    Document(&'node Document<T>),
    Emphasis(&'node Emphasis<T>),
    Heading(&'node Heading<T>),
    Html(&'node Html<T>),
    Image(&'node Image<T>),
    Item(&'node Item<T>),
    Line(&'node Line<T>),
    LineBreak(&'node LineBreak<T>),
    Link(&'node Link<T>),
    List(&'node List<T>),
    MarkdownBlock(&'node MarkdownBlock<T>),
    Paragraph(&'node Paragraph<T>),
    Section(&'node Section<T>),
    Strong(&'node Strong<T>),
    Table(&'node Table<T>),
    TableCell(&'node TableCell<T>),
    TableRow(&'node TableRow<T>),
    Text(&'node Text<T>),
    ThematicBreak(&'node ThematicBreak<T>),
}

/// What an extension reads a construct with: where the construct starts on
/// its line, how many containers hold it, the lines of the grid cell it lies
/// in, the source text, and whether the parse is strict.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct Context<'text> {
    /// The column the offered construct's first line starts at, counted
    /// from the start of its cell's line in a grid cell.
    pub column: usize,
    /// How many list items, markdown blocks and table cells hold the offered
    /// construct. No construct that `BLOCK_CAP` or more of them hold is
    /// offered.
    pub depth: usize,
    /// The lines of the grid cell the offered construct lies in, as ranges
    /// of the source, or `None` outside grid cells.
    pub lines: Option<&'text [Range<usize>]>,
    pub source: &'text str,
    pub strict: bool,
}

impl Context<'_> {
    /// Reads `range` of the source as a markdown block, with the extensions
    /// `T`, each line with at most `column` columns of its leading white
    /// space left out.
    /// In a grid cell, only the parts of `range` on the cell's lines are
    /// read, one after another. The block is one container deeper than the
    /// construct. Spans count from the start of the source.
    ///
    /// # Errors
    ///
    /// In a strict parse, returns the error of an extension that claims a
    /// construct inside `range` and cannot read it.
    pub fn markdown<T>(&self, range: Range<usize>) -> Result<MarkdownBlock<T>, ParseError>
    where
        T: Extension,
    {
        let depth = self.depth + 1;
        let (mut read, _, cells) = match self.lines {
            Some(lines) => block::parse_lines(
                self.source,
                &within(lines, &range),
                self.column,
                depth,
                range,
            ),
            None => block::parse_from(&self.source[..range.end], range.start, self.column, depth),
        };
        claim::markdown_block(&mut read, &Self { depth, ..*self }, &cells)?;
        Ok(read)
    }
}

/// An extension that claims nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Plain;

impl Extension for Plain {
    type Node<T>
        = Infallible
    where
        T: Extension;

    fn claim<T>(
        _construct: Construct<'_, T>,
        _context: &Context<'_>,
    ) -> Option<Result<Infallible, ParseError>>
    where
        T: Extension,
    {
        None
    }

    /// Never called: `Plain` claims nothing, so it has no node to write.
    fn write<T>(_node: &Infallible, _plain: Construct<'_, T>, _writer: &mut Writer)
    where
        T: Extension,
    {
    }
}

/// archie's own extension: a fenced block whose language is the word
/// `markdown` or `md`, in any case, holds a `MarkdownBlock`.
#[model_schema()]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Markdown;

impl Extension for Markdown {
    type Node<T>
        = MarkdownBlock<T>
    where
        T: Extension;

    fn claim<T>(
        construct: Construct<'_, T>,
        context: &Context<'_>,
    ) -> Option<Result<Self::Node<T>, ParseError>>
    where
        T: Extension,
    {
        let Construct::CodeBlock(block) = construct else {
            return None;
        };
        let language = block.language.as_deref()?;
        (language.eq_ignore_ascii_case("markdown") || language.eq_ignore_ascii_case("md"))
            .then(|| context.markdown::<T>(block.content.clone()))
    }

    /// Fenced with the info string of the code block it was read from.
    fn write<T>(node: &Self::Node<T>, plain: Construct<'_, T>, writer: &mut Writer)
    where
        T: Extension,
    {
        let info = if let Construct::CodeBlock(block) = plain {
            writer.info_string(block)
        } else {
            "markdown".to_owned()
        };
        writer.fenced(&info, |inner| inner.markdown_block(node));
    }
}

/// Two extensions together. The first one that claims a construct gets it.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Either<A, B> {
    First(A),
    Second(B),
}

impl<A, B> Extension for (A, B)
where
    A: Extension,
    B: Extension,
{
    type Node<T>
        = Either<A::Node<T>, B::Node<T>>
    where
        T: Extension;

    fn claim<T>(
        construct: Construct<'_, T>,
        context: &Context<'_>,
    ) -> Option<Result<Self::Node<T>, ParseError>>
    where
        T: Extension,
    {
        A::claim(construct.clone(), context)
            .map(|result| result.map(Either::First))
            .or_else(|| B::claim(construct, context).map(|result| result.map(Either::Second)))
    }

    fn write<T>(node: &Self::Node<T>, plain: Construct<'_, T>, writer: &mut Writer)
    where
        T: Extension,
    {
        match node {
            Either::First(first) => A::write(first, plain, writer),
            Either::Second(second) => B::write(second, plain, writer),
        }
    }
}

/// The extension set archie parses with: its own markdown extension first,
/// then the consumer's.
pub type With<E> = (Markdown, E);

/// The part of `range` on each of `lines` that it reaches.
fn within(lines: &[Range<usize>], range: &Range<usize>) -> Vec<Range<usize>> {
    lines
        .iter()
        .filter(|line| line.start <= range.end && range.start <= line.end)
        .map(|line| line.start.max(range.start)..line.end.min(range.end))
        .collect()
}
