use super::*;
use phone_domain::{
    CellRange, autosum_cell_range, clear_cell_range, find_workbook_cell, sort_cell_range,
};

pub(super) fn selection(ui: &Session, id: Id, sheet: usize) -> CellRange {
    ui.office
        .excel_selection
        .get()
        .filter(|(document, index, _)| *document == id && *index == sheet)
        .and_then(|(_, _, range)| CellRange::parse(&range).ok())
        .unwrap_or_else(|| CellRange::parse("A1").unwrap())
}
fn show(ui: &Session, id: Id, sheet: usize, range: CellRange) {
    let (column, row) = range.first();
    ui.office.sheet.set(sheet);
    ui.office.row_page.set(row / 12);
    ui.office.col_page.set(column / 6);
    ui.office
        .excel_selection
        .set(Some((id, sheet, range.label())));
}
pub(super) fn select(ui: Rc<Session>, id: Id, sheet: usize) {
    let range = selection(&ui, id, sheet).label();
    let state = ui.clone();
    ui.form_with_action(
        "select cells",
        vec![("range", range)],
        "done",
        Rc::new(move |values| match CellRange::parse(value(&values, 0)) {
            Ok(range) => show(&state, id, sheet, range),
            Err(error) => state.form_error.set(error),
        }),
    );
}

pub(super) fn find(vm: Rc<PhoneViewModel>, ui: Rc<Session>, id: Id, sheet: usize) {
    let state = ui.clone();
    ui.form_with_action(
        "find",
        vec![("text or formula", ui.query.get_untracked())],
        "find",
        Rc::new(move |values| {
            let query = value(&values, 0).trim();
            if query.is_empty() {
                state.form_error.set("Enter text to find.".into());
                return;
            }
            let range = selection(&state, id, sheet);
            let address = cell_address(range.first().0, range.first().1);
            let found = {
                let data = vm.data();
                data.document(id)
                    .and_then(|document| match &document.content {
                        DocumentContent::Excel(book) => {
                            find_workbook_cell(book, query, Some((sheet, &address)))
                        }
                        _ => None,
                    })
            };
            if let Some((sheet, address)) = found {
                state.query.set(query.into());
                if let Ok(range) = CellRange::parse(&address) {
                    show(&state, id, sheet, range);
                }
            } else {
                state
                    .form_error
                    .set("No matching text or formula was found.".into());
            }
        }),
    );
}

pub(super) fn autosum(vm: Rc<PhoneViewModel>, ui: Rc<Session>, id: Id, sheet: usize) {
    let range = selection(&ui, id, sheet);
    let (first_column, first_row) = range.first();
    let (last_column, last_row) = range.last();
    let destination = if last_row + 1 < phone_domain::MAX_ROWS {
        cell_address(first_column, last_row + 1)
    } else if last_column + 1 < phone_domain::MAX_COLUMNS {
        cell_address(last_column + 1, first_row)
    } else {
        String::new()
    };
    let state = ui.clone();
    ui.form_with_action(
        "autosum",
        vec![
            ("range", range.label()),
            ("function", "SUM".into()),
            ("result cell", destination),
        ],
        "insert",
        Rc::new(move |values| {
            let result = CellRange::parse(value(&values, 0)).and_then(|range| {
                edit_sheet(&vm, &state, id, sheet, |worksheet| {
                    autosum_cell_range(worksheet, range, value(&values, 1), value(&values, 2))
                })
            });
            match result {
                Ok(address) => {
                    if let Ok(range) = CellRange::parse(&address) {
                        show(&state, id, sheet, range);
                    }
                }
                Err(error) => state.form_error.set(error),
            }
        }),
    );
}

pub(super) fn clear(vm: Rc<PhoneViewModel>, ui: Rc<Session>, id: Id, sheet: usize) {
    let range = selection(&ui, id, sheet);
    let state = ui.clone();
    ui.form_with_action(
        &format!("clear {}?", range.label()),
        vec![],
        "clear",
        Rc::new(move |_| {
            if let Err(error) = edit_sheet(&vm, &state, id, sheet, |worksheet| {
                clear_cell_range(worksheet, range);
                Ok(())
            }) {
                state.form_error.set(error);
            }
        }),
    );
}

pub(super) fn sort(vm: Rc<PhoneViewModel>, ui: Rc<Session>, id: Id, sheet: usize) {
    let range = selection(&ui, id, sheet);
    let column = cell_address(range.first().0, 0)
        .trim_end_matches('1')
        .to_owned();
    let state = ui.clone();
    ui.form_with_action(
        "sort",
        vec![
            ("range", range.label()),
            ("sort by column", column),
            ("order", "ascending".into()),
            ("first row is a heading", "no".into()),
        ],
        "sort",
        Rc::new(move |values| {
            let result = (|| {
                let range = CellRange::parse(value(&values, 0))?;
                let column = value(&values, 1).trim().to_ascii_uppercase();
                if column.is_empty() || !column.bytes().all(|b| b.is_ascii_uppercase()) {
                    return Err("Enter a column letter, such as A.".into());
                }
                let (column, _) = cell_coordinates(&format!("{column}1"))
                    .ok_or("Enter a column from A to IV.")?;
                let descending = match value(&values, 2).trim().to_lowercase().as_str() {
                    "ascending" => false,
                    "descending" => true,
                    _ => return Err("Choose ascending or descending order.".into()),
                };
                let heading = match value(&values, 3).trim().to_lowercase().as_str() {
                    "yes" => true,
                    "no" => false,
                    _ => return Err("Enter yes or no for the heading row.".into()),
                };
                edit_sheet(&vm, &state, id, sheet, |worksheet| {
                    sort_cell_range(worksheet, range, column, descending, heading)
                })?;
                Ok(range)
            })();
            match result {
                Ok(range) => show(&state, id, sheet, range),
                Err(error) => state.form_error.set(error),
            }
        }),
    );
}

fn value(values: &[String], index: usize) -> &str {
    values.get(index).map(String::as_str).unwrap_or("")
}
fn edit_sheet<T>(
    vm: &PhoneViewModel,
    ui: &Session,
    id: Id,
    sheet: usize,
    edit: impl FnOnce(&mut Worksheet) -> Result<T, String>,
) -> Result<T, String> {
    let mut document = vm
        .data()
        .document(id)
        .cloned()
        .ok_or("This workbook no longer exists.")?;
    let DocumentContent::Excel(book) = &mut document.content else {
        return Err("This file is not a workbook.".into());
    };
    let worksheet = book
        .sheets
        .get_mut(sheet)
        .ok_or("This worksheet no longer exists.")?;
    let result = edit(worksheet)?;
    if !super::super::commands::save(vm, ui, document) {
        return Err(ui.form_error.get_untracked());
    }
    Ok(result)
}
