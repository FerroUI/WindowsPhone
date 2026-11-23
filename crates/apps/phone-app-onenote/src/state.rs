use ferroui_view::prelude::*;
use phone_domain::Id;
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Pane {
    #[default]
    Recent,
    Notebooks,
    Sections(u32),
    Pages(u32),
}
pub struct NoteSession {
    pub pane: Signal<Pane>,
    pub moving: Signal<Option<Id>>,
}
impl Default for NoteSession {
    fn default() -> Self {
        Self {
            pane: signal(Pane::Recent),
            moving: signal(None),
        }
    }
}
