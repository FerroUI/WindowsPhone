use crate::{common, session::Session};
use ferroui_base::layout::{HorizontalAlignment as H, VerticalAlignment as V};
use ferroui_view::prelude::*;
use phone_domain::{Id, TileSize};
use phone_state::PhoneViewModel;
use std::rc::Rc;

pub(super) fn view(vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Element {
    let Some(id) = ui.edit_tile.get() else {
        return grid().into();
    };
    let data = vm.data();
    let Some(tile) = data.tile(id).cloned() else {
        return grid().into();
    };
    let parent = data.parent_folder(id);
    let siblings = data
        .tile_siblings(id)
        .unwrap_or_default()
        .iter()
        .map(|tile| tile.id)
        .collect::<Vec<_>>();
    let index = siblings.iter().position(|item| *item == id).unwrap_or(0);
    let previous = index.checked_sub(1).map(|index| siblings[index]);
    let next = (index + 1 < siblings.len()).then(|| siblings.get(index + 2).copied());
    let background = if data.settings.dark {
        "#1F1F1F"
    } else {
        "#E6E6E6"
    };
    drop(data);
    let mut sizes = uniform_grid().columns(3).rows(1);
    for (size, label) in [
        (TileSize::Small, "small"),
        (TileSize::Medium, "medium"),
        (TileSize::Wide, "wide"),
    ] {
        let resize = vm.clone();
        let close = ui.clone();
        sizes = sizes.child(action(
            &format!("{}{}", if tile.size == size { "✓ " } else { "" }, label),
            move || {
                resize.set_tile_size(id, size);
                close.menu.set(false);
            },
        ));
    }
    let earlier = vm.clone();
    let later = vm.clone();
    let movement = rsx! {UniformGrid{columns:2,rows:1,
        Button{class:"phone-flat",is_enabled:previous.is_some(),height:48.0,on_click:move|_|if let Some(before)=previous{reorder(&earlier,parent,id,Some(before));},TextBlock{text:"move earlier",font_size:20.0}},
        Button{class:"phone-flat",is_enabled:next.is_some(),height:48.0,on_click:move|_|if let Some(before)=next{reorder(&later,parent,id,before);},TextBlock{text:"move later",font_size:20.0}}
    }};
    let mut commands = stack_panel()
        .spacing(0.0)
        .child(
            text_block()
                .text(tile.label.clone())
                .font_size(26.0)
                .margin((16.0, 12.0, 16.0, 8.0)),
        )
        .child(sizes)
        .child(movement);
    if let Some(parent) = parent {
        let extract = vm.clone();
        let close = ui.clone();
        commands = commands.child(action("move to Start", move || {
            extract.extract_folder_tile(parent, id);
            close.menu.set(false);
        }));
    }
    if tile.is_folder() {
        let rename = vm.clone();
        let rename_ui = ui.clone();
        let name = tile.label.clone();
        commands = commands.child(action("rename folder", move || {
            let rename = rename.clone();
            rename_ui.form(
                "folder name",
                vec![("name", name.clone())],
                Rc::new(move |fields| rename.rename_folder(id, fields[0].clone())),
            );
        }));
        let folder_ui = ui.clone();
        commands = commands.child(action("open folder", move || {
            folder_ui.folder.set(Some(id));
            folder_ui.edit_tile.set(None);
            folder_ui.menu.set(false);
        }));
    }
    let unpin = vm.clone();
    let unpin_ui = ui.clone();
    commands = commands.child(action("unpin from Start", move || {
        unpin.unpin(id);
        unpin_ui.edit_tile.set(None);
        unpin_ui.menu.set(false);
    }));
    let done = ui.clone();
    commands = commands.child(action("done", move || {
        done.menu.set(false);
        done.edit_tile.set(None);
    }));
    let dismiss = ui.clone();
    rsx! {Grid{background:"#66000000",
        Button{class:"phone-flat",horizontal_alignment:H::Stretch,vertical_alignment:V::Stretch,automation_properties_name:"Close tile actions",on_click:move|_|dismiss.menu.set(false)},
        Border{vertical_alignment:V::Bottom,background:background,padding:(6.0,0.0,6.0,8.0),{common::scroll(commands.into())}}
    }}
}
fn reorder(vm: &PhoneViewModel, folder: Option<Id>, tile: Id, before: Option<Id>) {
    if let Some(folder) = folder {
        vm.move_folder_tile(folder, tile, before);
    } else {
        vm.move_tile(tile, before);
    }
}
fn action(label: &str, callback: impl Fn() + 'static) -> Element {
    rsx! {Button{class:"phone-flat",height:48.0,horizontal_alignment:H::Stretch,horizontal_content_alignment:H::Left,padding:(14.0,5.0),on_click:move|_|callback(),TextBlock{text:label.to_owned(),font_size:21.0}}}
}
