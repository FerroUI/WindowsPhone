use crate::{
    common::{page, scroll},
    session::Session,
};
use ferroui_base::input::{InputElement, Key, KeyEventArgs, KeyModifiers};
use ferroui_base::interactivity::RoutingStrategies;
use ferroui_base::layout::{HorizontalAlignment as H, VerticalAlignment as V};
use ferroui_base::media::FontWeight;
use ferroui_view::prelude::*;
use phone_state::PhoneViewModel;
use std::rc::Rc;

pub fn view(vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Element {
    ui.calculator_revision.get();
    let display = ui.calculator.borrow().display();
    let memory = ui.calculator.borrow().has_memory();
    let (width, height, _) = crate::shell::phone_size(&vm);
    let display_size =
        ((width - 48.0) / (display.chars().count().max(7) as f64 * 0.59)).clamp(20.0, 52.0);
    let accent = vm.data().settings.accent.clone();
    let dark = vm.data().settings.dark;
    let keys = [
        ["MC", "MR", "M+", "M-"],
        ["%", "√", "x²", "1/x"],
        ["C", "CE", "⌫", "÷"],
        ["7", "8", "9", "×"],
        ["4", "5", "6", "−"],
        ["1", "2", "3", "+"],
        ["±", "0", ".", "="],
    ];
    let mut keypad = grid()
        .column_definitions("*,*,*,*")
        .row_definitions("*,*,*,*,*,*,*")
        .row_spacing(4.0)
        .column_spacing(4.0);
    for (r, labels) in keys.iter().enumerate() {
        for (c, label) in labels.iter().enumerate() {
            let state = ui.clone();
            let value = (*label).to_owned();
            let fill = if *label == "=" {
                accent.as_str()
            } else if r < 3 || c == 3 {
                if dark { "#333333" } else { "#DDDDDD" }
            } else {
                if dark { "#1B1B1B" } else { "#EEEEEE" }
            };
            keypad = keypad.child(rsx! { Button {
                Grid.row:r as i32,Grid.column:c as i32,class:"phone-flat",content:(*label).to_owned(),
                background:fill,foreground:if *label=="=" {"White"}else if dark {"White"}else{"Black"},
                font_size:if r<2 {18.0}else{29.0},font_weight:FontWeight::Light,
                horizontal_alignment:H::Stretch,vertical_alignment:V::Stretch,
                horizontal_content_alignment:H::Center,vertical_content_alignment:V::Center,
                min_height:42.0,padding:2.0,automation_properties_name:key_name(label),
                on_click:move |event| {event.set_handled(true);press(&state,&value);}
            } });
        }
    }
    let keyboard = ui.clone();
    let body = rsx! { Grid {name:"CalculatorRoot",focusable:true,row_definitions:"Auto,*",margin:(12.0,8.0,12.0,10.0),row_spacing:16.0,min_height:424.0,
        with:move |root|{
            let state=keyboard.clone();
            let token=root.add_handler_with(InputElement::key_down_event(),move |_,event|handle_key(&state,event),RoutingStrategies::TUNNEL,false);
            let weak=root.downgrade();
            on_cleanup(move||if let Some(root)=weak.upgrade(){root.remove_handler(InputElement::key_down_event(),token);});
        },
        on_loaded:move |event|if let Some(root)=event.source().and_then(|source|source.cast::<Grid>()){root.focus();},
        Grid{margin:(8.0,22.0,8.0,20.0),
            TextBlock{text:if memory{"M"}else{""},font_size:13.0,vertical_alignment:V::Top}
            TextBlock {text:display,font_size:display_size,font_weight:FontWeight::Light,text_alignment:ferroui_base::media::TextAlignment::Right,
                automation_properties_name:"Calculator result"}
        }
        Border {Grid.row:1,{keypad}}
    } };
    page(
        "calculator",
        if height < 600.0 { scroll(body) } else { body },
        Element::Empty,
    )
}

fn handle_key(ui: &Session, event: &KeyEventArgs) {
    if event
        .key_modifiers
        .intersects(KeyModifiers::CONTROL | KeyModifiers::META | KeyModifiers::ALT)
    {
        return;
    }
    let value = match event.key_symbol.as_deref() {
        Some(
            symbol @ ("0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "." | "+" | "-"
            | "*" | "/" | "=" | "%"),
        ) => Some(symbol),
        Some(",") => Some("."),
        Some("c" | "C") => Some("C"),
        _ => match event.key {
            Key::Return => Some("="),
            Key::Back => Some("⌫"),
            Key::Delete => Some("C"),
            Key::D0 | Key::NumPad0 => Some("0"),
            Key::D1 | Key::NumPad1 => Some("1"),
            Key::D2 | Key::NumPad2 => Some("2"),
            Key::D3 | Key::NumPad3 => Some("3"),
            Key::D4 | Key::NumPad4 => Some("4"),
            Key::D5 | Key::NumPad5 => Some("5"),
            Key::D6 | Key::NumPad6 => Some("6"),
            Key::D7 | Key::NumPad7 => Some("7"),
            Key::D8 | Key::NumPad8 => Some("8"),
            Key::D9 | Key::NumPad9 => Some("9"),
            Key::Add => Some("+"),
            Key::Subtract | Key::OemMinus => Some("−"),
            Key::Multiply => Some("×"),
            Key::Divide | Key::OemQuestion => Some("÷"),
            Key::Decimal | Key::OemPeriod => Some("."),
            Key::OemPlus => Some(if event.key_modifiers.contains(KeyModifiers::SHIFT) {
                "+"
            } else {
                "="
            }),
            _ => None,
        },
    };
    if let Some(value) = value {
        press(ui, value);
        event.set_handled(true);
    }
}

fn press(ui: &Session, key: &str) {
    ui.calculator.borrow_mut().press(key);
    ui.calculator_revision
        .update(|revision| *revision = revision.wrapping_add(1));
}

fn key_name(key: &str) -> String {
    match key {
        "⌫" => "Backspace",
        "÷" => "Divide",
        "×" => "Multiply",
        "−" => "Subtract",
        "=" => "Equals",
        "±" => "Change sign",
        "√" => "Square root",
        "x²" => "Square",
        "1/x" => "Reciprocal",
        "C" => "Clear",
        "CE" => "Clear entry",
        other => other,
    }
    .into()
}
