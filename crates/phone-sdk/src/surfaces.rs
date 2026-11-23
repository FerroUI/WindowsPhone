use crate::session::Session;
use ferroui_base::Ref;
use ferroui_controls::Control;
use ferroui_view::prelude::Element;
use phone_state::PhoneViewModel;
use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HostSurface {
    Camera,
    Video,
}
type SurfaceFactory = Rc<dyn Fn(HostSurface) -> Option<Ref<Control>>>;
type VolumeHandler = Rc<dyn Fn(&PhoneViewModel)>;
ferroui_base::ferro_thread_local! {
    static SURFACES: RefCell<Option<SurfaceFactory>> = const { RefCell::new(None) };
    static VOLUME: RefCell<Option<VolumeHandler>> = const { RefCell::new(None) };
    static CURRENT: RefCell<Weak<Session>> = const { RefCell::new(Weak::new()) };
}
pub fn set_host_surface_factory(factory: SurfaceFactory) {
    SURFACES.with(|slot| *slot.borrow_mut() = Some(factory));
}
pub fn host_surface(kind: HostSurface) -> Element {
    let factory = SURFACES.with(|slot| slot.borrow().clone());
    factory
        .and_then(|factory| factory(kind))
        .map(Element::Existing)
        .unwrap_or(Element::Empty)
}
pub fn set_volume_handler(handler: VolumeHandler) {
    VOLUME.with(|slot| *slot.borrow_mut() = Some(handler));
}
pub fn show_volume(vm: &PhoneViewModel) {
    let callback = VOLUME.with(|slot| slot.borrow().clone());
    if let Some(callback) = callback {
        callback(vm);
    }
}
pub fn set_current_session(ui: &Rc<Session>) {
    CURRENT.with(|slot| *slot.borrow_mut() = Rc::downgrade(ui));
}
pub fn current_session() -> Option<Rc<Session>> {
    CURRENT.with(|slot| slot.borrow().upgrade())
}
