use crate::{decode, Launch};
use ferroui_base::{
    layout::{HorizontalAlignment as H, VerticalAlignment as V},
    media::{FontWeight, TextWrapping},
};
use ferroui_view::prelude::*;
use phone_controls::{icon, Icon};
use phone_domain::{TileBadge, TileSize};
use phone_host::{
    platform::{DeviceNotification, DeviceRole, HostRole},
    HostPlatform,
};
use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
};

#[derive(Clone, Default)]
struct Preview {
    count: u32,
    title: String,
    body: String,
    timestamp: i64,
}
#[derive(Default)]
struct Cache {
    key: Cell<Option<(u64, u64)>>,
    packages: RefCell<BTreeMap<String, Preview>>,
}
fn group(rows: Vec<DeviceNotification>) -> BTreeMap<String, Preview> {
    let mut result = BTreeMap::<String, Preview>::new();
    for row in rows {
        let preview = result.entry(row.package_id).or_default();
        preview.count = preview.count.saturating_add(1);
        if row.timestamp >= preview.timestamp {
            preview.title = row.title;
            preview.body = row.body;
            preview.timestamp = row.timestamp;
        }
    }
    result
}
pub fn view(cx: phone_sdk::AppTileContext) -> Element {
    let small = cx.size == TileSize::Small;
    let Ok(target) = decode(&cx.path) else {
        return icon(Icon::List, if small { 31.0 } else { 56.0 }, "White");
    };
    let title = cx
        .app
        .vm
        .data()
        .app_links
        .iter()
        .find(|link| link.app == crate::APP_ID && link.path == cx.path)
        .map(|link| link.title.clone())
        .unwrap_or_else(|| "app".into());
    let initials = title
        .split_whitespace()
        .take(2)
        .filter_map(|word| word.chars().next())
        .flat_map(char::to_uppercase)
        .collect::<String>();
    let cache = cx.app.session.state::<Cache>();
    let key = (cx.app.vm.document_epoch(), cx.app.vm.device_revision());
    if cache.key.replace(Some(key)) != Some(key) {
        let state = cx.app.vm.device_state();
        *cache.packages.borrow_mut() = if cx.app.vm.platform_state().platform
            == HostPlatform::Android
            && state.snapshot.role(DeviceRole::NotificationListener) == HostRole::Active
        {
            group(state.notifications)
        } else {
            BTreeMap::new()
        };
    }
    // Catalog identities remain opaque to launching. The Android renderer alone
    // uses its observed ComponentName's package for package-scoped notifications.
    let preview = if matches!(target, Launch::App(_))
        && cx.app.vm.platform_state().platform == HostPlatform::Android
    {
        target
            .application()
            .split_once('/')
            .and_then(|(package, _)| cache.packages.borrow().get(package).cloned())
    } else {
        None
    };
    let count = preview.as_ref().map_or(0, |value| value.count);
    let content = if cx.live && cx.size == TileSize::Wide && preview.is_some() {
        let preview = preview.as_ref().unwrap();
        rsx! {StackPanel{margin:(13.0,12.0,46.0,36.0),spacing:7.0,
            TextBlock{text:preview.title.clone(),font_size:24.0,foreground:"White",text_wrapping:TextWrapping::Wrap},
            TextBlock{text:preview.body.clone(),font_size:17.0,foreground:"White",text_wrapping:TextWrapping::Wrap}
        }}
    } else {
        rsx! {TextBlock{text:initials,font_size:if small{29.0}else{54.0},font_weight:FontWeight::Light,foreground:"White",horizontal_alignment:H::Center,vertical_alignment:V::Center}}
    };
    rsx! {Grid{automation_properties_name:if count>0{format!("{title}, {count} device notifications")}else{title},
        {content}
        if cx.show_badge&&count>0{Border{horizontal_alignment:H::Right,vertical_alignment:V::Top,margin:if small{(0.0,3.0,4.0,0.0)}else{(0.0,8.0,9.0,0.0)},padding:(4.0,0.0),background:"#55000000",TextBlock{text:TileBadge::count_text(count),font_size:if small{18.0}else{27.0},foreground:"White"}}}
    }}
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn previews_count_only_observed_package_notifications_and_keep_latest() {
        let row = |package: &str, time, title: &str| DeviceNotification {
            package_id: package.into(),
            timestamp: time,
            title: title.into(),
            ..Default::default()
        };
        let groups = group(vec![
            row("one", 9, "old"),
            row("two", 30, "separate"),
            row("one", 20, "latest"),
        ]);
        assert_eq!(groups["one"].count, 2);
        assert_eq!(groups["one"].title, "latest");
        assert_eq!(groups["two"].count, 1);
    }
}
