use super::{GAP, content, layout, tile};
use crate::{common, session::Session};
use ferroui_base::layout::{HorizontalAlignment as H, VerticalAlignment as V};
use ferroui_view::prelude::*;
use phone_controls::{Icon, icon};
use phone_domain::{Tile, TileSize, pack_tiles};
use phone_state::PhoneViewModel;
use std::rc::Rc;

const HEADER: f64 = 50.0;

pub(super) fn height(folder: &Tile, cell: f64, columns: usize) -> f64 {
    HEADER + layout::rows(&pack_tiles(&folder.folder_tiles, columns)) as f64 * (cell + GAP) + GAP
}

pub(super) fn cover(
    vm: Rc<PhoneViewModel>,
    ui: Rc<Session>,
    folder: &Tile,
    w: f64,
    h: f64,
) -> Element {
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
        let badge = if child.target.is_none() {
            data.badge(child.app)
        } else {
            0
        };
        let reference_h = if child.size == TileSize::Small {
            86.0
        } else {
            180.0
        };
        let reference_w = reference_h * place.width as f64 / place.height as f64;
        let inner = content::view(
            vm.clone(),
            ui.clone(),
            child,
            reference_w,
            reference_h,
            badge,
        );
        let glyph = rsx! {Viewbox{stretch:ferroui_base::media::Stretch::Uniform,
            Grid{width:reference_w,height:reference_h,clip_to_bounds:true,
                {inner}
                if child.size!=TileSize::Small{TextBlock{text:child.label.clone(),font_size:14.0,foreground:"White",vertical_alignment:V::Bottom,margin:(9.0,0.0,5.0,7.0)}}
            }
        }};
        surface = surface.child(
            border()
                .key(format!("folder-preview-{}", child.id))
                .canvas_left(place.column as f64 * slot)
                .canvas_top(place.row as f64 * slot)
                .width(width)
                .height(height)
                .background(color)
                .clip_to_bounds(true)
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
                .key(format!("folder-child-{}", child.id))
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
    let close = ui.clone();
    rsx! {Grid{row_definitions:"50,*",
        Border{height:2.0,vertical_alignment:V::Top,background:accent.as_str()},
        Grid{column_definitions:"*,44",
        Button{name:format!("FolderName-{id}"),class:"phone-flat",horizontal_content_alignment:H::Left,padding:(0.0,4.0),automation_properties_name:format!("Rename folder {}",folder.label),
            on_click:move|_|{let vm=rename_vm.clone();rename.form("folder name",vec![("name",name.clone())],Rc::new(move|fields|vm.rename_folder(id,fields[0].clone())));},
            TextBlock{text:folder.label,font_size:20.0,foreground:accent.as_str()}
        },
        Button{Grid.column:1,class:"phone-flat",automation_properties_name:"Close folder",on_click:move|_|{close.folder.set(None);close.edit_tile.set(None);},{icon(Icon::Close,16.0,&accent)}}
        },
        Border{Grid.row:1,{surface}}
    }}
}
