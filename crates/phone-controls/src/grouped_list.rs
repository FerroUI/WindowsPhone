//! Sticky group headers over the native, vertically virtualized ListBox.
//!
//! The native panel owns realization, scrolling, focus and input. This adapter
//! observes only realized container geometry; it never estimates positions by
//! multiplying an index by a nominal row height. The caller renders one header
//! overlay using `current_group` and `translation` above the list's viewport.
use ferroui_base::media::TranslateTransform;
use ferroui_base::{Point, Ref, Vector, WeakRef};
use ferroui_controls::{ListBox, ScrollViewer};
use ferroui_view::{on_cleanup, signal, Signal};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

mod layout;
pub use layout::ListGroup;
use layout::{placement, MeasuredRow};

#[derive(Clone, Copy)]
enum Position {
    Offset(f64),
    Item {
        index: usize,
        inset: f64,
        attempts: u8,
    },
}

/// Retained group navigation and scroll state. Create one state per list, and
/// update its groups when the native ItemsSource changes. `reset` starts a new
/// query at the top; equal data and a remounted list retain their scroll offset.
pub struct GroupedListState {
    groups: RefCell<Vec<ListGroup>>,
    current: Signal<Option<String>>,
    translation: Ref<TranslateTransform>,
    header_height: f64,
    list: RefCell<Option<WeakRef<ListBox>>>,
    offset: Cell<f64>,
    anchor: Cell<Option<(usize, f64)>>,
    pending: Cell<Option<Position>>,
    updating: Cell<bool>,
}

impl GroupedListState {
    pub fn new(header_height: f64) -> Rc<Self> {
        assert!(header_height.is_finite() && header_height > 0.0);
        Rc::new(Self {
            groups: RefCell::new(Vec::new()),
            current: signal(None),
            translation: TranslateTransform::new(),
            header_height,
            list: RefCell::new(None),
            offset: Cell::new(0.0),
            anchor: Cell::new(None),
            pending: Cell::new(None),
            updating: Cell::new(false),
        })
    }

    pub fn set_groups(&self, mut groups: Vec<ListGroup>) {
        groups.sort_by_key(|group| group.index);
        groups.dedup_by_key(|group| group.index);
        if *self.groups.borrow() != groups {
            *self.groups.borrow_mut() = groups;
            self.publish(None, 0.0);
        }
    }

    pub fn current_group(&self) -> Option<String> {
        self.current.get()
    }

    pub fn translation(&self) -> Ref<TranslateTransform> {
        self.translation.clone()
    }

    /// Clear the saved position when filtering or replacing the list's identity.
    pub fn reset(&self) {
        self.offset.set(0.0);
        self.anchor.set(None);
        self.pending.set(Some(Position::Offset(0.0)));
        self.publish(None, 0.0);
        // The caller normally replaces ItemsSource in the same render. Wait
        // for its next native layout rather than observing the outgoing rows.
    }

    /// Realize a native header, then align its measured top with the viewport.
    /// The ScrollViewer still clamps the last group to its legal scroll range.
    /// Returns false for an absent group; it never changes ListBox selection.
    pub fn jump(&self, key: &str) -> bool {
        let index = self
            .groups
            .borrow()
            .iter()
            .find(|group| group.key == key)
            .map(|group| group.index);
        let Some(index) = index else { return false };
        self.pending.set(Some(Position::Item {
            index,
            inset: 0.0,
            attempts: 0,
        }));
        self.refresh();
        true
    }

    fn bind(&self, list: &Ref<ListBox>) {
        *self.list.borrow_mut() = Some(list.downgrade());
        // Native realization happens after attachment. Retain an explicit jump
        // made before mounting; otherwise restore the last observed position.
        if self.pending.get().is_none() {
            self.pending.set(Some(self.anchor.get().map_or(
                Position::Offset(self.offset.get()),
                |(index, inset)| Position::Item {
                    index,
                    inset,
                    attempts: 0,
                },
            )));
        }
    }

    fn refresh(&self) {
        if self.updating.replace(true) {
            return;
        }
        struct Updating<'a>(&'a Cell<bool>);
        impl Drop for Updating<'_> {
            fn drop(&mut self) {
                self.0.set(false);
            }
        }
        let _updating = Updating(&self.updating);
        let Some(list) = self.list.borrow().as_ref().and_then(WeakRef::upgrade) else {
            return;
        };
        if !list.is_attached_to_visual_tree() || !list.is_effectively_visible() {
            return;
        }
        let Some(scroll) = list
            .scroll()
            .and_then(|scroll| scroll.cast::<ScrollViewer>())
        else {
            return;
        };
        let Some(viewport) = scroll.presenter() else {
            return;
        };
        if viewport.bounds().height <= 0.0 {
            return;
        }
        if let Some(position) = self.pending.take() {
            match position {
                Position::Offset(y) => {
                    scroll.set_offset(Vector::new(scroll.offset().x, y));
                }
                Position::Item {
                    index,
                    inset,
                    attempts,
                } if index < list.items_view().count() => {
                    // This native operation realizes a remote target and runs
                    // the necessary layout before returning when possible.
                    list.scroll_into_view(index as i32);
                    let measured = list
                        .container_from_index(index as i32)
                        .and_then(|container| {
                            container.translate_point(Point::default(), &viewport)
                        });
                    if let Some(origin) = measured {
                        let old = scroll.offset().y;
                        scroll.set_offset(Vector::new(scroll.offset().x, old + origin.y + inset));
                        // A new viewport can refine the panel's estimates. One
                        // later layout remeasures the target rather than using
                        // a stale item pitch, without an unbounded layout loop.
                        if attempts < 3 && (scroll.offset().y - old).abs() > 0.01 {
                            self.pending.set(Some(Position::Item {
                                index,
                                inset,
                                attempts: attempts + 1,
                            }));
                        }
                    } else if attempts < 3 {
                        self.pending.set(Some(Position::Item {
                            index,
                            inset,
                            attempts: attempts + 1,
                        }));
                    }
                }
                Position::Item { .. } => {}
            }
        }
        self.offset.set(scroll.offset().y);
        let mut rows = list
            .get_realized_containers()
            .into_iter()
            .filter_map(|container| {
                let index = list.index_from_container(&container);
                let height = container.bounds().height;
                if index < 0 || !container.is_visible() || height <= 0.0 || !height.is_finite() {
                    return None;
                }
                let y = container.translate_point(Point::default(), &viewport)?.y;
                y.is_finite().then_some(MeasuredRow {
                    index: index as usize,
                    y,
                    height,
                })
            })
            .collect::<Vec<_>>();
        rows.sort_by_key(|row| row.index);
        self.anchor.set(
            rows.iter()
                .find(|row| row.y + row.height > 0.01)
                .map(|row| (row.index, -row.y)),
        );
        let (key, y) = placement(&self.groups.borrow(), &rows, self.header_height);
        self.publish(key, y);
    }

    fn publish(&self, key: Option<String>, y: f64) {
        if self.current.get_untracked() != key {
            self.current.set(key);
        }
        if (self.translation.y() - y).abs() > 0.001 {
            self.translation.set_y(y);
        }
    }
}

/// Attach once from the ListBox's declarative `with` callback. Subscriptions
/// belong to that rendered control and are removed when its view is disposed.
pub fn attach_grouped_list(list: &Ref<ListBox>, state: Rc<GroupedListState>) {
    state.bind(list);
    let observed = state.clone();
    let layout = list.layout_updated(move || observed.refresh());
    let observed = state.clone();
    let weak = list.downgrade();
    let scrolled = list.add_handler(ScrollViewer::scroll_changed_event(), move |_, event| {
        let Some(list) = weak.upgrade() else { return };
        if event
            .source()
            .and_then(|source| source.cast::<ScrollViewer>())
            == list
                .scroll()
                .and_then(|scroll| scroll.cast::<ScrollViewer>())
        {
            observed.refresh();
        }
    });
    let weak = list.downgrade();
    on_cleanup(move || {
        layout.dispose();
        if let Some(list) = weak.upgrade() {
            list.remove_handler(ScrollViewer::scroll_changed_event(), scrolled);
        }
    });
}

#[cfg(test)]
mod tests;
