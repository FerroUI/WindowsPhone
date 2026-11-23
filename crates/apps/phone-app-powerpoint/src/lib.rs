use ferroui_base::{layout::Orientation, media::TextWrapping};
use phone_controls::{action, app_bar, flat};
use phone_domain::{DocumentContent, Id};
use phone_office_support::*;

pub fn view(vm: Rc<PhoneViewModel>, ui: Rc<Session>, document: OfficeDocument) -> Element {
    let DocumentContent::PowerPoint(deck) = &document.content else {
        return Element::Empty;
    };
    let index = ui
        .state::<phone_office_support::OfficeSession>()
        .slide
        .get()
        .min(deck.slides.len() - 1);
    let slide = &deck.slides[index];
    let previous = ui.clone();
    let next = ui.clone();
    let count = deck.slides.len();
    let mut outline = stack_panel().spacing(4.0);
    for (number, item) in deck.slides.iter().enumerate() {
        let select = ui.clone();
        outline = outline.child(row(
            &format!(
                "{}. {}",
                number + 1,
                if item.title.is_empty() {
                    "Untitled slide"
                } else {
                    &item.title
                }
            ),
            if index == number { "current slide" } else { "" },
            move || {
                select
                    .state::<phone_office_support::OfficeSession>()
                    .slide
                    .set(number)
            },
        ));
    }
    let body = rsx! {StackPanel{spacing:12.0,
        Border{margin:(20.0,4.0),{text(&document.name,28.0)}}
        {compatibility(&document)}
        Border{margin:(12.0,0.0),padding:20.0,background:"White",min_height:210.0,
            StackPanel{spacing:18.0,
                TextBlock{text:if slide.title.is_empty(){"Tap edit to add a title".into()}else{slide.title.clone()},font_size:29.0,font_weight:FontWeight::SemiBold,foreground:"#222222",text_wrapping:TextWrapping::Wrap}
                TextBlock{text:slide.body.clone(),font_size:19.0,foreground:"#333333",text_wrapping:TextWrapping::Wrap}}}
        StackPanel{orientation:Orientation::Horizontal,horizontal_alignment:H::Center,spacing:18.0,
            {flat("‹",move||previous.state::<phone_office_support::OfficeSession>().slide.set(previous.state::<phone_office_support::OfficeSession>().slide.get_untracked().min(count-1).saturating_sub(1)))}
            TextBlock{text:format!("{} / {}",index+1,count),font_size:18.0,vertical_alignment:V::Center}
            {flat("›",move||next.state::<phone_office_support::OfficeSession>().slide.set((next.state::<phone_office_support::OfficeSession>().slide.get_untracked()+1).min(count-1)))}}
        if !slide.notes.is_empty(){StackPanel{margin:(20.0,0.0),spacing:6.0,{muted("speaker notes",14.0)}{text(&slide.notes,18.0)}}}
        {outline}
    }};
    let id = document.id;
    let edit_vm = vm.clone();
    let edit_ui = ui.clone();
    let export_vm = vm.clone();
    let export_ui = ui.clone();
    let export_doc = document.clone();
    let more = ui.clone();
    let rename_vm = vm.clone();
    let rename_ui = ui.clone();
    let rename_doc = document.clone();
    let share_vm = vm.clone();
    let share_doc = document.clone();
    let pin = vm.clone();
    let files_vm = vm.clone();
    let files_ui = ui.clone();
    let mut menu = phone_office_support::commands::history_menu(&vm, &ui, &document);
    menu.extend(vec![
        flat("share", move || {
            phone_office_support::commands::share_files(
                files_vm.clone(),
                files_ui.clone(),
                vec![id],
            )
        }),
        flat("rename presentation", move || {
            phone_office_support::commands::rename(
                rename_vm.clone(),
                rename_ui.clone(),
                rename_doc.clone(),
            )
        }),
        flat("pin to start", move || {
            pin.pin_document(id);
            pin.home();
        }),
        flat("share text", move || {
            phone_office_support::commands::share(&share_vm, &share_doc)
        }),
    ]);
    page(
        "powerpoint",
        scroll(body),
        app_bar(
            vec![
                action(Icon::Edit, "edit", move || {
                    edit(edit_vm.clone(), edit_ui.clone(), id, index)
                }),
                action(Icon::Save, "save a copy", move || {
                    phone_office_support::commands::export(
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
fn edit(vm: Rc<PhoneViewModel>, ui: Rc<Session>, id: Id, index: usize) {
    let slide = vm.data().document(id).and_then(|d| match &d.content {
        DocumentContent::PowerPoint(deck) => deck.slides.get(index).cloned(),
        _ => None,
    });
    let Some(slide) = slide else {
        return;
    };
    let state = ui.clone();
    ui.form(
        "edit slide",
        vec![
            ("title", slide.title),
            ("body", slide.body),
            ("note", slide.notes),
        ],
        Rc::new(move |values| {
            let document = vm.data().document(id).cloned();
            let Some(mut document) = document else {
                return;
            };
            if let DocumentContent::PowerPoint(deck) = &mut document.content {
                if let Some(slide) = deck.slides.get_mut(index) {
                    slide.title = values.first().cloned().unwrap_or_default();
                    slide.body = values.get(1).cloned().unwrap_or_default();
                    slide.notes = values.get(2).cloned().unwrap_or_default();
                    phone_office_support::commands::save(&vm, &state, document);
                }
            }
        }),
    );
}
