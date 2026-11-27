//! The two-section cyclic seam can expose both ends of the same section.
//!
//! Native Carousel correctly retains one input container for each item. Its
//! fractional panel therefore chooses one of those two logical slots. Mirror
//! only the other visible slot through the same live RedirectVisual mechanism
//! used by FerroUI's native Pivot header ghost. This is a render instance, not a
//! second control tree, bitmap snapshot, altered scroll offset or event target.
use ferroui_base::rendering::composition::{ElementComposition, RedirectVisual};
use ferroui_base::{Point, Ref, Vector, Vector3D, WeakRef};
use ferroui_controls::{Canvas, Carousel};
use std::cell::RefCell;
use std::rc::Rc;

pub(super) struct WrapPreview {
    host: WeakRef<Canvas>,
    visual: RefCell<Option<RedirectVisual>>,
}

impl WrapPreview {
    pub fn new(host: &Ref<Canvas>) -> Self {
        Self {
            host: host.downgrade(),
            visual: RefCell::new(None),
        }
    }

    /// Follow the actual arranged native containers. A two-item cycle is wider
    /// than the viewport, so at most one alternate slot can intersect it.
    pub fn refresh(&self, owner: &Carousel, item_extent: f64) {
        let Some(host) = self.host.upgrade() else {
            self.clear();
            return;
        };
        if owner.item_count() != 2
            || !owner.wrap_selection()
            || !owner.is_effectively_visible()
            || !host.is_attached_to_visual_tree()
            || !host.is_effectively_visible()
            || item_extent <= 0.0
            || !item_extent.is_finite()
        {
            self.clear();
            return;
        }
        let span = 2.0 * item_extent;
        for container in owner.get_realized_containers() {
            if !container.is_visible() {
                continue;
            }
            let Some(origin) = container.translate_point(Point::default(), &host) else {
                continue;
            };
            let size = container.bounds().size();
            if size.width <= 0.0 || size.height <= 0.0 {
                continue;
            }
            for direction in [-1.0, 1.0] {
                let x = origin.x + direction * span;
                // Native layout can ceil a 360.00000000000006-wide container
                // to 361 pixels. Its logical slot is still exactly 360 wide:
                // that rounding fringe must not manufacture an alternate slot
                // at the opposite viewport edge. Reject zero-area tangency,
                // without perturbing the native offset or slot placement.
                let overlap = (x + item_extent).min(host.bounds().width) - x.max(0.0);
                if overlap <= 1e-6 {
                    continue;
                }
                let (Some(source), Some(host_visual)) = (
                    ElementComposition::get_element_visual(&container),
                    ElementComposition::get_element_visual(&host),
                ) else {
                    self.clear();
                    return;
                };
                let mut current = self.visual.borrow_mut();
                if current.as_ref().is_some_and(|visual| {
                    !Rc::ptr_eq(visual.compositor(), host_visual.compositor())
                }) {
                    if let Some(visual) = current.take() {
                        visual.set_source(None);
                    }
                    ElementComposition::set_element_child_visual(&host, None);
                }
                let visual = current.get_or_insert_with(|| {
                    let visual = host_visual.compositor().create_redirect_visual();
                    // The private section's root clip is intentionally ignored
                    // by RedirectVisual; preserve it at the duplicate's bounds.
                    visual.set_clip_to_bounds(true);
                    ElementComposition::set_element_child_visual(
                        &host,
                        Some(visual.visual().clone()),
                    );
                    visual
                });
                if !visual.source().is_some_and(|old| Rc::ptr_eq(&old, &source)) {
                    visual.set_source(Some(source));
                }
                visual.set_offset(Vector3D::new(x, origin.y, 0.0));
                visual.set_size(Vector::new(size.width.min(item_extent), size.height));
                return;
            }
        }
        self.clear();
    }

    /// Clear before detaching: native ElementComposition immediately unparents
    /// its hand-in visual, and clearing Source releases the live subtree lease.
    pub fn clear(&self) {
        if let Some(visual) = self.visual.borrow_mut().take() {
            visual.set_source(None);
            if let Some(host) = self.host.upgrade() {
                ElementComposition::set_element_child_visual(&host, None);
            }
        }
    }
}

impl Drop for WrapPreview {
    fn drop(&mut self) {
        self.clear();
    }
}
