//! Shared notification templates for full tiles and scaled folder miniatures.
use super::live;
use crate::{common, images, session::Session};
use ferroui_base::layout::{HorizontalAlignment as H, VerticalAlignment as V};
use ferroui_base::media::{FontWeight, TextWrapping};
use ferroui_view::prelude::*;
use phone_controls::{Icon, icon};
use phone_domain::{
    TILE_CARD_SECONDS, Tile, TileBadge, TileBadgeGlyph, TileNotification, TileSize, TileTemplate,
};
use phone_state::PhoneViewModel;

pub(super) fn view(
    vm: &PhoneViewModel,
    ui: &Session,
    tile: &Tile,
    card: &TileNotification,
    h: f64,
) -> Element {
    let small = tile.size == TileSize::Small;
    let wide = tile.size == TileSize::Wide;
    let rotate = vm.live_tiles_enabled() && ui.edit_tile.get().is_none();
    let elapsed = vm.now().saturating_sub(card.starts_at).max(0) as u64;
    let bottom = if tile.appearance.show_label && !small {
        36.0
    } else {
        12.0
    };
    let mut phase = 0;
    let content = match &card.template {
        TileTemplate::Text { title, body } => words(tile, title, body, small, wide, bottom),
        TileTemplate::Flip {
            title,
            body,
            back_title,
            back_body,
        } => {
            phase = if rotate { elapsed / 4 % 2 } else { 0 };
            let (title, body) = if phase == 0 {
                (title, body)
            } else {
                (back_title, back_body)
            };
            words(tile, title, body, small, wide, bottom)
        }
        TileTemplate::Photo { photo, title, body } => {
            photo_card(vm, tile, *photo, title, body, small, wide, bottom)
        }
        TileTemplate::Cycle { photos, title } => {
            phase = if rotate && !photos.is_empty() {
                elapsed / TILE_CARD_SECONDS % photos.len() as u64
            } else {
                0
            };
            photos
                .get(phase as usize)
                .map(|id| photo_card(vm, tile, *id, title, "", small, wide, bottom))
                .unwrap_or_else(|| fallback(tile))
        }
        TileTemplate::List { title, lines } => {
            if small {
                fallback(tile)
            } else {
                let mut list = stack_panel()
                    .margin((12.0, 11.0, 12.0, bottom))
                    .spacing(7.0)
                    .child(title_block(tile.id, title, wide));
                for (index, line) in lines.iter().enumerate() {
                    list = list.child(rsx! {TextBlock{name:format!("TileCardLine-{}-{index}",tile.id),text:line.clone(),font_size:if wide{17.0}else{15.0},foreground:"White",text_wrapping:TextWrapping::Wrap}});
                }
                list.into()
            }
        }
        TileTemplate::Progress {
            title,
            body,
            percent,
        } => {
            if small {
                rsx! {TextBlock{name:format!("TileProgress-{}",tile.id),text:format!("{percent}%"),font_size:25.0,font_weight:FontWeight::Light,foreground:"White",horizontal_alignment:H::Center,vertical_alignment:V::Center}}
            } else {
                rsx! {Grid{margin:(12.0,11.0,12.0,bottom),row_definitions:"*,Auto,Auto",row_spacing:8.0,
                    StackPanel{spacing:6.0,{title_block(tile.id,title,wide)},TextBlock{name:format!("TileCardBody-{}",tile.id),text:body.clone(),font_size:17.0,foreground:"White",text_wrapping:TextWrapping::Wrap}},
                    TextBlock{Grid.row:1,name:format!("TileProgress-{}",tile.id),text:format!("{percent}%"),font_size:24.0,foreground:"White"},
                    ProgressBar{Grid.row:2,name:format!("TileProgressBar-{}",tile.id),minimum:0.0,maximum:100.0,value:*percent as f64,height:5.0,is_indeterminate:false,foreground:"White",background:"#55FFFFFF",automation_properties_name:title.clone()}
                }}
            }
        }
    };
    live::frame(
        format!("tile-card-{}-{}-{phase}", tile.id, card.tag),
        rsx! {Grid{name:format!("TileCard-{}",tile.id),clip_to_bounds:true,{content}}},
        h * 0.45,
        rotate && !vm.is_preview_snapshot() && phone_sdk::TileEntrancePolicy::current().animate,
    )
}
fn fallback(tile: &Tile) -> Element {
    icon(common::app_icon(tile.app), 31.0, "White")
}
fn title_block(id: u64, title: &str, wide: bool) -> Element {
    rsx! {TextBlock{name:format!("TileCardTitle-{id}"),text:title.to_owned(),font_size:if wide{24.0}else{21.0},font_weight:FontWeight::Light,foreground:"White",text_wrapping:TextWrapping::Wrap}}
}
fn words(tile: &Tile, title: &str, body: &str, small: bool, wide: bool, bottom: f64) -> Element {
    if small {
        return fallback(tile);
    }
    rsx! {StackPanel{margin:(12.0,11.0,12.0,bottom),spacing:7.0,
        {title_block(tile.id,title,wide)},
        TextBlock{name:format!("TileCardBody-{}",tile.id),text:body.to_owned(),font_size:if wide{18.0}else{16.0},foreground:"White",text_wrapping:TextWrapping::Wrap}
    }}
}
fn photo_card(
    vm: &PhoneViewModel,
    tile: &Tile,
    id: u64,
    title: &str,
    body: &str,
    small: bool,
    wide: bool,
    bottom: f64,
) -> Element {
    let source = vm
        .data()
        .photos
        .iter()
        .find(|photo| photo.id == id)
        .map(|photo| photo.source.clone());
    let Some(source) = source else {
        return fallback(tile);
    };
    rsx! {Grid{clip_to_bounds:true,{images::photo(&source)},Border{background:if small{"#22000000"}else{"#66000000"}},
        if !small {{words(tile,title,body,false,wide,bottom)}}
    }}
}

pub(super) fn badge(id: u64, badge: TileBadge, small: bool) -> Element {
    let value = match badge {
        TileBadge::Count(count) => {
            rsx! {TextBlock{name:format!("TileBadge-{id}"),text:TileBadge::count_text(count),font_size:if small{18.0}else{27.0},font_weight:FontWeight::Light,foreground:"White"}}
        }
        TileBadge::Glyph(glyph) => icon(
            match glyph {
                TileBadgeGlyph::Attention | TileBadgeGlyph::Error => Icon::Info,
                TileBadgeGlyph::Activity => Icon::Refresh,
                TileBadgeGlyph::Alarm => Icon::Alarm,
                TileBadgeGlyph::Available => Icon::Check,
                TileBadgeGlyph::Away => Icon::Timer,
                TileBadgeGlyph::Busy => Icon::Close,
                TileBadgeGlyph::NewMessage => Icon::Message,
                TileBadgeGlyph::Paused => Icon::Pause,
                TileBadgeGlyph::Playing => Icon::Play,
            },
            if small { 16.0 } else { 22.0 },
            "White",
        ),
    };
    rsx! {Border{name:format!("TileBadgeOverlay-{id}"),background:"#55000000",padding:if small{(4.0,0.0)}else{(5.0,1.0)},horizontal_alignment:H::Right,vertical_alignment:V::Top,margin:if small{(0.0,3.0,4.0,0.0)}else{(0.0,8.0,9.0,0.0)},is_hit_test_visible:false,{value}}}
}
