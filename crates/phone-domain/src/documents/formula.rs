//! Deliberately bounded spreadsheet arithmetic; never executes script or external links.
use super::{MAX_COLUMNS, MAX_ROWS, Worksheet};
use std::collections::HashSet;

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
    let normalized = address.replace('$', "").to_ascii_uppercase();
    let split = normalized.bytes().position(|b| b.is_ascii_digit())?;
    if split == 0 || split > 3 {
        return None;
    }
    let mut column = 0usize;
    for b in normalized[..split].bytes() {
        if !b.is_ascii_uppercase() {
            return None;
        }
        column = column
            .checked_mul(26)?
            .checked_add((b - b'A' + 1) as usize)?;
    }
    let row: usize = normalized[split..].parse().ok()?;
    (column > 0 && column <= MAX_COLUMNS && row > 0 && row <= MAX_ROWS)
        .then_some((column - 1, row - 1))
}

pub fn display_cell(sheet: &Worksheet, address: &str) -> String {
    let raw = sheet.cells.get(address).map(String::as_str).unwrap_or("");
    if !raw.starts_with('=') {
        return raw.strip_prefix('\'').unwrap_or(raw).to_owned();
    }
    let mut context = Context {
        sheet,
        visiting: HashSet::new(),
        budget: 20_000,
    };
    match context.cell(address, 0) {
        Ok(n) => format_number(n),
        Err(error) => error.into(),
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

struct Context<'a> {
    sheet: &'a Worksheet,
    visiting: HashSet<String>,
    budget: usize,
}
impl Context<'_> {
    fn cell(&mut self, address: &str, depth: usize) -> Result<f64, &'static str> {
        if depth > 64 || self.budget == 0 {
            return Err("#LIMIT!");
        }
        self.budget -= 1;
        let (c, r) = cell_coordinates(address).ok_or("#REF!")?;
        let address = cell_address(c, r);
        if !self.visiting.insert(address.clone()) {
            return Err("#CYCLE!");
        }
        let value = self.sheet.cells.get(&address).cloned().unwrap_or_default();
        let result = if let Some(expression) = value.strip_prefix('=') {
            Parser {
                input: expression.as_bytes(),
                position: 0,
                context: self,
                depth: depth + 1,
            }
            .finish()
        } else if value == "TRUE" {
            Ok(1.0)
        } else if value == "FALSE" {
            Ok(0.0)
        } else if value.trim().is_empty() {
            Ok(0.0)
        } else {
            value.trim().parse::<f64>().map_err(|_| "#VALUE!")
        };
        self.visiting.remove(&address);
        result.and_then(finite)
    }
}
fn finite(n: f64) -> Result<f64, &'static str> {
    if n.is_finite() { Ok(n) } else { Err("#NUM!") }
}

struct Parser<'a, 'b, 'c> {
    input: &'a [u8],
    position: usize,
    context: &'b mut Context<'c>,
    depth: usize,
}
impl Parser<'_, '_, '_> {
    fn finish(&mut self) -> Result<f64, &'static str> {
        if self.input.len() > 8192 {
            return Err("#LIMIT!");
        }
        let value = self.expression()?;
        self.space();
        if self.position != self.input.len() {
            return Err("#NAME?");
        }
        finite(value)
    }
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
    fn expression(&mut self) -> Result<f64, &'static str> {
        let mut left = self.product()?;
        loop {
            if self.take(b'+') {
                left += self.product()?;
            } else if self.take(b'-') {
                left -= self.product()?;
            } else {
                break;
            }
        }
        finite(left)
    }
    fn product(&mut self) -> Result<f64, &'static str> {
        let mut left = self.power()?;
        loop {
            if self.take(b'*') {
                left *= self.power()?;
            } else if self.take(b'/') {
                let right = self.power()?;
                if right == 0.0 {
                    return Err("#DIV/0!");
                }
                left /= right;
            } else {
                break;
            }
        }
        finite(left)
    }
    fn power(&mut self) -> Result<f64, &'static str> {
        let mut left = self.atom()?;
        if self.take(b'^') {
            self.depth += 1;
            if self.depth > 64 {
                return Err("#LIMIT!");
            }
            left = left.powf(self.power()?);
            self.depth -= 1;
        }
        if self.take(b'%') {
            left /= 100.0;
        }
        finite(left)
    }
    fn atom(&mut self) -> Result<f64, &'static str> {
        self.space();
        self.depth += 1;
        if self.depth > 64 {
            return Err("#LIMIT!");
        }
        let result = self.atom_inner();
        self.depth -= 1;
        result
    }
    fn atom_inner(&mut self) -> Result<f64, &'static str> {
        if self.take(b'+') {
            return self.atom();
        }
        if self.take(b'-') {
            return self.atom().map(|n| -n);
        }
        if self.take(b'(') {
            let v = self.expression()?;
            if !self.take(b')') {
                return Err("#VALUE!");
            }
            return Ok(v);
        }
        let start = self.position;
        if self
            .input
            .get(start)
            .is_some_and(|b| b.is_ascii_digit() || *b == b'.')
        {
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
            return std::str::from_utf8(&self.input[start..self.position])
                .map_err(|_| "#VALUE!")?
                .parse()
                .map_err(|_| "#VALUE!");
        }
        let name = self.identifier()?;
        if self.take(b'(') {
            self.function(&name)
        } else {
            self.context.cell(&name, self.depth)
        }
    }
    fn identifier(&mut self) -> Result<String, &'static str> {
        self.space();
        let start = self.position;
        while self
            .input
            .get(self.position)
            .is_some_and(|b| b.is_ascii_alphanumeric() || *b == b'$' || *b == b'_')
        {
            self.position += 1;
        }
        if self.position == start {
            return Err("#VALUE!");
        }
        Ok(String::from_utf8_lossy(&self.input[start..self.position]).to_ascii_uppercase())
    }
    fn function(&mut self, name: &str) -> Result<f64, &'static str> {
        if !["SUM", "AVERAGE", "MIN", "MAX", "COUNT", "ABS", "ROUND"].contains(&name) {
            return Err("#NAME?");
        }
        let mut values = Vec::new();
        if !self.take(b')') {
            loop {
                let start = self.position;
                if let Ok(first) = self.identifier() {
                    if self.take(b':') {
                        let last = self.identifier()?;
                        let (c1, r1) = cell_coordinates(&first).ok_or("#REF!")?;
                        let (c2, r2) = cell_coordinates(&last).ok_or("#REF!")?;
                        let count = (c1.abs_diff(c2) + 1) * (r1.abs_diff(r2) + 1);
                        if count > 10_000 || values.len() + count > 10_000 {
                            return Err("#LIMIT!");
                        }
                        for row in r1.min(r2)..=r1.max(r2) {
                            for col in c1.min(c2)..=c1.max(c2) {
                                let address = cell_address(col, row);
                                let raw = self
                                    .context
                                    .sheet
                                    .cells
                                    .get(&address)
                                    .map(String::as_str)
                                    .unwrap_or("");
                                if raw.trim().is_empty()
                                    || (!raw.starts_with('=') && raw.parse::<f64>().is_err())
                                {
                                    continue;
                                }
                                values.push(self.context.cell(&address, self.depth)?);
                            }
                        }
                    } else {
                        self.position = start;
                        values.push(self.expression()?);
                    }
                } else {
                    self.position = start;
                    values.push(self.expression()?);
                }
                if values.len() > 10_000 {
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
        match name {
            "SUM" => finite(values.iter().sum()),
            "COUNT" => Ok(values.len() as f64),
            "AVERAGE" => {
                if values.is_empty() {
                    Err("#DIV/0!")
                } else {
                    finite(values.iter().sum::<f64>() / values.len() as f64)
                }
            }
            "MIN" => Ok(values.into_iter().reduce(f64::min).unwrap_or(0.0)),
            "MAX" => Ok(values.into_iter().reduce(f64::max).unwrap_or(0.0)),
            "ABS" if values.len() == 1 => Ok(values[0].abs()),
            "ROUND" if values.len() == 2 && values[1].abs() <= 15.0 => {
                let scale = 10f64.powi(values[1].trunc() as i32);
                finite((values[0] * scale).round() / scale)
            }
            _ => Err("#VALUE!"),
        }
    }
}
