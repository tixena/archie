//! Inline nodes inside the text lines of a paragraph or a heading.

use core::mem;
use core::ops::Range;

use crate::parser::ast::{Code, Image, Inline, Link, LinkForm, Text};

/// Where the parts of a `[text](destination)` are.
struct Bracketed {
    close: usize,
    destination: Range<usize>,
    text: Range<usize>,
}

/// Where the parts of a `[[target|alias]]` are.
struct Wiki {
    alias: Option<Range<usize>>,
    close: usize,
    target: Range<usize>,
}

struct Scanner<'src> {
    /// Off inside the text of a link, where another link cannot start.
    links: bool,
    nodes: Vec<Inline>,
    source: &'src str,
    text: String,
    text_end: usize,
    text_start: Option<usize>,
}

impl<'src> Scanner<'src> {
    /// An image, or a `!` that starts none.
    fn bang(&mut self, at: usize, end: usize) -> usize {
        if let Some((image, close)) = image_at(self.source, at, end) {
            self.flush();
            self.nodes.push(Inline::Image(image));
            return close;
        }
        let source = self.source;
        self.push_text(at..at + 1, &source[at..=at]);
        at + 1
    }

    /// A link, or a `[` that starts none.
    fn bracket(&mut self, at: usize, end: usize) -> usize {
        if let Some(close) = self.link(at, end) {
            return close;
        }
        let source = self.source;
        self.push_text(at..at + 1, &source[at..=at]);
        at + 1
    }

    /// A code span closes at the next run of exactly as many backticks.
    fn code(&mut self, at: usize, end: usize) -> usize {
        let bytes = self.source.as_bytes();
        let ticks = run(bytes, at, end);
        let content = at + ticks;
        let mut close = content;
        while close < end {
            if bytes[close] != b'`' {
                close += 1;
                continue;
            }
            let found = run(bytes, close, end);
            if found == ticks {
                self.flush();
                self.nodes.push(Inline::Code(Code {
                    span: at..close + ticks,
                    value: self.source[content..close].to_owned(),
                }));
                return close + ticks;
            }
            close += found;
        }
        let source = self.source;
        self.push_text(at..content, &source[at..content]);
        content
    }

    /// A backslash before ASCII punctuation makes that character plain text.
    fn escape(&mut self, at: usize, end: usize) -> usize {
        let escaped = at + 1 < end && self.source.as_bytes()[at + 1].is_ascii_punctuation();
        let (from, to) = if escaped {
            (at + 1, at + 2)
        } else {
            (at, at + 1)
        };
        let source = self.source;
        self.push_text(at..to, &source[from..to]);
        to
    }

    fn finish(mut self) -> Vec<Inline> {
        self.flush();
        self.nodes
    }

    fn flush(&mut self) {
        if let Some(start) = self.text_start.take() {
            self.nodes.push(Inline::Text(Text {
                span: start..self.text_end,
                value: mem::take(&mut self.text),
            }));
        }
    }

    fn line(&mut self, range: Range<usize>) {
        let bytes = self.source.as_bytes();
        let mut at = range.start;
        while at < range.end {
            at = match bytes[at] {
                b'\\' => self.escape(at, range.end),
                b'`' => self.code(at, range.end),
                b'!' => self.bang(at, range.end),
                b'[' if self.links => self.bracket(at, range.end),
                _ => self.plain(at, range.end),
            };
        }
    }

    fn link(&mut self, at: usize, end: usize) -> Option<usize> {
        let source = self.source;
        if let Some(wiki) = wiki_at(source, at, end) {
            let children = wiki.alias.map_or_else(Vec::new, |alias| {
                vec![Inline::Text(Text {
                    value: source[alias.clone()].to_owned(),
                    span: alias,
                })]
            });
            self.flush();
            self.nodes.push(Inline::Link(Link {
                children,
                form: LinkForm::Wiki,
                span: at..wiki.close,
                target: source[wiki.target].to_owned(),
            }));
            return Some(wiki.close);
        }
        let bracketed = bracketed_at(source, at, end)?;
        let mut inner = Scanner::new(source, false);
        inner.line(bracketed.text);
        self.flush();
        self.nodes.push(Inline::Link(Link {
            children: inner.finish(),
            form: LinkForm::Markdown,
            span: at..bracketed.close,
            target: source[bracketed.destination].to_owned(),
        }));
        Some(bracketed.close)
    }

    const fn new(source: &'src str, links: bool) -> Self {
        Self {
            links,
            nodes: Vec::new(),
            source,
            text: String::new(),
            text_end: 0,
            text_start: None,
        }
    }

    /// Text up to the next character that can start another node.
    fn plain(&mut self, at: usize, end: usize) -> usize {
        let bytes = self.source.as_bytes();
        let mut next = at + 1;
        while next < end && !matches!(bytes[next], b'\\' | b'`' | b'[' | b'!') {
            next += 1;
        }
        let source = self.source;
        self.push_text(at..next, &source[at..next]);
        next
    }

    fn push_text(&mut self, range: Range<usize>, value: &str) {
        if self.text_start.is_none() {
            self.text_start = Some(range.start);
        }
        self.text_end = range.end;
        self.text.push_str(value);
    }
}

/// The inline nodes of consecutive text lines. A line break between two
/// lines is a `\n` in the text.
pub fn inlines(source: &str, lines: &[Range<usize>]) -> Vec<Inline> {
    let mut scanner = Scanner::new(source, true);
    let mut previous_end: Option<usize> = None;
    for line in lines {
        if let Some(end) = previous_end {
            scanner.push_text(end..line.start, "\n");
        }
        scanner.line(line.clone());
        previous_end = Some(line.end);
    }
    scanner.finish()
}

/// `[[target]]` or `[[target|alias]]` starting at `at`. Neither part holds a
/// bracket.
fn wiki_at(source: &str, at: usize, end: usize) -> Option<Wiki> {
    let inner = source[at..end].strip_prefix("[[")?;
    let length = inner.find("]]")?;
    let written = &inner[..length];
    if written.is_empty() || written.contains(['[', ']']) {
        return None;
    }
    let start = at + 2;
    let (target, alias) = written
        .find('|')
        .map_or((start..start + length, None), |bar| {
            (start..start + bar, Some(start + bar + 1..start + length))
        });
    Some(Wiki {
        alias: alias.filter(|range| !range.is_empty()),
        close: start + length + 2,
        target,
    })
}

/// `[text](destination)` starting at `at`. The text holds no bracket except
/// inside an image; the destination holds no space and no parenthesis.
fn bracketed_at(source: &str, at: usize, end: usize) -> Option<Bracketed> {
    let bytes = source.as_bytes();
    if bytes.get(at) != Some(&b'[') {
        return None;
    }
    let mut text_end = at + 1;
    while text_end < end && bytes[text_end] != b']' {
        text_end = match bytes[text_end] {
            b'\\' => text_end + 2,
            b'!' => image_at(source, text_end, end).map_or(text_end + 1, |(_, close)| close),
            b'[' => return None,
            _ => text_end + 1,
        };
    }
    if text_end + 1 >= end || bytes[text_end + 1] != b'(' {
        return None;
    }
    let destination_start = text_end + 2;
    let length = source[destination_start..end].find(')')?;
    let destination = destination_start..destination_start + length;
    if source[destination.clone()].contains([' ', '\t', '(']) {
        return None;
    }
    Some(Bracketed {
        close: destination.end + 1,
        destination,
        text: at + 1..text_end,
    })
}

/// An image starting at the `!` at `at`, and where it ends.
fn image_at(source: &str, at: usize, end: usize) -> Option<(Image, usize)> {
    let open = at + 1;
    if let Some(wiki) = wiki_at(source, open, end) {
        let image = Image {
            alt: wiki
                .alias
                .map_or_else(String::new, |alias| source[alias].to_owned()),
            form: LinkForm::Wiki,
            span: at..wiki.close,
            target: source[wiki.target].to_owned(),
        };
        return Some((image, wiki.close));
    }
    let bracketed = bracketed_at(source, open, end)?;
    let image = Image {
        alt: source[bracketed.text].to_owned(),
        form: LinkForm::Markdown,
        span: at..bracketed.close,
        target: source[bracketed.destination].to_owned(),
    };
    Some((image, bracketed.close))
}

/// The length of the run of backticks starting at `at`.
fn run(bytes: &[u8], at: usize, end: usize) -> usize {
    bytes[at..end]
        .iter()
        .take_while(|&&byte| byte == b'`')
        .count()
}
