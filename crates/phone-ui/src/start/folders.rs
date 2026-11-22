use super::{GAP, layout, tile};
use crate::{common, session::Session};
use ferroui_base::layout::{HorizontalAlignment as H, VerticalAlignment as V};
use ferroui_view::prelude::*;
use phone_controls::icon;
use phone_domain::{Tile, TileSize, TileTarget, pack_tiles};
use phone_state::PhoneViewModel;
use std::rc::Rc;

const HEADER: f64 = 50.0;

pub(super) fn height(folder: &Tile, cell: f64, columns: usize) -> f64 {
    HEADER + layout::rows(&pack_tiles(&folder.folder_tiles, columns)) as f64 * (cell + GAP) + GAP
}

pub(super) fn cover(vm: &PhoneViewModel, folder: &Tile, w: f64, h: f64) -> Element {
    let small = folder.size == TileSize::Small;
    let preview: Vec<Tile> = folder
        .folder_tiles
        .iter()
        .take(if small { 4 } else { 8 })
        .cloned()
        .collect();
    let places = pack_tiles(&preview, 4);
    let rows = layout::rows(&places).max(1);
    let available_h = (h - if small { 16.0 } else { 38.0 }).max(1.0);
    let slot = ((w - 16.0) / 4.0).min(available_h / rows as f64);
    let data = vm.data();
    let mut surface = canvas()
        .width(slot * 4.0)
        .height(slot * rows as f64)
        .horizontal_alignment(H::Left)
        .vertical_alignment(V::Top)
        .margin((8.0, 8.0, 8.0, 0.0));
    for place in places {
        let child = preview.iter().find(|tile| tile.id == place.id).unwrap();
        let color = if child.app.color().is_empty() {
            data.settings.accent.as_str()
        } else {
            child.app.color()
        };
        let width = place.width as f64 * slot - 3.0;
        let height = place.height as f64 * slot - 3.0;
        let glyph = if let Some(TileTarget::Contact(id)) = child.target {
            rsx! {TextBlock{text:data.contact(id).map(|contact|contact.initials()).unwrap_or_default(),font_size:(height*0.45).min(22.0),foreground:"White",horizontal_alignment:H::Center,vertical_alignment:V::Center}}
        } else {
            icon(
                common::app_icon(child.app),
                (width.min(height) * 0.55).clamp(8.0, 28.0),
                "White",
            )
        };
        surface = surface.child(
            border()
                .canvas_left(place.column as f64 * slot)
                .canvas_top(place.row as f64 * slot)
                .width(width)
                .height(height)
                .background(color)
                .child(glyph),
        );
    }
    surface.into()
}

pub(super) fn expanded(
    vm: Rc<PhoneViewModel>,
    ui: Rc<Session>,
    folder: Tile,
    cell: f64,
    columns: usize,
    y: f64,
) -> Element {
    let placements = pack_tiles(&folder.folder_tiles, columns);
    let mut surface = canvas()
        .name(format!("FolderTiles-{}", folder.id))
        .height(layout::rows(&placements) as f64 * (cell + GAP))
        .width(columns as f64 * (cell + GAP) - GAP);
    for place in placements {
        let child = folder
            .folder_tiles
            .iter()
            .find(|tile| tile.id == place.id)
            .unwrap()
            .clone();
        let x = place.column as f64 * (cell + GAP);
        let top = place.row as f64 * (cell + GAP);
        let w = place.width as f64 * (cell + GAP) - GAP;
        let h = place.height as f64 * (cell + GAP) - GAP;
        surface = surface.child(
            border()
                .canvas_left(x)
                .canvas_top(top)
                .width(w)
                .height(h)
                .child(tile::view(
                    vm.clone(),
                    ui.clone(),
                    child,
                    w,
                    h,
                    x,
                    y + HEADER + top,
                )),
        );
    }
    let rename = ui.clone();
    let rename_vm = vm.clone();
    let name = folder.label.clone();
    let id = folder.id;
    let accent = common::accent(&vm);
    rsx! {Grid{row_definitions:"50,*",
        Border{height:2.0,vertical_alignment:V::Top,background:accent.as_str()},
        Button{name:format!("FolderName-{id}"),class:"phone-flat",horizontal_content_alignment:H::Left,padding:(0.0,4.0),automation_properties_name:format!("Rename folder {}",folder.label),
            on_click:move|_|{let vm=rename_vm.clone();rename.form("folder name",vec![("name",name.clone())],Rc::new(move|fields|vm.rename_folder(id,fields[0].clone())));},
            TextBlock{text:folder.label,font_size:20.0,foreground:accent.as_str()}
        },
        Border{Grid.row:1,{surface}}
    }}
}
