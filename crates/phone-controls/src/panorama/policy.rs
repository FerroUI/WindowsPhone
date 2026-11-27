//! Keep the motion policy live on a retained native Carousel. Removable
//! property values suppress motion without replacing the application's values
//! or bindings underneath them.
use ferroui_base::data::BindingPriority;
use ferroui_base::reactive::IDisposable;
use ferroui_base::{Ref, WeakRef};
use ferroui_controls::Carousel;
use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
};

ferroui_base::ferro_thread_local! {
    static ACTIVE: RefCell<Vec<Weak<Policy>>> = const { RefCell::new(Vec::new()) };
}

pub(crate) fn refresh_motion() {
    let policies = ACTIVE.with(|active| {
        let mut active = active.borrow_mut();
        active.retain(|entry| entry.strong_count() > 0);
        active.iter().filter_map(Weak::upgrade).collect::<Vec<_>>()
    });
    for policy in policies {
        policy.refresh();
    }
}

pub(super) struct Policy {
    owner: WeakRef<Carousel>,
    swipe: RefCell<Option<Rc<dyn IDisposable>>>,
    transition: RefCell<Option<Rc<dyn IDisposable>>>,
    properties: RefCell<Vec<Rc<dyn IDisposable>>>,
    classes_listener: Cell<Option<u64>>,
    updating: Cell<bool>,
    updating_classes: Cell<bool>,
}

impl Policy {
    pub(super) fn attach(owner: &Ref<Carousel>) -> Rc<Self> {
        let policy = Rc::new(Self {
            owner: owner.downgrade(),
            swipe: RefCell::new(None),
            transition: RefCell::new(None),
            properties: RefCell::new(Vec::new()),
            classes_listener: Cell::new(None),
            updating: Cell::new(false),
            updating_classes: Cell::new(false),
        });
        let weak = Rc::downgrade(&policy);
        let properties = owner.property_changed(move |change| {
            if change.property() == Carousel::is_swipe_enabled_property().as_property() {
                if let Some(policy) = weak.upgrade() {
                    policy.sync_classes();
                }
            }
        });
        policy.properties.borrow_mut().push(properties);
        // A retained view may reconcile its declared class list without
        // changing IsSwipeEnabled. Keep input ownership correct in that case
        // too; our own class changes are guarded against reentrancy.
        let weak = Rc::downgrade(&policy);
        let classes_listener = owner.classes().add_listener(Rc::new(move || {
            if let Some(policy) = weak.upgrade() {
                policy.sync_classes();
            }
        }));
        policy.classes_listener.set(Some(classes_listener));
        ACTIVE.with(|active| {
            let mut active = active.borrow_mut();
            active.retain(|entry| entry.strong_count() > 0);
            active.push(Rc::downgrade(&policy));
        });
        policy.refresh();
        policy
    }

    fn refresh(&self) {
        if self.updating.replace(true) {
            return;
        }
        let _guard = Updating(&self.updating);
        loop {
            let enabled = crate::motion_enabled();
            if enabled {
                self.release();
            } else if let Some(owner) = self.owner.upgrade() {
                // Disable native input first. Its teardown cancels the held
                // gesture/completion and resets the old page transition before
                // we suppress that transition as well.
                if self.swipe.borrow().is_none() {
                    let lease = owner.set_value_with_priority(
                        Carousel::is_swipe_enabled_property(),
                        false,
                        BindingPriority::Animation,
                    );
                    self.swipe.replace(lease);
                }
                if self.transition.borrow().is_none() {
                    let lease = owner.set_value_with_priority(
                        Carousel::page_transition_property(),
                        None,
                        BindingPriority::Animation,
                    );
                    self.transition.replace(lease);
                }
            } else {
                self.release();
            }
            self.sync_classes();
            // Property notifications are synchronous. If a caller changed the
            // global policy in one of them, apply that final value as well.
            if enabled == crate::motion_enabled() {
                break;
            }
        }
    }

    fn release(&self) {
        // Restore the effective transition while input remains suppressed,
        // then let the native panel re-arm with the caller's current values.
        let transition = self.transition.borrow_mut().take();
        if let Some(transition) = transition {
            transition.dispose();
        }
        let swipe = self.swipe.borrow_mut().take();
        if let Some(swipe) = swipe {
            swipe.dispose();
        }
    }

    fn sync_classes(&self) {
        if self.updating_classes.replace(true) {
            return;
        }
        let _guard = Updating(&self.updating_classes);
        if let Some(owner) = self.owner.upgrade() {
            let native = owner.is_swipe_enabled();
            owner.classes().set("phone-native-input", native);
            owner.classes().set("phone-panorama-static", !native);
        }
    }
}

struct Updating<'a>(&'a Cell<bool>);
impl Drop for Updating<'_> {
    fn drop(&mut self) {
        self.0.set(false);
    }
}

impl Drop for Policy {
    fn drop(&mut self) {
        for property in self.properties.get_mut().drain(..) {
            property.dispose();
        }
        if let (Some(owner), Some(token)) = (self.owner.upgrade(), self.classes_listener.take()) {
            owner.classes().remove_listener(token);
        }
        self.release();
        self.sync_classes();
    }
}
