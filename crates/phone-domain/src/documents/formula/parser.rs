use super::{cell_coordinates, number, Error, Result, Value};
#[derive(Clone, Copy)]
pub(super) enum Unary {
    Plus,
    Minus,
    Percent,
}
#[derive(Clone, Copy)]
pub(super) enum Binary {
    Add,
    Sub,
    Mul,
    Div,
    Power,
    Concat,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}
pub(super) enum Expr {
    Literal(Value),
    Error(Error),
    Cell(String),
    Range((usize, usize), (usize, usize)),
    Unary(Unary, Box<Expr>),
    Chain(Box<Expr>, Vec<(Binary, Expr)>),
    Call(String, Vec<Expr>),
}

pub(super) fn parse(input: &str) -> Result<Expr> {
    if input.len() > 8192 {
        return Err("#LIMIT!");
    }
    let mut parser = Parser {
        input: input.as_bytes(),
        position: 0,
        nodes: 0,
    };
    let expression = parser.expression(0, 0)?;
    parser.space();
    if parser.position != parser.input.len() {
        return Err("#NAME?");
    }
    Ok(expression)
}
struct Parser<'a> {
    input: &'a [u8],
    position: usize,
    nodes: usize,
}
impl Parser<'_> {
    fn space(&mut self) {
        while self
            .input
            .get(self.position)
            .is_some_and(u8::is_ascii_whitespace)
        {
            self.position += 1;
        }
    }
    fn take(&mut self, byte: u8) -> bool {
        self.space();
        if self.input.get(self.position) == Some(&byte) {
            self.position += 1;
            true
        } else {
            false
        }
    }
    fn expression(&mut self, minimum: u8, depth: usize) -> Result<Expr> {
        if depth > 64 {
            return Err("#LIMIT!");
        }
        let left = self.atom(depth + 1)?;
        let mut operations = Vec::new();
        loop {
            self.space();
            let tail = &self.input[self.position..];
            let (operation, precedence, length) = match tail {
                [b'<', b'=', ..] => (Binary::Le, 1, 2),
                [b'>', b'=', ..] => (Binary::Ge, 1, 2),
                [b'<', b'>', ..] => (Binary::Ne, 1, 2),
                [b'=', ..] => (Binary::Eq, 1, 1),
                [b'<', ..] => (Binary::Lt, 1, 1),
                [b'>', ..] => (Binary::Gt, 1, 1),
                [b'&', ..] => (Binary::Concat, 2, 1),
                [b'+', ..] => (Binary::Add, 3, 1),
                [b'-', ..] => (Binary::Sub, 3, 1),
                [b'*', ..] => (Binary::Mul, 4, 1),
                [b'/', ..] => (Binary::Div, 4, 1),
                [b'^', ..] => (Binary::Power, 5, 1),
                _ => break,
            };
            if precedence < minimum {
                break;
            }
            self.position += length;
            let right = self.expression(precedence + 1, depth + 1)?;
            operations.push((operation, right));
        }
        if operations.is_empty() {
            Ok(left)
        } else {
            self.node(Expr::Chain(Box::new(left), operations))
        }
    }
    fn node(&mut self, expression: Expr) -> Result<Expr> {
        self.nodes += 1;
        if self.nodes > 8192 {
            Err("#LIMIT!")
        } else {
            Ok(expression)
        }
    }
    fn atom(&mut self, depth: usize) -> Result<Expr> {
        if depth > 64 {
            return Err("#LIMIT!");
        }
        self.space();
        let mut value = if self.take(b'+') {
            Expr::Unary(Unary::Plus, Box::new(self.atom(depth + 1)?))
        } else if self.take(b'-') {
            Expr::Unary(Unary::Minus, Box::new(self.atom(depth + 1)?))
        } else if self.take(b'(') {
            let result = self.expression(0, depth + 1)?;
            if !self.take(b')') {
                return Err("#VALUE!");
            }
            result
        } else if self.take(b'"') {
            let mut value = Vec::new();
            loop {
                let byte = *self.input.get(self.position).ok_or("#VALUE!")?;
                self.position += 1;
                if byte == b'"' {
                    if self.input.get(self.position) == Some(&b'"') {
                        self.position += 1;
                    } else {
                        break;
                    }
                }
                value.push(byte);
            }
            Expr::Literal(Value::Text(
                String::from_utf8(value).map_err(|_| "#VALUE!")?,
            ))
        } else if self
            .input
            .get(self.position)
            .is_some_and(|b| b.is_ascii_digit() || *b == b'.')
        {
            let start = self.position;
            while self
                .input
                .get(self.position)
                .is_some_and(|b| b.is_ascii_digit() || *b == b'.')
            {
                self.position += 1;
            }
            if self
                .input
                .get(self.position)
                .is_some_and(|b| *b == b'e' || *b == b'E')
            {
                self.position += 1;
                if self
                    .input
                    .get(self.position)
                    .is_some_and(|b| *b == b'+' || *b == b'-')
                {
                    self.position += 1;
                }
                while self
                    .input
                    .get(self.position)
                    .is_some_and(u8::is_ascii_digit)
                {
                    self.position += 1;
                }
            }
            let raw =
                std::str::from_utf8(&self.input[start..self.position]).map_err(|_| "#VALUE!")?;
            Expr::Literal(number(raw.parse().map_err(|_| "#VALUE!")?)?)
        } else {
            let name = self.identifier()?;
            if self.take(b'(') {
                let mut args = Vec::new();
                if !self.take(b')') {
                    loop {
                        args.push(self.expression(0, depth + 1)?);
                        if args.len() > 255 {
                            return Err("#LIMIT!");
                        }
                        if self.take(b')') {
                            break;
                        }
                        if !self.take(b',') && !self.take(b';') {
                            return Err("#VALUE!");
                        }
                    }
                }
                Expr::Call(name, args)
            } else if self.take(b':') {
                let last = self.identifier()?;
                Expr::Range(
                    cell_coordinates(&name).ok_or("#REF!")?,
                    cell_coordinates(&last).ok_or("#REF!")?,
                )
            } else if name == "TRUE" || name == "FALSE" {
                Expr::Literal(Value::Bool(name == "TRUE"))
            } else if cell_coordinates(&name).is_some() {
                Expr::Cell(name)
            } else {
                Expr::Error(if name.bytes().any(|b| b.is_ascii_digit()) {
                    "#REF!"
                } else {
                    "#NAME?"
                })
            }
        };
        while self.take(b'%') {
            value = self.node(Expr::Unary(Unary::Percent, Box::new(value)))?;
        }
        self.node(value)
    }
    fn identifier(&mut self) -> std::result::Result<String, Error> {
        self.space();
        let start = self.position;
        while self
            .input
            .get(self.position)
            .is_some_and(|b| b.is_ascii_alphanumeric() || *b == b'$' || *b == b'_')
        {
            self.position += 1;
        }
        if start == self.position {
            return Err("#VALUE!");
        }
        Ok(String::from_utf8_lossy(&self.input[start..self.position]).to_ascii_uppercase())
    }
}
