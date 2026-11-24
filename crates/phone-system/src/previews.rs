//! Native page snapshots are optional host capabilities. Captures happen on the
//! UI thread before reconciliation, never from rendering or layout callbacks.
pub(crate) mod bitmap;

use ferroui_base::media::IImage;
use ferroui_base::platform::IPlatformRenderInterface;
use ferroui_base::{FerroLocator, LocatorExtensions, PixelSize, Ref, Size, Visual, WeakRef};
use ferroui_controls::{Grid, TopLevel};
use ferroui_view::untrack;
use phone_domain::{AppId, Route};
use std::{
    cell::{Cell, RefCell},
    collections::VecDeque,
    rc::Rc,
};

const LIMIT: usize = 6;
const MAX_WIDTH: f64 = 270.0;
const MAX_HEIGHT: f64 = 560.0;

ferroui_base::ferro_thread_local! {
    static ENABLED:Cell<bool> = const {Cell::new(false)};
}

pub(crate) fn set_enabled(enabled: bool) {
    ENABLED.with(|flag| flag.set(enabled));
}

pub(crate) fn capture_enabled() -> bool {
    ENABLED.with(Cell::get)
}

#[derive(Clone)]
pub(crate) struct Preview {
    pub image: Rc<dyn IImage>,
    pub logical_size: Size,
}
#[derive(Clone)]
struct Presented {
    route: Route,
    obscured: bool,
    switcher: bool,
}

#[derive(Default)]
pub(crate) struct PreviewCache {
    screen: RefCell<Option<WeakRef<Grid>>>,
    presented: RefCell<Option<Presented>>,
    entries: RefCell<VecDeque<(AppId, Preview)>>,
    capturing: Cell<bool>,
}

impl PreviewCache {
    pub fn attach(&self, screen: &Ref<Grid>) {
        *self.screen.borrow_mut() = Some(screen.downgrade());
    }
    pub fn clear(&self) {
        self.entries.borrow_mut().clear();
    }
    pub fn remove(&self, app: AppId) {
        self.entries.borrow_mut().retain(|(key, _)| *key != app);
    }
    pub fn get(&self, app: AppId) -> Option<Preview> {
        self.entries
            .borrow()
            .iter()
            .find(|(key, _)| *key == app)
            .map(|(_, preview)| preview.clone())
    }

    /// The caller has not changed the old tree yet. The incoming switcher flag
    /// may already be true, while the previous screen still shows the app.
    pub fn before_render(&self, next: &Route, switcher: bool, recent: &[AppId]) {
        self.entries
            .borrow_mut()
            .retain(|(app, _)| recent.contains(app));
        if !ENABLED.with(Cell::get) || self.capturing.get() {
            return;
        }
        let previous = self.presented.borrow().clone();
        let Some(previous) = previous else {
            return;
        };
        let Some(app) = previous.route.app() else {
            return;
        };
        if previous.obscured
            || !recent.contains(&app)
            || (previous.route == *next && (!switcher || previous.switcher))
        {
            return;
        }
        let Some(preview) = untrack(|| self.capture()) else {
            return;
        };
        let mut entries = self.entries.borrow_mut();
        entries.retain(|(key, _)| *key != app);
        entries.push_front((app, preview));
        entries.truncate(LIMIT);
    }
    pub fn presented(&self, route: &Route, obscured: bool, switcher: bool) {
        *self.presented.borrow_mut() = Some(Presented {
            route: route.clone(),
            obscured,
            switcher,
        });
    }
    fn capture(&self) -> Option<Preview> {
        if self.capturing.replace(true) {
            return None;
        }
        let _guard = CaptureGuard(&self.capturing);
        let screen = self.screen.borrow().as_ref().and_then(WeakRef::upgrade)?;
        if !screen.is_attached_to_visual_tree()
            || !screen.is_visible()
            || !screen.is_measure_valid()
            || !screen.is_arrange_valid()
        {
            return None;
        }
        let visual = screen.clone().upcast::<Visual>();
        let top = TopLevel::get_top_level(Some(&visual))?;
        if top.layout_manager().is_in_layout_pass() {
            return None;
        }
        if FerroLocator::current()
            .get_service::<dyn IPlatformRenderInterface>()
            .is_none()
        {
            return None;
        }
        let size = screen.bounds().size();
        let (pixel_size, scale) = capture_size(size)?;
        // Matching DPI scales the complete logical page into the bounded pixel
        // buffer. A smaller buffer at 96 DPI would crop the page instead.
        Some(Preview {
            image: bitmap::capture(&visual, size, pixel_size, scale),
            logical_size: size,
        })
    }
}

struct CaptureGuard<'a>(&'a Cell<bool>);
impl Drop for CaptureGuard<'_> {
    fn drop(&mut self) {
        self.0.set(false);
    }
}

fn capture_size(size: Size) -> Option<(PixelSize, f64)> {
    if !size.width.is_finite()
        || !size.height.is_finite()
        || size.width <= 0.0
        || size.height <= 0.0
    {
        return None;
    }
    let scale = (MAX_WIDTH / size.width)
        .min(MAX_HEIGHT / size.height)
        .min(1.0);
    if !scale.is_finite() || scale <= 0.0 {
        return None;
    }
    let width = (size.width * scale).ceil().clamp(1.0, MAX_WIDTH) as i32;
    let height = (size.height * scale).ceil().clamp(1.0, MAX_HEIGHT) as i32;
    Some((PixelSize::new(width, height), scale))
}
