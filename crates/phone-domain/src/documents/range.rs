//! Bounded worksheet selections and operations used by the native Excel commands.
use super::{MAX_SHEET_CELLS, Workbook, Worksheet, cell_address, cell_coordinates};
use std::cmp::Ordering;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CellRange {
    first_column: usize,
    first_row: usize,
    last_column: usize,
    last_row: usize,
}
impl CellRange {
    pub fn parse(value: &str) -> Result<Self, String> {
        let mut parts = value.trim().split(':');
        let first = parts.next().unwrap_or("").trim();
        let last = parts.next().unwrap_or(first).trim();
        if parts.next().is_some() {
            return Err("Enter a cell or range, such as A1:C12.".into());
        }
        let (a, b) = cell_coordinates(first).ok_or("Enter a cell from A1 to IV10000.")?;
        let (c, d) = cell_coordinates(last).ok_or("Enter a cell from A1 to IV10000.")?;
        let range = Self {
            first_column: a.min(c),
            first_row: b.min(d),
            last_column: a.max(c),
            last_row: b.max(d),
        };
        if range.count() > MAX_SHEET_CELLS {
            return Err("Select at most 10,000 cells at a time.".into());
        }
        Ok(range)
    }
    pub fn count(self) -> usize {
        (self.last_column - self.first_column + 1) * (self.last_row - self.first_row + 1)
    }
    pub fn first(self) -> (usize, usize) {
        (self.first_column, self.first_row)
    }
    pub fn last(self) -> (usize, usize) {
        (self.last_column, self.last_row)
    }
    pub fn contains(self, column: usize, row: usize) -> bool {
        (self.first_column..=self.last_column).contains(&column)
            && (self.first_row..=self.last_row).contains(&row)
    }
    pub fn label(self) -> String {
        let first = cell_address(self.first_column, self.first_row);
        let last = cell_address(self.last_column, self.last_row);
        if first == last {
            first
        } else {
            format!("{first}:{last}")
        }
    }
}

pub fn clear_cell_range(sheet: &mut Worksheet, range: CellRange) {
    sheet.cells.retain(|address, _| {
        cell_coordinates(address).is_none_or(|(column, row)| !range.contains(column, row))
    });
}

pub fn autosum_cell_range(
    sheet: &mut Worksheet,
    range: CellRange,
    function: &str,
    destination: &str,
) -> Result<String, String> {
    let function = function.trim().to_ascii_uppercase();
    if !["SUM", "AVERAGE", "MIN", "MAX", "COUNT"].contains(&function.as_str()) {
        return Err("Choose SUM, AVERAGE, MIN, MAX or COUNT.".into());
    }
    let (column, row) =
        cell_coordinates(destination.trim()).ok_or("Enter a result cell from A1 to IV10000.")?;
    let destination = cell_address(column, row);
    if range.contains(column, row) {
        return Err("Put the result outside the selected range.".into());
    }
    if sheet
        .cells
        .get(&destination)
        .is_some_and(|value| !value.is_empty())
    {
        return Err("The result cell already contains data. Choose an empty cell.".into());
    }
    if !sheet.cells.contains_key(&destination) && sheet.cells.len() >= MAX_SHEET_CELLS {
        return Err("This worksheet already contains 10,000 populated cells.".into());
    }
    // An explicit one-cell range preserves aggregate semantics for blanks/text.
    let source = format!(
        "{}:{}",
        cell_address(range.first_column, range.first_row),
        cell_address(range.last_column, range.last_row)
    );
    sheet
        .cells
        .insert(destination.clone(), format!("={function}({source})"));
    Ok(destination)
}

/// Sort entire selected rows together. Formula relocation is deliberately rejected;
/// keeping formulas byte-identical while moving them would change their meaning.
pub fn sort_cell_range(
    sheet: &mut Worksheet,
    range: CellRange,
    column: usize,
    descending: bool,
    heading: bool,
) -> Result<(), String> {
    if !(range.first_column..=range.last_column).contains(&column) {
        return Err("Choose a sort column inside the selected range.".into());
    }
    let first_row = range.first_row + usize::from(heading);
    if first_row > range.last_row {
        return Ok(());
    }
    let mut rows = Vec::new();
    for row in first_row..=range.last_row {
        let mut values = Vec::new();
        for column in range.first_column..=range.last_column {
            let value = sheet.cells.get(&cell_address(column, row)).cloned();
            if value.as_ref().is_some_and(|value| value.starts_with('=')) {
                return Err("This range contains formulas. Sort a range of values to keep formula references intact.".into());
            }
            values.push(value);
        }
        rows.push(values);
    }
    let key = column - range.first_column;
    rows.sort_by(|a, b| compare_values(a[key].as_deref(), b[key].as_deref(), descending));
    for (offset, values) in rows.into_iter().enumerate() {
        for (column, value) in values.into_iter().enumerate() {
            let address = cell_address(range.first_column + column, first_row + offset);
            if let Some(value) = value {
                sheet.cells.insert(address, value);
            } else {
                sheet.cells.remove(&address);
            }
        }
    }
    Ok(())
}

fn compare_values(a: Option<&str>, b: Option<&str>, descending: bool) -> Ordering {
    let a = a.filter(|value| !value.is_empty());
    let b = b.filter(|value| !value.is_empty());
    let order = match (a, b) {
        (None, None) => return Ordering::Equal,
        (None, Some(_)) => return Ordering::Greater,
        (Some(_), None) => return Ordering::Less,
        (Some(a), Some(b)) => {
            let number = |s: &str| s.parse::<f64>().ok().filter(|n| n.is_finite());
            match (number(a), number(b)) {
                (Some(a), Some(b)) => a.total_cmp(&b),
                (Some(_), None) => Ordering::Less,
                (None, Some(_)) => Ordering::Greater,
                (None, None) => a
                    .trim_start_matches('\'')
                    .to_lowercase()
                    .cmp(&b.trim_start_matches('\'').to_lowercase()),
            }
        }
    };
    if descending { order.reverse() } else { order }
}

/// Searches stored text/formulas in sheet/row/column order and wraps once.
pub fn find_workbook_cell(
    book: &Workbook,
    query: &str,
    after: Option<(usize, &str)>,
) -> Option<(usize, String)> {
    let query = query.trim().to_lowercase();
    if query.is_empty() {
        return None;
    }
    let after = after.and_then(|(sheet, address)| {
        cell_coordinates(address).map(|(column, row)| (sheet, row, column))
    });
    let mut first = None;
    let mut next = None;
    for (sheet_index, sheet) in book.sheets.iter().enumerate() {
        for (address, value) in &sheet.cells {
            if !value.to_lowercase().contains(&query) {
                continue;
            }
            let Some((column, row)) = cell_coordinates(address) else {
                continue;
            };
            let position = (sheet_index, row, column);
            if first.is_none_or(|found| position < found) {
                first = Some(position);
            }
            if after.is_some_and(|after| position > after)
                && next.is_none_or(|found| position < found)
            {
                next = Some(position);
            }
        }
    }
    next.or(first)
        .map(|(sheet, row, column)| (sheet, cell_address(column, row)))
}
