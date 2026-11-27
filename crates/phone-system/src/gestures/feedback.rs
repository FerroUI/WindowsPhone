//! Direct visual feedback stays outside reactive document state. A canceled
//! pointer restores exactly the translation/opacity it acquired.
use super::controller::{Controller, Mode};
use ferroui_base::{Vector3D, WeakRef};
use ferroui_controls::Control;
use phone_domain::Route;

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
    let Some((mode, id, route, dx, dy)) =
        controller.contacts.borrow().values().find_map(|contact| {
            matches!(
                contact.mode,
                Mode::Swipe | Mode::Unlock | Mode::CloseNotifications | Mode::DismissNotification
            )
            .then(|| {
                let (dx, dy) = contact.motion.delta();
                (
                    contact.mode,
                    contact
                        .notification
                        .as_ref()
                        .map(|target| target.name().to_owned()),
                    contact.route.clone(),
                    dx,
                    dy,
                )
            })
        })
    else {
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
            Mode::DismissNotification => match id {
                Some(name) => name,
                None => return,
            },
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
        Mode::Swipe => (horizontal_offset(&route, dx, root.bounds().width), 0.0),
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

fn horizontal_offset(route: &Route, displacement: f64, width: f64) -> f64 {
    if !displacement.is_finite() || !width.is_finite() || width <= 0.0 {
        return 0.0;
    }
    let outside = match route {
        Route::Start | Route::Folder(_) => displacement > 0.0,
        Route::Apps => displacement < 0.0,
        _ => false,
    };
    if outside {
        // Start and All apps form a bounded pair, unlike a wrapping Pivot.
        // Resistance shows the edge without moving a page into an empty pane.
        let limit = width * 0.12;
        displacement.signum() * limit * (1.0 - (-displacement.abs() / limit).exp())
    } else {
        displacement.clamp(-width, width)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_shell_pans_track_the_finger_and_edges_resist() {
        assert_eq!(horizontal_offset(&Route::Start, -120.0, 400.0), -120.0);
        assert_eq!(horizontal_offset(&Route::Apps, 120.0, 400.0), 120.0);
        assert_eq!(horizontal_offset(&Route::Apps, 600.0, 400.0), 400.0);
        let start_edge = horizontal_offset(&Route::Start, 600.0, 400.0);
        let apps_edge = horizontal_offset(&Route::Apps, -600.0, 400.0);
        assert!(start_edge > 0.0 && start_edge < 48.0);
        assert_eq!(start_edge, -apps_edge);
        assert_eq!(horizontal_offset(&Route::Start, f64::NAN, 400.0), 0.0);
    }
}
