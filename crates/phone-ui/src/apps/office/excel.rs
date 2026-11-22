use super::*;
use ferroui_base::layout::Orientation;
use ferroui_controls::primitives::ScrollBarVisibility;
use phone_controls::{action, app_bar, flat};
use phone_domain::{DocumentContent, Id, Worksheet, cell_address, cell_coordinates, display_cell};

pub(super) fn view(vm: Rc<PhoneViewModel>, ui: Rc<Session>, document: OfficeDocument) -> Element {
    let DocumentContent::Excel(workbook) = &document.content else {
        return Element::Empty;
    };
    let index = ui.office.sheet.get().min(workbook.sheets.len() - 1);
    let sheet = &workbook.sheets[index];
    let page_rows = 12usize;
    let row_start = ui.office.row_page.get().min(833) * page_rows;
    let col_start = ui.office.col_page.get().min(42) * 6;
    let id = document.id;
    let mut grid = grid()
        .column_definitions("36,96,96,96,96,96,96")
        .row_definitions("32,42,42,42,42,42,42,42,42,42,42,42,42");
    for col in 0..6 {
        if col_start + col >= 256 {
            break;
        }
        let address = cell_address(col_start + col, 0);
        let label = address.trim_end_matches('1');
        grid=grid.child(rsx!{Border{Grid.column:(col+1)as i32,background:"#E7E7E7",border_brush:"#BDBDBD",border_thickness:(0.0,0.0,1.0,1.0),
            TextBlock{text:label,font_size:14.0,foreground:"#333333",horizontal_alignment:H::Center,vertical_alignment:V::Center}}});
    }
    for row in 0..page_rows {
        if row_start + row >= 10000 {
            break;
        }
        grid=grid.child(rsx!{Border{Grid.row:(row+1)as i32,background:"#E7E7E7",border_brush:"#BDBDBD",border_thickness:(0.0,0.0,1.0,1.0),
            TextBlock{text:(row_start+row+1).to_string(),font_size:13.0,foreground:"#333333",horizontal_alignment:H::Center,vertical_alignment:V::Center}}});
        for col in 0..6 {
            if col_start + col >= 256 {
                break;
            }
            let address = cell_address(col_start + col, row_start + row);
            let value = display_cell(sheet, &address);
            let model = vm.clone();
            let state = ui.clone();
            let target = address.clone();
            grid=grid.child(rsx!{Button{Grid.row:(row+1)as i32,Grid.column:(col+1)as i32,class:"phone-flat",padding:(5.0,2.0),min_width:0.0,min_height:0.0,
                horizontal_alignment:H::Stretch,vertical_alignment:V::Stretch,horizontal_content_alignment:H::Left,
                automation_properties_name:format!("Cell {address}"),automation_properties_help_text:value.clone(),background:"White",border_brush:"#D4D4D4",border_thickness:(0.0,0.0,1.0,1.0),
                on_click:move|_|edit_cell(model.clone(),state.clone(),id,index,target.clone()),
                TextBlock{text:value,font_size:17.0,foreground:"#202020"}}});
        }
    }
    let mut tabs = stack_panel()
        .orientation(Orientation::Horizontal)
        .spacing(6.0)
        .margin((12.0, 4.0));
    for (number, sheet) in workbook.sheets.iter().enumerate() {
        let state = ui.clone();
        let selected = number == index;
        tabs=tabs.child(rsx!{Button{class:"phone-flat",padding:(10.0,8.0),automation_properties_name:sheet.name.clone(),border_brush:if selected{"#107C41"}else{"Transparent"},border_thickness:(0.0,0.0,0.0,3.0),
            on_click:move|_|{state.office.sheet.set(number);state.office.row_page.set(0);state.office.col_page.set(0);},
            TextBlock{text:sheet.name.clone(),font_size:19.0,foreground:if selected{"#107C41"}else{"#777777"}}}});
    }
    let up = ui.clone();
    let down = ui.clone();
    let left = ui.clone();
    let right = ui.clone();
    let body = rsx! {StackPanel{spacing:4.0,
        Border{margin:(20.0,4.0),{text(&document.name,28.0)}}
        {compatibility(&document)}
        ScrollViewer{horizontal_scroll_bar_visibility:ScrollBarVisibility::Auto,vertical_scroll_bar_visibility:ScrollBarVisibility::Disabled,{tabs}}
        ScrollViewer{horizontal_scroll_bar_visibility:ScrollBarVisibility::Auto,vertical_scroll_bar_visibility:ScrollBarVisibility::Disabled,margin:(10.0,0.0),{grid}}
        StackPanel{orientation:Orientation::Horizontal,horizontal_alignment:H::Center,spacing:12.0,
            {flat("‹ columns",move||left.office.col_page.set(left.office.col_page.get_untracked().saturating_sub(1)))}
            {flat("columns ›",move||right.office.col_page.set((right.office.col_page.get_untracked()+1).min(42)))}}
        StackPanel{orientation:Orientation::Horizontal,horizontal_alignment:H::Center,spacing:18.0,
            {flat("↑ rows",move||up.office.row_page.set(up.office.row_page.get_untracked().saturating_sub(1)))}
            {flat("rows ↓",move||down.office.row_page.set((down.office.row_page.get_untracked()+1).min(833)))}}
    }};
    let go_vm = vm.clone();
    let go_ui = ui.clone();
    let add_vm = vm.clone();
    let add_ui = ui.clone();
    let export_vm = vm.clone();
    let export_ui = ui.clone();
    let export_doc = document.clone();
    let more = ui.clone();
    let rename_vm = vm.clone();
    let rename_ui = ui.clone();
    let rename_doc = document.clone();
    let sheet_vm = vm.clone();
    let sheet_ui = ui.clone();
    let help = vm.clone();
    let pin = vm.clone();
    let menu = vec![
        flat("rename workbook", move || {
            super::commands::rename(rename_vm.clone(), rename_ui.clone(), rename_doc.clone())
        }),
        flat("rename worksheet", move || {
            rename_sheet(sheet_vm.clone(), sheet_ui.clone(), id, index)
        }),
        flat("pin to start", move || {
            pin.pin_document(id);
            pin.home();
        }),
        flat("formula help", move || {
            help.set_status("Formulas: + − * / ^ %, cell references, SUM, AVERAGE, MIN, MAX, COUNT, ABS and ROUND. Example: =SUM(A1:A10).".into())
        }),
    ];
    page(
        "excel",
        scroll(body),
        app_bar(
            vec![
                action(Icon::Search, "go to cell", move || {
                    goto(go_vm.clone(), go_ui.clone(), id, index)
                }),
                action(Icon::Plus, "add sheet", move || {
                    add_sheet(&add_vm, &add_ui, id)
                }),
                action(Icon::Save, "save a copy", move || {
                    super::commands::export(
                        export_vm.clone(),
                        export_ui.clone(),
                        export_doc.clone(),
                    )
                }),
                action(Icon::More, "more", move || {
                    more.menu.set(!more.menu.get_untracked())
                }),
            ],
            ui.menu.get(),
            menu,
        ),
    )
}

fn edit_cell(vm: Rc<PhoneViewModel>, ui: Rc<Session>, id: Id, sheet: usize, address: String) {
    let value = vm
        .data()
        .document(id)
        .and_then(|d| match &d.content {
            DocumentContent::Excel(book) => book
                .sheets
                .get(sheet)
                .and_then(|s| s.cells.get(&address))
                .cloned(),
            _ => None,
        })
        .unwrap_or_default();
    let state = ui.clone();
    ui.form(
        &format!("cell {address}"),
        vec![("value or formula", value)],
        Rc::new(move |values| {
            let document = vm.data().document(id).cloned();
            let Some(mut document) = document else {
                return;
            };
            if let DocumentContent::Excel(book) = &mut document.content {
                if let Some(sheet) = book.sheets.get_mut(sheet) {
                    let value = values.first().cloned().unwrap_or_default();
                    if value.is_empty() {
                        sheet.cells.remove(&address);
                    } else {
                        sheet.cells.insert(address.clone(), value);
                    }
                    super::commands::save(&vm, &state, document);
                }
            }
        }),
    );
}
fn goto(vm: Rc<PhoneViewModel>, ui: Rc<Session>, _id: Id, _sheet: usize) {
    let state = ui.clone();
    ui.form(
        "go to cell",
        vec![("cell", "A1".into())],
        Rc::new(move |values| {
            let address = values
                .first()
                .map(String::as_str)
                .unwrap_or("")
                .trim()
                .to_ascii_uppercase();
            if let Some((column, row)) = cell_coordinates(&address) {
                state.office.row_page.set(row / 12);
                state.office.col_page.set(column / 6);
                vm.set_status(format!("{address}: tap its cell to edit."));
            } else {
                state
                    .form_error
                    .set("Enter a cell from A1 to IV10000.".into());
            }
        }),
    );
}
fn add_sheet(vm: &PhoneViewModel, ui: &Session, id: Id) {
    let document = vm.data().document(id).cloned();
    let Some(mut document) = document else {
        return;
    };
    if let DocumentContent::Excel(book) = &mut document.content {
        if book.sheets.len() >= 16 {
            vm.set_status("A workbook can contain up to 16 sheets.".into());
            return;
        }
        let mut number = book.sheets.len() + 1;
        while book
            .sheets
            .iter()
            .any(|s| s.name.eq_ignore_ascii_case(&format!("Sheet{number}")))
        {
            number += 1;
        }
        book.sheets.push(Worksheet {
            name: format!("Sheet{number}"),
            ..Default::default()
        });
        ui.office.sheet.set(book.sheets.len() - 1);
        ui.office.row_page.set(0);
        ui.office.col_page.set(0);
        vm.save_document(document);
    }
}
fn rename_sheet(vm: Rc<PhoneViewModel>, ui: Rc<Session>, id: Id, index: usize) {
    let value = vm
        .data()
        .document(id)
        .and_then(|d| match &d.content {
            DocumentContent::Excel(book) => book.sheets.get(index).map(|s| s.name.clone()),
            _ => None,
        })
        .unwrap_or_default();
    let state = ui.clone();
    ui.form(
        "rename worksheet",
        vec![("name", value)],
        Rc::new(move |values| {
            let document = vm.data().document(id).cloned();
            let Some(mut document) = document else {
                return;
            };
            if let DocumentContent::Excel(book) = &mut document.content {
                if let Some(sheet) = book.sheets.get_mut(index) {
                    sheet.name = values.first().cloned().unwrap_or_default().trim().into();
                    super::commands::save(&vm, &state, document);
                }
            }
        }),
    );
}
