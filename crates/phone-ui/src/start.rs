use crate::{common, images, session::Session};
use ferroui_base::input::HoldingState;
use ferroui_base::layout::{HorizontalAlignment as H, Orientation, VerticalAlignment as V};
use ferroui_base::media::{FontWeight, TextWrapping};
use ferroui_controls::primitives::ScrollBarVisibility;
use ferroui_view::prelude::*;
use phone_controls::{icon, Icon};
use phone_domain::{pack_tiles, AppId, Route, Tile, TileSize};
use phone_state::PhoneViewModel;
use std::rc::Rc;

pub fn view(vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Element {
    let width = crate::shell::phone_size(&vm).0;
    let data = vm.data();
    let tiles = data.tiles.clone();
    let columns = data.settings.columns();
    drop(data);
    let gap = 8.0;
    let padding = 14.0;
    let cell = (width - padding * 2.0 - gap * (columns as f64 - 1.0)) / columns as f64;
    let placements = pack_tiles(&tiles, columns);
    let folder = ui
        .folder
        .get()
        .and_then(|id| tiles.iter().find(|t| t.id == id && t.is_folder()).cloned());
    let folder_after = folder
        .as_ref()
        .and_then(|t| placements.iter().find(|p| p.id == t.id))
        .map(|p| p.row + p.height);
    let folder_height = folder
        .as_ref()
        .map(|t| ((t.children.len() + columns - 1) / columns) as f64 * (cell + 8.0) + 58.0)
        .unwrap_or(0.0);
    let rows = placements
        .iter()
        .map(|p| p.row + p.height)
        .max()
        .unwrap_or(0);
    let content_height = rows as f64 * (cell + gap) + folder_height;
    let mut canvas = canvas()
        .name("StartTiles")
        .width(width - padding * 2.0)
        .height(content_height);
    for placement in &placements {
        let tile = tiles.iter().find(|t| t.id == placement.id).unwrap().clone();
        let x = placement.column as f64 * (cell + gap);
        let mut y = placement.row as f64 * (cell + gap);
        if folder_after.is_some_and(|after| placement.row >= after) {
            y += folder_height;
        }
        let w = placement.width as f64 * (cell + gap) - gap;
        let h = placement.height as f64 * (cell + gap) - gap;
        canvas = canvas.child(
            border()
                .canvas_left(x)
                .canvas_top(y)
                .width(w)
                .height(h)
                .child(tile_view(vm.clone(), ui.clone(), tile, w, h, x, y)),
        );
    }
    if let (Some(folder), Some(after)) = (folder, folder_after) {
        canvas = canvas.child(
            border()
                .canvas_left(0.0)
                .canvas_top(after as f64 * (cell + gap))
                .width(width - padding * 2.0)
                .height(folder_height - 8.0)
                .child(folder_view(vm.clone(), ui.clone(), folder, cell, columns)),
        );
    }
    let all = vm.clone();
    let finish = ui.clone();
    let editing = ui.edit_tile.get().is_some();
    let remember = ui.clone();
    let restore = ui.clone();
    rsx! {ScrollViewer{name:"StartScroll",key:"start-scroll",horizontal_scroll_bar_visibility:ScrollBarVisibility::Disabled,vertical_scroll_bar_visibility:ScrollBarVisibility::Hidden,
        on_scroll_changed:move |event|if let Some(scroll)=event.source().and_then(|s|s.cast::<ScrollViewer>()){remember.scrolls.borrow_mut().insert("start".into(),scroll.offset().y);},
        on_loaded:move |event|if let Some(scroll)=event.source().and_then(|s|s.cast::<ScrollViewer>()){
            let offset=restore.scrolls.borrow().get("start").copied().unwrap_or(0.0);scroll.set_offset(ferroui_base::Vector::new(0.0,offset));
        },
        StackPanel{margin:(padding,18.0,padding,24.0),spacing:18.0,
            {canvas}
            Button{name:"AllApps",class:"phone-flat",height:58.0,horizontal_alignment:H::Right,automation_properties_name:if editing{"Finish arranging tiles"}else{"All apps"},
                on_click:move |_|{if editing{finish.edit_tile.set(None);}else{all.navigate(Route::Apps);}},
                StackPanel{orientation:Orientation::Horizontal,spacing:12.0,TextBlock{text:if editing{"done"}else{"all apps"},font_size:21.0,vertical_alignment:V::Center},
                    Border{width:34.0,height:34.0,corner_radius:17.0,border_brush:common::fg(&vm).as_str(),border_thickness:2.0,{icon(if editing{Icon::Check}else{Icon::Next},16.0,&common::fg(&vm))}}
                }
            }
        }
    }}
}
fn tile_view(
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
    let badge = data.badge(app);
    drop(data);
    let color = if app.color().is_empty() {
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
        folder_cover(&tile, &accent)
    } else {
        live_content(vm.clone(), &tile, w, h, badge)
    };
    let mut visual = grid().clip_to_bounds(true).background(color.as_str());
    if !wallpaper.is_empty() && app.color().is_empty() && app != AppId::Photos {
        // The image is aligned in Start coordinates; gutters remain the system colour.
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
        on_click:move |_|{
            if open.edit_tile.get_untracked().is_some(){open.edit_tile.set(Some(id));}
            else if is_folder{open.folder.update(|current|*current=if *current==Some(id){None}else{Some(id)});}
            else{open.open_app(&launch,app);}
        },
        on_holding:move |event|if event.holding_state()==HoldingState::Started{edit.edit_tile.set(Some(id));event.set_handled(true);},
        on_context_requested:move |event|{context.edit_tile.set(Some(id));event.set_handled(true);},
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
        let keyboard = ui.clone();
        let commands = vm.clone();
        result = result.child(
            border()
                .border_brush("White")
                .border_thickness(2.0)
                .is_hit_test_visible(false),
        );
        result=result.child(rsx!{Button{class:"phone-flat",name:format!("Unpin-{id}"),automation_properties_name:"Unpin tile",width:32.0,height:32.0,min_height:0.0,background:"Black",horizontal_alignment:H::Right,vertical_alignment:V::Top,
            on_click:move |_|{unpin.unpin(id);unpin_ui.edit_tile.set(None);},
            Border{border_brush:"White",border_thickness:2.0,corner_radius:16.0,{icon(Icon::Unpin,15.0,"White")}}
        }});
        result=result.child(rsx!{Button{class:"phone-flat",name:format!("Resize-{id}"),automation_properties_name:"Resize tile",width:32.0,height:32.0,min_height:0.0,background:"Black",horizontal_alignment:H::Right,vertical_alignment:V::Bottom,
            on_click:move |_|resize.resize_tile(id),
            Border{border_brush:"White",border_thickness:2.0,corner_radius:16.0,{icon(Icon::Resize,15.0,"White")}}
        }});
        result=result.child(rsx!{Button{class:"phone-flat",automation_properties_name:"Tile actions",width:32.0,height:32.0,min_height:0.0,background:"Black",horizontal_alignment:H::Left,vertical_alignment:V::Top,
            on_click:move |_|{let rename=commands.clone();keyboard.form("tile name",vec![("name",tile.label.clone())],Rc::new(move|fields|rename.rename_folder(id,fields[0].clone())));},
            {icon(Icon::More,18.0,"White")}
        }});
    }
    result.into()
}
fn live_content(vm: Rc<PhoneViewModel>, tile: &Tile, w: f64, _h: f64, badge: usize) -> Element {
    let small = tile.size == TileSize::Small;
    let data = vm.data();
    match tile.app {
        AppId::Photos if !data.photos.is_empty() => {
            let index = if data.settings.battery_saver {
                0
            } else {
                (vm.now() / 20).unsigned_abs() as usize % data.photos.len()
            };
            rsx! {Grid{{images::photo(&data.photos[index].source)}Border{background:"#11000000"}}}
        }
        AppId::Calendar if !small => {
            let date = chrono::DateTime::from_timestamp(vm.now(), 0).unwrap_or_default();
            rsx! {StackPanel{margin:(12.0,7.0),spacing:0.0,
                TextBlock{text:date.format("%A").to_string(),font_size:16.0,foreground:"White"},
                TextBlock{text:date.format("%-d").to_string(),font_size:70.0,font_weight:FontWeight::Light,foreground:"White"}
            }}
        }
        AppId::People if !small => {
            let mut grid = grid()
                .row_definitions("*,*")
                .column_definitions("*,*,*")
                .margin((0.0, 0.0, 0.0, 28.0));
            for (i, person) in data.contacts.iter().take(6).enumerate() {
                grid = grid.child(
                    border()
                        .grid_row((i / 3) as i32)
                        .grid_column((i % 3) as i32)
                        .background(person.color.as_str())
                        .child(icon(Icon::People, 28.0, "#DDFFFFFF")),
                );
            }
            grid.into()
        }
        AppId::Weather if !small => rsx! {Grid{margin:(14.0,12.0),column_definitions:"70,*",
            {icon(Icon::Weather,55.0,"White")}
            StackPanel{Grid.column:1,spacing:3.0,vertical_alignment:V::Center,TextBlock{text:"Weather",font_size:29.0,foreground:"White",font_weight:FontWeight::Light},TextBlock{text:"tap for your forecast",font_size:14.0,foreground:"White"}}
        }},
        AppId::OneNote if !small && !data.notes.is_empty() => {
            rsx! {StackPanel{margin:(12.0,12.0,12.0,35.0),spacing:8.0,{icon(Icon::Note,32.0,"White")}
                TextBlock{text:data.notes[0].title.clone(),font_size:19.0,foreground:"White",text_wrapping:TextWrapping::Wrap}
            }}
        }
        _ => {
            let mut inner = grid().child(icon(
                common::app_icon(tile.app),
                if small { 31.0 } else { 56.0 },
                "White",
            ));
            if badge > 0 {
                inner = inner.child(
                    text_block()
                        .text(badge.to_string())
                        .font_size(if small { 19.0 } else { 34.0 })
                        .font_weight(FontWeight::Light)
                        .foreground("White")
                        .horizontal_alignment(H::Right)
                        .vertical_alignment(V::Top)
                        .margin((
                            0.0,
                            if small { 3.0 } else { 12.0 },
                            if small { 5.0 } else { 14.0 },
                            0.0,
                        )),
                );
            }
            let _ = w;
            inner.into()
        }
    }
}
fn folder_cover(tile: &Tile, accent: &str) -> Element {
    let mut grid = uniform_grid()
        .columns(3)
        .rows(2)
        .margin((10.0, 10.0, 10.0, 30.0));
    for app in tile.children.iter().take(6) {
        grid = grid.child(
            border()
                .margin(3.0)
                .background(accent)
                .child(icon(common::app_icon(*app), 25.0, "White")),
        );
    }
    grid.into()
}
fn folder_view(
    vm: Rc<PhoneViewModel>,
    ui: Rc<Session>,
    folder: Tile,
    cell: f64,
    columns: usize,
) -> Element {
    let mut apps = wrap_panel().item_width(cell + 8.0).item_height(cell + 8.0);
    let accent = common::accent(&vm);
    for app in folder.children {
        let launch = vm.clone();
        let launch_ui = ui.clone();
        let remove = vm.clone();
        apps=apps.child(rsx!{Button{class:"phone-tile",width:cell,height:cell,margin:(0.0,0.0,8.0,8.0),automation_properties_name:format!("Open {} from folder",app.label()),
            on_click:move |_|launch_ui.open_app(&launch,app),on_context_requested:move |e|{remove.remove_from_folder(folder.id,app);e.set_handled(true);},
            {common::app_square(app,cell,&accent)}
        }});
    }
    let rename = ui.clone();
    let rename_vm = vm.clone();
    let name = folder.label.clone();
    let _ = columns;
    rsx! {StackPanel{spacing:10.0,
        Button{class:"phone-flat",on_click:move |_|{let vm=rename_vm.clone();rename.form("folder name",vec![("name",name.clone())],Rc::new(move|fields|vm.rename_folder(folder.id,fields[0].clone())));},
            TextBlock{text:folder.label,font_size:20.0,foreground:accent.as_str()}},
        {apps}
    }}
}
