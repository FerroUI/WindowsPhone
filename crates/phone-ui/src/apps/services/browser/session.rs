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
    pub(crate) view: Signal<BrowserView>,
}
impl Default for BrowserSession {
    fn default() -> Self {
        Self {
            view: signal(BrowserView::Page),
        }
    }
}
