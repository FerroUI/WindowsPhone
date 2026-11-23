//! Direct visual feedback stays outside reactive document state. A canceled
//! pointer restores exactly the translation/opacity it acquired.
use super::controller::{Controller, Mode};
use ferroui_base::{Vector3D, WeakRef};
use ferroui_controls::Control;

pub(super) struct Feedback {
    control: WeakRef<Control>,
    translation: Vector3D,
    opacity: f64,
}

pub(super) fn clear(controller: &Controller, animate: bool) {
    if let Some(feedback) = controller.feedback.borrow_mut().take() {
        if let Some(control) = feedback.control.upgrade() {
            let current = control.translation();
            control.set_translation(feedback.translation);
            control.set_opacity(feedback.opacity);
            if animate {
                phone_controls::settle(
                    &control,
                    current.x - feedback.translation.x,
                    current.y - feedback.translation.y,
                );
            }
        }
    }
}

pub(super) fn update(controller: &Controller) {
    let Some((mode, id, dx, dy)) = controller.contacts.borrow().values().find_map(|contact| {
        matches!(
            contact.mode,
            Mode::Swipe | Mode::Unlock | Mode::CloseNotifications | Mode::DismissNotification
        )
        .then(|| {
            let (dx, dy) = contact.motion.delta();
            (contact.mode, contact.notification, dx, dy)
        })
    }) else {
        return;
    };
    let Some(root) = controller.root.upgrade() else {
        return;
    };
    if controller.feedback.borrow().is_none() {
        let name = match mode {
            Mode::Swipe => "PhonePage".into(),
            Mode::Unlock => "LockScreen".into(),
            Mode::CloseNotifications => "ActionCenter".into(),
            Mode::DismissNotification => format!("Notification-{}", id.unwrap_or(0)),
            _ => return,
        };
        if let Some(control) =
            root.find_descendant_of_type_where::<Control>(false, |control| control.has_name(&name))
        {
            *controller.feedback.borrow_mut() = Some(Feedback {
                translation: control.translation(),
                opacity: control.opacity(),
                control: control.downgrade(),
            });
        }
    }
    let state = controller.feedback.borrow();
    let Some(feedback) = state.as_ref() else {
        return;
    };
    let Some(control) = feedback.control.upgrade() else {
        return;
    };
    let (x, y) = match mode {
        Mode::Swipe => (
            dx.clamp(-root.bounds().width * 0.55, root.bounds().width * 0.55) * 0.65,
            0.0,
        ),
        Mode::Unlock | Mode::CloseNotifications => (0.0, dy.min(0.0)),
        Mode::DismissNotification => {
            control.set_opacity(
                feedback.opacity * (1.0 - dx.abs() / root.bounds().width.max(1.0)).clamp(0.15, 1.0),
            );
            (dx, 0.0)
        }
        _ => return,
    };
    control.set_translation(Vector3D::new(
        feedback.translation.x + x,
        feedback.translation.y + y,
        feedback.translation.z,
    ));
}
