//! Lock wallpaper, one detailed status provider and five ordered quick-status slots.
use super::update;
use ferroui_base::layout::{HorizontalAlignment as H, VerticalAlignment as V};
use ferroui_view::prelude::*;
use phone_controls::{flat, icon, toggle, Icon};
use phone_domain::lock_status_apps;
use phone_sdk::{
    common::{app_icon, muted, text},
    session::Session,
};
use phone_state::PhoneViewModel;
use std::rc::Rc;

const DETAILS: u64 = 5;
const WALLPAPER: u64 = 6;
const TIMEOUT: u64 = 7;

pub(super) fn view(vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Element {
    let settings = vm.data().settings.clone();
    if let Some(selected) = ui.selected.get() {
        return match selected {
            WALLPAPER => wallpapers(vm, ui),
            TIMEOUT => timeouts(vm, ui),
            _ => providers(vm, ui, selected),
        };
    }
    let background = ui.selected;
    let detailed = ui.selected;
    let timeout = ui.selected;
    let status = vm.clone();
    let lock = ui.clone();
    let mut slots = grid().column_definitions("*,*,*,*,*").column_spacing(8.0);
    for (index, name) in settings.lock_quick_apps.iter().enumerate() {
        let select = ui.selected;
        let app = settings.lock_app(name);
        slots = slots.child(rsx! {Button {
            Grid.column:index as i32,class:"phone-flat",height:55.0,padding:0.0,
            border_brush:settings.foreground(),border_thickness:2.0,
            automation_properties_name:format!("Choose lock screen status {}: {}",index+1,if name.is_empty(){"none"}else{name}),
            on_click:move |_|select.set(Some(index as u64)),
            {icon(app.map(app_icon).unwrap_or(Icon::Plus),26.0,settings.foreground())}
        }});
    }
    rsx! {StackPanel{spacing:18.0,
        {muted("Background",18.0)}
        {picker(if settings.lock_wallpaper.is_empty(){"default photo"}else{"photo"},move ||background.set(Some(WALLPAPER)))}
        if !settings.lock_wallpaper.is_empty() {
            Border {height:160.0,clip_to_bounds:true,{phone_sdk::images::photo(&settings.lock_wallpaper)}}
        }
        {text("notifications",28.0)}
        {muted("Choose an app to show detailed status",17.0)}
        {picker(if settings.lock_detail_app.is_empty(){"none"}else{&settings.lock_detail_app},move ||detailed.set(Some(DETAILS)))}
        {muted("Choose apps to show quick status",17.0)}
        {slots}
        {toggle("Show notifications when locked",settings.notifications_on_lock,move |enabled|update(&status,|s|s.notifications_on_lock=enabled))}
        {muted("Screen times out after",18.0)}
        {picker(timeout_name(settings.lock_timeout),move ||timeout.set(Some(TIMEOUT)))}
        {flat("lock now",move ||lock.locked.set(true))}
        {muted("Your device's password and security remain controlled by its system settings.",15.0)}
    }}
}

fn providers(vm: Rc<PhoneViewModel>, ui: Rc<Session>, selected: u64) -> Element {
    let settings = vm.data().settings.clone();
    let current = if selected == DETAILS {
        settings.lock_detail_app.as_str()
    } else {
        settings
            .lock_quick_apps
            .get(selected as usize)
            .map(String::as_str)
            .unwrap_or("")
    };
    let mut choices = stack_panel().spacing(7.0).child(text(
        if selected == DETAILS {
            "detailed status"
        } else {
            "quick status"
        },
        30.0,
    ));
    for app in std::iter::once(None).chain(lock_status_apps().into_iter().map(Some)) {
        let name = app.map(|app| app.label()).unwrap_or("");
        let model = vm.clone();
        let close = ui.selected;
        choices = choices.child(flat(
            &format!(
                "{} {}",
                if current == name { "●" } else { "○" },
                if name.is_empty() { "none" } else { name }
            ),
            move || {
                update(&model, |s| {
                    if selected == DETAILS {
                        s.lock_detail_app = name.into();
                    } else if (selected as usize) < s.lock_quick_apps.len() {
                        let index = selected as usize;
                        let previous = s.lock_quick_apps[index].clone();
                        if !name.is_empty() {
                            if let Some(other) =
                                s.lock_quick_apps.iter().position(|value| value == name)
                            {
                                s.lock_quick_apps[other] = previous;
                            }
                        }
                        s.lock_quick_apps[index] = name.into();
                    }
                });
                close.set(None);
            },
        ));
    }
    let close = ui.selected;
    choices
        .child(flat("cancel", move || close.set(None)))
        .into()
}

fn wallpapers(vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Element {
    let photos = vm.data().photos.clone();
    let default_vm = vm.clone();
    let close = ui.selected;
    let mut choices = stack_panel()
        .spacing(10.0)
        .child(text("choose a photo", 30.0))
        .child(flat("default photo", move || {
            update(&default_vm, |s| s.lock_wallpaper.clear());
            close.set(None);
        }));
    let model = vm.clone();
    let close = ui.selected;
    choices = choices.child(super::photo_picker::view(
        photos,
        vm.viewport().1 - 350.0,
        Rc::new(move |source| {
            update(&model, |settings| settings.lock_wallpaper = source);
            close.set(None);
        }),
    ));
    let close = ui.selected;
    choices
        .child(flat("cancel", move || close.set(None)))
        .into()
}

fn timeouts(vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Element {
    let selected = vm.data().settings.lock_timeout;
    let mut choices = stack_panel()
        .spacing(8.0)
        .child(text("screen timeout", 30.0));
    for seconds in [30, 60, 180, 300, 600, 900, 0] {
        let model = vm.clone();
        let close = ui.selected;
        choices = choices.child(flat(
            &format!(
                "{} {}",
                if seconds == selected { "●" } else { "○" },
                timeout_name(seconds)
            ),
            move || {
                update(&model, |s| s.lock_timeout = seconds);
                close.set(None);
            },
        ));
    }
    choices.into()
}

fn timeout_name(seconds: u32) -> &'static str {
    match seconds {
        30 => "30 seconds",
        60 => "1 minute",
        180 => "3 minutes",
        300 => "5 minutes",
        600 => "10 minutes",
        900 => "15 minutes",
        _ => "never",
    }
}

pub(super) fn picker(value: &str, select: impl Fn() + 'static) -> Element {
    rsx! {Button {class:"phone-flat",horizontal_alignment:H::Stretch,horizontal_content_alignment:H::Stretch,
        border_brush:"#929292",border_thickness:2.0,padding:(10.0,8.0),min_height:47.0,automation_properties_name:value,
        on_click:move |_|select(),
        Grid{column_definitions:"*,24",column_spacing:8.0,
            TextBlock{text:value,font_size:23.0,text_wrapping:ferroui_base::media::TextWrapping::Wrap}
            TextBlock{Grid.column:1,text:"⌄",font_size:22.0,vertical_alignment:V::Center}
        }
    }}
}
