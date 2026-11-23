//! One pointer owner arbitrates native scrolling, shell swipes and held tile
//! drags. Unclaimed presses/releases remain ordinary FerroUI control input.
mod controller;
mod drag;
mod feedback;
mod motion;

use crate::session::Session;
use controller::Controller;
use ferroui_base::input::{HoldingState, InputElement, Key, KeyModifiers};
use ferroui_base::interactivity::RoutingStrategies;
use ferroui_base::Ref;
use ferroui_controls::{Control, Grid, TextBox};
use ferroui_view::on_cleanup;
use phone_domain::{AppId, Route};
use phone_state::PhoneViewModel;
use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

ferroui_base::ferro_thread_local! {
    static ACTIVE: RefCell<Weak<Controller>> = const { RefCell::new(Weak::new()) };
}

pub(crate) fn cancel() {
    if let Some(controller) = ACTIVE.with(|slot| slot.borrow().upgrade()) {
        controller.cancel();
    }
}

pub(crate) fn attach(root: &Ref<Grid>, vm: Rc<PhoneViewModel>, ui: Rc<Session>) {
    root.set_focusable(true);
    root.set_is_tab_stop(false);
    let focus = root.downgrade();
    let loaded = root.add_handler(Control::loaded_event(), move |_, _| {
        if let Some(root) = focus.upgrade() {
            root.focus();
        }
    });
    let controller = Controller::new(root, &vm, &ui);
    ACTIVE.with(|slot| *slot.borrow_mut() = Rc::downgrade(&controller));
    let press = controller.clone();
    let pressed = root.add_handler_with(
        InputElement::pointer_pressed_event(),
        move |_, event| press.pressed(event),
        RoutingStrategies::TUNNEL,
        true,
    );
    let movement = controller.clone();
    let moved = root.add_handler_with(
        InputElement::pointer_moved_event(),
        move |_, event| movement.moved(event),
        RoutingStrategies::TUNNEL,
        true,
    );
    let release = controller.clone();
    let released = root.add_handler_with(
        InputElement::pointer_released_event(),
        move |_, event| release.released(event),
        RoutingStrategies::TUNNEL,
        true,
    );
    let capture = controller.clone();
    let lost = root.add_handler_with(
        InputElement::pointer_capture_lost_event(),
        move |_, event| capture.lost(event.pointer().id()),
        RoutingStrategies::DIRECT,
        true,
    );
    let hold = controller.clone();
    let held = root.add_handler_with(
        InputElement::holding_event(),
        move |_, event| {
            if event.holding_state() == HoldingState::Started {
                hold.hold(event.pointer_event_args().pointer().id());
            }
        },
        RoutingStrategies::BUBBLE,
        true,
    );
    let keys = controller.clone();
    let keyboard_vm = vm.clone();
    let keyboard_ui = ui.clone();
    let key = root.add_handler_with(
        InputElement::key_down_event(),
        move |_, event| {
            if event.key_modifiers.contains(KeyModifiers::ALT) {
                return;
            }
            let editing = event
                .source()
                .and_then(|source| source.cast::<Control>())
                .is_some_and(|control| {
                    control
                        .get_self_and_visual_ancestors()
                        .any(|ancestor| ancestor.cast::<TextBox>().is_some())
                });
            if event.key == Key::Escape {
                let active = keys.cancel();
                if active && keyboard_ui.edit_tile.get_untracked().is_some() {
                    keyboard_ui.edit_tile.set(None);
                } else {
                    crate::back(&keyboard_vm);
                }
                event.set_handled(true);
                return;
            }
            if keyboard_ui.alarms.active.get_untracked().is_some() {
                if event.key == Key::Back {
                    crate::back(&keyboard_vm);
                    event.set_handled(true);
                }
                return;
            }
            if editing {
                return;
            }
            let ctrl = event
                .key_modifiers
                .intersects(KeyModifiers::CONTROL | KeyModifiers::META);
            match event.key {
                Key::Home if !ctrl => {
                    keys.cancel();
                    crate::home(&keyboard_vm);
                }
                Key::Back
                    if !ctrl
                        && (keyboard_vm.route() != Route::App(AppId::Calculator)
                            || keyboard_ui.dialog.get_untracked().is_some()
                            || keyboard_ui.menu.get_untracked()
                            || keyboard_ui.notification_open.get_untracked()
                            || keyboard_ui.switcher_open.get_untracked()
                            || keyboard_ui.locked.get_untracked()) =>
                {
                    keys.cancel();
                    crate::back(&keyboard_vm);
                }
                Key::F if ctrl => {
                    keys.cancel();
                    keyboard_vm.navigate(Route::Search);
                }
                Key::L if ctrl => {
                    keys.cancel();
                    keyboard_ui.locked.update(|locked| *locked = !*locked);
                }
                _ => return,
            }
            event.set_handled(true);
        },
        RoutingStrategies::TUNNEL,
        false,
    );
    let detach = controller.clone();
    let detached = root.detached_from_visual_tree(move |_| {
        detach.cancel();
    });
    let weak = root.downgrade();
    on_cleanup(move || {
        controller.cancel();
        detached.dispose();
        if let Some(root) = weak.upgrade() {
            root.remove_handler(Control::loaded_event(), loaded);
            root.remove_handler(InputElement::pointer_pressed_event(), pressed);
            root.remove_handler(InputElement::pointer_moved_event(), moved);
            root.remove_handler(InputElement::pointer_released_event(), released);
            root.remove_handler(InputElement::pointer_capture_lost_event(), lost);
            root.remove_handler(InputElement::holding_event(), held);
            root.remove_handler(InputElement::key_down_event(), key);
        }
    });
}
