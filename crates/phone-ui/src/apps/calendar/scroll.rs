//! Restore a day only after the native viewer has its scrollable dimensions.
use crate::session::Session;
use ferroui_base::{Ref, Vector};
use ferroui_controls::ScrollViewer;
use ferroui_view::on_cleanup;
use std::{cell::Cell, rc::Rc};

pub(super) fn restore_day(
    scroll: &Ref<ScrollViewer>,
    ui: &Rc<Session>,
    date: String,
    initial: f64,
) {
    let target = ui
        .calendar
        .day_scroll
        .borrow()
        .as_ref()
        .filter(|(saved_date, _)| *saved_date == date)
        .map(|(_, offset)| *offset)
        .unwrap_or(initial);
    // `with` runs once per mounted, date-keyed viewer. An unrelated render must
    // not reset this flag and snap a user's next wheel/drag back to the target.
    let pending = Cell::new(true);
    let weak_scroll = scroll.downgrade();
    let weak_ui = Rc::downgrade(ui);
    let token = scroll.scroll_changed(move |_, _| {
        let (Some(scroll), Some(ui)) = (weak_scroll.upgrade(), weak_ui.upgrade()) else {
            return;
        };
        let extent = scroll.extent().height;
        let viewport = scroll.viewport().height;
        if !extent.is_finite() || !viewport.is_finite() || extent <= 0.0 || viewport <= 0.0 {
            return;
        }
        // Loaded precedes a usable extent on this host; setting an offset there
        // is clamped to zero. ScrollChanged runs after native layout instead.
        if pending.replace(false) {
            scroll.set_offset(Vector::new(
                0.0,
                target.clamp(0.0, (extent - viewport).max(0.0)),
            ));
        }
        *ui.calendar.day_scroll.borrow_mut() = Some((date.clone(), scroll.offset().y));
    });
    let weak_scroll = scroll.downgrade();
    on_cleanup(move || {
        if let Some(scroll) = weak_scroll.upgrade() {
            scroll.remove_handler(ScrollViewer::scroll_changed_event(), token);
        }
    });
}
