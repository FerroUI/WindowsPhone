use ferroui_controls::{date_time_pickers::PickerPresenterBase, Control, TopLevel};

/// A picker popup lives above the page overlay. Cancel only this phone's native
/// date/time presenters so an arriving alarm cannot leave a wheel interactive
/// over its commands. Popup cancellation preserves the committed editor draft.
pub(super) fn dismiss_phone_pickers(control: &Control) {
    let Some(top) = TopLevel::get_top_level(Some(control)) else {
        return;
    };
    for popup in top.opened_popups() {
        let picker = popup
            .child()
            .is_some_and(|child| child.cast::<PickerPresenterBase>().is_some());
        let phone = popup.placement_target().is_some_and(|target| {
            target.name().as_deref() == Some("PhoneScreen")
                || target
                    .get_visual_ancestors()
                    .any(|ancestor| ancestor.name().as_deref() == Some("PhoneScreen"))
        });
        if picker && phone {
            popup.close();
        }
    }
}
