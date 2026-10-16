//! Turns tokens into an expression tree.

use core::ops::Range;

use crate::error::QueryError;
use crate::field::{Date, DateTime};
use crate::query::lexer::{CmpOp, Kind, Token, lex};

/// Deepest nesting of parentheses and `not`.
const MAX_DEPTH: usize = 64;
/// Most comparisons in one query.
const MAX_TERMS: usize = 256;

/// A parsed query.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    And(Box<Self>, Box<Self>),
    Cmp(Cmp),
    Not(Box<Self>),
    Or(Box<Self>, Box<Self>),
}

/// `fm.PATH OP LITERAL`.
#[derive(Debug, Clone, PartialEq)]
pub struct Cmp {
    pub literal: Literal,
    pub op: CmpOp,
    pub path: Vec<String>,
}

/// The value a field is compared with.
#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Bool(bool),
    Date(Date),
    DateTime(DateTime),
    Float(f64),
    Int(i64),
    Str(String),
}

struct Parser<'tokens> {
    depth: usize,
    end: usize,
    pos: usize,
    terms: usize,
    tokens: &'tokens [Token],
}

impl<'tokens> Parser<'tokens> {
    fn and(&mut self) -> Result<Expr, QueryError> {
        let mut left = self.unary()?;
        while self.eat_ident("and") {
            left = Expr::And(Box::new(left), Box::new(self.unary()?));
        }
        Ok(left)
    }

    fn cmp(&mut self) -> Result<Expr, QueryError> {
        self.terms += 1;
        if self.terms > MAX_TERMS {
            return Err(QueryError::new(
                "too many comparisons in one query (at most 256)",
                self.here(),
            ));
        }
        match self.next() {
            Some(Token {
                kind: Kind::Ident(name),
                ..
            }) if name == "fm" => {}
            other => {
                return Err(QueryError::new(
                    "a comparison starts with `fm.`",
                    self.span_of(other),
                ));
            }
        }
        let mut path = Vec::new();
        while self.eat(&Kind::Dot) {
            match self.next() {
                Some(Token {
                    kind: Kind::Ident(key) | Kind::Str(key),
                    ..
                }) => path.push(key.clone()),
                other => return Err(QueryError::new("expected a key", self.span_of(other))),
            }
        }
        if path.is_empty() {
            return Err(QueryError::new(
                "expected `.` and a key after `fm`",
                self.here(),
            ));
        }
        let op = match self.next() {
            Some(Token {
                kind: Kind::Op(op), ..
            }) => *op,
            other => {
                return Err(QueryError::new(
                    "expected `==`, `!=`, `<`, `<=`, `>`, `>=` or `has`",
                    self.span_of(other),
                ));
            }
        };
        let literal = self.literal()?;
        Ok(Expr::Cmp(Cmp { literal, op, path }))
    }

    fn eat(&mut self, kind: &Kind) -> bool {
        let found = self.peek().is_some_and(|token| &token.kind == kind);
        if found {
            self.pos += 1;
        }
        found
    }

    fn eat_ident(&mut self, word: &str) -> bool {
        let found = self
            .peek()
            .is_some_and(|token| matches!(&token.kind, Kind::Ident(name) if name == word));
        if found {
            self.pos += 1;
        }
        found
    }

    fn here(&self) -> Range<usize> {
        self.peek()
            .map_or(self.end..self.end, |token| token.span.clone())
    }

    fn literal(&mut self) -> Result<Literal, QueryError> {
        let token = self.next();
        match token {
            Some(Token {
                kind: Kind::Str(text),
                ..
            }) => Ok(Literal::Str(text.clone())),
            Some(Token {
                kind: Kind::Ident(word),
                ..
            }) if word == "true" || word == "false" => Ok(Literal::Bool(word == "true")),
            Some(Token {
                kind: Kind::Word(word),
                span,
            }) => word_literal(word).ok_or_else(|| {
                QueryError::new("expected a number, a date or a datetime", span.clone())
            }),
            other => Err(QueryError::new(
                "expected a string, number, date or true/false",
                self.span_of(other),
            )),
        }
    }

    fn nested(&mut self) -> Result<Expr, QueryError> {
        if self.eat_ident("not") {
            return Ok(Expr::Not(Box::new(self.unary()?)));
        }
        self.pos += 1;
        let inner = self.or()?;
        if !self.eat(&Kind::RParen) {
            return Err(QueryError::new("expected `)`", self.here()));
        }
        Ok(inner)
    }

    fn next(&mut self) -> Option<&'tokens Token> {
        let token = self.tokens.get(self.pos);
        if token.is_some() {
            self.pos += 1;
        }
        token
    }

    fn or(&mut self) -> Result<Expr, QueryError> {
        let mut left = self.and()?;
        while self.eat_ident("or") {
            left = Expr::Or(Box::new(left), Box::new(self.and()?));
        }
        Ok(left)
    }

    fn peek(&self) -> Option<&'tokens Token> {
        self.tokens.get(self.pos)
    }

    fn span_of(&self, token: Option<&Token>) -> Range<usize> {
        token.map_or(self.end..self.end, |found| found.span.clone())
    }

    fn unary(&mut self) -> Result<Expr, QueryError> {
        let opens = self.peek().is_some_and(|token| {
            matches!(&token.kind, Kind::LParen)
                || matches!(&token.kind, Kind::Ident(name) if name == "not")
        });
        if !opens {
            return self.cmp();
        }
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            return Err(QueryError::new(
                "query nested too deep (at most 64 levels of `(` and `not`)",
                self.here(),
            ));
        }
        let result = self.nested();
        self.depth -= 1;
        result
    }
}

/// Parses a whole query.
pub fn parse(text: &str) -> Result<Expr, QueryError> {
    let tokens = lex(text)?;
    let mut parser = Parser {
        depth: 0,
        end: text.len(),
        pos: 0,
        terms: 0,
        tokens: &tokens,
    };
    let expr = parser.or()?;
    if let Some(extra) = parser.peek() {
        return Err(QueryError::new(
            "unexpected input after the query",
            extra.span.clone(),
        ));
    }
    Ok(expr)
}

fn word_literal(word: &str) -> Option<Literal> {
    if let Some(datetime) = DateTime::parse(word) {
        return Some(Literal::DateTime(datetime));
    }
    if let Some(date) = Date::parse(word) {
        return Some(Literal::Date(date));
    }
    if let Ok(number) = word.parse::<i64>() {
        return Some(Literal::Int(number));
    }
    word.parse::<f64>().ok().map(Literal::Float)
}
