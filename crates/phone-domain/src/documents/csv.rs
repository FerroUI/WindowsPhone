use super::{MAX_COLUMNS, MAX_ROWS, MAX_SHEET_CELLS, Worksheet, cell_address, cell_coordinates};

pub fn sheet_to_csv(sheet: &Worksheet) -> String {
    let bounds = sheet
        .cells
        .keys()
        .filter_map(|a| cell_coordinates(a))
        .fold((0, 0), |(c, r), (nc, nr)| (c.max(nc), r.max(nr)));
    (0..=bounds.1)
        .map(|row| {
            (0..=bounds.0)
                .map(|col| {
                    let value = sheet
                        .cells
                        .get(&cell_address(col, row))
                        .map(String::as_str)
                        .unwrap_or("");
                    if value.contains([',', '"', '\r', '\n']) {
                        format!("\"{}\"", value.replace('"', "\"\""))
                    } else {
                        value.into()
                    }
                })
                .collect::<Vec<_>>()
                .join(",")
        })
        .collect::<Vec<_>>()
        .join("\r\n")
}

pub fn sheet_from_csv(input: &str) -> Result<Worksheet, String> {
    let mut sheet = Worksheet::default();
    let mut row = 0;
    let mut col = 0;
    let mut value = String::new();
    let mut quoted = false;
    let mut closed = false;
    let mut chars = input.trim_start_matches('\u{feff}').chars().peekable();
    while let Some(ch) = chars.next() {
        if quoted {
            if ch == '"' {
                if chars.peek() == Some(&'"') {
                    chars.next();
                    value.push('"');
                } else {
                    quoted = false;
                    closed = true;
                }
            } else {
                value.push(ch);
            }
        } else {
            match ch {
                '"' if value.is_empty() && !closed => quoted = true,
                ',' => {
                    put(&mut sheet, row, col, &mut value)?;
                    col += 1;
                    closed = false;
                }
                '\r' | '\n' => {
                    if ch == '\r' && chars.peek() == Some(&'\n') {
                        chars.next();
                    }
                    put(&mut sheet, row, col, &mut value)?;
                    row += 1;
                    col = 0;
                    closed = false;
                }
                _ if closed => return Err("This CSV has text after a closing quote.".into()),
                _ => value.push(ch),
            }
        }
    }
    if quoted {
        return Err("This CSV has an unclosed quoted value.".into());
    }
    if !value.is_empty() || col > 0 {
        put(&mut sheet, row, col, &mut value)?;
    }
    Ok(sheet)
}
fn put(sheet: &mut Worksheet, row: usize, col: usize, value: &mut String) -> Result<(), String> {
    if row >= MAX_ROWS || col >= MAX_COLUMNS || sheet.cells.len() >= MAX_SHEET_CELLS {
        return Err("This CSV exceeds the worksheet limit (10,000 rows, 256 columns, 10,000 populated cells).".into());
    }
    if !value.is_empty() {
        sheet
            .cells
            .insert(cell_address(col, row), std::mem::take(value));
    }
    Ok(())
}
