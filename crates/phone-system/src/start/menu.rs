use crate::{common, session::Session};
use ferroui_base::layout::{HorizontalAlignment as H, VerticalAlignment as V};
use ferroui_view::prelude::*;
use phone_domain::{Id, Tile, TileAppearance, TileBackground, TileSize};
use phone_state::PhoneViewModel;
use std::{cell::Cell, rc::Rc};

mod notifications;

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum Page {
    #[default]
    Main,
    Appearance,
    Photos,
    Folder,
    CreateFolder,
    Notifications,
}
struct State {
    tile: Cell<Option<(Id, u64)>>,
    page: Signal<Page>,
    query: Signal<String>,
    photo_page: Signal<usize>,
}
impl Default for State {
    fn default() -> Self {
        Self {
            tile: Cell::new(None),
            page: signal(Page::Main),
            query: signal(String::new()),
            photo_page: signal(0),
        }
    }
}
pub(super) fn toggle(ui: &Session) {
    if !ui.menu.get_untracked() {
        ui.state::<State>().page.set(Page::Main);
    }
    ui.menu.update(|value| *value = !*value);
}

pub(super) fn view(vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Element {
    let Some(id) = ui.edit_tile.get() else {
        return grid().into();
    };
    let Some(tile) = vm.data().tile(id).cloned() else {
        return grid().into();
    };
    let state = ui.state::<State>();
    if state.tile.replace(Some((id, vm.document_epoch()))) != Some((id, vm.document_epoch())) {
        state.page.set(Page::Main);
        state.query.set(String::new());
        state.photo_page.set(0);
    }
    let page = state.page.get();
    let heading = match page {
        Page::Main => tile.display_label(),
        Page::Appearance => "customize tile",
        Page::Photos => "tile background",
        Page::Folder => "move to folder",
        Page::CreateFolder => "create a folder with",
        Page::Notifications => "live cards",
    };
    let mut body = stack_panel().spacing(0.0).child(
        text_block()
            .text(heading.to_owned())
            .font_size(26.0)
            .margin((16.0, 12.0, 16.0, 8.0)),
    );
    if page != Page::Main {
        let state = state.clone();
        body = body.child(action("back to tile actions", move || {
            state.page.set(Page::Main)
        }));
    }
    body = body.child(match page {
        Page::Main => main(vm.clone(), ui.clone(), &tile),
        Page::Appearance => appearance(vm.clone(), ui.clone(), &tile),
        Page::Photos => photos(vm.clone(), ui.clone(), id),
        Page::Folder | Page::CreateFolder => {
            folders(vm.clone(), ui.clone(), id, page == Page::CreateFolder)
        }
        Page::Notifications => notifications::view(vm.clone(), ui.clone(), &tile),
    });
    let done = ui.clone();
    body = body.child(action("done", move || {
        done.menu.set(false);
        done.edit_tile.set(None);
    }));
    let background = if vm.data().settings.dark {
        "#1F1F1F"
    } else {
        "#E6E6E6"
    };
    let max_height = (crate::shell::phone_size(&vm).1 - 100.0).max(160.0);
    let dismiss = ui.clone();
    rsx! {Grid{background:"#66000000",
        Button{class:"phone-flat",horizontal_alignment:H::Stretch,vertical_alignment:V::Stretch,automation_properties_name:"Close tile actions",on_click:move|_|dismiss.menu.set(false)},
        Border{name:"TileMenuSurface",vertical_alignment:V::Bottom,max_height:max_height,background:background,padding:(6.0,0.0,6.0,8.0),{common::scroll(body.into())}}
    }}
}

fn main(vm: Rc<PhoneViewModel>, ui: Rc<Session>, tile: &Tile) -> Element {
    let id = tile.id;
    let data = vm.data();
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
    let first = siblings.first().copied().filter(|first| *first != id);
    let has_folders = data
        .tiles
        .iter()
        .any(|value| value.is_folder() && value.id != parent.unwrap_or(0));
    let can_create = data
        .tiles
        .iter()
        .any(|value| !value.is_folder() && value.id != id);
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
    let top = vm.clone();
    let bottom = vm.clone();
    let movement = rsx! {UniformGrid{columns:2,rows:2,
        Button{name:"TileMoveEarlier",class:"phone-flat",is_enabled:previous.is_some(),height:48.0,automation_properties_name:"Move tile earlier",on_click:move|_|if let Some(before)=previous{reorder(&earlier,parent,id,Some(before));},TextBlock{text:"move earlier",font_size:20.0}},
        Button{name:"TileMoveLater",class:"phone-flat",is_enabled:next.is_some(),height:48.0,automation_properties_name:"Move tile later",on_click:move|_|if let Some(before)=next{reorder(&later,parent,id,before);},TextBlock{text:"move later",font_size:20.0}},
        Button{name:"TileMoveFirst",class:"phone-flat",is_enabled:first.is_some(),height:48.0,automation_properties_name:"Move tile to first position",on_click:move|_|if let Some(before)=first{reorder(&top,parent,id,Some(before));},TextBlock{text:"move to first",font_size:20.0}},
        Button{name:"TileMoveLast",class:"phone-flat",is_enabled:next.is_some(),height:48.0,automation_properties_name:"Move tile to last position",on_click:move|_|reorder(&bottom,parent,id,None),TextBlock{text:"move to last",font_size:20.0}}
    }};
    let customize = ui.state::<State>();
    let mut commands = stack_panel()
        .spacing(0.0)
        .child(sizes)
        .child(movement)
        .child(action("customize", move || {
            customize.page.set(Page::Appearance)
        }));
    if let Some(parent) = parent {
        let extract = vm.clone();
        let close = ui.clone();
        commands = commands.child(action("move to Start", move || {
            extract.extract_folder_tile(parent, id);
            close.menu.set(false);
        }));
    }
    if !tile.is_folder() {
        if has_folders {
            let state = ui.state::<State>();
            commands = commands.child(action("move to folder", move || {
                state.page.set(Page::Folder)
            }));
        }
        if can_create {
            let state = ui.state::<State>();
            commands = commands.child(action("create folder", move || {
                state.page.set(Page::CreateFolder)
            }));
        }
        let state = ui.state::<State>();
        commands = commands.child(action("live cards", move || {
            state.page.set(Page::Notifications)
        }));
    } else {
        let rename = vm.clone();
        let rename_ui = ui.clone();
        let name = tile.label.clone();
        commands = commands.child(action("rename folder", move || {
            let vm = rename.clone();
            rename_ui.form(
                "folder name",
                vec![("name", name.clone())],
                Rc::new(move |fields| vm.rename_folder(id, fields[0].clone())),
            );
        }));
        let folder_ui = ui.clone();
        commands = commands.child(action("open folder", move || {
            folder_ui.folder.set(Some(id));
            folder_ui.edit_tile.set(None);
            folder_ui.menu.set(false);
        }));
        let dissolve = vm.clone();
        let confirm = ui.clone();
        commands = commands.child(action("dissolve folder", move || {
            let vm = dissolve.clone();
            let ui = confirm.clone();
            confirm.confirm(
                "Move all folder tiles to Start?",
                Rc::new(move || {
                    vm.dissolve_folder(id);
                    ui.folder.set(None);
                    ui.edit_tile.set(None);
                }),
            );
        }));
    }
    let unpin = vm.clone();
    let unpin_ui = ui.clone();
    commands
        .child(action("unpin from Start", move || {
            unpin.unpin(id);
            unpin_ui.edit_tile.set(None);
            unpin_ui.menu.set(false);
        }))
        .into()
}

fn update(
    vm: &PhoneViewModel,
    id: Id,
    change: impl FnOnce(&mut TileAppearance),
) -> Result<(), String> {
    let mut appearance = vm
        .data()
        .tile(id)
        .ok_or("This tile is no longer pinned.")?
        .appearance
        .clone();
    change(&mut appearance);
    vm.set_tile_appearance(id, appearance)
}
fn apply(vm: &PhoneViewModel, id: Id, change: impl FnOnce(&mut TileAppearance)) {
    if let Err(error) = update(vm, id, change) {
        vm.set_status(error);
    }
}
fn appearance(vm: Rc<PhoneViewModel>, ui: Rc<Session>, tile: &Tile) -> Element {
    let id = tile.id;
    let rename = vm.clone();
    let rename_ui = ui.clone();
    let label = tile.appearance.label.clone().unwrap_or_default();
    let mut body = stack_panel()
        .spacing(4.0)
        .child(action("tile name", move || {
            let vm = rename.clone();
            let ui = rename_ui.clone();
            rename_ui.form(
                "tile name",
                vec![("name (blank uses app name)", label.clone())],
                Rc::new(move |fields| {
                    let value = fields[0].trim();
                    if let Err(error) = update(&vm, id, |appearance| {
                        appearance.label = (!value.is_empty()).then(|| value.to_owned())
                    }) {
                        ui.form_error.set(error);
                    }
                }),
            );
        }));
    for (label, background) in [
        ("app background", TileBackground::App),
        ("accent background", TileBackground::Accent),
    ] {
        let change = vm.clone();
        body = body.child(action(label, move || {
            apply(&change, id, |appearance| {
                appearance.background = background.clone()
            })
        }));
    }
    let color = vm.clone();
    let color_ui = ui.clone();
    let current = match &tile.appearance.background {
        TileBackground::Color(value) => value.clone(),
        _ => vm.data().settings.accent.clone(),
    };
    body = body.child(action("custom color", move || {
        let vm = color.clone();
        let ui = color_ui.clone();
        color_ui.form(
            "tile color",
            vec![("#RRGGBB", current.clone())],
            Rc::new(move |fields| {
                if let Err(error) = update(&vm, id, |appearance| {
                    appearance.background = TileBackground::Color(fields[0].trim().to_owned())
                }) {
                    ui.form_error.set(error);
                }
            }),
        );
    }));
    let photos = ui.state::<State>();
    body = body.child(action("photo background", move || {
        photos.query.set(String::new());
        photos.photo_page.set(0);
        photos.page.set(Page::Photos);
    }));
    for (label, value, field) in [
        ("show tile name", tile.appearance.show_label, 0),
        ("show badge", tile.appearance.show_badge, 1),
        ("live content", tile.appearance.live, 2),
        ("rotate queued cards", tile.appearance.queue, 3),
    ] {
        if tile.is_folder() && field >= 1 {
            continue;
        }
        let vm = vm.clone();
        body = body.child(border().margin((14.0, 0.0)).child(phone_controls::toggle(
            label,
            value,
            move |value| {
                apply(&vm, id, |appearance| match field {
                    0 => appearance.show_label = value,
                    1 => appearance.show_badge = value,
                    2 => appearance.live = value,
                    _ => appearance.queue = value,
                })
            },
        )));
    }
    let reset = vm.clone();
    body = body.child(action("reset appearance", move || {
        if let Err(error) = reset.set_tile_appearance(id, TileAppearance::default()) {
            reset.set_status(error);
        }
    }));
    body.into()
}

fn photos(vm: Rc<PhoneViewModel>, ui: Rc<Session>, id: Id) -> Element {
    const LIMIT: usize = 18;
    let state = ui.state::<State>();
    let query = state.query.get().to_lowercase();
    let photos = vm
        .data()
        .photos
        .iter()
        .filter(|photo| photo.name.to_lowercase().contains(&query))
        .cloned()
        .collect::<Vec<_>>();
    let page = state
        .photo_page
        .get()
        .min(photos.len().saturating_sub(1) / LIMIT);
    let mut body = stack_panel().spacing(8.0).margin((14.0, 4.0));
    body = body.child(phone_controls::input("search photos").text(state.query.two_way()));
    for photo in photos.iter().skip(page * LIMIT).take(LIMIT) {
        let vm = vm.clone();
        let state = state.clone();
        let photo_id = photo.id;
        body=body.child(rsx! {Button{name:format!("TilePhoto-{photo_id}"),class:"phone-flat",height:74.0,horizontal_content_alignment:H::Stretch,automation_properties_name:format!("Use {} as tile background",photo.name),
            on_click:move|_|{apply(&vm,id,|appearance|appearance.background=TileBackground::Photo(photo_id));state.page.set(Page::Appearance);},
            Grid{column_definitions:"64,*",column_spacing:12.0,Border{width:64.0,height:64.0,clip_to_bounds:true,{crate::images::thumbnail(&photo.source)}},TextBlock{Grid.column:1,text:photo.name.clone(),font_size:21.0,vertical_alignment:V::Center}}
        }});
    }
    if photos.is_empty() {
        body = body.child(common::muted(
            "No matching photos. Add a photo in Photos first.",
            18.0,
        ));
    }
    if page > 0 {
        let state = state.clone();
        body = body.child(action("previous photos", move || {
            state.photo_page.set(page - 1)
        }));
    }
    if (page + 1) * LIMIT < photos.len() {
        let state = state.clone();
        body = body.child(action("next photos", move || {
            state.photo_page.set(page + 1)
        }));
    }
    body.into()
}
fn folders(vm: Rc<PhoneViewModel>, ui: Rc<Session>, id: Id, create: bool) -> Element {
    let data = vm.data();
    let parent = data.parent_folder(id);
    let candidates = data
        .tiles
        .iter()
        .filter(|tile| tile.id != id && Some(tile.id) != parent && tile.is_folder() != create)
        .cloned()
        .collect::<Vec<_>>();
    drop(data);
    let mut body = stack_panel().spacing(0.0);
    for tile in candidates {
        let vm = vm.clone();
        let ui = ui.clone();
        let target = tile.id;
        body = body.child(action(tile.display_label(), move || {
            if create {
                let vm = vm.clone();
                let done = ui.clone();
                ui.form(
                    "folder name",
                    vec![("name", "folder".into())],
                    Rc::new(move |fields| {
                        vm.create_folder(id, target, fields[0].clone());
                        done.edit_tile.set(None);
                    }),
                );
            } else {
                vm.move_into_folder(id, target);
                ui.folder.set(Some(target));
                ui.edit_tile.set(None);
                ui.menu.set(false);
            }
        }));
    }
    body.into()
}
fn reorder(vm: &PhoneViewModel, folder: Option<Id>, tile: Id, before: Option<Id>) {
    if let Some(folder) = folder {
        vm.move_folder_tile(folder, tile, before);
    } else {
        vm.move_tile(tile, before);
    }
}
fn action(label: &str, callback: impl Fn() + 'static) -> Element {
    rsx! {Button{name:format!("TileMenu-{}",label.replace(' ',"-")),class:"phone-flat",min_height:48.0,horizontal_alignment:H::Stretch,horizontal_content_alignment:H::Left,padding:(14.0,5.0),automation_properties_name:label.to_owned(),on_click:move|_|callback(),TextBlock{text:label.to_owned(),font_size:21.0,text_wrapping:ferroui_base::media::TextWrapping::Wrap}}}
}
