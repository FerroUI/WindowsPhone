//! Session-only offsets for hub panes that are destroyed by search/detail pages.
//! The native viewer still owns input, extent, viewport, and offset coercion.
use super::*;
use ferroui_base::{Ref, Vector, WeakRef};
use ferroui_controls::{ScrollViewer, primitives::ScrollBarVisibility};
use ferroui_view::on_cleanup;
use std::{
    cell::{Cell, RefCell},
    rc::Weak,
};

#[derive(Clone, Copy)]
pub(super) enum Pane {
    Recent,
    Places,
    Search,
}

impl Pane {
    fn name(self) -> &'static str {
        match self {
            Self::Recent => "OfficeRecentScroll",
            Self::Places => "OfficePlacesScroll",
            Self::Search => "OfficeSearchScroll",
        }
    }
}

#[derive(Default)]
struct Position {
    offset: Cell<f64>,
    generation: Cell<u64>,
    binding: RefCell<Weak<Binding>>,
}

impl Position {
    fn reset(&self) {
        self.offset.set(0.0);
        self.generation.set(self.generation.get().wrapping_add(1));
    }
}

#[derive(Default)]
pub(super) struct State {
    epoch: Cell<Option<u64>>,
    recent: Rc<Position>,
    places: Rc<Position>,
    search: Rc<Position>,
    query: RefCell<Option<String>>,
}

impl State {
    pub(super) fn reset(&self) {
        self.epoch.set(None);
        self.recent.reset();
        self.places.reset();
        self.reset_search();
    }

    pub(super) fn reset_search(&self) {
        self.search.reset();
        self.query.borrow_mut().take();
    }

    fn position(&self, epoch: u64, pane: Pane, query: &str) -> Rc<Position> {
        if self.epoch.get() != Some(epoch) {
            self.reset();
            self.epoch.set(Some(epoch));
        }
        match pane {
            Pane::Recent => self.recent.clone(),
            Pane::Places => self.places.clone(),
            Pane::Search => {
                if self.query.borrow().as_deref() != Some(query) {
                    self.search.reset();
                    *self.query.borrow_mut() = Some(query.into());
                }
                self.search.clone()
            }
        }
    }
}

struct Binding {
    vm: Weak<PhoneViewModel>,
    viewer: WeakRef<ScrollViewer>,
    position: Rc<Position>,
    epoch: u64,
    generation: u64,
    target: f64,
    pending: Cell<bool>,
}

impl Binding {
    fn observe(&self, restore: bool) {
        let (Some(vm), Some(viewer)) = (self.vm.upgrade(), self.viewer.upgrade()) else {
            return;
        };
        if vm.document_epoch() != self.epoch
            || self.position.generation.get() != self.generation
            || !viewer.is_attached_to_visual_tree()
            || !viewer.is_effectively_visible()
            || !viewer.extent().height.is_finite()
            || !viewer.viewport().height.is_finite()
            || viewer.viewport().height <= 0.0
        {
            return;
        }
        if self.pending.get() {
            if !restore {
                return;
            }
            // Loaded can precede a useful viewport. Restore once after layout,
            // allowing the viewer to clamp to its current, measured content.
            self.pending.set(false);
            viewer.set_offset(Vector::new(0.0, self.target));
        }
        self.position.offset.set(viewer.offset().y);
    }
}

fn attach(viewer: &Ref<ScrollViewer>, vm: &Rc<PhoneViewModel>, position: Rc<Position>, epoch: u64) {
    // Reconciliation can mount a replacement before disposing the outgoing
    // tree. Flush its last real offset before advancing the binding generation.
    let previous = position.binding.borrow().upgrade();
    if let Some(previous) = previous {
        previous.observe(false);
    }
    let generation = position.generation.get().wrapping_add(1);
    position.generation.set(generation);
    let binding = Rc::new(Binding {
        vm: Rc::downgrade(vm),
        viewer: viewer.downgrade(),
        epoch,
        generation,
        target: position.offset.get(),
        position: position.clone(),
        pending: Cell::new(true),
    });
    *position.binding.borrow_mut() = Rc::downgrade(&binding);
    let observed = binding.clone();
    let layout = viewer.layout_updated(move || observed.observe(true));
    let observed = binding.clone();
    let changed = viewer.scroll_changed(move |_, event| {
        // Ignore a nested viewer's routed notification, if content gains one.
        if event
            .source()
            .and_then(|source| source.cast::<ScrollViewer>())
            == observed.viewer.upgrade()
        {
            observed.observe(true);
        }
    });
    on_cleanup(move || {
        // Preserve a last native offset even if its coalesced ScrollChanged
        // event has not run yet. A superseded binding cannot write it back.
        binding.observe(false);
        layout.dispose();
        if let Some(viewer) = binding.viewer.upgrade() {
            viewer.remove_handler(ScrollViewer::scroll_changed_event(), changed);
        }
    });
}

pub(super) fn pane(
    vm: Rc<PhoneViewModel>,
    ui: &Session,
    pane: Pane,
    query: &str,
    content: Element,
) -> Element {
    let epoch = vm.document_epoch();
    let position = ui.state::<State>().position(epoch, pane, query);
    rsx! {ScrollViewer {
        key:format!("{}-{epoch}-{query}", pane.name()), name:pane.name(),
        horizontal_scroll_bar_visibility:ScrollBarVisibility::Disabled,
        vertical_scroll_bar_visibility:ScrollBarVisibility::Hidden,
        with:move |viewer|attach(viewer, &vm, position.clone(), epoch),
        {content}
    }}
}
