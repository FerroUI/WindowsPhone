use super::session::BrowserView;
use crate::session::Session;
use phone_domain::{Action, AppId, BrowserAction, Route, canonical_url};
use phone_state::{HostRequest, PhoneViewModel};

pub(super) fn browse(vm: &PhoneViewModel, ui: &Session, value: &str) {
    match super::super::web_url(value).and_then(|value| canonical_url(&value)) {
        Ok(url) => {
            vm.dispatch(Action::Browser(BrowserAction::Navigate(url)));
            sync(vm, ui, true);
        }
        Err(error) => vm.set_status(error),
    }
}
pub(super) fn change(vm: &PhoneViewModel, ui: &Session, action: BrowserAction) {
    vm.dispatch(Action::Browser(action));
    sync(vm, ui, true);
}
/// A retained page can reattach after a host resize. Only restore a different
/// saved address; reattachment must not reload a page or close an open tab list.
pub(super) fn restore(vm: &PhoneViewModel, ui: &Session) {
    if ui.browser_url.get_untracked() != vm.data().browser.url() {
        sync(vm, ui, true);
    }
}
pub(super) fn sync(vm: &PhoneViewModel, ui: &Session, load: bool) {
    let url = vm.data().browser.url().to_owned();
    ui.browser_url.set(url.clone());
    ui.entry.set(url.clone());
    ui.pivot.set(0);
    ui.browser.view.set(BrowserView::Page);
    ui.menu.set(false);
    if load {
        vm.request(if url.is_empty() {
            HostRequest::ClearBrowser
        } else {
            HostRequest::Browse { url }
        });
    }
}
/// Records only a host acknowledgement of an app-initiated navigation.
/// The bridge cannot observe arbitrary cross-origin iframe history or titles.
pub fn browser_navigated(vm: &PhoneViewModel, ui: &Session, value: &str) {
    let Ok(url) = canonical_url(value) else {
        return;
    };
    if vm.data().browser.url() != url {
        vm.dispatch(Action::Browser(BrowserAction::Navigate(url.clone())));
    }
    ui.browser_url.set(url.clone());
    ui.entry.set(url);
    ui.browser.view.set(BrowserView::Page);
    ui.pivot.set(0);
    ui.menu.set(false);
}
pub fn browser_back(vm: &PhoneViewModel, ui: &Session) -> bool {
    if vm.route() != Route::App(AppId::InternetExplorer) {
        return false;
    }
    if ui.browser.view.get_untracked() != BrowserView::Page {
        ui.browser.view.set(BrowserView::Page);
        return true;
    }
    if vm.data().browser.current().can_back() {
        change(vm, ui, BrowserAction::Back);
        return true;
    }
    false
}
pub(super) fn show(ui: &Session, view: BrowserView) {
    ui.menu.set(false);
    ui.browser.view.set(view);
}
