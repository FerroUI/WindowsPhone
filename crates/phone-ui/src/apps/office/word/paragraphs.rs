use super::super::*;
use ferroui_base::{
    BoxedValue,
    media::{FontStyle, TextDecorations, TextWrapping},
};
use ferroui_controls::ItemsSource;
use phone_domain::Paragraph;
use std::cell::RefCell;

#[derive(Clone, PartialEq)]
struct ParagraphRow {
    index: usize,
    paragraph: Paragraph,
    highlighted: bool,
}
struct Rows {
    paragraphs: Vec<Paragraph>,
    query: String,
    source: ItemsSource,
}
impl Rows {
    fn new(paragraphs: Vec<Paragraph>, query: String) -> Self {
        let source =
            ItemsSource::from_items(paragraphs.iter().enumerate().map(|(index, paragraph)| {
                Some(Rc::new(ParagraphRow {
                    index,
                    paragraph: paragraph.clone(),
                    highlighted: !query.is_empty()
                        && paragraph.text.to_lowercase().contains(&query),
                }) as BoxedValue)
            }));
        Self {
            paragraphs,
            query,
            source,
        }
    }
}

/// Preserve the native source/template on clock changes and realize only visible paragraphs.
pub(super) fn view(paragraphs: Vec<Paragraph>, query: String, ui: Rc<Session>) -> Element {
    rerender(move || {
        let cached =
            use_hook(|| Rc::new(RefCell::new(Rows::new(paragraphs.clone(), query.clone()))));
        let source = {
            let mut cache = cached.borrow_mut();
            if cache.paragraphs != paragraphs || cache.query != query {
                *cache = Rows::new(paragraphs.clone(), query.clone());
            }
            cache.source.clone()
        };
        let template = use_hook({
            let ui = ui.clone();
            move || {
                template(move |row: ParagraphRow| {
                    let ui = ui.clone();
                    rerender(move || {
                        let choose = ui.clone();
                        let selected = ui.office.paragraph.get() == row.index;
                        let paragraph = &row.paragraph;
                        let contents = if paragraph.text.is_empty() {
                            "Tap edit to start writing".into()
                        } else {
                            format!(
                                "{}{}",
                                if paragraph.bullet { "• " } else { "" },
                                paragraph.text
                            )
                        };
                        rsx! {Button{class:"phone-flat",horizontal_alignment:H::Stretch,horizontal_content_alignment:H::Stretch,
                        automation_properties_name:format!("Paragraph {}",row.index+1),padding:10.0,margin:(8.0,3.0),border_thickness:(3.0,0.0,0.0,0.0),
                        border_brush:if selected{"#185ABD"}else{"Transparent"},background:if row.highlighted{"#FFF4A3"}else{"White"},
                        on_click:move|_|choose.office.paragraph.set(row.index),
                        TextBlock{text:contents,font_size:if paragraph.heading>0{32.0-paragraph.heading as f64*3.0}else{19.0},
                            foreground:if paragraph.text.is_empty(){"#777777"}else{"#202020"},text_wrapping:TextWrapping::Wrap,
                            font_weight:if paragraph.bold||paragraph.heading>0{FontWeight::Bold}else{FontWeight::Normal},
                            font_style:if paragraph.italic{FontStyle::Italic}else{FontStyle::Normal},
                            text_decorations:if paragraph.underline{Some(TextDecorations::underline())}else{None}}}}
                    })
                })
            }
        });
        rsx! {ListBox{class:"phone-list",background:"White",items_source:source,item_template:template,selected_index:ui.office.paragraph.get()as i32}}
    })
}
