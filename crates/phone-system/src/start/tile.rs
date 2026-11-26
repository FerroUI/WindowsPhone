use super::{cards, content, folders, wallpaper};
use crate::session::Session;
use ferroui_base::input::HoldingState;
use ferroui_base::layout::{HorizontalAlignment as H, VerticalAlignment as V};
use ferroui_view::prelude::*;
use phone_controls::{icon, Icon};
use phone_domain::{AppId, Tile, TileBackground, TileBadge, TileNotification, TileSize};
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
    let id = tile.id;
    let is_folder = tile.is_folder();
    let selected = ui.edit_tile.get() == Some(id);
    let target = ui.drop_target.get() == Some(id);
    let open = ui.clone();
    let launch = vm.clone();
    let edit = ui.clone();
    let context = ui.clone();
    let tilt_ui = Rc::downgrade(&ui);
    let card = current_card(&vm, &ui, &tile);
    let badge = badge(&vm, &tile, card);
    let details = card
        .map(|card| card.template.accessible_text())
        .unwrap_or_default();
    let badge_text = badge
        .map(|badge| format!(", {}", badge.accessible_text()))
        .unwrap_or_default();
    let visual = visual(vm.clone(), ui.clone(), &tile, w, h, Some((x, y)));
    let button = rsx! {Button{key:format!("tile-{id}"),name:format!("Tile-{id}"),class:"phone-tile",width:w,height:h,padding:0.0,
        with:move|button|{let ui=tilt_ui.clone();phone_controls::attach_press_tilt(button,move||ui.upgrade().is_some_and(|ui|ui.edit_tile.get_untracked().is_none()));},
        automation_properties_name:format!("{} tile",tile.display_label()),horizontal_content_alignment:H::Stretch,vertical_content_alignment:V::Stretch,
        automation_properties_help_text:if is_folder{format!("Folder, {} tiles{badge_text}. Hold to arrange.",tile.folder_tiles.len())}else{format!("{} tile{badge_text}. {details} Hold to customize, resize, move or unpin.",match tile.size{TileSize::Small=>"Small",TileSize::Medium=>"Medium",TileSize::Wide=>"Wide"})},
        on_click:move|_|{
            if open.edit_tile.get_untracked().is_some(){
                if is_folder&&open.edit_tile.get_untracked()==Some(id){open.folder.update(|folder|*folder=if *folder==Some(id){None}else{Some(id)});}
                open.edit_tile.set(Some(id));
            }else if is_folder{open.folder.update(|folder|*folder=if *folder==Some(id){None}else{Some(id)});}
            else{open.folder.set(None);open.edit_tile.set(None);open.menu.set(false);open.switcher_open.set(false);crate::open_tile(&launch,id);}
        },
        on_holding:move|event|if event.holding_state()==HoldingState::Started{edit.edit_tile.set(Some(id));event.set_handled(true);},
        on_context_requested:move|event|{context.edit_tile.set(Some(id));event.set_handled(true);},
        {visual}
    }};
    let mut overlays = vec![button];
    if target {
        overlays.push(
            rsx! {Border{border_brush:"White",border_thickness:4.0,is_hit_test_visible:false}},
        );
    }
    if selected {
        let unpin = vm.clone();
        let unpin_ui = ui.clone();
        let resize = vm.clone();
        let actions = ui.clone();
        overlays.push(
            rsx! {Border{border_brush:"White",border_thickness:2.0,is_hit_test_visible:false}},
        );
        overlays.push(rsx!{Button{class:"phone-flat",name:format!("Unpin-{id}"),automation_properties_name:"Unpin tile",width:32.0,height:32.0,min_height:0.0,background:"Black",horizontal_alignment:H::Right,vertical_alignment:V::Top,
            on_click:move|_|{unpin.unpin(id);unpin_ui.edit_tile.set(None);unpin_ui.menu.set(false);},
            Border{border_brush:"White",border_thickness:2.0,corner_radius:16.0,{icon(Icon::Unpin,15.0,"White")}}
        }});
        overlays.push(rsx!{Button{class:"phone-flat",name:format!("Resize-{id}"),automation_properties_name:"Resize tile",width:32.0,height:32.0,min_height:0.0,background:"Black",horizontal_alignment:H::Right,vertical_alignment:V::Bottom,
            on_click:move|_|resize.resize_tile(id),
            Border{border_brush:"White",border_thickness:2.0,corner_radius:16.0,{icon(Icon::Resize,15.0,"White")}}
        }});
        overlays.push(rsx!{Button{class:"phone-flat",name:format!("TileActions-{id}"),automation_properties_name:"Tile actions",width:32.0,height:32.0,min_height:0.0,background:"Black",horizontal_alignment:H::Left,vertical_alignment:V::Top,
            on_click:move|_|super::menu::toggle(&actions),{icon(Icon::More,18.0,"White")}
        }});
    }
    rsx! {Grid{for overlay in overlays{{overlay}}}}
}

fn current_card<'a>(
    vm: &PhoneViewModel,
    ui: &Session,
    tile: &'a Tile,
) -> Option<&'a TileNotification> {
    if !vm.data().settings.live_tiles {
        return None;
    }
    tile.active_notification(
        vm.now(),
        vm.live_tiles_enabled() && ui.edit_tile.get().is_none(),
    )
}

fn badge(vm: &PhoneViewModel, tile: &Tile, card: Option<&TileNotification>) -> Option<TileBadge> {
    if !tile.appearance.show_badge {
        return None;
    }
    if let Some(badge) = card.and_then(|card| card.badge) {
        return (badge != TileBadge::Count(0)).then_some(badge);
    }
    if tile.is_folder() || tile.target.is_some() {
        return None;
    }
    let count = vm.app_badge(tile.app);
    (count > 0).then_some(TileBadge::Count(count))
}

/// A noninteractive tile surface shared by Start and folder previews.
pub(super) fn visual(
    vm: Rc<PhoneViewModel>,
    ui: Rc<Session>,
    tile: &Tile,
    w: f64,
    h: f64,
    plane: Option<(f64, f64)>,
) -> Element {
    let id = tile.id;
    let small = tile.size == TileSize::Small;
    let data = vm.data();
    let extension = if let Some(phone_domain::TileTarget::AppLink(link)) = tile.target {
        data.app_links
            .iter()
            .find(|value| value.id == link)
            .and_then(|value| ui.registry().by_id(&value.app).map(|app| app.manifest))
    } else {
        None
    };
    let branded = extension
        .map(|value| value.accent)
        .unwrap_or_else(|| tile.app.color());
    let color = match &tile.appearance.background {
        TileBackground::Color(color) => color.clone(),
        TileBackground::App if !branded.is_empty() && !tile.is_folder() => branded.into(),
        _ => data.settings.accent.clone(),
    };
    let mut layers = Vec::<Element>::new();
    if let TileBackground::Photo(photo) = &tile.appearance.background {
        if let Some(photo) = data.photos.iter().find(|value| value.id == *photo) {
            layers.push(crate::images::photo(&photo.source));
            layers.push(rsx! {Border{background:"#55000000",is_hit_test_visible:false}});
        }
    } else if let Some((x, y)) = plane {
        if tile.appearance.background == TileBackground::App
            && !data.settings.wallpaper.is_empty()
            && (branded.is_empty() || tile.is_folder())
            && tile.app != AppId::Photos
        {
            let (width, height, _) = crate::shell::phone_size(&vm);
            layers.push(wallpaper::view(
                &ui,
                id,
                &data.settings.wallpaper,
                width - 28.0,
                height - 80.0,
                x,
                y,
            ));
            layers.push(rsx! {Border{background:"#33000000",is_hit_test_visible:false}});
        }
    }
    drop(data);
    let card = current_card(&vm, &ui, tile);
    let content = if tile.is_folder() {
        folders::cover(vm.clone(), ui.clone(), tile, w, h)
    } else if let Some(card) = card {
        cards::view(&vm, &ui, tile, card, h)
    } else if !tile.appearance.live
        || (tile.app == AppId::Photos && tile.appearance.background != TileBackground::App)
    {
        icon(
            extension
                .map(|app| app.icon)
                .unwrap_or_else(|| crate::common::app_icon(tile.app)),
            if small { 31.0 } else { 56.0 },
            "White",
        )
    } else {
        content::view(vm.clone(), ui.clone(), tile, w, h, 0)
    };
    layers.push(content);
    if let Some(badge) = badge(&vm, tile, card) {
        layers.push(cards::badge(id, badge, small));
    }
    if !small && tile.appearance.show_label {
        if matches!(tile.appearance.background, TileBackground::Photo(_))
            || (tile.app == AppId::Photos && !tile.is_folder())
        {
            layers.push(rsx! {Border{height:34.0,background:"#66000000",vertical_alignment:V::Bottom,is_hit_test_visible:false}});
        }
        layers.push(rsx! {TextBlock{name:format!("TileLabel-{id}"),text:tile.display_label().to_owned(),foreground:"White",font_size:14.0,margin:(10.0,0.0,8.0,8.0),vertical_alignment:V::Bottom,is_hit_test_visible:false}});
    }
    rsx! {Grid{name:format!("TileVisual-{id}"),width:w,height:h,clip_to_bounds:true,background:color.as_str(),for layer in layers{{layer}}}}
}
