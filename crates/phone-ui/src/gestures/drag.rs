use super::controller::{Contact, Controller, Hover, Mode};
use crate::session::Session;
use ferroui_base::{Point, Vector, Vector3D, Visual, WeakRef};
use ferroui_controls::{Canvas, Control, ScrollViewer};
use phone_state::PhoneViewModel;
use std::time::{Duration, Instant};

pub(super) struct VisualDrag {
    control: WeakRef<Control>,
    anchor: Point,
    translation: Vector3D,
    opacity: f64,
    z_index: i32,
}

pub(super) fn clear(controller: &Controller) {
    let drag = controller.drag_visual.borrow_mut().take();
    if let Some(drag) = drag {
        if let Some(control) = drag.control.upgrade() {
            control.set_translation(drag.translation);
            control.set_opacity(drag.opacity);
            control.set_z_index(drag.z_index);
        }
    }
}

/// Re-evaluate targets after auto-scrolling even while the finger is stationary.
pub(super) fn update(controller: &Controller, scroll: bool) -> bool {
    let Some((pointer_id, source, point, origin, moved)) = controller
        .contacts
        .borrow()
        .iter()
        .find_map(|(id, contact)| {
            (contact.mode == Mode::Drag).then(|| {
                (
                    *id,
                    contact.tile,
                    Point::new(contact.motion.position.0, contact.motion.position.1),
                    Point::new(contact.motion.origin.0, contact.motion.origin.1),
                    contact.motion.distance >= 8.0,
                )
            })
        })
    else {
        return false;
    };
    let (Some(root), Some(ui), Some(source)) =
        (controller.root.upgrade(), controller.ui.upgrade(), source)
    else {
        return false;
    };
    if !moved {
        return true;
    }
    if ui.edit_tile.get_untracked() != Some(source) {
        ui.edit_tile.set(Some(source));
    }
    if controller.drag_visual.borrow().is_none() {
        let name = format!("Tile-{source}");
        if let Some(tile) =
            root.find_descendant_of_type_where::<Control>(false, |control| control.has_name(&name))
        {
            if let Some(control) = tile
                .get_self_and_visual_ancestors()
                .filter_map(|visual| visual.cast::<Control>())
                .find(|control| {
                    control
                        .visual_parent()
                        .is_some_and(|parent| parent.cast::<Canvas>().is_some())
                })
            {
                if let Some(anchor) = root.translate_point(origin, &control) {
                    let visual = VisualDrag {
                        control: control.downgrade(),
                        anchor,
                        translation: control.translation(),
                        opacity: control.opacity(),
                        z_index: control.z_index(),
                    };
                    control.set_z_index(10_000);
                    control.set_opacity(0.88);
                    *controller.drag_visual.borrow_mut() = Some(visual);
                }
            }
        }
    }
    if scroll {
        if let Some(scroller) = root.find_descendant_of_type_where::<ScrollViewer>(false, |view| {
            view.has_name("StartScroll")
        }) {
            if let Some(local) = root.translate_point(point, &scroller) {
                let bounds = scroller.bounds();
                if local.x >= 0.0
                    && local.x <= bounds.width
                    && local.y >= 0.0
                    && local.y <= bounds.height
                {
                    let delta = if local.y < 36.0 {
                        -12.0
                    } else if local.y > bounds.height - 36.0 {
                        12.0
                    } else {
                        0.0
                    };
                    if delta != 0.0 {
                        let offset = scroller.offset();
                        let max = (scroller.extent().height - scroller.viewport().height).max(0.0);
                        scroller
                            .set_offset(Vector::new(offset.x, (offset.y + delta).clamp(0.0, max)));
                    }
                }
            }
        }
    }
    let dragged = controller
        .drag_visual
        .borrow()
        .as_ref()
        .and_then(|drag| drag.control.upgrade());
    if let (Some(control), Some(drag)) = (&dragged, controller.drag_visual.borrow().as_ref()) {
        if let Some(local) = root.translate_point(point, control) {
            control.set_translation(Vector3D::new(
                drag.translation.x + local.x - drag.anchor.x,
                drag.translation.y + local.y - drag.anchor.y,
                drag.translation.z,
            ));
        }
    }
    let target = root
        .input_hit_test_filtered(
            point,
            &|visual| {
                dragged.as_ref().is_none_or(|control| {
                    let source_visual: &Visual = control;
                    !std::ptr::eq(source_visual, visual)
                })
            },
            false,
        )
        .and_then(|hit| {
            hit.get_self_and_visual_ancestors()
                .filter_map(|visual| visual.cast::<Control>())
                .find_map(|control| {
                    let id = control.name().and_then(|name| {
                        name.strip_prefix("Tile-")
                            .and_then(|id| id.parse::<u64>().ok())
                    })?;
                    if id == source {
                        return None;
                    }
                    let local = root.translate_point(point, &control)?;
                    let bounds = control.bounds();
                    let center = local.x > bounds.width * 0.23
                        && local.x < bounds.width * 0.77
                        && local.y > bounds.height * 0.23
                        && local.y < bounds.height * 0.77;
                    let after = local.y > bounds.height * 0.70
                        || (local.y > bounds.height * 0.30 && local.x > bounds.width * 0.70);
                    Some((id, center, after))
                })
        });
    {
        let mut contacts = controller.contacts.borrow_mut();
        let Some(contact) = contacts.get_mut(&pointer_id) else {
            return false;
        };
        contact.hover = target.map(|(target, center, after)| {
            let entered = contact
                .hover
                .as_ref()
                .filter(|old| old.target == target && old.center == center)
                .map(|old| old.entered)
                .unwrap_or_else(Instant::now);
            Hover {
                target,
                center,
                after,
                entered,
            }
        });
    }
    let target = target.map(|(id, _, _)| id);
    if ui.drop_target.get_untracked() != target {
        ui.drop_target.set(target);
    }
    true
}
pub(super) fn finish(vm: &PhoneViewModel, ui: &Session, contact: &Contact) {
    let (Some(source), Some(hover)) = (contact.tile, contact.hover.as_ref()) else {
        return;
    };
    if hover.center && hover.entered.elapsed() >= Duration::from_millis(600) {
        let name = vm
            .data()
            .tile(hover.target)
            .filter(|tile| tile.is_folder())
            .map(|tile| tile.label.clone())
            .unwrap_or_else(|| "Folder".into());
        vm.create_folder(source, hover.target, name);
        ui.edit_tile.set(Some(hover.target));
    } else {
        vm.move_tile_relative(source, hover.target, hover.after);
        ui.edit_tile.set(Some(source));
    }
}
