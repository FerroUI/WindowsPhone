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
        return viewer(vm, ui, photo);
    }
    let selected = ui.pivot.get().min(2);
    let mut tabs = stack_panel()
        .orientation(Orientation::Horizontal)
        .margin((20.0, 0.0));
    for (i, label) in ["all", "albums", "favorites"].iter().enumerate() {
        let switch = ui.clone();
        tabs = tabs.child(pivot(label, i == selected, move || switch.pivot.set(i)));
    }
    let content = if selected == 1 {
        let switch = ui.clone();
        scroll(row(
            "Camera Roll",
            &format!("{} photos", photos.len()),
            move || switch.pivot.set(0),
        ))
    } else {
        let visible = photos
            .into_iter()
            .filter(|p| selected != 2 || p.favorite)
            .collect::<Vec<_>>();
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
    let body = rsx! {Grid{row_definitions:"Auto,*",{tabs} Border{Grid.row:1,{content}}}};
    let import = vm.clone();
    let camera = vm.clone();
    let more = ui.clone();
    let secondary = vec![flat("select favorites", {
        let ui = ui.clone();
        move || {
            ui.pivot.set(2);
            ui.menu.set(false);
        }
    })];
    page(
        "photos",
        body,
        app_bar(
            vec![
                action(Icon::Plus, "add photos", move || {
                    import.request(HostRequest::ImportPhotos)
                }),
                action(Icon::Camera, "camera", move || {
                    camera.navigate(phone_domain::Route::App(phone_domain::AppId::Camera))
                }),
                action(Icon::More, "more", move || more.menu.update(|v| *v = !*v)),
            ],
            ui.menu.get(),
            secondary,
        ),
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
    let all = vm.data().photos.iter().map(|p| p.id).collect::<Vec<_>>();
    let prev_ids = all.clone();
    let navigation = rsx! {Grid{column_definitions:"Auto,*,Auto",margin:(12.0,0.0),
        Border{width:44.0,{flat("‹",move||step(&previous,&prev_ids,id,-1))}}
        TextBlock{Grid.column:1,text:photo.name.clone(),font_size:15.0,horizontal_alignment:H::Center,vertical_alignment:ferroui_base::layout::VerticalAlignment::Center}
        Border{Grid.column:2,width:44.0,{flat("›",move||step(&next,&all,id,1))}}
    }};
    let secondary = vec![
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
    let index = ids.iter().position(|id| *id == current).unwrap_or(0);
    let index = (index as isize + direction).rem_euclid(ids.len() as isize) as usize;
    ui.state::<crate::PhotosSession>().zoom.set(1.0);
    ui.selected.set(Some(ids[index]));
}

mod registration;
pub use registration::registration;
