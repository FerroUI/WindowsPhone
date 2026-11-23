use crate::{common, session::Session};
use ferroui_base::layout::{HorizontalAlignment as H, Orientation, VerticalAlignment as V};
use ferroui_base::media::{FontWeight, Stretch, TextWrapping};
use ferroui_controls::primitives::ScrollBarVisibility;
use ferroui_view::prelude::*;
use phone_controls::{icon, Icon};
use phone_domain::Route;
use phone_sdk::AppManifest;
use phone_state::{PhoneViewModel, RecentApp};
use std::rc::Rc;

pub fn view(vm: Rc<PhoneViewModel>, ui: Rc<Session>, overlay: bool) -> Element {
    let (screen_width, screen_height, _) = crate::shell::phone_size(&vm);
    let available_height = screen_height - if overlay { 54.0 } else { 80.0 };
    let compact = available_height < 430.0;
    let header_height = if compact { 58.0 } else { 84.0 };
    let card_width = (screen_width - 48.0).clamp(120.0, 270.0);
    let max_height = (available_height - header_height - 128.0).max(1.0);
    let mut cards = stack_panel()
        .orientation(Orientation::Horizontal)
        .spacing(22.0)
        .margin((24.0, 10.0))
        .vertical_alignment(V::Top);
    let registry = ui.registry();
    for entry in vm.recent_entries() {
        let registration = match &entry {
            RecentApp::Builtin(app) => registry.for_route(&Route::App(*app)),
            RecentApp::Extension(id) => registry.by_id(id),
        };
        let Some(registration) = registration else {
            continue;
        };
        cards = cards.child(card(
            vm.clone(),
            ui.clone(),
            registration.manifest,
            card_width,
            max_height,
            screen_height / screen_width,
        ));
    }
    let done = ui.clone();
    let done_vm = vm.clone();
    let rows = format!("{header_height},*,54");
    rsx! {Grid{name:"AppSwitcher",background:"#222222",row_definitions:rows.as_str(),clip_to_bounds:true,
        TextBlock{text:"recent apps",font_size:if compact{26.0}else{35.0},font_weight:FontWeight::Light,foreground:"White",margin:(22.0,8.0),vertical_alignment:V::Center},
        ScrollViewer{Grid.row:1,horizontal_scroll_bar_visibility:ScrollBarVisibility::Hidden,vertical_scroll_bar_visibility:ScrollBarVisibility::Disabled,{cards}},
        Button{Grid.row:2,class:"phone-flat",horizontal_content_alignment:H::Center,on_click:move |_|{done.switcher_open.set(false);if done_vm.route()==Route::Switcher{done_vm.back();}},TextBlock{text:"back",font_size:18.0,foreground:"White"}}
    }}
}

fn card(
    vm: Rc<PhoneViewModel>,
    ui: Rc<Session>,
    app: AppManifest,
    width: f64,
    max_height: f64,
    fallback_ratio: f64,
) -> Element {
    let color = if app.accent.is_empty() {
        common::accent(&vm)
    } else {
        app.accent.into()
    };
    let preview = app
        .builtin
        .and_then(|id| ui.state::<crate::previews::PreviewCache>().get(id));
    let ratio = preview
        .as_ref()
        .map(|preview| preview.logical_size.height / preview.logical_size.width)
        .unwrap_or(fallback_ratio);
    let height = (width * ratio).min(max_height);
    let content = match preview {
        Some(preview) => rsx! {Image{
            name:format!("AppPreview-{}",app.builtin.map(|id|id.icon()).unwrap_or(app.id)),source:Some(preview.image),
            stretch:Stretch::Uniform,is_hit_test_visible:false
        }},
        None => rsx! {Border{background:color.as_str(),
            StackPanel{vertical_alignment:V::Center,spacing:16.0,
                {icon(app.icon,(height*0.24).clamp(24.0,78.0),"White")},
                if height > 180.0 {TextBlock{text:app.title.to_lowercase(),font_size:28.0,foreground:"White",margin:(16.0,0.0),text_wrapping:TextWrapping::Wrap}}
            }
        }},
    };
    let close_vm = vm.clone();
    let close_ui = ui.clone();
    let open_vm = vm.clone();
    let open_ui = ui.clone();
    rsx! {StackPanel{width:width,spacing:10.0,
        Grid{height:44.0,column_definitions:"*,40",clip_to_bounds:true,
            TextBlock{text:app.title,font_size:20.0,foreground:"White",vertical_alignment:V::Center},
            Button{Grid.column:1,class:"phone-flat",automation_properties_name:format!("Close {}",app.title),on_click:move |_|{
                let active=close_ui.registry().for_route(&close_vm.route()).is_some_and(|current|current.manifest.id==app.id);
                close_ui.registry().close(&close_vm,&close_ui,app.id);
                if active {close_ui.selected.set(None);close_ui.pivot.set(0);}
                let key=app.builtin.map(|id|id.label()).unwrap_or(app.id);
                close_ui.app_selections.borrow_mut().remove(key);close_ui.app_pivots.borrow_mut().remove(key);
                if let Some(id)=app.builtin {close_ui.state::<crate::previews::PreviewCache>().remove(id);close_vm.close_app(id);}
                else {close_vm.close_extension(app.id);}
            },{icon(Icon::Close,18.0,"White")}}
        },
        Button{class:"phone-flat",height:height,min_height:0.0,padding:0.0,horizontal_content_alignment:H::Stretch,vertical_content_alignment:V::Stretch,
            automation_properties_name:format!("Switch to {}",app.title),on_click:move |_|{
                if let Some(id)=app.builtin {open_ui.open_app(&open_vm,id);} else {
                    open_ui.folder.set(None);open_ui.edit_tile.set(None);open_ui.switcher_open.set(false);open_vm.resume_extension(app.id);
                }
            },Border{background:"Black",border_brush:"#777777",border_thickness:1.0,clip_to_bounds:true,{content}}
        }
    }}
}
