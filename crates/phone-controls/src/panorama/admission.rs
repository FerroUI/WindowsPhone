//! Inactive previews retain enabled ink without receiving pointer, tab or
//! root-level default/cancel commands. Temporary property leases preserve the
//! application's bindings underneath the presentation scope.
use ferroui_base::data::BindingPriority;
use ferroui_base::input::{
    FocusManager, KeyModifiers, KeyboardNavigation, KeyboardNavigationMode, NavigationMethod,
};
use ferroui_base::reactive::IDisposable;
use ferroui_base::{Ref, WeakRef};
use ferroui_controls::{Button, Carousel};
use std::{cell::RefCell, rc::Rc};

#[derive(Default)]
pub(super) struct Admission {
    commands: RefCell<Vec<CommandGuard>>,
}

struct CommandGuard {
    button: WeakRef<Button>,
    leases: Vec<Rc<dyn IDisposable>>,
}

impl Drop for CommandGuard {
    fn drop(&mut self) {
        for lease in self.leases.drain(..) {
            lease.dispose();
        }
    }
}

/// Native fractional keyboard/programmatic transitions do not set IsSwiping.
/// Admit the selected pane only when its actual logical offset is at rest too.
pub(super) fn is_settled(owner: &Carousel) -> bool {
    if owner.is_swiping() {
        return false;
    }
    let count = owner.item_count();
    if count < 2 {
        return true;
    }
    let Some(offset) = owner.items_panel_root().and_then(|panel| {
        panel
            .as_scrollable()
            .map(|scrollable| scrollable.offset().x)
    }) else {
        return false;
    };
    let delta = (offset - owner.selected_index() as f64).rem_euclid(count as f64);
    delta.is_finite() && delta.min(count as f64 - delta) < 1e-6
}

impl Admission {
    pub(super) fn refresh(&self, owner: &Carousel) {
        let settled = is_settled(owner);
        let mut inactive = Vec::<Ref<Button>>::new();
        for container in owner.get_realized_containers() {
            let active =
                settled && owner.index_from_container(&container) == owner.selected_index();
            if !active && container.is_keyboard_focus_within() && !owner.focus() {
                // A caller may deliberately make the host non-focusable. Do
                // not leave a focused row in an inactive section in that case.
                if let Some(manager) = FocusManager::get_focus_manager(owner) {
                    manager.focus(None, NavigationMethod::Unspecified, KeyModifiers::NONE);
                }
            }
            container.set_is_hit_test_visible(active);
            KeyboardNavigation::set_tab_navigation(
                &container,
                if active {
                    KeyboardNavigationMode::Continue
                } else {
                    KeyboardNavigationMode::None
                },
            );
            if !active {
                inactive.extend(
                    container
                        .get_visual_descendants()
                        .filter_map(|visual| visual.cast::<Button>()),
                );
            }
        }
        let mut commands = self.commands.borrow_mut();
        commands.retain(|guard| {
            guard.button.upgrade().is_some_and(|button| {
                inactive
                    .iter()
                    .any(|candidate| Ref::ptr_eq(candidate, &button))
            })
        });
        for button in inactive {
            if commands.iter().any(|guard| {
                guard
                    .button
                    .upgrade()
                    .is_some_and(|current| Ref::ptr_eq(&current, &button))
            }) {
                continue;
            }
            // Default/cancel Buttons listen at TopLevel, so hit-test/tab gating
            // alone cannot contain them. A removable highest-priority value
            // scopes those roles without overwriting their local values or
            // reactive bindings. Updates underneath become effective on return.
            let leases = [Button::is_default_property(), Button::is_cancel_property()]
                .into_iter()
                .filter_map(|property| {
                    button.set_value_with_priority(property, false, BindingPriority::Animation)
                })
                .collect();
            commands.push(CommandGuard {
                button: button.downgrade(),
                leases,
            });
        }
    }
}
