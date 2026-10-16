//! The claim pass: every node of a built tree offered to the extensions,
//! children before parents.

use core::ops::Range;

use crate::parser::ast::{
    Block, BlockQuote, Code, CodeBlock, Document, Emphasis, Heading, Html, Image, Inline, Item,
    Kind, Line, LineBreak, Link, List, MarkdownBlock, Paragraph, Section, Strong, Table, TableCell,
    TableRow, Text, ThematicBreak,
};
use crate::parser::block::Cells;
use crate::parser::error::ParseError;
use crate::parser::extension::{Construct, Context, Extension};

/// What the pass carries down the tree: the context of the nodes it offers,
/// and each grid cell's lines, by the cell's span.
#[derive(Clone, Copy)]
struct Pass<'text> {
    cells: &'text Cells,
    context: Context<'text>,
}

impl<'text> Pass<'text> {
    /// The context of a node whose span starts at `start`: its `column` is
    /// how far `start` is from the start of its line, or of its cell's line
    /// in a grid cell.
    fn at(&self, start: usize) -> Context<'text> {
        let source = self.context.source;
        let line_start = self
            .context
            .lines
            .and_then(|lines| lines.iter().rfind(|line| line.start <= start))
            .map_or_else(
                || source[..start].rfind('\n').map_or(0, |newline| newline + 1),
                |line| line.start,
            );
        Context {
            column: start - line_start,
            ..self.context
        }
    }

    /// The pass inside the cell that spans `span`: a grid cell's nodes read
    /// only its lines.
    fn inside(&self, span: &Range<usize>) -> Self {
        self.cells.get(span).map_or(*self, |lines| Self {
            context: Context {
                lines: Some(lines),
                ..self.context
            },
            ..*self
        })
    }
}

/// A node that offers its children, then itself, to the extensions.
trait Offer {
    fn offer(&mut self, pass: &Pass<'_>) -> Result<(), ParseError>;
}

impl<T> Offer for Block<T>
where
    T: Extension,
{
    fn offer(&mut self, pass: &Pass<'_>) -> Result<(), ParseError> {
        match self {
            Self::BlockQuote(block_quote) => block_quote.offer(pass),
            Self::CodeBlock(code_block) => code_block.offer(pass),
            Self::Html(html) => html.offer(pass),
            Self::List(list) => list.offer(pass),
            Self::Paragraph(paragraph) => paragraph.offer(pass),
            Self::Table(table) => table.offer(pass),
            Self::ThematicBreak(thematic_break) => thematic_break.offer(pass),
        }
    }
}

impl<T> Offer for BlockQuote<T>
where
    T: Extension,
{
    fn offer(&mut self, pass: &Pass<'_>) -> Result<(), ParseError> {
        let claimed = T::claim(Construct::BlockQuote(self), &pass.at(self.span.start));
        settle(&mut self.kind, claimed, pass)
    }
}

impl<T> Offer for Code<T>
where
    T: Extension,
{
    fn offer(&mut self, pass: &Pass<'_>) -> Result<(), ParseError> {
        let claimed = T::claim(Construct::Code(self), &pass.at(self.span.start));
        settle(&mut self.kind, claimed, pass)
    }
}

impl<T> Offer for CodeBlock<T>
where
    T: Extension,
{
    fn offer(&mut self, pass: &Pass<'_>) -> Result<(), ParseError> {
        let claimed = T::claim(Construct::CodeBlock(self), &pass.at(self.span.start));
        settle(&mut self.kind, claimed, pass)
    }
}

impl<T> Offer for Document<T>
where
    T: Extension,
{
    fn offer(&mut self, pass: &Pass<'_>) -> Result<(), ParseError> {
        self.body.offer(pass)?;
        let claimed = T::claim(Construct::Document(self), &pass.at(self.span.start));
        settle(&mut self.kind, claimed, pass)
    }
}

impl<T> Offer for Emphasis<T>
where
    T: Extension,
{
    fn offer(&mut self, pass: &Pass<'_>) -> Result<(), ParseError> {
        each(&mut self.children, pass)?;
        let claimed = T::claim(Construct::Emphasis(self), &pass.at(self.span.start));
        settle(&mut self.kind, claimed, pass)
    }
}

impl<T> Offer for Heading<T>
where
    T: Extension,
{
    fn offer(&mut self, pass: &Pass<'_>) -> Result<(), ParseError> {
        each(&mut self.children, pass)?;
        let claimed = T::claim(Construct::Heading(self), &pass.at(self.span.start));
        settle(&mut self.kind, claimed, pass)
    }
}

impl<T> Offer for Html<T>
where
    T: Extension,
{
    fn offer(&mut self, pass: &Pass<'_>) -> Result<(), ParseError> {
        let claimed = T::claim(Construct::Html(self), &pass.at(self.span.start));
        settle(&mut self.kind, claimed, pass)
    }
}

impl<T> Offer for Image<T>
where
    T: Extension,
{
    fn offer(&mut self, pass: &Pass<'_>) -> Result<(), ParseError> {
        let claimed = T::claim(Construct::Image(self), &pass.at(self.span.start));
        settle(&mut self.kind, claimed, pass)
    }
}

impl<T> Offer for Inline<T>
where
    T: Extension,
{
    fn offer(&mut self, pass: &Pass<'_>) -> Result<(), ParseError> {
        match self {
            Self::Code(code) => code.offer(pass),
            Self::Emphasis(emphasis) => emphasis.offer(pass),
            Self::Html(html) => html.offer(pass),
            Self::Image(image) => image.offer(pass),
            Self::LineBreak(line_break) => line_break.offer(pass),
            Self::Link(link) => link.offer(pass),
            Self::Strong(strong) => strong.offer(pass),
            Self::Text(text) => text.offer(pass),
        }
    }
}

impl<T> Offer for Item<T>
where
    T: Extension,
{
    fn offer(&mut self, pass: &Pass<'_>) -> Result<(), ParseError> {
        self.body.offer(pass)?;
        let claimed = T::claim(Construct::Item(self), &pass.at(self.span.start));
        settle(&mut self.kind, claimed, pass)
    }
}

impl<T> Offer for Line<T>
where
    T: Extension,
{
    fn offer(&mut self, pass: &Pass<'_>) -> Result<(), ParseError> {
        each(&mut self.children, pass)?;
        let claimed = T::claim(Construct::Line(self), &pass.at(self.span.start));
        settle(&mut self.kind, claimed, pass)
    }
}

impl<T> Offer for LineBreak<T>
where
    T: Extension,
{
    fn offer(&mut self, pass: &Pass<'_>) -> Result<(), ParseError> {
        let claimed = T::claim(Construct::LineBreak(self), &pass.at(self.span.start));
        settle(&mut self.kind, claimed, pass)
    }
}

impl<T> Offer for Link<T>
where
    T: Extension,
{
    fn offer(&mut self, pass: &Pass<'_>) -> Result<(), ParseError> {
        each(&mut self.children, pass)?;
        let claimed = T::claim(Construct::Link(self), &pass.at(self.span.start));
        settle(&mut self.kind, claimed, pass)
    }
}

impl<T> Offer for List<T>
where
    T: Extension,
{
    fn offer(&mut self, pass: &Pass<'_>) -> Result<(), ParseError> {
        each(&mut self.items, pass)?;
        let claimed = T::claim(Construct::List(self), &pass.at(self.span.start));
        settle(&mut self.kind, claimed, pass)
    }
}

impl<T> Offer for MarkdownBlock<T>
where
    T: Extension,
{
    fn offer(&mut self, pass: &Pass<'_>) -> Result<(), ParseError> {
        each(&mut self.sections, pass)?;
        let claimed = T::claim(Construct::MarkdownBlock(self), &pass.at(self.span.start));
        settle(&mut self.kind, claimed, pass)
    }
}

impl<T> Offer for Paragraph<T>
where
    T: Extension,
{
    fn offer(&mut self, pass: &Pass<'_>) -> Result<(), ParseError> {
        each(&mut self.lines, pass)?;
        let claimed = T::claim(Construct::Paragraph(self), &pass.at(self.span.start));
        settle(&mut self.kind, claimed, pass)
    }
}

impl<T> Offer for Section<T>
where
    T: Extension,
{
    fn offer(&mut self, pass: &Pass<'_>) -> Result<(), ParseError> {
        self.heading.offer(pass)?;
        each(&mut self.blocks, pass)?;
        each(&mut self.subsections, pass)?;
        let claimed = T::claim(Construct::Section(self), &pass.at(self.span.start));
        settle(&mut self.kind, claimed, pass)
    }
}

impl<T> Offer for Strong<T>
where
    T: Extension,
{
    fn offer(&mut self, pass: &Pass<'_>) -> Result<(), ParseError> {
        each(&mut self.children, pass)?;
        let claimed = T::claim(Construct::Strong(self), &pass.at(self.span.start));
        settle(&mut self.kind, claimed, pass)
    }
}

impl<T> Offer for Table<T>
where
    T: Extension,
{
    fn offer(&mut self, pass: &Pass<'_>) -> Result<(), ParseError> {
        each(&mut self.head, pass)?;
        each(&mut self.rows, pass)?;
        let claimed = T::claim(Construct::Table(self), &pass.at(self.span.start));
        settle(&mut self.kind, claimed, pass)
    }
}

impl<T> Offer for TableCell<T>
where
    T: Extension,
{
    /// A grid cell, and every node inside it, is offered with the cell's
    /// lines.
    fn offer(&mut self, pass: &Pass<'_>) -> Result<(), ParseError> {
        let inside = pass.inside(&self.span);
        self.body.offer(&inside)?;
        let claimed = T::claim(Construct::TableCell(self), &inside.at(self.span.start));
        settle(&mut self.kind, claimed, pass)
    }
}

impl<T> Offer for TableRow<T>
where
    T: Extension,
{
    fn offer(&mut self, pass: &Pass<'_>) -> Result<(), ParseError> {
        each(&mut self.cells, pass)?;
        let claimed = T::claim(Construct::TableRow(self), &pass.at(self.span.start));
        settle(&mut self.kind, claimed, pass)
    }
}

impl<T> Offer for Text<T>
where
    T: Extension,
{
    fn offer(&mut self, pass: &Pass<'_>) -> Result<(), ParseError> {
        let claimed = T::claim(Construct::Text(self), &pass.at(self.span.start));
        settle(&mut self.kind, claimed, pass)
    }
}

impl<T> Offer for ThematicBreak<T>
where
    T: Extension,
{
    fn offer(&mut self, pass: &Pass<'_>) -> Result<(), ParseError> {
        let claimed = T::claim(Construct::ThematicBreak(self), &pass.at(self.span.start));
        settle(&mut self.kind, claimed, pass)
    }
}

/// Offers every node of `document`, the document last. `cells` holds each
/// grid cell's lines.
pub fn document<T>(
    document: &mut Document<T>,
    context: &Context<'_>,
    cells: &Cells,
) -> Result<(), ParseError>
where
    T: Extension,
{
    document.offer(&Pass {
        cells,
        context: *context,
    })
}

/// Offers every node of `block`, the block last. `cells` holds each grid
/// cell's lines.
pub fn markdown_block<T>(
    block: &mut MarkdownBlock<T>,
    context: &Context<'_>,
    cells: &Cells,
) -> Result<(), ParseError>
where
    T: Extension,
{
    block.offer(&Pass {
        cells,
        context: *context,
    })
}

fn each<N>(nodes: &mut [N], pass: &Pass<'_>) -> Result<(), ParseError>
where
    N: Offer,
{
    nodes.iter_mut().try_for_each(|node| node.offer(pass))
}

/// Puts a claimed node into `kind`. A claim that fails leaves the plain
/// node, or fails a strict parse.
fn settle<T>(
    kind: &mut Kind<T>,
    claimed: Option<Result<T::Node<T>, ParseError>>,
    pass: &Pass<'_>,
) -> Result<(), ParseError>
where
    T: Extension,
{
    match claimed {
        Some(Ok(node)) => {
            kind.0 = Some(Box::new(node));
            Ok(())
        }
        Some(Err(error)) if pass.context.strict => Err(error),
        None | Some(Err(_)) => Ok(()),
    }
}
