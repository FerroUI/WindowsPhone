use super::{content, folders};
use crate::{images, session::Session};
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
    let content = if is_folder {
        folders::cover(&vm, &tile, w, h)
    } else {
        content::view(vm.clone(), ui.clone(), &tile, w, h, badge)
    };
    let mut visual = grid().clip_to_bounds(true).background(color.as_str());
    if !wallpaper.is_empty() && (app.color().is_empty() || is_folder) && app != AppId::Photos {
        visual = visual.child(
            canvas().child(
                border()
                    .canvas_left(-x)
                    .canvas_top(-y)
                    .width(crate::shell::phone_size(&vm).0 - 28.0)
                    .height(950.0)
                    .child(images::photo(&wallpaper)),
            ),
        );
        visual = visual.child(border().background("#33000000"));
    }
    visual = visual.child(content);
    if !small {
        visual = visual.child(
            text_block()
                .text(tile.label.clone())
                .foreground("White")
                .font_size(14.0)
                .margin((10.0, 0.0, 8.0, 8.0))
                .vertical_alignment(V::Bottom)
                .is_hit_test_visible(false),
        );
    }
    let mut result=grid().child(rsx!{Button{key:format!("tile-{id}"),name:format!("Tile-{id}"),class:"phone-tile",width:w,height:h,padding:0.0,
        automation_properties_name:format!("{} tile",tile.label),horizontal_content_alignment:H::Stretch,vertical_content_alignment:V::Stretch,
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
    }});
    if target {
        result = result.child(
            border()
                .border_brush("White")
                .border_thickness(4.0)
                .is_hit_test_visible(false),
        );
    }
    if selected {
        let unpin = vm.clone();
        let unpin_ui = ui.clone();
        let resize = vm.clone();
        let actions = ui.clone();
        result = result.child(
            border()
                .border_brush("White")
                .border_thickness(2.0)
                .is_hit_test_visible(false),
        );
        result=result.child(rsx!{Button{class:"phone-flat",name:format!("Unpin-{id}"),automation_properties_name:"Unpin tile",width:32.0,height:32.0,min_height:0.0,background:"Black",horizontal_alignment:H::Right,vertical_alignment:V::Top,
            on_click:move|_|{unpin.unpin(id);unpin_ui.edit_tile.set(None);unpin_ui.menu.set(false);},
            Border{border_brush:"White",border_thickness:2.0,corner_radius:16.0,{icon(Icon::Unpin,15.0,"White")}}
        }});
        result=result.child(rsx!{Button{class:"phone-flat",name:format!("Resize-{id}"),automation_properties_name:"Resize tile",width:32.0,height:32.0,min_height:0.0,background:"Black",horizontal_alignment:H::Right,vertical_alignment:V::Bottom,
            on_click:move|_|resize.resize_tile(id),
            Border{border_brush:"White",border_thickness:2.0,corner_radius:16.0,{icon(Icon::Resize,15.0,"White")}}
        }});
        result=result.child(rsx!{Button{class:"phone-flat",name:format!("TileActions-{id}"),automation_properties_name:"Tile actions",width:32.0,height:32.0,min_height:0.0,background:"Black",horizontal_alignment:H::Left,vertical_alignment:V::Top,
            on_click:move|_|actions.menu.update(|menu|*menu=!*menu),{icon(Icon::More,18.0,"White")}
        }});
    }
    result.into()
}
