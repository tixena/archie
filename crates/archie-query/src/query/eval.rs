//! Evaluates a parsed query against a resource.

use core::cmp::Ordering;

use crate::field::{Date, DateTime, Field};
use crate::query::lexer::CmpOp;
use crate::query::parse::{Cmp, Expr, Literal};
use crate::resource::Resource;

/// Whether `resource` satisfies `expr`.
pub fn eval(expr: &Expr, resource: &Resource) -> bool {
    match expr {
        Expr::And(left, right) => eval(left, resource) && eval(right, resource),
        Expr::Cmp(cmp) => compare(cmp, resource),
        Expr::Not(inner) => !eval(inner, resource),
        Expr::Or(left, right) => eval(left, resource) || eval(right, resource),
    }
}

fn compare(cmp: &Cmp, resource: &Resource) -> bool {
    let path: Vec<&str> = cmp.path.iter().map(String::as_str).collect();
    resource.field(&path).map_or(cmp.op == CmpOp::Ne, |field| {
        let order = order(field, &cmp.literal);
        match cmp.op {
            CmpOp::Eq => order == Some(Ordering::Equal),
            CmpOp::Ge => matches!(order, Some(Ordering::Greater | Ordering::Equal)),
            CmpOp::Gt => order == Some(Ordering::Greater),
            CmpOp::Has => has(field, &cmp.literal),
            CmpOp::Le => matches!(order, Some(Ordering::Less | Ordering::Equal)),
            CmpOp::Lt => order == Some(Ordering::Less),
            CmpOp::Ne => order != Some(Ordering::Equal),
        }
    })
}

fn has(field: &Field, literal: &Literal) -> bool {
    if let Field::List(items) = field {
        return items
            .iter()
            .any(|item| order(item, literal) == Some(Ordering::Equal));
    }
    if let (Field::Str(text), Literal::Str(part)) = (field, literal) {
        return text.contains(part.as_str());
    }
    false
}

/// How `field` compares with `literal`; `None` when they are not comparable.
fn order(field: &Field, literal: &Literal) -> Option<Ordering> {
    match (field, literal) {
        (Field::Bool(left), Literal::Bool(right)) => Some(left.cmp(right)),
        (Field::Float(left), Literal::Float(right)) => left.partial_cmp(right),
        (Field::Float(left), Literal::Int(right)) => {
            int_vs_float(*right, *left).map(Ordering::reverse)
        }
        (Field::Int(left), Literal::Float(right)) => int_vs_float(*left, *right),
        (Field::Int(left), Literal::Int(right)) => Some(left.cmp(right)),
        (Field::Str(left), Literal::Str(right)) => Some(left.as_str().cmp(right.as_str())),
        (_, Literal::Date(_) | Literal::DateTime(_)) => {
            Some(field_instant(field)?.cmp(&literal_instant(literal)?))
        }
        _ => None,
    }
}

/// How `int` compares with `float`, exactly; `None` for NaN.
#[expect(
    clippy::as_conversions,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    reason = "`float` is range-checked first, so the cast to i64 is exact; `i64::MAX as f64` rounds to exactly 2^63"
)]
fn int_vs_float(int: i64, float: f64) -> Option<Ordering> {
    const TWO_POW_63: f64 = i64::MAX as f64;
    const MINUS_TWO_POW_63: f64 = i64::MIN as f64;
    if float.is_nan() {
        return None;
    }
    if float >= TWO_POW_63 {
        return Some(Ordering::Less);
    }
    if float < MINUS_TWO_POW_63 {
        return Some(Ordering::Greater);
    }
    let whole = float.trunc();
    let order = int.cmp(&(whole as i64));
    if order == Ordering::Equal {
        return 0.0_f64.partial_cmp(&float.fract());
    }
    Some(order)
}

/// A date-like field as an instant. A string counts when it is written as a date or datetime.
fn field_instant(field: &Field) -> Option<i128> {
    match field {
        Field::Date(date) => Some(date.instant()),
        Field::DateTime(datetime) => Some(datetime.instant()),
        Field::Str(text) => DateTime::parse(text)
            .map(DateTime::instant)
            .or_else(|| Date::parse(text).map(Date::instant)),
        Field::Bool(_)
        | Field::Float(_)
        | Field::Int(_)
        | Field::List(_)
        | Field::Map(_)
        | Field::Null
        | Field::Time(_) => None,
    }
}

fn literal_instant(literal: &Literal) -> Option<i128> {
    match literal {
        Literal::Date(date) => Some(date.instant()),
        Literal::DateTime(datetime) => Some(datetime.instant()),
        Literal::Bool(_) | Literal::Float(_) | Literal::Int(_) | Literal::Str(_) => None,
    }
}
