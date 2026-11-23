use super::controller::{Contact, Controller, Hover, Mode};
use crate::session::Session;
use ferroui_base::{Point, Vector, Vector3D, Visual, WeakRef};
use ferroui_controls::{Canvas, Control, ScrollViewer};
use phone_domain::Id;
use phone_state::PhoneViewModel;
use std::time::{Duration, Instant};

pub(super) struct VisualDrag {
    control: WeakRef<Control>,
    anchor: Point,
    translation: Vector3D,
    opacity: f64,
    z_index: i32,
    target: Option<WeakRef<Control>>,
}

#[derive(Clone, Copy)]
pub(super) enum DropArea {
    Start,
    Folder(Id),
}

pub(super) fn clear(controller: &Controller) {
    let drag = controller.drag_visual.borrow_mut().take();
    if let Some(drag) = drag {
        if let Some(target) = drag.target.and_then(|target| target.upgrade()) {
            target.classes().remove("folder-drop");
        }
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
                        target: None,
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
    let hit = root.input_hit_test_filtered(
        point,
        &|visual| {
            dragged.as_ref().is_none_or(|control| {
                let source_visual: &Visual = control;
                !std::ptr::eq(source_visual, visual)
            })
        },
        false,
    );
    let drop_area = hit.as_ref().and_then(|hit| {
        let mut over_start = false;
        for control in hit
            .get_self_and_visual_ancestors()
            .filter_map(|visual| visual.cast::<Control>())
        {
            if let Some(id) = control.name().and_then(|name| {
                name.strip_prefix("FolderTiles-")
                    .and_then(|id| id.parse().ok())
            }) {
                return Some(DropArea::Folder(id));
            }
            if control.has_name("StartTiles") || control.has_name("StartScroll") {
                over_start = true;
            }
        }
        over_start.then_some(DropArea::Start)
    });
    let target = hit.and_then(|hit| {
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
        contact.drop_area = drop_area;
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
    let ready = controller
        .contacts
        .borrow()
        .get(&pointer_id)
        .and_then(|contact| contact.hover.as_ref())
        .filter(|hover| hover.center && hover.entered.elapsed() >= Duration::from_millis(600))
        .map(|hover| hover.target)
        .filter(|target| {
            controller
                .vm
                .upgrade()
                .is_some_and(|vm| can_group(&vm, source, *target))
        });
    if let Some(drag) = controller.drag_visual.borrow_mut().as_mut() {
        let old = drag.target.as_ref().and_then(|target| target.upgrade());
        let name = ready.map(|id| format!("Tile-{id}"));
        if !old
            .as_ref()
            .is_some_and(|target| name.as_ref().is_some_and(|name| target.has_name(name)))
        {
            if let Some(old) = old {
                old.classes().remove("folder-drop");
            }
            drag.target = name
                .and_then(|name| {
                    root.find_descendant_of_type_where::<Control>(false, |control| {
                        control.has_name(&name)
                    })
                })
                .map(|target| {
                    target.classes().add("folder-drop");
                    target.downgrade()
                });
        }
    }
    let target = target.map(|(id, _, _)| id);
    if ui.drop_target.get_untracked() != target {
        ui.drop_target.set(target);
    }
    true
}
pub(super) fn finish(vm: &PhoneViewModel, ui: &Session, contact: &Contact) {
    let Some(source) = contact.tile else { return };
    let Some(hover) = contact.hover.as_ref() else {
        match contact.drop_area {
            Some(DropArea::Start) => {
                let parent = vm.data().parent_folder(source);
                if let Some(folder) = parent {
                    vm.extract_folder_tile(folder, source);
                } else {
                    vm.move_tile(source, None);
                }
                ui.edit_tile.set(Some(source));
            }
            Some(DropArea::Folder(folder)) => {
                let last = vm
                    .data()
                    .tile(folder)
                    .and_then(|folder| folder.folder_tiles.last())
                    .map(|tile| tile.id);
                if let Some(last) = last {
                    vm.move_tile_relative(source, last, true);
                    ui.edit_tile.set(Some(source));
                }
            }
            None => {}
        }
        return;
    };
    if hover.center
        && hover.entered.elapsed() >= Duration::from_millis(600)
        && can_group(vm, source, hover.target)
    {
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

fn can_group(vm: &PhoneViewModel, source: Id, target: Id) -> bool {
    let data = vm.data();
    source != target
        && data.parent_folder(target).is_none()
        && data.parent_folder(source) != Some(target)
        && data.parent_folder(target) != Some(source)
}
