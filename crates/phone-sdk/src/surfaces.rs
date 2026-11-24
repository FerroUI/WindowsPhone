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
    let read = Rc::downgrade(ui);
    let write = Rc::downgrade(ui);
    phone_controls::set_application_bar_controller(
        Rc::new(move || {
            read.upgrade()
                .is_some_and(|ui| ui.menu.is_alive() && ui.menu.get())
        }),
        Rc::new(move |expanded| {
            if let Some(ui) = write.upgrade() {
                if ui.menu.is_alive() && ui.menu.get_untracked() != expanded {
                    ui.menu.set(expanded);
                }
            }
        }),
    );
}
pub fn current_session() -> Option<Rc<Session>> {
    // A retained Rc can outlive the view's reactive owner. It must not expose
    // dead signals to a newly rendered page or application-bar callback.
    CURRENT.with(|slot| slot.borrow().upgrade().filter(|ui| ui.menu.is_alive()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferroui_base::interactivity::RoutedEventArgs;
    use ferroui_base::platform::IPlatformRenderInterface;
    use ferroui_base::FerroLocator;
    use ferroui_controls::Button;
    use ferroui_view::prelude::Owner;

    #[test]
    fn a_retained_session_cannot_expose_a_disposed_views_signals() {
        let _render = FerroLocator::enter_scope();
        FerroLocator::current_mutable()
            .bind::<dyn IPlatformRenderInterface>()
            .to_constant(ferroui_headless::testing::headless_platform_render_interface());
        let view = Owner::new_root();
        let registry = Rc::new(crate::AppRegistry::new(vec![]).unwrap());
        let ui = view.with(|| Rc::new(Session::new(0, registry)));
        set_current_session(&ui);
        assert!(current_session().is_some());
        view.dispose();
        assert!(!ui.menu.is_alive());
        assert!(current_session().is_none());

        // Construction reads expansion and the ellipsis writes it. Both must
        // tolerate the live Session allocation with its disposed view owner.
        let (bar, owner) = phone_controls::app_bar(vec![], false, vec![]).mount_root();
        let more = bar
            .find_descendant_of_type_where::<Button>(false, |button| button.has_name("AppBarMore"))
            .unwrap();
        more.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
        assert!(bar.classes().contains("collapsed"));
        owner.dispose();
        CURRENT.with(|slot| *slot.borrow_mut() = Weak::new());
    }
}
