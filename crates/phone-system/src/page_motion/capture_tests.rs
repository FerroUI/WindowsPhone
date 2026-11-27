//! Exercise the mounted shell and the production RenderTargetBitmap path.
//! Choreography tests inject a counted image; this regression must also prove
//! that document restoration leaves an eligible, loaded native capture source.
use super::snapshot;
use ferroui_base::input::RawInputModifiers;
use ferroui_base::media::TranslateTransform;
use ferroui_base::threading::{CancellationToken, Dispatcher};
use ferroui_base::{Point, Ref};
use ferroui_controls::{AppBuilder, Application, Control, Image, Window};
use ferroui_headless::{
    FerroHeadlessPlatform, FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions,
    HeadlessUnitTestSession, HeadlessWindowExtensions,
};
use phone_domain::{PhoneData, Route};
use phone_sdk::AppRegistry;
use phone_state::PhoneViewModel;
use std::{rc::Rc, time::Duration};

fn frame() {
    Dispatcher::ui_thread().run_jobs(None);
    FerroHeadlessPlatform::force_render_timer_tick(1);
    Dispatcher::ui_thread().run_jobs(None);
}

fn named(root: &Ref<Control>, name: &str) -> Ref<Control> {
    root.find_descendant_of_type_where::<Control>(false, |part| part.has_name(name))
        .unwrap_or_else(|| panic!("missing mounted shell control {name}"))
}

fn slide_x(page: &Ref<Control>) -> f64 {
    page.render_transform()
        .and_then(|transform| {
            transform.as_object().and_then(|object| {
                object
                    .downcast_ref::<TranslateTransform>()
                    .map(|translation| translation.x())
            })
        })
        .expect("the mounted page has a native slide transform")
}

struct CaptureEnabled;
impl Drop for CaptureEnabled {
    fn drop(&mut self) {
        crate::set_preview_capture_enabled(false);
    }
}

#[test]
fn restored_start_page_registers_a_fresh_capture_source_for_the_next_shell_swipe() {
    let session = HeadlessUnitTestSession::start_new(|| {
        AppBuilder::configure::<Application>().use_headless(FerroHeadlessPlatformOptions::default())
    });
    let result = session
        .dispatch(
            || {
                phone_controls::install_theme(&Application::current().unwrap());
                crate::set_preview_capture_enabled(true);
                let _capture = CaptureEnabled;
                let mut data = PhoneData::default();
                data.settings.lock_timeout = 0;
                data.settings.reduced_motion = false;
                data.settings.battery_saver = false;
                data.settings.live_tiles = false;
                data.alarms.clear();
                data.events.clear();
                let vm = PhoneViewModel::new(data.clone(), Rc::new(|_| {}));
                vm.set_viewport(320.0, 740.0);
                let root = crate::create_view(
                    vm.clone(),
                    Rc::new(AppRegistry::new(Vec::new()).unwrap()),
                );
                let window = Window::new();
                window.set_width(320.0);
                window.set_height(740.0);
                window.set_content(Some(Control::boxed(root.clone())));
                window.show();
                frame();

                let initial = named(&root, "PhonePage");
                assert!(snapshot::capture(&initial).is_some(),
                    "the real native page and headless render-target capability satisfy capture guards");
                vm.set_clock(vm.now() + 60);
                frame();
                assert!(initial.ptr_eq(&named(&root, "PhonePage")),
                    "ordinary revisions retain the page and its native state");

                // This is importData's transaction, including its document
                // epoch change and same-route return to Start. It must not
                // leave NavigationMotion pointing at an old or unloaded visit.
                vm.replace_data_after_save(data, |_| Ok(())).unwrap();
                frame();
                assert_eq!(vm.route(), Route::Start);
                let restored = named(&root, "PhonePage");
                let image = named(&root, "PhoneOutgoingImage").cast::<Image>().unwrap();
                assert!(image.source().is_none(), "restoration never captures the old document");

                let from = Point::new(250.0, 180.0);
                let to = Point::new(110.0, 180.0);
                let touch = window.touch_begin(from, RawInputModifiers::NONE);
                window.touch_move(&*touch, to, RawInputModifiers::NONE);
                frame();
                assert!(restored.translation().x < -100.0,
                    "the real shell gesture owns the horizontal drag before release");
                window.touch_end(&*touch, to, RawInputModifiers::NONE);
                frame();

                assert_eq!(vm.route(), Route::Apps);
                let incoming = named(&root, "PhonePage");
                let outgoing = named(&root, "PhoneOutgoingPage");
                let source = image.source().expect(
                    "the production capture path supplies an outgoing image after restoration",
                );
                let bitmap = source.as_bitmap().expect("a real RenderTargetBitmap-backed image");
                assert_eq!(bitmap.pixel_size().width, 320);
                assert_eq!(bitmap.pixel_size().height, 660);
                assert_eq!(source.size(), restored.bounds().size());
                drop(source);
                assert!(!incoming.is_enabled() && !incoming.is_hit_test_visible());
                assert_eq!(outgoing.opacity(), 1.0);
                assert!((slide_x(&incoming) - slide_x(&outgoing) - 320.0).abs() < 1e-8,
                    "unloading the old visit cannot cancel the paired new-visit motion");

                crate::lock();
                frame();
                assert!(image.source().is_none(), "locking releases the real captured bitmap");
                assert!(incoming.render_transform().is_none());
                assert!(outgoing.render_transform().is_none());
                assert_eq!(outgoing.opacity(), 0.0);
                crate::host_closing(&vm);
                window.close();
                window.set_content(None);
                frame();
            },
            CancellationToken::none(),
        )
        .wait_timeout(Duration::from_secs(30));
    session.dispose();
    result.expect("mounted shell restoration and capture regression completes");
}
