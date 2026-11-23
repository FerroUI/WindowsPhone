use super::{cell_address, number, text, Context, Expr, Result, Value, MAX_RANGE};

impl Context<'_> {
    pub(super) fn function(&mut self, name: &str, args: &[Expr], depth: usize) -> Result<Value> {
        self.spend(depth)?;
        match name {
            "IF" if (2..=3).contains(&args.len()) => {
                let condition = self.eval(&args[0], depth)?.boolean()?;
                if condition {
                    self.eval(&args[1], depth)
                } else if args.len() == 3 {
                    self.eval(&args[2], depth)
                } else {
                    Ok(Value::Bool(false))
                }
            }
            "IFERROR" if args.len() == 2 => match self.eval(&args[0], depth) {
                Err(error @ ("#LIMIT!" | "#CYCLE!")) => Err(error),
                Err(_) => self.eval(&args[1], depth),
                value => value,
            },
            "TRUE" if args.is_empty() => Ok(Value::Bool(true)),
            "FALSE" if args.is_empty() => Ok(Value::Bool(false)),
            "PI" if args.is_empty() => number(std::f64::consts::PI),
            "SUM" | "AVERAGE" | "MIN" | "MAX" | "COUNT" | "COUNTA" | "COUNTBLANK" | "PRODUCT"
            | "MEDIAN" | "SUMSQ" => self.aggregate(name, args, depth),
            "AND" | "OR" if !args.is_empty() => {
                let mut seen = false;
                let mut result = name == "AND";
                self.visit(args, depth, |value, reference| {
                    let value = value?;
                    if reference && matches!(value, Value::Text(_) | Value::Blank) {
                        return Ok(());
                    }
                    let value = value.boolean()?;
                    seen = true;
                    result = if name == "AND" {
                        result && value
                    } else {
                        result || value
                    };
                    Ok(())
                })?;
                if seen {
                    Ok(Value::Bool(result))
                } else {
                    Err("#VALUE!")
                }
            }
            "NOT" if args.len() == 1 => Ok(Value::Bool(!self.eval(&args[0], depth)?.boolean()?)),
            "ABS" | "INT" | "SIGN" | "SQRT" | "SIN" | "COS" | "TAN" | "LN" | "LOG10" | "EXP"
                if args.len() == 1 =>
            {
                let value = self.eval(&args[0], depth)?.number()?;
                number(match name {
                    "ABS" => value.abs(),
                    "INT" => value.floor(),
                    "SIGN" => {
                        if value == 0.0 {
                            0.0
                        } else {
                            value.signum()
                        }
                    }
                    "SQRT" => value.sqrt(),
                    "SIN" => value.sin(),
                    "COS" => value.cos(),
                    "TAN" => value.tan(),
                    "LN" => value.ln(),
                    "LOG10" => value.log10(),
                    "EXP" => value.exp(),
                    _ => unreachable!(),
                })
            }
            "ROUND" | "ROUNDUP" | "ROUNDDOWN" | "TRUNC"
                if args.len() == 2 || name == "TRUNC" && args.len() == 1 =>
            {
                let value = self.eval(&args[0], depth)?.number()?;
                let digits = if args.len() == 2 {
                    self.eval(&args[1], depth)?.number()?.trunc()
                } else {
                    0.0
                };
                if digits.abs() > 308.0 {
                    return Err("#NUM!");
                }
                let scale = 10f64.powi(digits as i32);
                if scale == 0.0 {
                    return Err("#NUM!");
                }
                let scaled = super::finite(value * scale)?;
                number(
                    match name {
                        "ROUND" => scaled.round(),
                        "ROUNDUP" => scaled.abs().ceil().copysign(scaled),
                        _ => scaled.trunc(),
                    } / scale,
                )
            }
            "MOD" | "POWER" if args.len() == 2 => {
                let a = self.eval(&args[0], depth)?.number()?;
                let b = self.eval(&args[1], depth)?.number()?;
                if name == "MOD" {
                    if b == 0.0 {
                        return Err("#DIV/0!");
                    }
                    number(a - b * (a / b).floor())
                } else {
                    number(a.powf(b))
                }
            }
            "LOG" if (1..=2).contains(&args.len()) => {
                let value = self.eval(&args[0], depth)?.number()?;
                let base = if args.len() == 2 {
                    self.eval(&args[1], depth)?.number()?
                } else {
                    10.0
                };
                if base <= 0.0 || base == 1.0 || value <= 0.0 {
                    return Err("#NUM!");
                }
                number(value.log(base))
            }
            "LEN" | "LOWER" | "UPPER" | "TRIM" if args.len() == 1 => {
                let value = self.eval(&args[0], depth)?.text();
                match name {
                    // UTF-16 units follow the legacy Excel/Windows Phone model.
                    "LEN" => number(value.encode_utf16().count() as f64),
                    "LOWER" => text(value.to_lowercase()),
                    "UPPER" => text(value.to_uppercase()),
                    _ => text(
                        value
                            .split(' ')
                            .filter(|part| !part.is_empty())
                            .collect::<Vec<_>>()
                            .join(" "),
                    ),
                }
            }
            "LEFT" | "RIGHT" if (1..=2).contains(&args.len()) => {
                let value = self.eval(&args[0], depth)?.text();
                let count = if args.len() == 2 {
                    self.eval(&args[1], depth)?.number()?.trunc()
                } else {
                    1.0
                };
                if count < 0.0 {
                    return Err("#VALUE!");
                }
                let units = value.encode_utf16().collect::<Vec<_>>();
                let count = (count as usize).min(units.len());
                let range = if name == "LEFT" {
                    0..count
                } else {
                    units.len() - count..units.len()
                };
                // A lone UTF-16 surrogate has no Rust string representation.
                text(String::from_utf16(&units[range]).map_err(|_| "#VALUE!")?)
            }
            "MID" if args.len() == 3 => {
                let value = self.eval(&args[0], depth)?.text();
                let start = self.eval(&args[1], depth)?.number()?.trunc();
                let count = self.eval(&args[2], depth)?.number()?.trunc();
                if start < 1.0 || count < 0.0 {
                    return Err("#VALUE!");
                }
                let units = value.encode_utf16().collect::<Vec<_>>();
                let start = ((start - 1.0) as usize).min(units.len());
                let end = start.saturating_add(count as usize).min(units.len());
                text(String::from_utf16(&units[start..end]).map_err(|_| "#VALUE!")?)
            }
            "CONCATENATE" if !args.is_empty() => {
                let mut result = String::new();
                for arg in args {
                    let value = self.eval(arg, depth)?.text();
                    if result.len() + value.len() > super::MAX_TEXT {
                        return Err("#LIMIT!");
                    }
                    result.push_str(&value);
                }
                text(result)
            }
            "IF" | "IFERROR" | "TRUE" | "FALSE" | "PI" | "AND" | "OR" | "NOT" | "ABS" | "INT"
            | "SIGN" | "SQRT" | "SIN" | "COS" | "TAN" | "LN" | "LOG10" | "EXP" | "ROUND"
            | "ROUNDUP" | "ROUNDDOWN" | "TRUNC" | "MOD" | "POWER" | "LOG" | "LEN" | "LOWER"
            | "UPPER" | "TRIM" | "LEFT" | "RIGHT" | "MID" | "CONCATENATE" => Err("#VALUE!"),
            _ => Err("#NAME?"),
        }
    }

    fn aggregate(&mut self, name: &str, args: &[Expr], depth: usize) -> Result<Value> {
        if name == "COUNTBLANK"
            && (args.len() != 1 || !matches!(args[0], Expr::Cell(_) | Expr::Range(..)))
        {
            return Err("#VALUE!");
        }
        let mut count = 0usize;
        let mut values = Vec::new();
        self.visit(args, depth, |value, reference| {
            // Resource limits cannot be turned into a partial numeric result.
            if matches!(value, Err("#LIMIT!")) {
                return Err("#LIMIT!");
            }
            if name == "COUNTA" {
                if !matches!(value, Ok(Value::Blank)) {
                    count += 1;
                }
                return Ok(());
            }
            if name == "COUNTBLANK" {
                if matches!(&value, Ok(Value::Blank))
                    || matches!(&value, Ok(Value::Text(text)) if text.is_empty())
                {
                    count += 1;
                }
                return Ok(());
            }
            let value = match value {
                Err(_) if name == "COUNT" => return Ok(()),
                value => value?,
            };
            let numeric = match value {
                Value::Number(value) => Some(value),
                Value::Blank => None,
                _ if reference => None,
                _ => match value.number() {
                    Ok(number) => Some(number),
                    Err(_) if name == "COUNT" => None,
                    error => return error.map(|_| ()),
                },
            };
            if let Some(value) = numeric {
                values.push(value);
            }
            Ok(())
        })?;
        match name {
            "COUNTA" | "COUNTBLANK" => number(count as f64),
            "COUNT" => number(values.len() as f64),
            "SUM" => number(values.iter().sum()),
            "SUMSQ" => number(values.iter().map(|v| v * v).sum()),
            "PRODUCT" => number(if values.is_empty() {
                0.0
            } else {
                values.iter().product()
            }),
            "AVERAGE" if values.is_empty() => Err("#DIV/0!"),
            "AVERAGE" => number(values.iter().sum::<f64>() / values.len() as f64),
            "MIN" => number(values.into_iter().reduce(f64::min).unwrap_or(0.0)),
            "MAX" => number(values.into_iter().reduce(f64::max).unwrap_or(0.0)),
            "MEDIAN" if values.is_empty() => Err("#NUM!"),
            "MEDIAN" => {
                values.sort_by(f64::total_cmp);
                let middle = values.len() / 2;
                number(if values.len() % 2 == 1 {
                    values[middle]
                } else {
                    values[middle - 1] / 2.0 + values[middle] / 2.0
                })
            }
            _ => unreachable!(),
        }
    }

    fn visit(
        &mut self,
        args: &[Expr],
        depth: usize,
        mut visit: impl FnMut(Result<Value>, bool) -> Result<()>,
    ) -> Result<()> {
        let mut total = 0usize;
        for arg in args {
            match arg {
                Expr::Range((c1, r1), (c2, r2)) => {
                    let count = (c1.abs_diff(*c2) + 1) * (r1.abs_diff(*r2) + 1);
                    total += count;
                    if total > MAX_RANGE {
                        return Err("#LIMIT!");
                    }
                    for row in *r1.min(r2)..=*r1.max(r2) {
                        for col in *c1.min(c2)..=*c1.max(c2) {
                            let value = self.cell(&cell_address(col, row), depth + 1);
                            visit(value, true)?;
                        }
                    }
                }
                value => {
                    total += 1;
                    if total > MAX_RANGE {
                        return Err("#LIMIT!");
                    }
                    let result = self.eval(value, depth + 1);
                    visit(result, matches!(value, Expr::Cell(_)))?;
                }
            }
        }
        Ok(())
    }
}
