use super::*;
use ferroui_base::media::DrawingContext;
use ferroui_base::Rect;
use phone_domain::AppId;
use std::any::Any;

struct TestImage;
impl IImage for TestImage {
    fn size(&self) -> Size {
        Size::new(400.0, 800.0)
    }
    fn draw(&self, _: &mut DrawingContext<'_>, _: Rect, _: Rect) {}
    fn as_any(&self) -> &dyn Any {
        self
    }
}

fn preview() -> Preview {
    Preview {
        image: Rc::new(TestImage),
        logical_size: Size::new(400.0, 800.0),
    }
}

fn extension(id: &str, path: &str) -> Route {
    Route::Extension {
        app: id.into(),
        path: path.into(),
    }
}

#[test]
fn extension_previews_share_the_builtin_departure_and_privacy_contract() {
    let cache = PreviewCache::default();
    for (route, identity) in [
        (
            Route::Setting("appearance".into()),
            RecentApp::Builtin(AppId::Settings),
        ),
        (
            extension("tests.notes", "note/42"),
            RecentApp::Extension("tests.notes".into()),
        ),
    ] {
        let recent = [identity.clone()];
        cache.presented(&route, false, false);
        assert_eq!(
            cache.capture_candidate(&Route::Start, false, &recent),
            Some(identity.clone())
        );
        assert_eq!(
            cache.capture_candidate(&route, true, &recent),
            Some(identity.clone()),
            "entering the switcher captures the previously uncovered page"
        );
        assert_eq!(
            cache.capture_candidate(&route, false, &recent),
            None,
            "ordinary redraws do not recapture"
        );
        cache.presented(&route, true, false);
        assert_eq!(
            cache.capture_candidate(&Route::Start, false, &recent),
            None,
            "dialogs, editors, lock and other privacy takeovers never enter the cache"
        );
        cache.presented(&route, true, true);
        assert_eq!(
            cache.capture_candidate(&route, true, &recent),
            None,
            "the switcher cannot capture itself"
        );
        cache.presented(&route, false, false);
        assert_eq!(
            cache.capture_candidate(&Route::Start, false, &[]),
            None,
            "closed and disabled apps are ineligible"
        );
    }
    cache.presented(&Route::Apps, false, false);
    assert_eq!(cache.capture_candidate(&Route::Start, false, &[]), None);
}

#[test]
fn builtin_and_extension_previews_use_one_bounded_identity_safe_cache() {
    let cache = PreviewCache::default();
    let builtin = RecentApp::Builtin(AppId::Settings);
    let extension = RecentApp::Extension("phone.settings".into());
    cache.record(builtin.clone(), preview());
    cache.record(extension.clone(), preview());
    cache.remove(&extension);
    assert!(
        cache.get(&builtin).is_some(),
        "distinct identity variants cannot evict each other"
    );
    assert!(cache.get(&extension).is_none());
    for index in 0..7 {
        let app = RecentApp::Extension(format!("tests.app{index}"));
        cache.record(app, preview());
    }
    assert_eq!(cache.entries.borrow().len(), LIMIT);
    assert!(cache
        .get(&RecentApp::Extension("tests.app0".into()))
        .is_none());
    let refreshed = RecentApp::Extension("tests.app2".into());
    let image = preview();
    cache.record(refreshed.clone(), image.clone());
    assert_eq!(
        cache.entries.borrow().len(),
        LIMIT,
        "refresh replaces an existing bitmap"
    );
    assert_eq!(cache.entries.borrow().front().unwrap().0, refreshed);
    assert!(Rc::ptr_eq(
        &cache.get(&refreshed).unwrap().image,
        &image.image
    ));
    cache.capture_candidate(&Route::Start, false, &[refreshed.clone()]);
    assert_eq!(
        cache.entries.borrow().len(),
        1,
        "removed recent entries release their bitmaps before a later capture"
    );
    cache.clear();
    assert!(cache.entries.borrow().is_empty());
}

#[test]
fn extension_switcher_card_renders_its_snapshot_resumes_its_detail_and_releases_on_close() {
    use ferroui_base::input::{Key, PhysicalKey, RawInputModifiers};
    use ferroui_base::threading::{CancellationToken, Dispatcher};
    use ferroui_controls::{AppBuilder, Application, Button, Control, Image, Window};
    use ferroui_headless::{
        FerroHeadlessPlatform, FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions,
        HeadlessUnitTestSession, HeadlessWindowExtensions,
    };
    use ferroui_view::prelude::*;
    use phone_domain::PhoneData;
    use phone_sdk::{AppManifest, AppRegistration, AppRegistry};
    use phone_state::PhoneViewModel;

    let session = HeadlessUnitTestSession::start_new(|| {
        AppBuilder::configure::<Application>().use_headless(FerroHeadlessPlatformOptions::default())
    });
    session
        .dispatch(
            || {
                phone_controls::set_motion_enabled(false);
                phone_controls::install_theme(&Application::current().unwrap());
                let registry = Rc::new(
                    AppRegistry::new(vec![AppRegistration::new(
                        AppManifest::new("tests.notes", "Notes", phone_controls::Icon::Note),
                        |_| Element::Empty,
                    )])
                    .unwrap(),
                );
                let vm = PhoneViewModel::new(PhoneData::default(), Rc::new(|_| {}));
                registry.bind(&vm);
                vm.navigate(extension("tests.notes", "note/42"));
                vm.home();
                let ui = Rc::new(crate::session::Session::new(vm.now(), registry));
                let cache = ui.state::<PreviewCache>();
                let identity = RecentApp::Extension("tests.notes".into());
                let snapshot = preview();
                cache.record(identity.clone(), snapshot.clone());
                let builtin = RecentApp::Builtin(AppId::Settings);
                cache.record(builtin.clone(), preview());
                let (root, owner) = crate::navigation::switcher(vm.clone(), ui, false).mount_root();
                let window = Window::new();
                window.set_width(400.0);
                window.set_height(820.0);
                window.set_content(Some(Control::boxed(root.clone())));
                window.show();
                let frame = || {
                    Dispatcher::ui_thread().run_jobs(None);
                    FerroHeadlessPlatform::force_render_timer_tick(1);
                    Dispatcher::ui_thread().run_jobs(None);
                };
                frame();
                let image = root
                    .find_descendant_of_type_where::<Image>(false, |image| {
                        image.has_name("AppPreview-tests.notes")
                    })
                    .expect("the SDK app uses its actual cached page image");
                assert!(Rc::ptr_eq(&image.source().unwrap(), &snapshot.image));
                for name in ["AppResume-tests.notes", "AppClose-tests.notes"] {
                    let button = root
                        .find_descendant_of_type_where::<Button>(false, |button| {
                            button.has_name(name)
                        })
                        .unwrap();
                    assert!(button.focus());
                    window.key_press(
                        Key::Space,
                        RawInputModifiers::NONE,
                        PhysicalKey::Space,
                        None,
                    );
                    window.key_release(
                        Key::Space,
                        RawInputModifiers::NONE,
                        PhysicalKey::Space,
                        None,
                    );
                    frame();
                    if name == "AppResume-tests.notes" {
                        assert_eq!(vm.route(), extension("tests.notes", "note/42"));
                        assert!(cache.get(&identity).is_some());
                    }
                }
                assert!(!vm.recent_entries().contains(&identity));
                assert!(cache.get(&identity).is_none());
                assert!(
                    cache.get(&builtin).is_some(),
                    "closing an extension does not clear another app's preview"
                );
                window.close();
                window.set_content(None);
                owner.dispose();
            },
            CancellationToken::none(),
        )
        .wait_timeout(std::time::Duration::from_secs(20))
        .expect("SDK switcher regression completes");
    session.dispose();
}

#[test]
fn preview_bitmap_budget_preserves_full_page_geometry() {
    let (size, scale) = capture_size(Size::new(400.0, 820.0)).unwrap();
    assert_eq!(size, PixelSize::new(270, 554));
    assert_eq!(scale, 0.675);
    let (size, scale) = capture_size(Size::new(300.0, 1400.0)).unwrap();
    assert_eq!(size, PixelSize::new(120, 560));
    assert_eq!(scale, 0.4);
    for size in [
        Size::new(0.0, 100.0),
        Size::new(10.0, -1.0),
        Size::new(f64::NAN, 20.0),
        Size::new(30.0, f64::INFINITY),
    ] {
        assert!(capture_size(size).is_none());
    }
}
