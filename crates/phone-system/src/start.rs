//! The Start canvas uses half-tile cells for both root tiles and live folders.
//! Tile identity, size and secondary activation targets survive every move.
mod cadence;
mod cards;
mod clock;
mod content;
mod folders;
mod layout;
mod live;
mod menu;
mod tile;
mod wallpaper;

use crate::{common, session::Session};
use ferroui_base::layout::{HorizontalAlignment as H, Orientation, VerticalAlignment as V};
use ferroui_controls::primitives::ScrollBarVisibility;
use ferroui_view::prelude::*;
use phone_controls::{Icon, icon};
use phone_domain::{Route, pack_tiles};
use phone_state::PhoneViewModel;
use std::{cell::Cell, rc::Rc};

pub(super) const GAP: f64 = 8.0;
pub(super) const PADDING: f64 = 14.0;

pub(crate) use clock::tick as tick_notifications;

pub fn view(vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Element {
    // Isolate the cadence dependency from the shell and other mounted pages.
    rerender(move || {
        // Loaded is deferred. Keep this first-mount policy through any
        // pre-Loaded reconciliation; later cadence updates animate normally.
        let mounted = use_hook(|| Rc::new(Cell::new(false)));
        let handoff = ui
            .state::<crate::page_motion::NavigationMotion>()
            .start_continues_neighbor();
        provide_context(phone_sdk::TileEntrancePolicy {
            animate: !vm.is_preview_snapshot() && !(handoff && !mounted.get()),
        });
        content_view(vm.clone(), ui.clone(), mounted)
    })
}

fn content_view(vm: Rc<PhoneViewModel>, ui: Rc<Session>, mounted: Rc<Cell<bool>>) -> Element {
    let width = crate::shell::phone_size(&vm).0;
    let data = vm.data();
    let tiles = data.tiles.clone();
    let columns = data.settings.columns();
    let wallpaper_motion = vm.motion_enabled();
    let folder_id = ui
        .edit_tile
        .get()
        .and_then(|id| data.parent_folder(id))
        .or(ui.folder.get())
        .or_else(|| match vm.route() {
            Route::Folder(id) => Some(id),
            _ => None,
        });
    let folder = folder_id.and_then(|id| data.tile(id).filter(|tile| tile.is_folder()).cloned());
    drop(data);
    clock::configure(&vm, &ui, &tiles);
    let cell = (width - PADDING * 2.0 - GAP * (columns as f64 - 1.0)) / columns as f64;
    let placements = pack_tiles(&tiles, columns);
    let folder_after = folder
        .as_ref()
        .and_then(|tile| layout::folder_band(&placements, tile.id));
    let folder_height = folder
        .as_ref()
        .map(|tile| folders::height(tile, cell, columns))
        .unwrap_or(0.0);
    let rows = layout::rows(&placements);
    let mut slots = Vec::<Element>::new();
    for placement in &placements {
        let item = tiles
            .iter()
            .find(|tile| tile.id == placement.id)
            .unwrap()
            .clone();
        let x = placement.column as f64 * (cell + GAP);
        let y = placement.row as f64 * (cell + GAP)
            + if folder_after.is_some_and(|after| placement.row >= after) {
                folder_height
            } else {
                0.0
            };
        let w = placement.width as f64 * (cell + GAP) - GAP;
        let h = placement.height as f64 * (cell + GAP) - GAP;
        slots.push(rsx!{Border{key:format!("tile-slot-{}",placement.id),Canvas.left:x,Canvas.top:y,width:w,height:h,
            {tile::view(vm.clone(),ui.clone(),item,w,h,x,y)}
        }});
    }
    if let (Some(folder), Some(after)) = (folder, folder_after) {
        let y = after as f64 * (cell + GAP);
        slots.push(
            rsx! {Border{key:format!("folder-band-{}",folder.id),Canvas.left:0.0,Canvas.top:y,
                    width:width-PADDING*2.0,height:folder_height-GAP,clip_to_bounds:true,
                    on_loaded:|event| {
                        if let Some(control) =
                            event.source().and_then(|source| source.cast::<Control>())
                        {
                            phone_controls::reveal(&control);
                        }
                    },
                    {folders::expanded(vm.clone(),ui.clone(),folder,cell,columns,y)}
            }},
        );
    }
    let surface = rsx! {Canvas{name:"StartTiles",width:width-PADDING*2.0,
        height:rows as f64*(cell+GAP)+folder_height,for slot in slots{{slot}}
    }};
    let all = vm.clone();
    let finish = ui.clone();
    let editing = ui.edit_tile.get().is_some();
    let remember = ui.clone();
    let restore = ui.clone();
    let scroll = rsx! {ScrollViewer{name:"StartScroll",key:"start-scroll",horizontal_scroll_bar_visibility:ScrollBarVisibility::Disabled,vertical_scroll_bar_visibility:ScrollBarVisibility::Hidden,
        on_scroll_changed:move|event|if let Some(scroll)=event.source().and_then(|source|source.cast::<ScrollViewer>()){
            remember.scrolls.borrow_mut().insert("start".into(),scroll.offset().y);
            wallpaper::update(&remember,&scroll,wallpaper_motion);
        },
        on_loaded:move|event|if let Some(scroll)=event.source().and_then(|source|source.cast::<ScrollViewer>()){
            let offset=restore.scrolls.borrow().get("start").copied().unwrap_or(0.0);scroll.set_offset(ferroui_base::Vector::new(0.0,offset));
            wallpaper::update(&restore,&scroll,wallpaper_motion);
        },
        StackPanel{margin:(PADDING,wallpaper::TOP_INSET,PADDING,24.0),spacing:18.0,
            {surface}
            Button{name:"AllApps",class:"phone-flat",height:58.0,horizontal_alignment:H::Right,automation_properties_name:if editing{"Finish arranging tiles"}else{"All apps"},
                on_click:move|_|{if editing{finish.edit_tile.set(None);finish.menu.set(false);}else{all.navigate(Route::Apps);}},
                StackPanel{orientation:Orientation::Horizontal,spacing:12.0,
                    TextBlock{text:if editing{"done"}else{"all apps"},font_size:21.0,vertical_alignment:V::Center},
                    Border{width:34.0,height:34.0,corner_radius:17.0,border_brush:common::fg(&vm).as_str(),border_thickness:2.0,{icon(if editing{Icon::Check}else{Icon::Forward},16.0,&common::fg(&vm))}}
                }
            }
        }
    }};
    let mut layers = vec![scroll];
    if ui.menu.get() && ui.edit_tile.get().is_some() {
        layers.push(menu::view(vm, ui));
    }
    rsx! {Grid{on_loaded:move |_|mounted.set(true),for layer in layers{{layer}}}}
}
