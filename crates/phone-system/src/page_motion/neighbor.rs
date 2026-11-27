//! One gesture-owned neighboring-page image. The temporary native tree has an
//! isolated model/session and is disposed immediately after the bounded capture.
//! Pointer movement changes only compositor translation, never route or data.
use super::{NavigationMotion, admission::PageAdmission, animation, snapshot};
use crate::session::{Session, SystemSessionExt};
use ferroui_base::input::{KeyboardNavigation, KeyboardNavigationMode};
use ferroui_base::interactivity::RoutedEventHandlerToken;
use ferroui_base::layout::ILayoutManager;
use ferroui_base::threading::{CancellationToken, Dispatcher, DispatcherPriority};
use ferroui_base::{Ref, Size, Vector3D, Visual, WeakRef};
use ferroui_controls::{Border, Control, Image, TopLevel};
use ferroui_view::prelude::*;
use phone_domain::Route;
use phone_state::PhoneViewModel;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

#[cfg(test)]
#[path = "neighbor_tests.rs"]
mod tests;

#[derive(Default)]
pub(super) struct NeighborPreview {
    stage: RefCell<Option<WeakRef<Border>>>,
    surface: RefCell<Option<WeakRef<Border>>>,
    generation: Cell<u64>,
    request: RefCell<Option<Request>>,
    returning: RefCell<Option<Returning>>,
}

struct RenderOwner(Owner);
impl Drop for RenderOwner {
    fn drop(&mut self) {
        if self.0.is_alive() {
            self.0.dispose();
        }
    }
}

struct Staging {
    root: Ref<Control>,
    _owner: RenderOwner,
}

struct Request {
    generation: u64,
    source: Route,
    epoch: u64,
    viewport: (f64, f64),
    page_size: Size,
    direction: f64,
    delta: f64,
    origin: Vector3D,
    page: WeakRef<Control>,
    vm: Weak<PhoneViewModel>,
    ui: Weak<Session>,
    staging: Option<Staging>,
}

struct Returning {
    generation: u64,
    cancellation: CancellationToken,
    page: WeakRef<Control>,
    context: Request,
    admission: PageAdmission,
    unloaded: RoutedEventHandlerToken,
}

/// Failure or unwinding must not leave an enabled staging tree or retain its
/// owner. The asynchronous capture callback itself holds only a weak manager.
struct CaptureGuard<'a> {
    preview: &'a NeighborPreview,
    stage: Ref<Border>,
    generation: u64,
    completed: bool,
}
impl Drop for CaptureGuard<'_> {
    fn drop(&mut self) {
        self.stage.set_is_enabled(false);
        if !self.completed && self.preview.generation.get() == self.generation {
            self.preview.cancel();
        }
    }
}

fn permitted(vm: &PhoneViewModel, ui: &Session, route: &Route) -> bool {
    matches!(route, Route::Start | Route::Apps)
        && vm.route() == *route
        && vm.motion_enabled()
        && crate::previews::capture_enabled()
        && vm.status().is_empty()
        && !ui.preview_obscured(route)
        && !phone_controls::has_open_phone_picker()
}

impl Request {
    fn valid(&self) -> bool {
        let (Some(vm), Some(ui), Some(page)) =
            (self.vm.upgrade(), self.ui.upgrade(), self.page.upgrade())
        else {
            return false;
        };
        permitted(&vm, &ui, &self.source)
            && vm.document_epoch() == self.epoch
            && vm.viewport() == self.viewport
            && page.is_attached_to_visual_tree()
            && page.bounds().size() == self.page_size
            && page.render_transform().is_none()
    }
}

impl NeighborPreview {
    pub(super) fn ready(&self, source: &Route) -> bool {
        self.request
            .borrow()
            .as_ref()
            .is_some_and(|request| request.source == *source && request.valid())
            && self.surface().is_some_and(|surface| {
                surface.opacity() > 0.0
                    && surface
                        .child()
                        .and_then(|child| child.cast::<Image>())
                        .is_some_and(|image| image.source().is_some())
            })
    }

    fn surface(&self) -> Option<Ref<Border>> {
        self.surface.borrow().as_ref().and_then(WeakRef::upgrade)
    }

    fn clear_image(&self) {
        if let Some(surface) = self.surface() {
            surface.set_opacity(0.0);
            surface.set_translation(Vector3D::default());
            if let Some(image) = surface.child().and_then(|child| child.cast::<Image>()) {
                image.set_source(None);
            }
        }
    }

    fn discard_stage(&self, staging: Staging) {
        if let Some(stage) = self.stage.borrow().as_ref().and_then(WeakRef::upgrade) {
            if stage
                .child()
                .is_some_and(|child| child.ptr_eq(&staging.root))
            {
                stage.set_child(None::<Ref<Control>>);
            }
        }
        drop(staging);
    }

    /// Retire the generation before cancellation callbacks or owner cleanup.
    pub(super) fn cancel(&self) -> bool {
        self.generation.set(self.generation.get().wrapping_add(1));
        let request = self.request.borrow_mut().take();
        let returning = self.returning.borrow_mut().take();
        let active = request.is_some() || returning.is_some();
        if let Some(returning) = returning {
            returning.cancellation.cancel();
            if let Some(page) = returning.page.upgrade() {
                page.remove_handler(Control::unloaded_event(), returning.unloaded);
            }
            returning.admission.restore(returning.context.valid());
        }
        if let Some(mut request) = request {
            if let Some(page) = request.page.upgrade() {
                // Restore only our held translation; a later owner may have
                // already installed its own geometry during cancellation.
                let current = page.translation();
                if (current.x - request.origin.x - request.delta).abs() < 1e-8
                    && current.y == request.origin.y
                    && current.z == request.origin.z
                {
                    page.set_translation(request.origin);
                }
            }
            if let Some(staging) = request.staging.take() {
                self.discard_stage(staging);
            }
        }
        self.clear_image();
        active
    }

    fn reconcile(&self) -> bool {
        let invalid = self
            .request
            .borrow()
            .as_ref()
            .is_some_and(|request| !request.valid());
        invalid && self.cancel()
    }

    fn update(
        self: &Rc<Self>,
        vm: &Rc<PhoneViewModel>,
        ui: &Rc<Session>,
        page: &Ref<Control>,
        source: &Route,
        origin: Vector3D,
        delta: f64,
    ) {
        if !permitted(vm, ui, source) || page.render_transform().is_some() {
            self.cancel();
            return;
        }
        let direction = if *source == Route::Start { 1.0 } else { -1.0 };
        // An outward edge pan never exposes a fabricated third page. Moving
        // through the origin may resume the existing inward preview later.
        let inward = delta * direction < 0.0;
        if !inward {
            if let Some(surface) = self.surface() {
                surface.set_opacity(0.0);
            }
            if let Some(request) = self.request.borrow_mut().as_mut() {
                request.delta = delta;
            }
            return;
        }
        self.reconcile();
        if self.request.borrow().is_none() {
            self.cancel();
            let Some(stage) = self.stage.borrow().as_ref().and_then(WeakRef::upgrade) else {
                return;
            };
            if self.surface().is_none() || page.bounds().width <= 0.0 || page.bounds().height <= 0.0
            {
                return;
            }
            let generation = self.generation.get();
            let size = page.bounds().size();
            let destination = if direction > 0.0 {
                Route::Apps
            } else {
                Route::Start
            };
            let owner = RenderOwner(Owner::new_root());
            phone_sdk::images::cached_only(&owner.0);
            let weak = Rc::downgrade(self);
            let root = owner.0.with(|| {
                let model = vm.preview_snapshot(destination.clone());
                let session = Rc::new(Session::new(vm.now(), ui.registry()));
                *session.scrolls.borrow_mut() = ui.scrolls.borrow().clone();
                session.document_epoch.set(vm.document_epoch());
                *session.last_route.borrow_mut() = destination.clone();
                let background = crate::common::bg(&model);
                let content = match destination {
                    // Native catalog composition owns keyed-list hooks. It
                    // requires a component render scope just like shell.view;
                    // the scope still belongs to our isolated capture owner.
                    Route::Apps => {
                        rerender(move || crate::app_list::preview(model.clone(), session.clone()))
                    }
                    _ => crate::start::view(model, session),
                };
                border()
                    .name("PhoneNeighborCapture")
                    .width(size.width)
                    .height(size.height)
                    .clip_to_bounds(true)
                    .background(background.as_str())
                    .on_loaded(move |_| {
                        let weak = weak.clone();
                        // Child Loaded handlers restore scroll offsets first.
                        // No root rerender or route callback is scheduled.
                        Dispatcher::ui_thread().post_local(
                            move || {
                                if let Some(preview) = weak.upgrade() {
                                    preview.capture(generation);
                                }
                            },
                            DispatcherPriority::BACKGROUND,
                        );
                    })
                    .child(content)
                    .into_element()
                    .mount()
            });
            *self.request.borrow_mut() = Some(Request {
                generation,
                source: source.clone(),
                epoch: vm.document_epoch(),
                viewport: vm.viewport(),
                page_size: size,
                direction,
                delta,
                origin,
                page: page.downgrade(),
                vm: Rc::downgrade(vm),
                ui: Rc::downgrade(ui),
                staging: Some(Staging {
                    root: root.clone(),
                    _owner: owner,
                }),
            });
            stage.set_child(root);
        }
        if let Some(request) = self.request.borrow_mut().as_mut() {
            request.delta = delta;
        }
        self.position();
    }

    fn position(&self) {
        let request = self.request.borrow();
        let Some(request) = request.as_ref() else {
            return;
        };
        if let Some(surface) = self.surface() {
            surface.set_translation(Vector3D::new(
                request.direction * request.page_size.width + request.delta,
                0.0,
                0.0,
            ));
            let ready = surface
                .child()
                .and_then(|child| child.cast::<Image>())
                .is_some_and(|image| image.source().is_some());
            surface.set_opacity(if ready && request.delta * request.direction < 0.0 {
                1.0
            } else {
                0.0
            });
        }
    }

    fn capture(self: &Rc<Self>, generation: u64) {
        if generation != self.generation.get() {
            return;
        }
        if self.reconcile() {
            return;
        }
        let root = {
            let request = self.request.borrow();
            let Some(request) = request
                .as_ref()
                .filter(|request| request.generation == generation && request.valid())
            else {
                return;
            };
            request.staging.as_ref().map(|stage| stage.root.clone())
        };
        let Some(root) = root else {
            return;
        };
        let Some(stage) = self.stage.borrow().as_ref().and_then(WeakRef::upgrade) else {
            self.cancel();
            return;
        };
        let mut guard = CaptureGuard {
            preview: self,
            stage: stage.clone(),
            generation,
            completed: false,
        };
        // The hidden staging tree rejects input while waiting for layout.
        // Temporarily enable it only during this synchronous capture so
        // disabled-control styling is not baked into the preview. Its hit-test
        // and keyboard-navigation barriers remain in place throughout.
        stage.set_is_enabled(true);
        let visual = root.clone().upcast::<Visual>();
        if let Some(top) = TopLevel::get_top_level(Some(&visual)) {
            if !top.layout_manager().is_in_layout_pass() {
                top.layout_manager().execute_layout_pass();
            }
        }
        let captured = ferroui_view::untrack(|| snapshot::capture(&root));
        stage.set_is_enabled(false);
        let staging = {
            let mut request = self.request.borrow_mut();
            let Some(request) = request
                .as_mut()
                .filter(|request| request.generation == generation)
            else {
                return;
            };
            request.staging.take()
        };
        if let Some(staging) = staging {
            self.discard_stage(staging);
        }
        if self.reconcile() {
            return;
        }
        if self.generation.get() != generation {
            return;
        }
        if let (Some(preview), Some(surface)) = (captured, self.surface()) {
            if let Some(image) = surface.child().and_then(|child| child.cast::<Image>()) {
                image.set_source(Some(preview.image));
            }
        }
        self.position();
        guard.completed = true;
    }

    fn finish(self: &Rc<Self>, page: &Ref<Control>, delta: f64, animate: bool) -> bool {
        if self.reconcile() {
            return false;
        }
        let edge = self
            .request
            .borrow()
            .as_ref()
            .map(|request| request.direction * request.page_size.width);
        let surface = self.surface();
        let ready = surface
            .as_ref()
            .filter(|surface| surface.opacity() > 0.0)
            .and_then(|surface| surface.child())
            .and_then(|child| child.cast::<Image>())
            .is_some_and(|image| image.source().is_some());
        if !animate
            || !ready
            || !phone_controls::motion_enabled()
            || page.render_transform().is_some()
            || delta.abs() <= f64::EPSILON
        {
            self.cancel();
            return false;
        }
        let Some(edge) = edge else {
            self.cancel();
            return false;
        };
        let Some(mut request) = self.request.borrow_mut().take() else {
            return false;
        };
        if let Some(staging) = request.staging.take() {
            self.discard_stage(staging);
        }
        let surface = surface.unwrap().upcast::<Control>();
        surface.set_translation(Vector3D::default());
        let generation = self.generation.get();
        let cancellation = CancellationToken::new();
        let weak = Rc::downgrade(self);
        let unloaded = page.add_handler(Control::unloaded_event(), move |_, _| {
            if let Some(preview) = weak.upgrade() {
                preview.complete(generation);
            }
        });
        let admission = PageAdmission::block(page);
        if self.generation.get() != generation || !request.valid() {
            page.remove_handler(Control::unloaded_event(), unloaded);
            return false;
        }
        *self.returning.borrow_mut() = Some(Returning {
            generation,
            cancellation: cancellation.clone(),
            page: page.downgrade(),
            context: request,
            admission,
            unloaded,
        });
        let task = animation::settle_pair(page, &surface, delta, edge, cancellation);
        let weak = Rc::downgrade(self);
        task.on_completed(move || {
            if let Some(preview) = weak.upgrade() {
                preview.complete(generation);
            }
        });
        true
    }

    fn complete(&self, generation: u64) {
        let active = self
            .returning
            .borrow()
            .as_ref()
            .is_some_and(|state| state.generation == generation);
        if active {
            self.cancel();
        }
    }
}

impl NavigationMotion {
    pub(crate) fn attach_neighbor(&self, surface: &Ref<Border>) {
        *self.neighbor.surface.borrow_mut() = Some(surface.downgrade());
        surface.set_is_hit_test_visible(false);
        surface.set_is_enabled(false);
        if self.neighbor.request.borrow().is_none() && self.neighbor.returning.borrow().is_none() {
            self.neighbor.clear_image();
        }
    }

    pub(crate) fn attach_neighbor_stage(&self, stage: &Ref<Border>) {
        *self.neighbor.stage.borrow_mut() = Some(stage.downgrade());
        stage.set_opacity(0.0);
        stage.set_is_hit_test_visible(false);
        stage.set_is_enabled(false);
        stage.set_focusable(false);
        stage.set_is_tab_stop(false);
        KeyboardNavigation::set_tab_navigation(stage, KeyboardNavigationMode::None);
    }

    pub(crate) fn reconcile_neighbor(&self) {
        if self.neighbor.reconcile() {
            crate::gestures::cancel();
        }
    }

    pub(crate) fn drag_neighbor(
        &self,
        vm: &Rc<PhoneViewModel>,
        ui: &Rc<Session>,
        page: &Ref<Control>,
        route: &Route,
        origin: Vector3D,
        delta: f64,
    ) {
        if self.host.eligible.get() {
            self.neighbor.update(vm, ui, page, route, origin, delta);
        }
    }

    pub(crate) fn finish_neighbor(&self, page: &Ref<Control>, delta: f64, animate: bool) -> bool {
        self.neighbor.finish(page, delta, animate)
    }
}
