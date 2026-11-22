use ferroui_base::layout::HorizontalAlignment as H;
use ferroui_base::media::{FontWeight, TextWrapping};
use ferroui_controls::primitives::ScrollBarVisibility;
use ferroui_view::prelude::*;
use phone_controls::{Icon, icon};
use phone_domain::AppId;
use phone_state::PhoneViewModel;

pub fn fg(vm: &PhoneViewModel) -> String {
    vm.data().settings.foreground().into()
}
pub fn bg(vm: &PhoneViewModel) -> String {
    vm.data().settings.background().into()
}
pub fn accent(vm: &PhoneViewModel) -> String {
    vm.data().settings.accent.clone()
}

pub fn text(value: &str, size: f64) -> Element {
    let size = size * phone_controls::text_scale();
    rsx! { TextBlock { text:value.to_owned(), font_size:size, font_weight:FontWeight::Light,text_wrapping:TextWrapping::Wrap } }
}
pub fn muted(value: &str, size: f64) -> Element {
    let size = size * phone_controls::text_scale();
    rsx! { TextBlock { text:value.to_owned(), font_size:size, foreground:"#929292",text_wrapping:TextWrapping::Wrap } }
}
pub fn scroll(content: Element) -> Element {
    rsx! { ScrollViewer { horizontal_scroll_bar_visibility:ScrollBarVisibility::Disabled,
    vertical_scroll_bar_visibility:ScrollBarVisibility::Hidden, {content} } }
}
pub fn page(title: &str, body: Element, bar: Element) -> Element {
    rsx! { Grid {row_definitions:"Auto,*,Auto",clip_to_bounds:true,
        TextBlock { text:title.to_uppercase(),font_size:13.0,font_weight:FontWeight::SemiBold,margin:(20.0,8.0,16.0,9.0) }
        Border {Grid.row:1,{body}}
        Border {Grid.row:2,{bar}}
    } }
}
pub fn row(title: &str, subtitle: &str, on_click: impl Fn() + 'static) -> Element {
    rsx! { Button { class:"phone-flat", horizontal_alignment:H::Stretch, horizontal_content_alignment:H::Stretch,
        automation_properties_name:title, automation_properties_help_text:subtitle,
        padding:(20.0,12.0), min_height:66.0, on_click:move |_|on_click(),
        StackPanel {spacing:3.0,{text(title,26.0)} if !subtitle.is_empty() {{muted(subtitle,15.0)}}}
    } }
}
pub fn empty(title: &str, subtitle: &str) -> Element {
    rsx! { StackPanel {margin:(20.0,30.0),spacing:16.0,{text(title,30.0)}{muted(subtitle,18.0)}} }
}
pub fn title(value: &str) -> Element {
    rsx! {TextBlock{text:value.to_owned(),font_size:48.0,font_weight:FontWeight::Light,margin:(18.0,0.0,0.0,18.0)}}
}

pub fn app_icon(app: AppId) -> Icon {
    match app {
        AppId::Phone => Icon::Phone,
        AppId::Messaging => Icon::Message,
        AppId::People => Icon::People,
        AppId::Outlook => Icon::Mail,
        AppId::Calendar => Icon::Calendar,
        AppId::Photos => Icon::Photos,
        AppId::Camera => Icon::Camera,
        AppId::InternetExplorer => Icon::Explorer,
        AppId::Music => Icon::Music,
        AppId::Video => Icon::Video,
        AppId::Games => Icon::Games,
        AppId::Store => Icon::Store,
        AppId::Settings => Icon::Settings,
        AppId::Alarms => Icon::Alarm,
        AppId::Calculator => Icon::Calculator,
        AppId::OneNote => Icon::Note,
        AppId::Maps => Icon::Maps,
        AppId::Cortana => Icon::Cortana,
        AppId::Weather => Icon::Weather,
        AppId::Files => Icon::Folder,
        AppId::Office => Icon::Office,
        AppId::BatterySaver => Icon::Battery,
        AppId::DataSense => Icon::Signal,
    }
}
pub fn app_square(app: AppId, size: f64, accent: &str) -> Element {
    let color = match app {
        AppId::Games | AppId::Office | AppId::OneNote | AppId::Music | AppId::Video => app.color(),
        _ => accent,
    };
    rsx! {Border{width:size,height:size,background:color,{icon(app_icon(app),size*0.56,"White")}}}
}
pub fn clock(vm: &PhoneViewModel) -> String {
    use chrono::{DateTime, Timelike};
    let date = DateTime::from_timestamp(vm.local_now(), 0).unwrap_or_default();
    if vm.data().settings.time_24h {
        format!("{:02}:{:02}", date.hour(), date.minute())
    } else {
        format!(
            "{}:{:02}",
            if date.hour() % 12 == 0 {
                12
            } else {
                date.hour() % 12
            },
            date.minute()
        )
    }
}
