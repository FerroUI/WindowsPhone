//! Temporary input ownership for visible moving pages. Enabled styling stays
//! intact while pointer, tab, focused-editor and root button commands pause.
use ferroui_base::data::BindingPriority;
use ferroui_base::input::{InputElement, KeyboardNavigation, KeyboardNavigationMode};
use ferroui_base::reactive::{IDisposable, Subscription};
use ferroui_base::{Ref, WeakRef};
use ferroui_controls::{Button, Control};
use std::{cell::RefCell, rc::Rc};

pub(super) struct PageAdmission {
    state: Option<State>,
}
struct State {
    page: WeakRef<Control>,
    hit: bool,
    focusable: bool,
    tab_stop: bool,
    tab_navigation: KeyboardNavigationMode,
    focused: Option<WeakRef<InputElement>>,
    _commands: Rc<CommandGuards>,
    layout: Option<Subscription>,
}
impl Drop for State {
    fn drop(&mut self) {
        if let Some(layout) = self.layout.take() {
            layout.dispose();
        }
    }
}

#[derive(Default)]
struct CommandGuards(RefCell<Vec<CommandGuard>>);
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
impl CommandGuards {
    fn refresh(&self, page: &Ref<Control>) {
        let buttons: Vec<_> = page
            .get_visual_descendants()
            .filter_map(|visual| visual.cast::<Button>())
            .collect();
        let mut commands = self.0.borrow_mut();
        commands.retain(|guard| {
            guard
                .button
                .upgrade()
                .is_some_and(|button| buttons.iter().any(|current| Ref::ptr_eq(current, &button)))
        });
        for button in buttons {
            if commands.iter().any(|guard| {
                guard
                    .button
                    .upgrade()
                    .is_some_and(|current| Ref::ptr_eq(&current, &button))
            }) {
                continue;
            }
            // IsDefault/IsCancel register at TopLevel. Removable values contain
            // those commands without overwriting local values or live bindings.
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

impl PageAdmission {
    pub(super) fn block(page: &Ref<Control>) -> Self {
        let commands = Rc::new(CommandGuards::default());
        commands.refresh(page);
        let weak_commands = Rc::downgrade(&commands);
        let weak_page = page.downgrade();
        // Loaded callbacks and reactive tile updates may add new command
        // buttons after the slide begins. Reconcile the live native subtree;
        // detached/replaced buttons release their leases immediately on layout.
        let layout = page.layout_updated(move || {
            if let (Some(commands), Some(page)) = (weak_commands.upgrade(), weak_page.upgrade()) {
                commands.refresh(&page);
                Self::contain_focus(&page);
            }
        });
        let state = State {
            page: page.downgrade(),
            hit: page.is_hit_test_visible(),
            focusable: page.focusable(),
            tab_stop: page.is_tab_stop(),
            tab_navigation: KeyboardNavigation::get_tab_navigation(page),
            focused: page.get_visual_descendants().find_map(|visual| {
                visual
                    .cast::<InputElement>()
                    .filter(|input| input.is_focused())
                    .map(|input| input.downgrade())
            }),
            _commands: commands,
            layout: Some(layout),
        };
        let guard = Self { state: Some(state) };
        page.set_is_hit_test_visible(false);
        KeyboardNavigation::set_tab_navigation(page, KeyboardNavigationMode::None);
        Self::contain_focus(page);
        guard
    }

    fn contain_focus(page: &Ref<Control>) {
        if page.is_keyboard_focus_within() && !page.is_focused() {
            page.set_focusable(true);
            page.set_is_tab_stop(false);
            page.focus();
        }
    }

    /// Call after the motion transform retires. A superseding privacy/route
    /// owner uses false; normal completion may return focus if it still belongs
    /// to the inert page host and the original editor remains attached.
    pub(super) fn restore(mut self, allow_focus: bool) {
        self.restore_state(allow_focus);
    }

    fn restore_state(&mut self, allow_focus: bool) {
        let Some(mut state) = self.state.take() else {
            return;
        };
        // Focus/property restoration can run synchronous application callbacks.
        // Retire reconciliation before those callbacks can trigger more layout.
        if let Some(layout) = state.layout.take() {
            layout.dispose();
        }
        if let Some(page) = state.page.upgrade() {
            page.set_is_hit_test_visible(state.hit);
            KeyboardNavigation::set_tab_navigation(&page, state.tab_navigation);
            let restore_focus =
                allow_focus && page.is_focused() && page.is_attached_to_visual_tree();
            page.set_focusable(state.focusable);
            page.set_is_tab_stop(state.tab_stop);
            if restore_focus {
                if let Some(focused) = state.focused.as_ref().and_then(WeakRef::upgrade) {
                    if focused.is_attached_to_visual_tree()
                        && focused.is_effectively_enabled()
                        && focused.is_effectively_visible()
                    {
                        focused.focus();
                    }
                }
            }
        }
    }
}
impl Drop for PageAdmission {
    fn drop(&mut self) {
        self.restore_state(false);
    }
}
