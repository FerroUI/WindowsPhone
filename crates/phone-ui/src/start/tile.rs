use super::{content, folders, wallpaper};
use crate::session::Session;
use ferroui_base::input::HoldingState;
use ferroui_base::layout::{HorizontalAlignment as H, VerticalAlignment as V};
use ferroui_view::prelude::*;
use phone_controls::{Icon, icon};
use phone_domain::{AppId, Tile, TileSize};
use phone_state::PhoneViewModel;
use std::rc::Rc;

pub(super) fn view(
    vm: Rc<PhoneViewModel>,
    ui: Rc<Session>,
    tile: Tile,
    w: f64,
    h: f64,
    x: f64,
    y: f64,
) -> Element {
    let app = tile.app;
    let id = tile.id;
    let small = tile.size == TileSize::Small;
    let is_folder = tile.is_folder();
    let data = vm.data();
    let accent = data.settings.accent.clone();
    let wallpaper = data.settings.wallpaper.clone();
    let badge = if tile.target.is_none() {
        data.badge(app)
    } else {
        0
    };
    drop(data);
    let color = if app.color().is_empty() || is_folder {
        accent.clone()
    } else {
        app.color().into()
    };
    let selected = ui.edit_tile.get() == Some(id);
    let target = ui.drop_target.get() == Some(id);
    let open = ui.clone();
    let launch = vm.clone();
    let edit = ui.clone();
    let context = ui.clone();
    let tilt_ui = Rc::downgrade(&ui);
    let content = if is_folder {
        folders::cover(vm.clone(), ui.clone(), &tile, w, h)
    } else {
        content::view(vm.clone(), ui.clone(), &tile, w, h, badge)
    };
    let mut layers = Vec::<Element>::new();
    if !wallpaper.is_empty() && (app.color().is_empty() || is_folder) && app != AppId::Photos {
        let (width, height, _) = crate::shell::phone_size(&vm);
        layers.push(wallpaper::view(
            &ui,
            id,
            &wallpaper,
            width - 28.0,
            height - 80.0,
            x,
            y,
        ));
        layers.push(rsx! {Border{background:"#33000000"}});
    }
    layers.push(content);
    if !small {
        if app == AppId::Photos && !is_folder {
            layers.push(rsx!{Border{height:34.0,background:"#66000000",vertical_alignment:V::Bottom,is_hit_test_visible:false}});
        }
        layers.push(
            rsx! {TextBlock{text:tile.label.clone(),foreground:"White",font_size:14.0,
                margin:(10.0,0.0,8.0,8.0),vertical_alignment:V::Bottom,is_hit_test_visible:false
            }},
        );
    }
    let visual = rsx! {Grid{name:format!("TileVisual-{id}"),clip_to_bounds:true,background:color.as_str(),
        for layer in layers{{layer}}
    }};
    let button = rsx! {Button{key:format!("tile-{id}"),name:format!("Tile-{id}"),class:"phone-tile",width:w,height:h,padding:0.0,
        with:move|button|{let ui=tilt_ui.clone();phone_controls::attach_press_tilt(button,move||ui.upgrade().is_some_and(|ui|ui.edit_tile.get_untracked().is_none()));},
        automation_properties_name:format!("{} tile",tile.label),horizontal_content_alignment:H::Stretch,vertical_content_alignment:V::Stretch,
        automation_properties_help_text:if is_folder{format!("Folder, {} tiles. Hold to arrange.",tile.folder_tiles.len())}else{format!("{} tile{}. Hold to resize, move or unpin.",match tile.size{TileSize::Small=>"Small",TileSize::Medium=>"Medium",TileSize::Wide=>"Wide"},if badge>0{format!(", {badge} new items")}else{String::new()})},
        on_click:move|_|{
            if open.edit_tile.get_untracked().is_some(){
                if is_folder&&open.edit_tile.get_untracked()==Some(id){open.folder.update(|folder|*folder=if *folder==Some(id){None}else{Some(id)});}
                open.edit_tile.set(Some(id));
            }else if is_folder{open.folder.update(|folder|*folder=if *folder==Some(id){None}else{Some(id)});}
            else{open.folder.set(None);open.edit_tile.set(None);open.menu.set(false);open.switcher_open.set(false);launch.open_tile(id);}
        },
        on_holding:move|event|if event.holding_state()==HoldingState::Started{edit.edit_tile.set(Some(id));event.set_handled(true);},
        on_context_requested:move|event|{context.edit_tile.set(Some(id));event.set_handled(true);},
        {visual}
    }};
    let mut overlays = vec![button];
    if target {
        overlays.push(
            rsx! {Border{border_brush:"White",border_thickness:4.0,is_hit_test_visible:false}},
        );
    }
    if selected {
        let unpin = vm.clone();
        let unpin_ui = ui.clone();
        let resize = vm.clone();
        let actions = ui.clone();
        overlays.push(
            rsx! {Border{border_brush:"White",border_thickness:2.0,is_hit_test_visible:false}},
        );
        overlays.push(rsx!{Button{class:"phone-flat",name:format!("Unpin-{id}"),automation_properties_name:"Unpin tile",width:32.0,height:32.0,min_height:0.0,background:"Black",horizontal_alignment:H::Right,vertical_alignment:V::Top,
            on_click:move|_|{unpin.unpin(id);unpin_ui.edit_tile.set(None);unpin_ui.menu.set(false);},
            Border{border_brush:"White",border_thickness:2.0,corner_radius:16.0,{icon(Icon::Unpin,15.0,"White")}}
        }});
        overlays.push(rsx!{Button{class:"phone-flat",name:format!("Resize-{id}"),automation_properties_name:"Resize tile",width:32.0,height:32.0,min_height:0.0,background:"Black",horizontal_alignment:H::Right,vertical_alignment:V::Bottom,
            on_click:move|_|resize.resize_tile(id),
            Border{border_brush:"White",border_thickness:2.0,corner_radius:16.0,{icon(Icon::Resize,15.0,"White")}}
        }});
        overlays.push(rsx!{Button{class:"phone-flat",name:format!("TileActions-{id}"),automation_properties_name:"Tile actions",width:32.0,height:32.0,min_height:0.0,background:"Black",horizontal_alignment:H::Left,vertical_alignment:V::Top,
            on_click:move|_|actions.menu.update(|menu|*menu=!*menu),{icon(Icon::More,18.0,"White")}
        }});
    }
    rsx! {Grid{for overlay in overlays{{overlay}}}}
}
