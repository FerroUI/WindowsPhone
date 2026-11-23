use crate::{
    commands,
    model::{self, Library},
    state::{LibraryPage, LibrarySession},
};
use ferroui_base::{
    layout::{HorizontalAlignment as H, Orientation, VerticalAlignment as V},
    BoxedValue,
};
use ferroui_controls::ItemsSource;
use ferroui_view::prelude::*;
use phone_controls::{icon, input, Icon};
use phone_domain::{AppId, Id, MediaTrack};
use phone_sdk::{common::*, session::Session};
use phone_state::PhoneViewModel;
use std::{cell::RefCell, collections::BTreeMap, rc::Rc};

pub fn view(
    vm: Rc<PhoneViewModel>,
    ui: Rc<Session>,
    app: AppId,
    library: &Library,
    tracks: Vec<MediaTrack>,
) -> Element {
    let state = ui.state::<LibrarySession>();
    let section = state.page(app).get();
    let mut header = stack_panel().spacing(5.0).margin((20.0, 4.0, 20.0, 8.0));
    let mut categories = stack_panel()
        .orientation(Orientation::Horizontal)
        .spacing(14.0);
    let categories_list = if app == AppId::Music {
        vec![
            ("songs", LibraryPage::Songs),
            ("artists", LibraryPage::Artists),
            ("albums", LibraryPage::Albums),
            ("playlists", LibraryPage::Playlists),
        ]
    } else {
        vec![
            ("videos", LibraryPage::Songs),
            ("playlists", LibraryPage::Playlists),
        ]
    };
    for (label, target) in categories_list {
        let destination = target.clone();
        let switch = ui.clone();
        let chosen = section == target;
        categories=categories.child(rsx!{Button{class:"phone-flat",min_height:44.0,padding:0.0,automation_properties_name:label,
            on_click:move |_|{switch.state::<LibrarySession>().page(app).set(destination.clone());switch.query.set(String::new());},
            TextBlock{text:label,font_size:18.0,foreground:if chosen{phone_controls::palette().foreground}else{phone_controls::secondary_color()}}
        }});
    }
    header = header.child(categories);
    if !matches!(section, LibraryPage::Select(_)) {
        header = header.child(
            input("Search collection")
                .name("MediaSearch")
                .text(ui.query.two_way()),
        );
    }
    let query = ui.query.get().trim().to_lowercase();
    let body = match &section {
        LibraryPage::Artists | LibraryPage::Albums => {
            let albums = section == LibraryPage::Albums;
            let mut groups = BTreeMap::<String, usize>::new();
            for track in &tracks {
                let key = if albums {
                    library.album(track.id)
                } else {
                    commands::artist_name(track)
                };
                *groups.entry(key.into()).or_default() += 1;
            }
            let mut items = stack_panel();
            for (name, count) in groups
                .into_iter()
                .filter(|(name, _)| name.to_lowercase().contains(&query))
            {
                let switch = ui.clone();
                let target = if albums {
                    LibraryPage::Album(name.clone())
                } else {
                    LibraryPage::Artist(name.clone())
                };
                items = items.child(row(&name, &format!("{count} songs"), move || {
                    switch
                        .state::<LibrarySession>()
                        .page(app)
                        .set(target.clone());
                    switch.query.set(String::new());
                }));
            }
            if tracks.is_empty() {
                items = items.child(empty(
                    "your collection",
                    "Add music to browse by artist and album.",
                ));
            }
            scroll(items.into())
        }
        LibraryPage::Playlists => {
            let new_vm = vm.clone();
            let new_ui = ui.clone();
            let mut items = stack_panel().child(row(
                "new playlist",
                "Choose and arrange your collection.",
                move || commands::create_playlist(&new_vm, &new_ui, app),
            ));
            for group in library
                .playlists
                .groups
                .iter()
                .filter(|group| group.name.to_lowercase().contains(&query))
            {
                let count = group
                    .items
                    .iter()
                    .filter(|id| tracks.iter().any(|track| track.id == **id))
                    .count();
                let id = group.id;
                let switch = ui.clone();
                items = items.child(row(&group.name, &format!("{count} items"), move || {
                    switch
                        .state::<LibrarySession>()
                        .page(app)
                        .set(LibraryPage::Playlist(id));
                    switch.query.set(String::new());
                }));
            }
            scroll(items.into())
        }
        LibraryPage::Select(id) => {
            let name = library
                .playlists
                .get(*id)
                .map(|p| p.name.as_str())
                .unwrap_or("playlist");
            header = header.child(text(name, 27.0)).child(muted(
                &format!("{} selected", state.selection.get().len()),
                15.0,
            ));
            track_list(
                vm.clone(),
                ui.clone(),
                app,
                tracks.clone(),
                library,
                None,
                true,
            )
        }
        LibraryPage::Playlist(id) => {
            if let Some(group) = library.playlists.get(*id) {
                let id = *id;
                header = header.child(text(&group.name, 29.0));
                let (add_vm, add_ui, rename_vm, rename_ui, delete_vm, delete_ui) = (
                    vm.clone(),
                    ui.clone(),
                    vm.clone(),
                    ui.clone(),
                    vm.clone(),
                    ui.clone(),
                );
                header=header.child(rsx!{Grid{column_definitions:"*,*,*",column_spacing:12.0,
                    {small_button("add items",move||commands::begin_selection(&add_vm,&add_ui,app,id))}
                    Border{Grid.column:1,{small_button("rename",move||commands::rename_playlist(&rename_vm,&rename_ui,app,id))}}
                    Border{Grid.column:2,{small_button("delete",move||commands::delete_playlist(&delete_vm,&delete_ui,app,id))}}
                }});
                let visible = commands::visible(library, &section, &tracks);
                let visible = filter(visible, &query, library);
                if visible.is_empty() {
                    scroll(empty(
                        "your playlist",
                        "Tap add items to choose from your collection.",
                    ))
                } else {
                    track_list(
                        vm.clone(),
                        ui.clone(),
                        app,
                        visible,
                        library,
                        Some(id),
                        false,
                    )
                }
            } else {
                scroll(empty(
                    "playlist unavailable",
                    "This playlist may have been removed.",
                ))
            }
        }
        _ => {
            if let LibraryPage::Artist(name) | LibraryPage::Album(name) = &section {
                header = header.child(text(name, 29.0));
            }
            let visible = filter(
                commands::visible(library, &section, &tracks),
                &query,
                library,
            );
            if visible.is_empty() {
                scroll(empty(
                    if query.is_empty() {
                        "your collection"
                    } else {
                        "no results"
                    },
                    if query.is_empty() {
                        "Use add music or add videos to import from your device."
                    } else {
                        "Try a different title, artist or album."
                    },
                ))
            } else {
                track_list(vm.clone(), ui.clone(), app, visible, library, None, false)
            }
        }
    };
    rsx! {Grid{row_definitions:"Auto,*",{header}Border{Grid.row:1,{body}}}}
}
fn filter(tracks: Vec<MediaTrack>, query: &str, library: &Library) -> Vec<MediaTrack> {
    tracks
        .into_iter()
        .filter(|track| {
            query.is_empty()
                || track.title.to_lowercase().contains(query)
                || track.artist.to_lowercase().contains(query)
                || library.album(track.id).to_lowercase().contains(query)
        })
        .collect()
}
fn small_button(label: &str, handler: impl Fn() + 'static) -> Element {
    rsx! {Button{class:"phone-flat",min_height:44.0,padding:0.0,automation_properties_name:label,on_click:move |_|handler(),TextBlock{text:label,font_size:17.0}}}
}

#[derive(Clone, PartialEq)]
struct TrackRow {
    track: MediaTrack,
    subtitle: String,
    playlist: Option<Id>,
    selecting: bool,
}
struct Rows {
    items: Vec<TrackRow>,
    source: ItemsSource,
}
impl Rows {
    fn new(items: Vec<TrackRow>) -> Self {
        let source = ItemsSource::from_items(
            items
                .iter()
                .cloned()
                .map(|row| Some(Rc::new(row) as BoxedValue)),
        );
        Self { items, source }
    }
}
fn track_list(
    vm: Rc<PhoneViewModel>,
    ui: Rc<Session>,
    app: AppId,
    tracks: Vec<MediaTrack>,
    library: &Library,
    playlist: Option<Id>,
    selecting: bool,
) -> Element {
    let items = tracks
        .into_iter()
        .map(|track| {
            let subtitle = if app == AppId::Music {
                format!(
                    "{} · {}",
                    commands::artist_name(&track),
                    library.album(track.id)
                )
            } else {
                commands::artist_name(&track).to_owned()
            };
            TrackRow {
                track,
                subtitle,
                playlist,
                selecting,
            }
        })
        .collect::<Vec<_>>();
    rerender(move || {
        let cached = use_hook(|| Rc::new(RefCell::new(Rows::new(items.clone()))));
        let source = {
            let mut cache = cached.borrow_mut();
            if cache.items != items {
                *cache = Rows::new(items.clone());
            }
            cache.source.clone()
        };
        let row_template = use_hook({
            let vm = vm.clone();
            let ui = ui.clone();
            move || {
                template(move |row: TrackRow| {
                    let vm = vm.clone();
                    let ui = ui.clone();
                    rerender(move || track_row(vm.clone(), ui.clone(), app, row.clone()))
                })
            }
        });
        rsx! {ListBox{class:"phone-list",items_source:source,item_template:row_template}}
    })
}
fn track_row(vm: Rc<PhoneViewModel>, ui: Rc<Session>, app: AppId, row: TrackRow) -> Element {
    let id = row.track.id;
    let selected = ui.state::<LibrarySession>().selection.get().contains(&id);
    let play_vm = vm.clone();
    let play_ui = ui.clone();
    let title = row.track.title.clone();
    let mut content = grid().column_definitions(if row.playlist.is_some() {
        "*,44,44,44"
    } else {
        "*"
    });
    content=content.child(rsx!{Button{class:"phone-flat",min_height:68.0,padding:(20.0,10.0),horizontal_content_alignment:H::Stretch,automation_properties_name:title.clone(),
        on_click:move |_|{if row.selecting{play_ui.state::<LibrarySession>().selection.update(|values|{if !values.remove(&id){values.insert(id);}});}else{commands::play(&play_vm,&play_ui,app,id);}},
        Grid{column_definitions:if row.selecting{"30,*"}else{"0,*"},column_spacing:if row.selecting{10.0}else{0.0},
            if row.selecting{Border{width:24.0,height:24.0,border_thickness:2.0,border_brush:phone_controls::palette().foreground,vertical_alignment:V::Center,if selected{{icon(Icon::Check,18.0,phone_controls::palette().foreground)}}}}
            StackPanel{Grid.column:1,spacing:3.0,{text(&title,23.0)}{muted(&row.subtitle,14.0)}}
        }
    }});
    if let Some(playlist) = row.playlist {
        for (column, delta, symbol, label) in [
            (1, -1, Icon::ChevronUp, "move up"),
            (2, 1, Icon::ChevronDown, "move down"),
            (3, 0, Icon::Close, "remove from playlist"),
        ] {
            let model = vm.clone();
            let label = format!("{label}: {}", row.track.title);
            content=content.child(rsx!{Button{Grid.column:column,class:"phone-flat",width:44.0,min_height:50.0,padding:8.0,automation_properties_name:label,
                on_click:move |_|{let result=model::change(&model,app,|library|{if delta==0{library.playlists.remove_item(playlist,id);Ok(())}else{library.playlists.move_item(playlist,id,delta)}});if let Err(error)=result{model.set_status(error);}},
                {icon(symbol,18.0,phone_controls::palette().foreground)}
            }});
        }
    }
    content.into()
}
