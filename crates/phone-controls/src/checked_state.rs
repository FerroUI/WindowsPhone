//! Two-way selection properties with a render-owned callback lifetime.
use ferroui_base::BoxedValue;
use ferroui_view::{element::PropValue, IntoProp};
use std::rc::Rc;

/// A controlled native selection. Model-to-view reconciliation sets the new
/// value before listening; native pointer, keyboard and automation changes
/// publish through the current callback without echoing a model refresh.
pub struct CheckedState {
    value: Option<bool>,
    changed: Rc<dyn Fn(Option<bool>)>,
}

pub fn checked_state(
    value: Option<bool>,
    changed: impl Fn(Option<bool>) + 'static,
) -> CheckedState {
    CheckedState {
        value,
        changed: Rc::new(changed),
    }
}

impl IntoProp<Option<bool>, CheckedState> for CheckedState {
    fn into_prop(self) -> PropValue {
        let value = self.value;
        let changed = self.changed;
        PropValue::TwoWay {
            get: Rc::new(move || Rc::new(value) as BoxedValue),
            set: Rc::new(move |value| {
                if let Some(value) = value.downcast_ref::<Option<bool>>() {
                    changed(*value);
                }
            }),
            // A new render supplies a new controlled value and callback.
            // FerroUI stops the prior subscription, initializes this value,
            // and only then subscribes to changes of the retained control.
            source: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use ferroui_controls::ToggleSwitch;
    use ferroui_view::prelude::*;
    use std::{cell::RefCell, rc::Rc};

    #[test]
    fn refreshing_model_selection_does_not_echo_through_the_old_callback() {
        let owner = Owner::new_root();
        let value = owner.with(|| signal(false));
        let changes = Rc::new(RefCell::new(Vec::new()));
        let recorded = changes.clone();
        let root = owner.with(|| {
            rerender(move || {
                let recorded = recorded.clone();
                crate::toggle("alarm enabled", value.get(), move |next| {
                    recorded.borrow_mut().push(next)
                })
            })
            .mount()
        });
        let switch = root
            .cast::<ToggleSwitch>()
            .or_else(|| root.find_descendant_of_type_where::<ToggleSwitch>(false, |_| true))
            .expect("mounted native switch");
        value.set(true);
        assert_eq!(switch.is_checked(), Some(true));
        value.set(false);
        assert_eq!(switch.is_checked(), Some(false));
        assert!(
            changes.borrow().is_empty(),
            "model refresh is not user interaction"
        );
        switch.set_is_checked(Some(true));
        assert_eq!(*changes.borrow(), [true]);
        value.set(true);
        assert_eq!(
            *changes.borrow(),
            [true],
            "the model accepting input cannot echo it"
        );
        owner.dispose();
    }
}
