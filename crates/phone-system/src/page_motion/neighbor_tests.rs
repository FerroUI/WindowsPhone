//! Native input and real RenderTargetBitmap allocation exercise the held phase.
//! Headless drawing records bitmap dimensions, not pixel/color equivalence;
//! completed compositor geometry and visible content are browser evidence.
use super::*;
use ferroui_base::Point;
use ferroui_base::input::{Key, PhysicalKey, RawInputModifiers};
use ferroui_base::media::TranslateTransform;
use ferroui_controls::{AppBuilder, Application, Button, Grid, TextBlock, Window};
use ferroui_headless::{
    FerroHeadlessPlatform, FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions,
    HeadlessUnitTestSession, HeadlessWindowExtensions,
};
use phone_domain::{AppId, PhoneData, Tile, TileSize};
use phone_host::platform::{DeviceApplication, HostFeature, PlatformRequest, PlatformState};
use phone_sdk::{AppRegistration, AppRegistry, AppTileContext};
use phone_state::{HostRequest, HostServices};
use std::time::Duration;

thread_local! {
    static RENDERED_SNAPSHOTS: RefCell<Vec<(Weak<PhoneViewModel>, Weak<Session>)>> = const { RefCell::new(Vec::new()) };
    static LIFECYCLES: Cell<usize> = const { Cell::new(0) };
    static DEFAULT_COMMANDS: Cell<usize> = const { Cell::new(0) };
    static LIVE_ENTRANCES: RefCell<Vec<bool>> = const { RefCell::new(Vec::new()) };
}

fn tile(context: AppTileContext) -> Element {
    let preview = context.app.vm.is_preview_snapshot();
    if preview {
        assert!(context.live, "snapshot preserves selected live content");
        assert!(
            !context.animate,
            "a captured tile does not run a new entrance"
        );
        RENDERED_SNAPSHOTS.with(|snapshots| {
            snapshots.borrow_mut().push((
                Rc::downgrade(&context.app.vm),
                Rc::downgrade(&context.app.session),
            ))
        });
        // Exercise the supplied mutable interfaces. These writes/requests
        // cannot reach the active model/session or their host callbacks.
        context.app.vm.set_status("preview only".into());
        context.app.vm.navigate(Route::App(AppId::Weather));
        context.app.vm.request(HostRequest::StopCamera);
        context.app.session.query.set("preview only".into());
    } else {
        LIVE_ENTRANCES.with(|values| values.borrow_mut().push(context.animate));
    }
    let label = text_block()
        .name("PreviewWeatherContent")
        .text("cloudy 18°")
        .into_element();
    let label = grid()
        .child(label)
        .child(
            button()
                .name("TileDefaultCommand")
                .is_default(true)
                .is_cancel(true)
                .content("tile command")
                .on_click(|_| DEFAULT_COMMANDS.with(|count| count.set(count.get() + 1))),
        )
        .into_element();
    let content = if preview {
        grid()
            .child(label)
            .child(phone_sdk::images::thumbnail("preview-only-unloaded-image"))
            .into_element()
    } else {
        label
    };
    let animate = context.animate;
    border()
        .name("RegisteredTileMotion")
        .key(format!("registered-tile-{}", context.now))
        .on_loaded(move |event| {
            if animate {
                if let Some(control) = event.source().and_then(|source| source.cast::<Control>()) {
                    phone_controls::live_tile_enter(&control, 24.0);
                }
            }
        })
        .child(content)
        .into()
}

fn frame() {
    Dispatcher::ui_thread().run_jobs(None);
    FerroHeadlessPlatform::force_render_timer_tick(1);
    Dispatcher::ui_thread().run_jobs(None);
}

fn run(test: impl FnOnce() + Send + 'static) {
    let session = HeadlessUnitTestSession::start_new(|| {
        AppBuilder::configure::<Application>().use_headless(FerroHeadlessPlatformOptions::default())
    });
    let result = session
        .dispatch(test, CancellationToken::none())
        .wait_timeout(Duration::from_secs(30));
    session.dispose();
    result.expect("mounted held-neighbor scenario completes");
}

fn named(root: &Ref<Control>, name: &str) -> Ref<Control> {
    root.find_descendant_of_type_where::<Control>(false, |part| part.has_name(name))
        .unwrap_or_else(|| panic!("missing mounted control {name}"))
}

fn slide_x(page: &Ref<Control>) -> f64 {
    page.render_transform()
        .and_then(|transform| {
            transform.as_object().and_then(|object| {
                object
                    .downcast_ref::<TranslateTransform>()
                    .map(|value| value.x())
            })
        })
        .expect("native paired slide exists")
}

struct Phone {
    root: Ref<Control>,
    window: Ref<Window>,
    vm: Rc<PhoneViewModel>,
    ui: Rc<Session>,
    requests: Rc<RefCell<Vec<HostRequest>>>,
    saves: Rc<Cell<usize>>,
    image_requests: Rc<Cell<usize>>,
}
impl Phone {
    fn new(route: Route, native_apps: bool) -> Self {
        phone_controls::install_theme(&Application::current().unwrap());
        crate::set_preview_capture_enabled(true);
        let mut data = PhoneData::default();
        data.settings.lock_timeout = 0;
        data.settings.reduced_motion = false;
        data.settings.battery_saver = false;
        data.settings.live_tiles = true;
        data.alarms.clear();
        data.events.clear();
        data.tiles = vec![Tile::new(900, AppId::Weather, TileSize::Wide)];
        let saves = Rc::new(Cell::new(0));
        let saved = saves.clone();
        let vm = PhoneViewModel::new(data, Rc::new(move |_| saved.set(saved.get() + 1)));
        vm.set_viewport(320.0, 740.0);
        let requests = Rc::new(RefCell::new(Vec::new()));
        let requested = requests.clone();
        vm.set_services(HostServices {
            request: Rc::new(move |request| requested.borrow_mut().push(request)),
        });
        if native_apps {
            vm.set_platform_state(PlatformState {
                features: vec![HostFeature::InstalledApps],
                ..Default::default()
            })
            .unwrap();
            vm.set_device_applications(vec![DeviceApplication {
                id: "org.example/.Native".into(),
                title: "Native example".into(),
            }])
            .unwrap();
        }
        vm.navigate(route);
        let registry = Rc::new(
            AppRegistry::new(vec![
                AppRegistration::builtin(AppId::Weather, |_, _| Element::Empty)
                    .with_tile(tile)
                    .with_lifecycle(|_, _| LIFECYCLES.with(|count| count.set(count.get() + 1))),
            ])
            .unwrap(),
        );
        let root = crate::create_view(vm.clone(), registry);
        let image_requests = Rc::new(Cell::new(0));
        let image_requested = image_requests.clone();
        phone_sdk::images::set_loader(Rc::new(move |_| {
            image_requested.set(image_requested.get() + 1)
        }));
        let window = Window::new();
        window.set_width(320.0);
        window.set_height(740.0);
        window.set_content(Some(Control::boxed(root.clone())));
        window.show();
        frame();
        let ui = crate::SESSION.with(|slot| slot.borrow().upgrade()).unwrap();
        Self {
            root,
            window,
            vm,
            ui,
            requests,
            saves,
            image_requests,
        }
    }

    fn part(&self, name: &str) -> Ref<Control> {
        named(&self.root, name)
    }
    fn image(&self) -> Ref<Image> {
        self.part("PhoneNeighborImage").cast::<Image>().unwrap()
    }
    fn motion(&self) -> Rc<NavigationMotion> {
        self.ui.state::<NavigationMotion>()
    }
    fn assert_clean(&self) {
        assert!(self.image().source().is_none(), "held image is released");
        let surface = self.part("PhoneNeighborPage");
        assert_eq!(surface.opacity(), 0.0);
        assert_eq!(surface.translation(), Vector3D::default());
        assert!(surface.render_transform().is_none());
        assert!(
            self.part("PhoneNeighborStage")
                .cast::<Border>()
                .unwrap()
                .child()
                .is_none(),
            "temporary owner is unmounted"
        );
        assert!(self.motion().neighbor.request.borrow().is_none());
        assert!(self.motion().neighbor.returning.borrow().is_none());
    }
}
impl Drop for Phone {
    fn drop(&mut self) {
        crate::host_closing(&self.vm);
        self.window.close();
        self.window.set_content(None);
        crate::set_preview_capture_enabled(false);
        phone_sdk::images::set_loader(Rc::new(|_| {}));
        frame();
    }
}

fn held_then_commit(route: Route, direction: f64) {
    let phone = Phone::new(route.clone(), false);
    let original = phone.vm.export_json().unwrap();
    let page = phone.part("PhonePage");
    let before_requests = phone.requests.borrow().len();
    let before_lifecycle = LIFECYCLES.with(Cell::get);
    let from = Point::new(if direction > 0.0 { 250.0 } else { 70.0 }, 180.0);
    let to = Point::new(from.x - direction * 136.0, from.y);
    let touch = phone.window.touch_begin(from, RawInputModifiers::NONE);
    phone
        .window
        .touch_move(&*touch, to, RawInputModifiers::NONE);
    frame();
    assert_eq!(
        phone.vm.route(),
        route,
        "holding cannot navigate the live model"
    );
    assert_eq!(phone.vm.export_json().unwrap(), original);
    assert_eq!(phone.vm.status(), "");
    assert_eq!(phone.ui.query.get_untracked(), "");
    assert_eq!(phone.requests.borrow().len(), before_requests);
    assert_eq!(LIFECYCLES.with(Cell::get), before_lifecycle);
    assert_eq!(phone.saves.get(), 0);
    assert_eq!(
        phone.image_requests.get(),
        0,
        "preview render callbacks cannot enqueue image-loader work"
    );
    let neighbor = phone.part("PhoneNeighborPage");
    let source = phone
        .image()
        .source()
        .expect("production capture supplies the held neighboring image");
    let bitmap = source
        .as_bitmap()
        .expect("real bounded RenderTargetBitmap allocation");
    assert_eq!(
        (bitmap.pixel_size().width, bitmap.pixel_size().height),
        (320, 660)
    );
    assert_eq!(source.size(), page.bounds().size());
    drop(source);
    assert_eq!(page.translation().x, -direction * 136.0);
    assert!((neighbor.translation().x - page.translation().x - direction * 320.0).abs() < 1e-8);
    assert_eq!(neighbor.opacity(), 1.0);
    assert!(!neighbor.is_enabled() && !neighbor.is_hit_test_visible());
    assert!(
        phone
            .part("PhoneNeighborStage")
            .cast::<Border>()
            .unwrap()
            .child()
            .is_none()
    );
    if route == Route::Apps {
        RENDERED_SNAPSHOTS.with(|snapshots| {
            let snapshots = snapshots.borrow();
            assert!(
                !snapshots.is_empty(),
                "the registered native tile rendered in the isolated preview"
            );
            for (vm, ui) in snapshots.iter() {
                assert!(
                    vm.upgrade().is_none() && ui.upgrade().is_none(),
                    "capture releases the entire model/session owner"
                );
            }
        });
    }
    phone.window.touch_end(&*touch, to, RawInputModifiers::NONE);
    frame();
    assert_eq!(
        phone.vm.route(),
        if direction > 0.0 {
            Route::Apps
        } else {
            Route::Start
        }
    );
    phone.assert_clean();
    let outgoing = phone.part("PhoneOutgoingPage");
    let incoming = phone.part("PhonePage");
    assert!(
        phone
            .part("PhoneOutgoingImage")
            .cast::<Image>()
            .unwrap()
            .source()
            .is_some(),
        "commit transfers to the existing outgoing capture path"
    );
    assert!((slide_x(&incoming) - slide_x(&outgoing) - direction * 320.0).abs() < 1e-8);
    assert!(incoming.is_enabled() && !incoming.is_hit_test_visible());
    assert_eq!(
        KeyboardNavigation::get_tab_navigation(&incoming),
        KeyboardNavigationMode::None
    );
    assert_eq!(phone.vm.export_json().unwrap(), original);
}

#[test]
fn held_start_neighbor_is_noninteractive_and_commits_into_paired_paging() {
    run(|| held_then_commit(Route::Start, 1.0));
}

#[test]
fn held_apps_neighbor_isolates_tile_callbacks_and_disposes_the_preview_owner() {
    run(|| held_then_commit(Route::Apps, -1.0));
}

#[test]
fn displayed_start_handoff_skips_first_tile_entrance_but_later_updates_and_ordinary_entry_animate()
{
    run(|| {
        let phone = Phone::new(Route::Apps, false);
        assert!(LIVE_ENTRANCES.with(|values| values.borrow().is_empty()));
        let from = Point::new(70.0, 360.0);
        let to = Point::new(206.0, 360.0);
        let touch = phone.window.touch_begin(from, RawInputModifiers::NONE);
        phone
            .window
            .touch_move(&*touch, to, RawInputModifiers::NONE);
        assert!(phone.image().source().is_some());
        phone.window.touch_end(&*touch, to, RawInputModifiers::NONE);
        frame();
        assert_eq!(phone.vm.route(), Route::Start);
        assert!(phone.motion().start_continues_neighbor());
        assert!(
            LIVE_ENTRANCES.with(|values| !values.borrow().is_empty()
                && values.borrow().iter().all(|animate| !animate))
        );
        let first = phone.part("RegisteredTileMotion");
        assert!(
            first.render_transform().is_none(),
            "the live tile replaces the displayed preview without a second entrance"
        );
        assert_eq!(first.opacity(), 1.0);

        phone.ui.live_clock.set(phone.vm.now() + 8);
        frame();
        let updated = phone.part("RegisteredTileMotion");
        assert!(
            !first.ptr_eq(&updated),
            "a later cadence creates its new live frame"
        );
        assert_eq!(
            LIVE_ENTRANCES.with(|values| values.borrow().last().copied()),
            Some(true)
        );
        assert!(
            updated.render_transform().is_some(),
            "later live content still runs native tile motion"
        );

        phone.motion().cancel();
        assert!(!phone.motion().start_continues_neighbor());
        phone.vm.navigate(Route::Apps);
        frame();
        phone.motion().cancel();
        phone.vm.home(); // A fresh ordinary entry has no displayed neighbor handoff.
        frame();
        assert!(!phone.motion().start_continues_neighbor());
        assert_eq!(
            LIVE_ENTRANCES.with(|values| values.borrow().last().copied()),
            Some(true)
        );
        assert!(
            phone
                .part("RegisteredTileMotion")
                .render_transform()
                .is_some()
        );
        let data = phone.vm.data().clone();
        phone.vm.replace_data_after_save(data, |_| Ok(())).unwrap();
        frame();
        assert!(
            !phone.motion().start_continues_neighbor(),
            "document restoration retires the visit policy"
        );
    });
}

#[test]
fn committed_shell_slide_preserves_tile_ink_and_blocks_native_pointer_tab_and_root_commands() {
    run(|| {
        for captured in [true, false] {
            let phone = Phone::new(Route::Apps, false);
            crate::set_preview_capture_enabled(captured);
            let from = Point::new(70.0, 360.0);
            let to = Point::new(206.0, 360.0);
            let touch = phone.window.touch_begin(from, RawInputModifiers::NONE);
            phone
                .window
                .touch_move(&*touch, to, RawInputModifiers::NONE);
            phone.window.touch_end(&*touch, to, RawInputModifiers::NONE);
            frame();
            let page = phone.part("PhonePage");
            let tile = phone.part("Tile-900").cast::<Button>().unwrap();
            let command = phone.part("TileDefaultCommand").cast::<Button>().unwrap();
            assert_eq!(
                phone
                    .part("PhoneOutgoingImage")
                    .cast::<Image>()
                    .unwrap()
                    .source()
                    .is_some(),
                captured,
                "both real capture and unavailable-capture fallback must preserve the same input gate"
            );
            assert!(page.is_enabled() && !page.is_hit_test_visible());
            assert!(tile.is_effectively_enabled());
            assert_eq!(
                tile.opacity(),
                1.0,
                "the visible destination must not inherit disabled tile ink"
            );
            assert_eq!(
                KeyboardNavigation::get_tab_navigation(&page),
                KeyboardNavigationMode::None
            );
            assert!(!command.is_default() && !command.is_cancel());
            let deferred = Button::new();
            deferred.set_is_default(true);
            deferred.set_is_cancel(true);
            deferred.add_handler(Button::click_event(), |_, _| {
                DEFAULT_COMMANDS.with(|count| count.set(count.get() + 1));
            });
            let contents = phone
                .part("RegisteredTileMotion")
                .cast::<Border>()
                .unwrap()
                .child()
                .unwrap()
                .cast::<Grid>()
                .unwrap();
            let added = deferred.clone();
            Dispatcher::ui_thread().post_local(
                move || contents.children().add(added.upcast::<Control>()),
                DispatcherPriority::LOADED,
            );
            frame();
            assert!(deferred.is_attached_to_visual_tree());
            assert!(
                !deferred.is_default() && !deferred.is_cancel(),
                "deferred native children inherit the active command gate"
            );
            phone.window.key_press(
                Key::Return,
                RawInputModifiers::NONE,
                PhysicalKey::Enter,
                None,
            );
            phone.window.key_release(
                Key::Return,
                RawInputModifiers::NONE,
                PhysicalKey::Enter,
                None,
            );
            phone.window.key_text_input("blocked");
            assert_eq!(DEFAULT_COMMANDS.with(Cell::get), 0);
            assert_eq!(phone.ui.query.get_untracked(), "");
            phone
                .window
                .key_press(Key::Tab, RawInputModifiers::NONE, PhysicalKey::Tab, None);
            phone
                .window
                .key_release(Key::Tab, RawInputModifiers::NONE, PhysicalKey::Tab, None);
            assert!(
                !page.is_keyboard_focus_within(),
                "tab must skip the moving destination"
            );
            assert_eq!(phone.vm.route(), Route::Start);
            command.set_is_cancel(false);
            deferred.set_is_cancel(false);
            phone.motion().cancel();
            assert!(page.is_enabled() && page.is_hit_test_visible());
            assert_eq!(
                KeyboardNavigation::get_tab_navigation(&page),
                KeyboardNavigationMode::Continue
            );
            assert!(command.is_default());
            assert!(
                !command.is_cancel(),
                "cleanup preserves newer local values underneath its lease"
            );
            assert!(deferred.is_default() && !deferred.is_cancel());
            assert_eq!(tile.opacity(), 1.0);
            assert!(page.render_transform().is_none());
            assert!(
                phone
                    .part("PhoneOutgoingImage")
                    .cast::<Image>()
                    .unwrap()
                    .source()
                    .is_none()
            );
            phone.assert_clean();
        }
    });
}

#[test]
fn native_catalog_preview_does_not_refresh_and_pending_cancel_disposes_before_capture() {
    run(|| {
        let phone = Phone::new(Route::Start, true);
        let motion = phone.motion();
        let page = phone.part("PhonePage");
        page.set_translation(Vector3D::new(-80.0, 0.0, 0.0));
        motion.drag_neighbor(
            &phone.vm,
            &phone.ui,
            &page,
            &Route::Start,
            Vector3D::default(),
            -80.0,
        );
        let owner = motion
            .neighbor
            .request
            .borrow()
            .as_ref()
            .unwrap()
            .staging
            .as_ref()
            .unwrap()
            ._owner
            .0;
        phone.window.layout_manager().execute_layout_pass();
        Dispatcher::ui_thread().run_jobs(Some(DispatcherPriority::LOADED));
        let staged = phone.part("PhoneNeighborCapture");
        assert!(
            staged
                .find_descendant_of_type_where::<TextBlock>(false, |text| text.text().as_deref()
                    == Some("Native example"))
                .is_some(),
            "observed OS catalog remains in the native preview tree"
        );
        assert!(
            !phone.requests.borrow().iter().any(|request| matches!(
                request,
                HostRequest::Platform {
                    request: PlatformRequest::ListApplications
                }
            )),
            "preview Loaded must not enumerate the host"
        );
        assert!(
            phone.image().source().is_none(),
            "capture waits until the child Loaded work completes"
        );
        motion.cancel();
        assert!(
            !owner.is_alive(),
            "pending cancellation disposes bindings even before allocation"
        );
        frame();
        phone.assert_clean();
        assert_eq!(page.translation(), Vector3D::default());
        assert_eq!(phone.vm.route(), Route::Start);
    });
}

#[test]
fn rejected_native_pan_settles_both_edges_and_host_cancellation_releases_input() {
    run(|| {
        let phone = Phone::new(Route::Apps, false);
        let page = phone.part("PhonePage");
        let input = phone.part("AppSearch");
        assert!(input.focus());
        let from = Point::new(70.0, 360.0);
        let to = Point::new(86.0, 360.0); // Above axis slop, below distance and flick thresholds.
        let touch = phone.window.touch_begin(from, RawInputModifiers::NONE);
        phone
            .window
            .touch_move(&*touch, to, RawInputModifiers::NONE);
        assert!(phone.image().source().is_some());
        phone.window.touch_end(&*touch, to, RawInputModifiers::NONE);
        assert_eq!(phone.vm.route(), Route::Apps);
        let neighbor = phone.part("PhoneNeighborPage");
        assert!((slide_x(&neighbor) - slide_x(&page) + 320.0).abs() < 1e-8);
        assert!(
            page.is_enabled() && !page.is_hit_test_visible(),
            "source retains normal enabled styling"
        );
        assert_eq!(
            KeyboardNavigation::get_tab_navigation(&page),
            KeyboardNavigationMode::None
        );
        assert!(
            !input.is_focused(),
            "the source editor cannot receive text during return"
        );
        phone.window.key_text_input("blocked");
        assert_eq!(phone.ui.query.get_untracked(), "");
        crate::host_visibility(false);
        frame();
        phone.assert_clean();
        assert!(page.is_enabled() && page.is_hit_test_visible());
        assert_eq!(
            KeyboardNavigation::get_tab_navigation(&page),
            KeyboardNavigationMode::Continue
        );
        assert!(page.render_transform().is_none());
    });
}

#[test]
fn rejected_pan_blocks_tile_root_key_commands_and_restores_their_latest_values() {
    run(|| {
        let phone = Phone::new(Route::Start, false);
        let command = phone.part("TileDefaultCommand").cast::<Button>().unwrap();
        assert!(command.is_default() && command.is_cancel());
        let from = Point::new(250.0, 360.0);
        let to = Point::new(234.0, 360.0);
        let touch = phone.window.touch_begin(from, RawInputModifiers::NONE);
        phone
            .window
            .touch_move(&*touch, to, RawInputModifiers::NONE);
        phone.window.touch_end(&*touch, to, RawInputModifiers::NONE);
        assert!(phone.motion().neighbor.returning.borrow().is_some());
        assert!(!command.is_default() && !command.is_cancel());
        phone.window.key_press(
            Key::Return,
            RawInputModifiers::NONE,
            PhysicalKey::Enter,
            None,
        );
        phone.window.key_release(
            Key::Return,
            RawInputModifiers::NONE,
            PhysicalKey::Enter,
            None,
        );
        assert_eq!(DEFAULT_COMMANDS.with(Cell::get), 0);
        command.set_is_cancel(false);
        phone.motion().cancel();
        assert!(command.is_default());
        assert!(
            !command.is_cancel(),
            "the latest local value remains underneath the temporary lease"
        );
        phone.assert_clean();
    });
}

#[test]
fn held_images_retire_on_touch_cancel_privacy_resize_restore_and_motion_policy() {
    run(|| {
        let phone = Phone::new(Route::Start, false);
        let original = phone.vm.data().clone();
        for cause in [
            "pointer", "privacy", "resize", "restore", "reduced", "battery",
        ] {
            let from = Point::new(250.0, 180.0);
            let to = Point::new(114.0, 180.0);
            let touch = phone.window.touch_begin(from, RawInputModifiers::NONE);
            phone
                .window
                .touch_move(&*touch, to, RawInputModifiers::NONE);
            assert!(
                phone.image().source().is_some(),
                "{cause} interrupts a real held image"
            );
            match cause {
                "pointer" => touch.dispose(),
                "privacy" => crate::lock(),
                "resize" => phone.vm.set_viewport(340.0, 740.0),
                "restore" => phone
                    .vm
                    .replace_data_after_save(original.clone(), |_| Ok(()))
                    .unwrap(),
                "reduced" | "battery" => {
                    let mut settings = phone.vm.data().settings.clone();
                    settings.reduced_motion = cause == "reduced";
                    settings.battery_saver = cause == "battery";
                    phone.vm.set_settings(settings);
                }
                _ => unreachable!(),
            }
            frame();
            phone.assert_clean();
            assert_eq!(phone.vm.route(), Route::Start);
            assert_eq!(phone.part("PhonePage").translation(), Vector3D::default());
            touch.dispose();
            phone.ui.locked.set(false);
            phone.vm.set_viewport(320.0, 740.0);
            phone
                .vm
                .replace_data_after_save(original.clone(), |_| Ok(()))
                .unwrap();
            frame();
        }
        for reduced in [true, false] {
            let mut data = original.clone();
            data.settings.reduced_motion = reduced;
            data.settings.battery_saver = !reduced;
            phone.vm.replace_data_after_save(data, |_| Ok(())).unwrap();
            frame();
            let touch = phone
                .window
                .touch_begin(Point::new(250.0, 180.0), RawInputModifiers::NONE);
            phone
                .window
                .touch_move(&*touch, Point::new(114.0, 180.0), RawInputModifiers::NONE);
            phone.assert_clean();
            touch.dispose();
        }
    });
}
