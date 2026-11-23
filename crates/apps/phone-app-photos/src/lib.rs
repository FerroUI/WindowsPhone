mod albums;
mod state;
pub use state::PhotosSession;
mod photo_grid;
use ferroui_base::layout::{HorizontalAlignment as H, Orientation};
use ferroui_view::prelude::*;
use phone_controls::{action, app_bar, flat, pivot, Icon};
use phone_domain::{Action, Photo};
use phone_sdk::{common::*, images, session::Session};
use phone_state::{HostRequest, PhoneViewModel};
use std::rc::Rc;

pub fn view(vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Element {
    let photos = vm.data().photos.clone();
    if let Some(photo) = ui
        .selected
        .get()
        .and_then(|id| photos.iter().find(|p| p.id == id))
        .cloned()
    {
        if ui.state::<PhotosSession>().choosing_album.get() {
            return albums::chooser(vm, ui, photo.id);
        }
        return viewer(vm, ui, photo);
    }
    let selected = ui.pivot.get().min(2);
    let mut tabs = stack_panel()
        .orientation(Orientation::Horizontal)
        .margin((20.0, 0.0));
    for (i, label) in ["all", "albums", "favorites"].iter().enumerate() {
        let switch = ui.clone();
        tabs = tabs.child(pivot(label, i == selected, move || {
            switch.state::<PhotosSession>().selecting.set(false);
            switch
                .state::<PhotosSession>()
                .selection
                .set(Default::default());
            switch.pivot.set(i);
        }));
    }
    let tabs = phone_controls::pivot_header(tabs.into(), ["all", "albums", "favorites"][selected]);
    let album = ui.state::<PhotosSession>().album.get();
    let selecting = ui.state::<PhotosSession>().selecting.get();
    let content = if selected == 1 && album.is_none() {
        albums::list(vm.clone(), ui.clone())
    } else {
        let visible = if selecting {
            photos
        } else {
            albums::visible(&vm, &ui)
        };
        if visible.is_empty() {
            scroll(empty(
                if selected == 2 {
                    "your favorites"
                } else {
                    "your photos"
                },
                if selected == 2 {
                    "Open a photo and tap the heart to add it here."
                } else {
                    "Add photos or take a picture to fill your Camera Roll."
                },
            ))
        } else {
            let size = ((vm.viewport().0.min(400.0) - 56.0) / 3.0).max(70.0);
            photo_grid::view(visible, size, ui.clone())
        }
    };
    let album_header = if selected == 1 {
        album.and_then(|id| {
            albums::read(&vm)
                .ok()
                .and_then(|value| value.albums.get(id).map(|group| group.name.clone()))
        })
    } else {
        None
    };
    let mut header = stack_panel().child(tabs);
    if let Some(name) = album_header {
        header = header.child(border().margin((20.0, 6.0)).child(text(&name, 28.0)));
    }
    if selecting {
        header = header.child(border().margin((20.0, 6.0)).child(muted(
            &format!(
                "{} selected",
                ui.state::<PhotosSession>().selection.get().len()
            ),
            16.0,
        )));
    }
    let body = rsx! {Grid{row_definitions:"Auto,*",{header} Border{Grid.row:1,{content}}}};
    let import = vm.clone();
    let camera = vm.clone();
    let more = ui.clone();
    let new_vm = vm.clone();
    let new_ui = ui.clone();
    let mut secondary = vec![
        flat("new album", move || albums::create(&new_vm, &new_ui, None)),
        flat("select favorites", {
            let ui = ui.clone();
            move || {
                ui.pivot.set(2);
                ui.menu.set(false);
            }
        }),
    ];
    if let Some(id) = album.filter(|_| selected == 1) {
        let (edit_vm, edit_ui, rename_vm, rename_ui, delete_vm, delete_ui) = (
            vm.clone(),
            ui.clone(),
            vm.clone(),
            ui.clone(),
            vm.clone(),
            ui.clone(),
        );
        secondary.push(flat("add or remove photos", move || {
            edit_ui.menu.set(false);
            albums::select(&edit_vm, &edit_ui, id);
        }));
        secondary.push(flat("rename album", move || {
            albums::rename(&rename_vm, &rename_ui, id)
        }));
        secondary.push(flat("delete album", move || {
            albums::delete(&delete_vm, &delete_ui, id)
        }));
    }
    let actions = if selecting {
        let save_vm = vm.clone();
        let save_ui = ui.clone();
        let cancel = ui.clone();
        vec![
            action(Icon::Check, "done", move || {
                albums::save_selection(&save_vm, &save_ui)
            }),
            action(Icon::Close, "cancel", move || {
                cancel.state::<PhotosSession>().selecting.set(false);
                cancel
                    .state::<PhotosSession>()
                    .selection
                    .set(Default::default());
            }),
        ]
    } else {
        vec![
            action(Icon::Plus, "add photos", move || {
                import.request(HostRequest::ImportPhotos)
            }),
            action(Icon::Camera, "camera", move || {
                camera.navigate(phone_domain::Route::App(phone_domain::AppId::Camera))
            }),
            action(Icon::More, "more", move || more.menu.update(|v| *v = !*v)),
        ]
    };
    page(
        "photos",
        body,
        app_bar(actions, ui.menu.get() && !selecting, secondary),
    )
}

fn viewer(vm: Rc<PhoneViewModel>, ui: Rc<Session>, photo: Photo) -> Element {
    let favorite = vm.clone();
    let id = photo.id;
    let sharing = vm.clone();
    let source = photo.source.clone();
    let name = photo.name.clone();
    let remove_vm = vm.clone();
    let remove_ui = ui.clone();
    let removed_source = photo.source.clone();
    let more = ui.clone();
    let zoom = ui.state::<crate::PhotosSession>().zoom.get();
    let zoom_in = ui.clone();
    let zoom_out = ui.clone();
    let mut picture = grid().background("Black").clip_to_bounds(true);
    if zoom > 1.0 {
        picture = picture.child(
            scroll_viewer()
                .horizontal_scroll_bar_visibility(
                    ferroui_controls::primitives::ScrollBarVisibility::Hidden,
                )
                .vertical_scroll_bar_visibility(
                    ferroui_controls::primitives::ScrollBarVisibility::Hidden,
                )
                .child(
                    border()
                        .width(vm.viewport().0.min(400.0) * zoom)
                        .height((vm.viewport().1 - 180.0).max(200.0) * zoom)
                        .child(images::photo_fit(&photo.source)),
                ),
        );
    } else {
        picture = picture.child(images::photo_fit(&photo.source));
    }
    let previous = ui.clone();
    let next = ui.clone();
    let all = albums::visible(&vm, &ui)
        .iter()
        .map(|p| p.id)
        .collect::<Vec<_>>();
    let prev_ids = all.clone();
    let navigation = rsx! {Grid{column_definitions:"Auto,*,Auto",margin:(12.0,0.0),
        Border{width:44.0,{flat("‹",move||step(&previous,&prev_ids,id,-1))}}
        TextBlock{Grid.column:1,text:photo.name.clone(),font_size:15.0,horizontal_alignment:H::Center,vertical_alignment:ferroui_base::layout::VerticalAlignment::Center}
        Border{Grid.column:2,width:44.0,{flat("›",move||step(&next,&all,id,1))}}
    }};
    let album_ui = ui.clone();
    let rename_vm = vm.clone();
    let rename_ui = ui.clone();
    let mut secondary = vec![
        flat("add to album", move || {
            album_ui.menu.set(false);
            album_ui.state::<PhotosSession>().choosing_album.set(true);
        }),
        flat("rename photo", move || {
            albums::rename_photo(&rename_vm, &rename_ui, id)
        }),
        flat("zoom in", move || {
            zoom_in
                .state::<crate::PhotosSession>()
                .zoom
                .update(|z| *z = (*z + 0.5).min(3.0))
        }),
        flat("zoom out", move || {
            zoom_out
                .state::<crate::PhotosSession>()
                .zoom
                .update(|z| *z = (*z - 0.5).max(1.0))
        }),
        flat("delete", move || {
            let vm = remove_vm.clone();
            let ui = remove_ui.clone();
            let source = removed_source.clone();
            remove_ui.confirm(
                "Delete this photo?",
                Rc::new(move || {
                    if let Err(error) = albums::forget_photo(&vm, id) {
                        ui.form_error.set(error);
                        return;
                    }
                    vm.dispatch(Action::DeletePhoto(id));
                    if !vm.data().photos.iter().any(|photo| photo.source == source) {
                        images::forget(&source);
                        if source.starts_with("media:") {
                            vm.request(HostRequest::DeleteMedia {
                                source: source.clone(),
                            });
                        }
                    }
                    ui.selected.set(None);
                    ui.dialog.set(None);
                }),
            );
        }),
    ];
    if let Some(album) = ui
        .state::<PhotosSession>()
        .album
        .get()
        .filter(|_| ui.pivot.get() == 1)
    {
        let remove_vm = vm.clone();
        let close = ui.clone();
        secondary.push(flat("remove from album", move || {
            albums::remove_photo(&remove_vm, album, id);
            close.selected.set(None);
            close.menu.set(false);
        }));
    }
    rsx! {Grid{row_definitions:"Auto,*,Auto",{navigation}
        Border{Grid.row:1,{picture}}
        Border{Grid.row:2,{app_bar(vec![
            action(Icon::Heart,if photo.favorite{"remove favorite"}else{"favorite"},move||favorite.dispatch(Action::ToggleFavoritePhoto(id))),
            action(Icon::Share,"share",move||sharing.request(HostRequest::Share{title:name.clone(),text:name.clone(),url:source.clone()})),
            action(Icon::More,"more",move||more.menu.update(|v|*v=!*v)),
        ],ui.menu.get(),secondary)}}
    }}
}

fn step(ui: &Session, ids: &[u64], current: u64, direction: isize) {
    if ids.is_empty() {
        return;
    }
    let index = ids
        .iter()
        .position(|id| *id == current)
        .map(|index| (index as isize + direction).rem_euclid(ids.len() as isize) as usize)
        .unwrap_or(if direction < 0 { ids.len() - 1 } else { 0 });
    ui.state::<crate::PhotosSession>().zoom.set(1.0);
    ui.selected.set(Some(ids[index]));
}

mod registration;
pub use registration::registration;
