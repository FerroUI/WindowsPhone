//! Enter a native picker only after its popup is connected to the visual tree.
use ferroui_base::{
    Ref, WeakRef,
    input::{FocusManager, KeyModifiers, NavigationMethod},
    reactive::Subscription,
    threading::{Dispatcher, DispatcherPriority},
};
use ferroui_controls::{
    Control, Grid,
    primitives::{DateTimePickerPanel, Popup, TemplateAppliedEventArgs, TemplatedControl},
};
use ferroui_view::on_cleanup;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

struct PickerFocus {
    owner: WeakRef<TemplatedControl>,
    generation: Cell<u64>,
    subscriptions: RefCell<Option<(Subscription, Subscription)>>,
    presentation: RefCell<Option<Rc<super::presentation::PhonePicker>>>,
}

/// Attach once from a date/time picker's declarative `with` callback.
///
/// Native presenter initialization can precede visual attachment, when focus
/// is rejected. Keep its keyboard behavior and selected-value bindings intact;
/// enter the first selector after the native popup-open/layout call completes.
pub fn attach_picker_focus(picker: &TemplatedControl) {
    let state = Rc::new(PickerFocus {
        owner: picker.to_ref().downgrade(),
        generation: Cell::new(0),
        subscriptions: RefCell::new(None),
        presentation: RefCell::new(None),
    });
    let template_state = state.clone();
    let template = picker.template_applied(move |_, event| template_state.bind(event));
    let unload_state = state.clone();
    let unloaded = picker.unloaded(move |_, _| {
        unload_state.invalidate();
        if let Some(presentation) = unload_state.presentation.borrow().as_ref() {
            presentation.close_immediately();
        }
    });
    on_cleanup(move || {
        state.invalidate();
        drop(state.presentation.borrow_mut().take());
        drop(state.subscriptions.borrow_mut().take());
        if let Some(picker) = state.owner.upgrade() {
            picker.remove_handler(TemplatedControl::template_applied_event(), template);
            picker.remove_handler(Control::unloaded_event(), unloaded);
        }
    });
}

impl PickerFocus {
    fn invalidate(&self) -> u64 {
        let generation = self.generation.get().wrapping_add(1);
        self.generation.set(generation);
        generation
    }

    fn bind(self: &Rc<Self>, event: &TemplateAppliedEventArgs) {
        self.invalidate();
        // Palette/template changes replace these guards. They must never leave
        // callbacks on an old popup or accumulate callbacks on the new one.
        drop(self.subscriptions.borrow_mut().take());
        drop(self.presentation.borrow_mut().take());
        let scope = event.name_scope();
        let (Some(popup), Some(presenter)) = (
            scope.find_as::<Popup>("PART_Popup"),
            scope.find_as::<TemplatedControl>("PART_PickerPresenter"),
        ) else {
            return;
        };
        if let Some(owner) = self.owner.upgrade() {
            *self.presentation.borrow_mut() = Some(super::presentation::PhonePicker::attach(
                &owner, &popup, &presenter,
            ));
        }
        let weak_state = Rc::downgrade(self);
        let weak_popup = popup.downgrade();
        let weak_presenter = presenter.downgrade();
        let opened = popup.opened(move || {
            if let (Some(state), Some(popup), Some(presenter)) = (
                weak_state.upgrade(),
                weak_popup.upgrade(),
                weak_presenter.upgrade(),
            ) {
                state.schedule(&popup, &presenter);
            }
        });
        let weak_state = Rc::downgrade(self);
        let closed = popup.closed(move || {
            if let Some(state) = weak_state.upgrade() {
                state.invalidate();
            }
        });
        *self.subscriptions.borrow_mut() = Some((opened, closed));
    }

    fn schedule(self: &Rc<Self>, popup: &Ref<Popup>, presenter: &Ref<TemplatedControl>) {
        let generation = self.invalidate();
        let state = Rc::downgrade(self);
        let popup = popup.downgrade();
        let presenter = presenter.downgrade();
        // Opened follows host publication but precedes the picker's forced
        // initial layout. One dispatcher job runs after that synchronous work;
        // no timer, frame observer, or repeated focus request is needed.
        Dispatcher::ui_thread().post_local(
            move || {
                let (Some(state), Some(popup), Some(presenter)) =
                    (state.upgrade(), popup.upgrade(), presenter.upgrade())
                else {
                    return;
                };
                let Some(owner) = state.owner.upgrade() else {
                    return;
                };
                if state.generation.get() != generation
                    || !popup.is_open()
                    || !owner.is_attached_to_visual_tree()
                    || !owner.is_effectively_visible()
                    || !presenter.is_attached_to_visual_tree()
                    || presenter.is_keyboard_focus_within()
                {
                    return;
                }
                if let Some(selector) = first_selector(&presenter) {
                    selector.focus_with(NavigationMethod::Pointer, KeyModifiers::NONE);
                }
            },
            DispatcherPriority::LOADED,
        );
    }
}

fn first_selector(presenter: &TemplatedControl) -> Option<Ref<DateTimePickerPanel>> {
    let parts = presenter.get_template_descendants();
    let selector = |name: &str| {
        parts
            .iter()
            .find(|part| part.name().as_deref() == Some(name))
            .and_then(|part| part.cast::<DateTimePickerPanel>())
            .filter(|panel| FocusManager::can_focus(panel))
    };
    if let Some(hour) = selector("PART_HourSelector") {
        return Some(hour);
    }
    // Match DatePickerPresenter's native culture-dependent order, including
    // hidden date fields. The month is not universally the first selector.
    [
        ("PART_MonthHost", "PART_MonthSelector"),
        ("PART_DayHost", "PART_DaySelector"),
        ("PART_YearHost", "PART_YearSelector"),
    ]
    .into_iter()
    .filter_map(|(host_name, selector_name)| {
        let host = parts
            .iter()
            .find(|part| part.name().as_deref() == Some(host_name))?
            .cast::<Control>()?;
        if !host.is_effectively_visible() {
            return None;
        }
        Some((Grid::get_column(&host), selector(selector_name)?))
    })
    .min_by_key(|(column, _)| *column)
    .map(|(_, selector)| selector)
}
