//! Bounded, typed spreadsheet formulas. Never runs scripts or external links.
mod functions;
mod parser;
#[cfg(test)]
mod tests;
use super::{Worksheet, MAX_COLUMNS, MAX_ROWS};
use parser::{Binary, Expr, Unary};
use std::collections::{HashMap, HashSet};

type Error = &'static str;
type Result<T> = std::result::Result<T, Error>;
const MAX_TEXT: usize = 32_767;
const MAX_RANGE: usize = 10_000;

pub fn cell_address(column: usize, row: usize) -> String {
    let mut n = column + 1;
    let mut letters = Vec::new();
    while n > 0 {
        n -= 1;
        letters.push((b'A' + (n % 26) as u8) as char);
        n /= 26;
    }
    format!(
        "{}{}",
        letters.into_iter().rev().collect::<String>(),
        row + 1
    )
}
pub fn cell_coordinates(address: &str) -> Option<(usize, usize)> {
    let mut bytes = address.bytes().peekable();
    if bytes.peek() == Some(&b'$') {
        bytes.next();
    }
    let mut column = 0usize;
    let mut letters = 0;
    while bytes.peek().is_some_and(u8::is_ascii_alphabetic) {
        let byte = bytes.next()?.to_ascii_uppercase();
        column = column
            .checked_mul(26)?
            .checked_add((byte - b'A' + 1) as usize)?;
        letters += 1;
    }
    if letters == 0 || letters > 3 {
        return None;
    }
    if bytes.peek() == Some(&b'$') {
        bytes.next();
    }
    let mut row = 0usize;
    let mut digits = 0;
    for byte in bytes {
        if !byte.is_ascii_digit() {
            return None;
        }
        row = row.checked_mul(10)?.checked_add((byte - b'0') as usize)?;
        digits += 1;
    }
    (digits > 0 && column > 0 && column <= MAX_COLUMNS && row > 0 && row <= MAX_ROWS)
        .then_some((column.saturating_sub(1), row.saturating_sub(1)))
}

pub fn display_cell(sheet: &Worksheet, address: &str) -> String {
    let raw = sheet.cells.get(address).map(String::as_str).unwrap_or("");
    if !raw.starts_with('=') {
        return raw.strip_prefix('\'').unwrap_or(raw).to_owned();
    }
    match evaluate_cell(sheet, address) {
        Ok(Value::Blank) => "0".into(),
        Ok(value) => value.text(),
        Err(error) => error.into(),
    }
}
fn evaluate_cell(sheet: &Worksheet, address: &str) -> Result<Value> {
    Context {
        sheet,
        visiting: HashSet::new(),
        cache: HashMap::new(),
        budget: 20_000,
    }
    .cell(address, 0)
}

/// Open XML formula caches preserve scalar types; text such as "12" must
/// never be silently converted to a numeric cached value.
pub(super) fn cached_formula(sheet: &Worksheet, address: &str) -> (&'static str, String) {
    match evaluate_cell(sheet, address) {
        Ok(Value::Number(value)) => ("", value.to_string()),
        Ok(Value::Blank) => ("", "0".into()),
        Ok(Value::Bool(value)) => ("b", if value { "1" } else { "0" }.into()),
        Ok(Value::Text(value)) => ("str", value),
        Err("#CYCLE!" | "#LIMIT!") => ("e", "#N/A".into()),
        Err(error) => ("e", error.into()),
    }
}

fn format_number(n: f64) -> String {
    if n == 0.0 {
        return "0".into();
    }
    if n.abs() >= 1e14 || n.abs() < 1e-8 {
        return format!("{n:.8e}");
    }
    let s = format!("{n:.10}");
    s.trim_end_matches('0').trim_end_matches('.').to_owned()
}

#[derive(Clone, Debug, PartialEq)]
enum Value {
    Number(f64),
    Text(String),
    Bool(bool),
    Blank,
}
impl Value {
    fn number(&self) -> Result<f64> {
        match self {
            Self::Number(n) => finite(*n),
            Self::Bool(value) => Ok(u8::from(*value) as f64),
            Self::Blank => Ok(0.0),
            Self::Text(value) => value
                .trim()
                .parse::<f64>()
                .map_err(|_| "#VALUE!")
                .and_then(finite),
        }
    }
    fn boolean(&self) -> Result<bool> {
        match self {
            Self::Text(value) if value.eq_ignore_ascii_case("TRUE") => Ok(true),
            Self::Text(value) if value.eq_ignore_ascii_case("FALSE") => Ok(false),
            Self::Text(_) => Err("#VALUE!"),
            _ => self.number().map(|value| value != 0.0),
        }
    }
    fn text(&self) -> String {
        match self {
            Self::Number(n) => format_number(*n),
            Self::Text(value) => value.clone(),
            Self::Bool(value) => if *value { "TRUE" } else { "FALSE" }.into(),
            Self::Blank => String::new(),
        }
    }
}
fn finite(n: f64) -> Result<f64> {
    if n.is_finite() {
        Ok(n)
    } else {
        Err("#NUM!")
    }
}
fn number(n: f64) -> Result<Value> {
    finite(n).map(Value::Number)
}
fn text(value: String) -> Result<Value> {
    if value.len() <= MAX_TEXT {
        Ok(Value::Text(value))
    } else {
        Err("#LIMIT!")
    }
}

struct Context<'a> {
    sheet: &'a Worksheet,
    visiting: HashSet<String>,
    cache: HashMap<String, Result<Value>>,
    budget: usize,
}
impl Context<'_> {
    fn spend(&mut self, depth: usize) -> Result<()> {
        if depth > 64 || self.budget == 0 {
            return Err("#LIMIT!");
        }
        self.budget -= 1;
        Ok(())
    }
    fn cell(&mut self, address: &str, depth: usize) -> Result<Value> {
        self.spend(depth)?;
        let (column, row) = cell_coordinates(address).ok_or("#REF!")?;
        let address = cell_address(column, row);
        if let Some(value) = self.cache.get(&address) {
            return value.clone();
        }
        if !self.visiting.insert(address.clone()) {
            return Err("#CYCLE!");
        }
        let raw = self
            .sheet
            .cells
            .get(&address)
            .map(String::as_str)
            .unwrap_or("");
        let value = if let Some(expression) = raw.strip_prefix('=') {
            parser::parse(expression).and_then(|expression| self.eval(&expression, depth + 1))
        } else if let Some(literal) = raw.strip_prefix('\'') {
            text(literal.into())
        } else if raw.is_empty() {
            Ok(Value::Blank)
        } else if raw.eq_ignore_ascii_case("TRUE") || raw.eq_ignore_ascii_case("FALSE") {
            Ok(Value::Bool(raw.eq_ignore_ascii_case("TRUE")))
        } else if let Ok(value) = raw.trim().parse::<f64>() {
            number(value)
        } else {
            text(raw.into())
        };
        self.visiting.remove(&address);
        // Cache scalar results, but do not retain large strings for thousands
        // of cell references in a single evaluation.
        if !matches!(&value, Ok(Value::Text(value)) if value.len() > 256) {
            self.cache.insert(address, value.clone());
        }
        value
    }
    fn eval(&mut self, expr: &Expr, depth: usize) -> Result<Value> {
        self.spend(depth)?;
        match expr {
            Expr::Literal(value) => Ok(value.clone()),
            Expr::Error(error) => Err(error),
            Expr::Cell(address) => self.cell(address, depth + 1),
            Expr::Range(..) => Err("#VALUE!"),
            Expr::Unary(operation, value) => {
                let value = self.eval(value, depth + 1)?.number()?;
                number(match operation {
                    Unary::Plus => value,
                    Unary::Minus => -value,
                    Unary::Percent => value / 100.0,
                })
            }
            Expr::Chain(first, operations) => {
                // Flat arithmetic stays iterative; an 8 KiB sum must not
                // consume one stack frame for every '+' operator.
                let mut value = self.eval(first, depth + 1)?;
                for (operation, right) in operations {
                    value = binary(*operation, value, self.eval(right, depth + 1)?)?;
                }
                Ok(value)
            }
            Expr::Call(name, args) => self.function(name, args, depth + 1),
        }
    }
}

fn binary(operation: Binary, left: Value, right: Value) -> Result<Value> {
    match operation {
        Binary::Concat => text(left.text() + &right.text()),
        Binary::Eq | Binary::Ne | Binary::Lt | Binary::Le | Binary::Gt | Binary::Ge => {
            let comparison = compare(&left, &right)?;
            Ok(Value::Bool(match operation {
                Binary::Eq => comparison.is_eq(),
                Binary::Ne => !comparison.is_eq(),
                Binary::Lt => comparison.is_lt(),
                Binary::Le => !comparison.is_gt(),
                Binary::Gt => comparison.is_gt(),
                Binary::Ge => !comparison.is_lt(),
                _ => unreachable!(),
            }))
        }
        _ => {
            let (a, b) = (left.number()?, right.number()?);
            number(match operation {
                Binary::Add => a + b,
                Binary::Sub => a - b,
                Binary::Mul => a * b,
                Binary::Div if b == 0.0 => return Err("#DIV/0!"),
                Binary::Div => a / b,
                Binary::Power => a.powf(b),
                _ => unreachable!(),
            })
        }
    }
}

fn compare(a: &Value, b: &Value) -> Result<std::cmp::Ordering> {
    use std::cmp::Ordering;
    match (a, b) {
        (Value::Blank, Value::Blank) => Ok(Ordering::Equal),
        (Value::Blank, Value::Text(text)) => Ok("".cmp(&text.to_lowercase())),
        (Value::Text(text), Value::Blank) => Ok(text.to_lowercase().as_str().cmp("")),
        (Value::Text(a), Value::Text(b)) => Ok(a.to_lowercase().cmp(&b.to_lowercase())),
        (Value::Blank, other) => compare(&Value::Number(0.0), other),
        (other, Value::Blank) => compare(other, &Value::Number(0.0)),
        (Value::Number(a), Value::Number(b)) => a.partial_cmp(b).ok_or("#NUM!"),
        (Value::Bool(a), Value::Bool(b)) => Ok(a.cmp(b)),
        (Value::Number(_), _) | (Value::Text(_), Value::Bool(_)) => Ok(Ordering::Less),
        (_, Value::Number(_)) | (Value::Bool(_), Value::Text(_)) => Ok(Ordering::Greater),
    }
}
