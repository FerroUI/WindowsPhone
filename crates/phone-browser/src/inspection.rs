//! Live native control geometry and state for end-to-end input verification.
use ferroui_base::media::{Rotate3DTransform, TranslateTransform};
use ferroui_base::rendering::composition::CompositionVisual;
use ferroui_base::{input::InputElement, Matrix, Point, Rect, Visual};
use ferroui_controls::{
    automation::{peers::ControlAutomationPeer, AutomationProperties},
    primitives::ToggleButton,
    Application, Button, ListBox, ListBoxItem, Pivot, PivotHeaderItem, ScrollViewer, TextBlock,
    TextBox, ToggleSwitch,
};
use serde_json::json;
use wasm_bindgen::prelude::*;

/// Maps a control into the root's coordinates as it appeared in the last
/// rendered frame. `Visual::transform_to_visual` alone misses compositor
/// expressions, including the native Pivot's carousel/parallax translations.
fn rendered_transform(visual: &Visual, root: &CompositionVisual) -> Option<Matrix> {
    // Drawing items need not own a composition visual. Keep them lightweight:
    // only use the layout transform up to their nearest materialized ancestor.
    let mut anchor = visual.to_ref();
    while !anchor.has_composition_visual() {
        anchor = anchor.visual_parent()?;
    }
    let mut transform = visual.transform_to_visual(&anchor)?;
    let mut composition = (*anchor.composition_visual()?).clone();
    while !std::ptr::eq(composition.as_ref(), root) {
        // These are the same local-to-parent matrices consumed by native hit
        // testing. The base Translation property does not contain animated
        // expression values. Wait for a rendered frame when readback is absent.
        transform *= composition.try_get_valid_readback()?.matrix;
        composition = composition.parent()?;
    }
    Some(transform)
}

#[wasm_bindgen(js_name=inspectView)]
pub fn inspect_view() -> String {
    let Some(root) = Application::current()
        .and_then(|app| app.application_lifetime())
        .and_then(|lifetime| {
            lifetime
                .as_single_top_level_application_lifetime()
                .and_then(|lifetime| lifetime.top_level())
        })
    else {
        return "null".into();
    };
    let viewport = root.bounds();
    let root_composition = root
        .has_composition_visual()
        .then(|| root.composition_visual())
        .flatten();
    let elements = root
        .get_visual_descendants()
        .filter_map(|visual| {
            let name = visual.name();
            let header = visual.cast::<PivotHeaderItem>();
            let switch = visual.cast::<ToggleSwitch>();
            let text = if let Some(text) = visual.cast::<TextBlock>() {
                text.text()
            } else if let Some(text) = visual.cast::<TextBox>() {
                text.text()
            } else if let Some(header) = &header {
                header.content().as_ref().map(|content| {
                    ferroui_base::data::core::ValueTypes::to_display_string(Some(content))
                })
            } else if visual.cast::<Button>().is_some()
                || visual.cast::<ListBox>().is_some()
                || visual.cast::<ListBoxItem>().is_some()
                || visual.cast::<ScrollViewer>().is_some()
                || switch.is_some()
                || name.is_some()
            {
                None
            } else {
                return None;
            };
            let bounds = visual.bounds();
            if !visual.is_effectively_visible() || bounds.width <= 0.0 || bounds.height <= 0.0 {
                return None;
            }
            let transform = rendered_transform(&visual, root_composition.as_ref()?)?;
            let local = Rect::from_size(bounds.size());
            let rendered = local.transform_to_aabb(transform);
            if !rendered.intersects(Rect::from_size(viewport.size())) {
                return None;
            }
            let point = Point::new(0.0, 0.0).transform(transform);
            let centre = Point::new(bounds.width / 2.0, bounds.height / 2.0)
                .transform(transform);
            // A descendant hit bubbles to this control. An ancestor hit does not:
            // accepting it would mark clipped/offscreen children as clickable.
            let hit = root
                .input_hit_test_with(centre, false)
                .is_some_and(|hit| {
                    let hit: &Visual = &hit;
                    std::ptr::eq(hit, &*visual) || visual.is_visual_ancestor_of(hit)
                });
            let input = visual.cast::<InputElement>();
            let automation_name = AutomationProperties::get_name(&visual).or_else(|| {
                header
                    .as_ref()
                    .map(|header| ControlAutomationPeer::create_peer_for_element(header).get_name())
            });
            let checked = visual
                .cast::<ToggleButton>()
                .and_then(|toggle| toggle.is_checked());
            // Read the actual animation-owned native transform. Tests sample
            // this across rendered frames; it does not drive animation state.
            let projection = if name.as_deref() == Some("PhonePage") {
                visual.render_transform().and_then(|transform| {
                    let object = transform.as_object()?;
                    if let Some(rotation) = object.downcast_ref::<Rotate3DTransform>() {
                        Some(json!({"kind":"turnstile", "angleY":rotation.angle_y(),
                            "centerX":rotation.center_x(), "depth":rotation.depth()}))
                    } else {
                        object.downcast_ref::<TranslateTransform>().map(
                            |translation| json!({"kind":"slide", "translateX":translation.x()}),
                        )
                    }
                })
            } else {
                None
            };
            Some(json!({
                "type":visual.get_type().name(),"name":name,"text":text,
                "automationName":automation_name,
                "focused":input.as_ref().is_some_and(|input|input.is_focused()),
                "enabled":input.as_ref().map(|input|input.is_effectively_enabled()),
                "checked":checked,
                "selectedIndex":visual.cast::<Pivot>().map(|pivot|pivot.selected_index()),
                "projection":projection,
                // Keep logical sizes: motion assertions derive the projection
                // hinge and camera distance from the page's unprojected width.
                "x":point.x,"y":point.y,"width":bounds.width,"height":bounds.height,"hit":hit
            }))
        })
        .collect::<Vec<_>>();
    json!({"width":viewport.width,"height":viewport.height,"elements":elements}).to_string()
}
