//! The claim pass: every node of a built tree offered to the extensions,
//! children before parents.

use crate::parser::ast::{
    Block, Code, CodeBlock, Document, Heading, Image, Inline, Item, Kind, Line, Link, List,
    MarkdownBlock, Paragraph, Section, Table, TableCell, TableRow, Text,
};
use crate::parser::error::ParseError;
use crate::parser::extension::{Construct, Context, Extension};

/// A node that offers its children, then itself, to the extensions.
trait Offer {
    fn offer(&mut self, context: &Context<'_>) -> Result<(), ParseError>;
}

impl<T> Offer for Block<T>
where
    T: Extension,
{
    fn offer(&mut self, context: &Context<'_>) -> Result<(), ParseError> {
        match self {
            Self::CodeBlock(code_block) => code_block.offer(context),
            Self::List(list) => list.offer(context),
            Self::Paragraph(paragraph) => paragraph.offer(context),
            Self::Table(table) => table.offer(context),
        }
    }
}

impl<T> Offer for Code<T>
where
    T: Extension,
{
    fn offer(&mut self, context: &Context<'_>) -> Result<(), ParseError> {
        let claimed = T::claim(Construct::Code(self), &located(context, self.span.start));
        settle(&mut self.kind, claimed, context)
    }
}

impl<T> Offer for CodeBlock<T>
where
    T: Extension,
{
    fn offer(&mut self, context: &Context<'_>) -> Result<(), ParseError> {
        let claimed = T::claim(
            Construct::CodeBlock(self),
            &located(context, self.span.start),
        );
        settle(&mut self.kind, claimed, context)
    }
}

impl<T> Offer for Document<T>
where
    T: Extension,
{
    fn offer(&mut self, context: &Context<'_>) -> Result<(), ParseError> {
        self.body.offer(context)?;
        let claimed = T::claim(
            Construct::Document(self),
            &located(context, self.span.start),
        );
        settle(&mut self.kind, claimed, context)
    }
}

impl<T> Offer for Heading<T>
where
    T: Extension,
{
    fn offer(&mut self, context: &Context<'_>) -> Result<(), ParseError> {
        each(&mut self.children, context)?;
        let claimed = T::claim(Construct::Heading(self), &located(context, self.span.start));
        settle(&mut self.kind, claimed, context)
    }
}

impl<T> Offer for Image<T>
where
    T: Extension,
{
    fn offer(&mut self, context: &Context<'_>) -> Result<(), ParseError> {
        let claimed = T::claim(Construct::Image(self), &located(context, self.span.start));
        settle(&mut self.kind, claimed, context)
    }
}

impl<T> Offer for Inline<T>
where
    T: Extension,
{
    fn offer(&mut self, context: &Context<'_>) -> Result<(), ParseError> {
        match self {
            Self::Code(code) => code.offer(context),
            Self::Image(image) => image.offer(context),
            Self::Link(link) => link.offer(context),
            Self::Text(text) => text.offer(context),
        }
    }
}

impl<T> Offer for Item<T>
where
    T: Extension,
{
    fn offer(&mut self, context: &Context<'_>) -> Result<(), ParseError> {
        self.body.offer(context)?;
        let claimed = T::claim(Construct::Item(self), &located(context, self.span.start));
        settle(&mut self.kind, claimed, context)
    }
}

impl<T> Offer for Line<T>
where
    T: Extension,
{
    fn offer(&mut self, context: &Context<'_>) -> Result<(), ParseError> {
        each(&mut self.children, context)?;
        let claimed = T::claim(Construct::Line(self), &located(context, self.span.start));
        settle(&mut self.kind, claimed, context)
    }
}

impl<T> Offer for Link<T>
where
    T: Extension,
{
    fn offer(&mut self, context: &Context<'_>) -> Result<(), ParseError> {
        each(&mut self.children, context)?;
        let claimed = T::claim(Construct::Link(self), &located(context, self.span.start));
        settle(&mut self.kind, claimed, context)
    }
}

impl<T> Offer for List<T>
where
    T: Extension,
{
    fn offer(&mut self, context: &Context<'_>) -> Result<(), ParseError> {
        each(&mut self.items, context)?;
        let claimed = T::claim(Construct::List(self), &located(context, self.span.start));
        settle(&mut self.kind, claimed, context)
    }
}

impl<T> Offer for MarkdownBlock<T>
where
    T: Extension,
{
    fn offer(&mut self, context: &Context<'_>) -> Result<(), ParseError> {
        each(&mut self.sections, context)?;
        let claimed = T::claim(
            Construct::MarkdownBlock(self),
            &located(context, self.span.start),
        );
        settle(&mut self.kind, claimed, context)
    }
}

impl<T> Offer for Paragraph<T>
where
    T: Extension,
{
    fn offer(&mut self, context: &Context<'_>) -> Result<(), ParseError> {
        each(&mut self.lines, context)?;
        let claimed = T::claim(
            Construct::Paragraph(self),
            &located(context, self.span.start),
        );
        settle(&mut self.kind, claimed, context)
    }
}

impl<T> Offer for Section<T>
where
    T: Extension,
{
    fn offer(&mut self, context: &Context<'_>) -> Result<(), ParseError> {
        self.heading.offer(context)?;
        each(&mut self.blocks, context)?;
        each(&mut self.subsections, context)?;
        let claimed = T::claim(Construct::Section(self), &located(context, self.span.start));
        settle(&mut self.kind, claimed, context)
    }
}

impl<T> Offer for Table<T>
where
    T: Extension,
{
    fn offer(&mut self, context: &Context<'_>) -> Result<(), ParseError> {
        each(&mut self.head, context)?;
        each(&mut self.rows, context)?;
        let claimed = T::claim(Construct::Table(self), &located(context, self.span.start));
        settle(&mut self.kind, claimed, context)
    }
}

impl<T> Offer for TableCell<T>
where
    T: Extension,
{
    fn offer(&mut self, context: &Context<'_>) -> Result<(), ParseError> {
        self.body.offer(context)?;
        let claimed = T::claim(
            Construct::TableCell(self),
            &located(context, self.span.start),
        );
        settle(&mut self.kind, claimed, context)
    }
}

impl<T> Offer for TableRow<T>
where
    T: Extension,
{
    fn offer(&mut self, context: &Context<'_>) -> Result<(), ParseError> {
        each(&mut self.cells, context)?;
        let claimed = T::claim(
            Construct::TableRow(self),
            &located(context, self.span.start),
        );
        settle(&mut self.kind, claimed, context)
    }
}

impl<T> Offer for Text<T>
where
    T: Extension,
{
    fn offer(&mut self, context: &Context<'_>) -> Result<(), ParseError> {
        let claimed = T::claim(Construct::Text(self), &located(context, self.span.start));
        settle(&mut self.kind, claimed, context)
    }
}

/// Offers every node of `document`, the document last.
pub fn document<T>(document: &mut Document<T>, context: &Context<'_>) -> Result<(), ParseError>
where
    T: Extension,
{
    document.offer(context)
}

/// Offers every node of `block`, the block last.
pub fn markdown_block<T>(
    block: &mut MarkdownBlock<T>,
    context: &Context<'_>,
) -> Result<(), ParseError>
where
    T: Extension,
{
    block.offer(context)
}

/// `context` for a node whose span starts at `start`: its `column` is how far
/// `start` is from the start of its line.
fn located<'text>(context: &Context<'text>, start: usize) -> Context<'text> {
    let line_start = context.source[..start]
        .rfind('\n')
        .map_or(0, |newline| newline + 1);
    Context {
        column: start - line_start,
        ..*context
    }
}

fn each<N>(nodes: &mut [N], context: &Context<'_>) -> Result<(), ParseError>
where
    N: Offer,
{
    nodes.iter_mut().try_for_each(|node| node.offer(context))
}

/// Puts a claimed node into `kind`. A claim that fails leaves the plain
/// node, or fails a strict parse.
fn settle<T>(
    kind: &mut Kind<T>,
    claimed: Option<Result<T::Node<T>, ParseError>>,
    context: &Context<'_>,
) -> Result<(), ParseError>
where
    T: Extension,
{
    match claimed {
        Some(Ok(node)) => {
            kind.0 = Some(Box::new(node));
            Ok(())
        }
        Some(Err(error)) if context.strict => Err(error),
        None | Some(Err(_)) => Ok(()),
    }
}
