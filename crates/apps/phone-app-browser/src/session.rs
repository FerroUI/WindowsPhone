use ferroui_view::prelude::*;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum BrowserView {
    #[default]
    Page,
    Tabs,
    Favorites,
    History,
    Settings,
}

pub struct BrowserSession {
    pub url: Signal<String>,
    pub(crate) view: Signal<BrowserView>,
}
impl Default for BrowserSession {
    fn default() -> Self {
        Self {
            url: signal(String::new()),
            view: signal(BrowserView::Page),
        }
    }
}
