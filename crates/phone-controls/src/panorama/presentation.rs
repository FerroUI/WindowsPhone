use super::layout::{Geometry, TITLE_FONT_SIZE, TITLE_TOP};
use super::policy::Policy;
use super::wrap_preview::WrapPreview;
use super::{Configuration, admission::Admission};
use ferroui_base::input::InputElement;
use ferroui_base::media::{Brushes, TranslateTransform};
use ferroui_base::reactive::{IDisposable, Subscription};
use ferroui_base::{Ref, Thickness, Visual, WeakRef};
use ferroui_controls::primitives::SelectingItemsControl;
use ferroui_controls::{Border, Canvas, Carousel, Grid, TextBlock};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

pub(super) struct Presentation {
    owner: WeakRef<Carousel>,
    root: WeakRef<Grid>,
    viewport: WeakRef<Border>,
    title: WeakRef<TextBlock>,
    backgrounds: Vec<WeakRef<Border>>,
    title_transform: Ref<TranslateTransform>,
    background_transforms: Vec<Ref<TranslateTransform>>,
    wrap_preview: WrapPreview,
    admission: Admission,
    _policy: Rc<Policy>,
    subscriptions: RefCell<Vec<Subscription>>,
    properties: RefCell<Vec<Rc<dyn IDisposable>>>,
    origin: Cell<Option<f64>>,
    geometry: Cell<Option<Geometry>>,
    updating: Cell<bool>,
}

impl PartialEq for Presentation {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl Presentation {
    pub fn attach(
        owner: &Ref<Carousel>,
        root: &Ref<Grid>,
        viewport: &Ref<Border>,
        title: &Ref<TextBlock>,
        backgrounds: &[Ref<Border>],
        wrap_preview: &Ref<Canvas>,
    ) -> Rc<Self> {
        let title_transform = TranslateTransform::new();
        title.set_render_transform(Some((&title_transform).into()));
        let background_transforms = backgrounds
            .iter()
            .map(|part| {
                let transform = TranslateTransform::new();
                part.set_render_transform(Some((&transform).into()));
                transform
            })
            .collect();
        let state = Rc::new(Self {
            owner: owner.downgrade(),
            root: root.downgrade(),
            viewport: viewport.downgrade(),
            title: title.downgrade(),
            backgrounds: backgrounds.iter().map(|part| part.downgrade()).collect(),
            title_transform,
            background_transforms,
            wrap_preview: WrapPreview::new(wrap_preview),
            admission: Admission::default(),
            _policy: Policy::attach(owner),
            subscriptions: RefCell::new(Vec::new()),
            properties: RefCell::new(Vec::new()),
            origin: Cell::new(None),
            geometry: Cell::new(None),
            updating: Cell::new(false),
        });
        let weak = Rc::downgrade(&state);
        let layout = root.layout_updated(move || {
            if let Some(state) = weak.upgrade() {
                state.refresh();
            }
        });
        let weak = Rc::downgrade(&state);
        let attached = root.attached_to_visual_tree(move |_| {
            if let Some(state) = weak.upgrade() {
                state.refresh();
            }
        });
        let weak = Rc::downgrade(&state);
        let detached = root.detached_from_visual_tree(move |_| {
            if let Some(state) = weak.upgrade() {
                state.wrap_preview.clear();
            }
        });
        let weak = Rc::downgrade(&state);
        let visible = root.is_effectively_visible_changed(move || {
            if let Some(state) = weak.upgrade() {
                state.refresh();
            }
        });
        let weak = Rc::downgrade(&state);
        let properties = owner.property_changed(move |change| {
            if change.property() == Configuration::title_property().as_property()
                || change.property() == Carousel::is_swipe_enabled_property().as_property()
                || change.property() == Carousel::is_swiping_property().as_property()
                || change.property()
                    == SelectingItemsControl::selected_index_property().as_property()
                || change.property()
                    == InputElement::is_effectively_enabled_property().as_property()
                || change.property() == Visual::bounds_property().as_property()
            {
                if let Some(state) = weak.upgrade() {
                    state.refresh();
                }
            }
        });
        state
            .subscriptions
            .borrow_mut()
            .extend([layout, attached, detached, visible]);
        state.properties.borrow_mut().push(properties);
        state.refresh();
        state
    }

    fn refresh(&self) {
        if self.updating.replace(true) {
            return;
        }
        struct Guard<'a>(&'a Cell<bool>);
        impl Drop for Guard<'_> {
            fn drop(&mut self) {
                self.0.set(false);
            }
        }
        let _guard = Guard(&self.updating);
        let (Some(owner), Some(root), Some(viewport), Some(title)) = (
            self.owner.upgrade(),
            self.root.upgrade(),
            self.viewport.upgrade(),
            self.title.upgrade(),
        ) else {
            return;
        };
        if let Some(value) = owner.get_value(Configuration::title_property()) {
            if title.text().as_deref() != Some(value.as_str()) {
                title.set_text(Some(&value));
            }
        }
        title.set_font_size(TITLE_FONT_SIZE * crate::text_scale());
        Canvas::set_top(&title, TITLE_TOP * crate::text_scale());
        let width = root.bounds().width;
        if width <= 0.0 || !width.is_finite() {
            return;
        }
        let count = owner.item_count().max(0) as usize;
        let geometry = Geometry::new(width, count);
        if self.geometry.get() != Some(geometry) {
            self.geometry.set(Some(geometry));
            // A centered native fractional viewport is extended and clipped
            // on its left. The selected section begins exactly at x=0.
            viewport.set_margin(Thickness::new(-geometry.inset, 0.0, 0.0, 0.0));
            owner.set_viewport_fraction(geometry.viewport_fraction);
            for background in &self.backgrounds {
                if let Some(background) = background.upgrade() {
                    background.set_width(geometry.background_extent);
                }
            }
        }
        let motion = owner.is_swipe_enabled() && crate::motion_enabled();
        let Some(panel) = owner.items_panel_root() else {
            return;
        };
        if panel.background().is_none() {
            panel.set_background(Some(Brushes::transparent()));
        }
        let offset = panel
            .as_scrollable()
            .map_or(owner.selected_index().max(0) as f64, |scrollable| {
                scrollable.offset().x
            });
        let origin = self.origin.get().unwrap_or_else(|| {
            let origin = owner.selected_index().max(0) as f64;
            self.origin.set(Some(origin));
            origin
        });
        let phase = if motion { offset - origin } else { 0.0 };
        self.title_transform
            .set_x(geometry.title_x(phase, count, title.desired_size().width));
        let background_x = geometry.background_x(phase, count);
        for (index, transform) in self.background_transforms.iter().enumerate() {
            transform.set_x(background_x + index as f64 * geometry.background_extent);
        }
        self.admission.refresh(&owner);
        self.wrap_preview.refresh(&owner, geometry.item_extent);
    }
}

impl Drop for Presentation {
    fn drop(&mut self) {
        for property in self.properties.get_mut().drain(..) {
            property.dispose();
        }
        self.subscriptions.get_mut().clear();
    }
}
