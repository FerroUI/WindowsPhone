use super::*;
use ferroui_base::animation::SignedDuration;
use ferroui_base::input::{Key, PhysicalKey, RawInputModifiers};
use ferroui_base::threading::{CancellationToken, Dispatcher};
use ferroui_base::utilities::DateTimeOffset;
use ferroui_base::{ObjectType, Ref};
use ferroui_controls::primitives::{DateTimePickerPanel, Popup, TemplatedControl};
use ferroui_controls::{AppBuilder, Button, DatePicker, Grid, Window};
use ferroui_headless::{
    FerroHeadlessPlatform, FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions,
    HeadlessUnitTestSession, HeadlessWindowExtensions,
};
use phone_domain::{Alarm, AppId, Id, PhoneData};
use phone_sdk::{AppRegistration, AppRegistry};
use std::time::Duration;

fn original_date() -> DateTimeOffset {
    DateTimeOffset::new(2026, 11, 24, 0, 0, 0, SignedDuration::ZERO)
}

struct EditorState {
    date: Signal<Option<DateTimeOffset>>,
}

impl Default for EditorState {
    fn default() -> Self {
        Self {
            date: signal(Some(original_date())),
        }
    }
}

// The registry's app-local overlay contract is the same one used by Calendar's
// editor. Keep this SDK fixture independent of concrete application crates.
fn editor(_: Rc<PhoneViewModel>, ui: Rc<Session>) -> Element {
    let state = ui.state::<EditorState>();
    rsx! {Grid {
        DatePicker {name:"EditorDate",class:"phone-picker",margin:20.0,
            vertical_alignment:V::Top,selected_date:state.date.two_way(),
            with:|picker|phone_controls::attach_picker_focus(picker)}
    }}
}

fn frame() {
    Dispatcher::ui_thread().run_jobs(None);
    FerroHeadlessPlatform::force_render_timer_tick(1);
    Dispatcher::ui_thread().run_jobs(None);
}

fn key(window: &Window, key: Key, physical: PhysicalKey) {
    window.key_press(key, RawInputModifiers::NONE, physical, None);
    window.key_release(key, RawInputModifiers::NONE, physical, None);
    frame();
}

fn part<T: ObjectType>(owner: &TemplatedControl, name: &str) -> Ref<T> {
    owner
        .get_template_descendants()
        .into_iter()
        .find(|part| part.has_name(name))
        .and_then(|part| part.cast::<T>())
        .unwrap_or_else(|| panic!("missing native picker part {name}"))
}

struct Fixture {
    vm: Rc<PhoneViewModel>,
    ui: Rc<Session>,
    root: Ref<Control>,
    window: Ref<Window>,
    alarm_id: Id,
}

impl Fixture {
    fn new() -> Self {
        let mut data = PhoneData::default();
        data.settings.lock_timeout = 0;
        data.settings.reduced_motion = true;
        data.settings.live_tiles = false;
        data.events.clear();
        let alarm_id = data.next_id;
        data.next_id += 1;
        data.alarms = vec![Alarm {
            id: alarm_id,
            enabled: true,
            hour: 7,
            ..Default::default()
        }];
        let vm = PhoneViewModel::new(data, Rc::new(|_| {}));
        vm.set_viewport(320.0, 740.0);
        vm.set_clock(phone_domain::days_from_date(2026, 11, 24) * 86_400 + 12 * 3_600);
        vm.set_utc_offset(0);
        vm.navigate(Route::App(AppId::Calendar));
        let registry = Rc::new(
            AppRegistry::new(vec![
                AppRegistration::builtin(AppId::Calendar, editor).with_overlay(|_| true),
                AppRegistration::builtin(AppId::Settings, |_, _| rsx! {Grid {}}),
            ])
            .unwrap(),
        );
        let root = crate::create_view(vm.clone(), registry);
        let window = Window::new();
        window.set_width(320.0);
        window.set_height(740.0);
        window.set_content(Some(Control::boxed(root.clone())));
        window.show();
        frame();
        let ui = crate::SESSION.with(|slot| slot.borrow().upgrade().unwrap());
        Self {
            vm,
            ui,
            root,
            window,
            alarm_id,
        }
    }

    fn stage_month(&self) -> (Ref<DatePicker>, Ref<Popup>, Ref<TemplatedControl>) {
        let picker = self
            .root
            .find_descendant_of_type::<DatePicker>(false)
            .unwrap();
        let popup = part::<Popup>(&picker, "PART_Popup");
        let presenter = popup
            .child()
            .and_then(|child| child.cast::<TemplatedControl>())
            .unwrap();
        assert!(part::<Button>(&picker, "PART_FlyoutButton").focus());
        key(&self.window, Key::Space, PhysicalKey::Space);
        assert!(popup.is_open() && phone_controls::has_open_phone_picker());
        let month = part::<DateTimePickerPanel>(&presenter, "PART_MonthSelector");
        assert_eq!(month.selected_value(), 11);
        assert!(month.focus());
        key(&self.window, Key::Down, PhysicalKey::ArrowDown);
        assert_eq!(month.selected_value(), 12);
        assert_eq!(picker.selected_date(), Some(original_date()));
        (picker, popup, presenter)
    }

    fn close(self) {
        crate::host_closing(&self.vm);
        self.window.close();
        self.window.set_content(None);
        frame();
    }
}

#[test]
fn app_editor_picker_survives_shell_clock_and_size_rerenders_without_entering_previews() {
    let session = HeadlessUnitTestSession::start_new(|| {
        AppBuilder::configure::<Application>().use_headless(FerroHeadlessPlatformOptions::default())
    });
    session
        .dispatch(
            || {
                phone_controls::install_theme(&Application::current().unwrap());
                let fixture = Fixture::new();
                let (picker, popup, presenter) = fixture.stage_month();
                let month = part::<DateTimePickerPanel>(&presenter, "PART_MonthSelector");
                for _ in 0..3 {
                    // The view-model publishes a clock revision at the minute
                    // boundary, exercising the real root reconciliation path.
                    fixture.vm.set_clock(fixture.vm.now() + 60);
                    frame();
                    assert!(popup.is_open(), "clock redraw preserves the editor's popup");
                    assert!(phone_controls::has_open_phone_picker());
                    assert!(Ref::ptr_eq(
                        &picker,
                        &fixture
                            .root
                            .find_descendant_of_type::<DatePicker>(false)
                            .unwrap()
                    ));
                    assert!(Ref::ptr_eq(
                        &month,
                        &part::<DateTimePickerPanel>(&presenter, "PART_MonthSelector")
                    ));
                    assert_eq!(month.selected_value(), 12, "staged month survives redraw");
                    assert_eq!(picker.selected_date(), Some(original_date()));
                    assert!(fixture.ui.preview_obscured(&fixture.vm.route()));
                    assert!(!fixture.ui.shell_obscured(&fixture.vm.route()));
                }
                fixture.window.set_width(360.0);
                fixture.window.set_height(780.0);
                fixture.vm.set_viewport(360.0, 780.0);
                frame();
                assert!(
                    popup.is_open(),
                    "layout invalidation preserves the same popup"
                );
                assert_eq!(presenter.bounds().width, 360.0);
                assert_eq!(presenter.bounds().height, 700.0);
                assert_eq!(month.selected_value(), 12);
                assert_eq!(picker.selected_date(), Some(original_date()));
                assert!(fixture.ui.preview_obscured(&fixture.vm.route()));
                fixture.close();
            },
            CancellationToken::none(),
        )
        .wait_timeout(Duration::from_secs(30))
        .expect("mounted editor picker redraw regression completes");
    session.dispose();
}

#[test]
fn editor_picker_cancels_staged_selection_before_shell_and_lifecycle_takeovers() {
    let session = HeadlessUnitTestSession::start_new(|| {
        AppBuilder::configure::<Application>().use_headless(FerroHeadlessPlatformOptions::default())
    });
    session
        .dispatch(
            || {
                phone_controls::install_theme(&Application::current().unwrap());
                for boundary in [
                    "lock",
                    "Home",
                    "route",
                    "document restore",
                    "app switcher",
                    "notifications",
                    "alarm",
                    "host suspension",
                ] {
                    let fixture = Fixture::new();
                    let (picker, popup, presenter) = fixture.stage_month();
                    match boundary {
                        "lock" => crate::lock(),
                        "Home" => crate::home(&fixture.vm),
                        "route" => fixture.vm.navigate(Route::App(AppId::Settings)),
                        "document restore" => {
                            let data = fixture.vm.data().clone();
                            fixture.vm.replace_data(data).unwrap();
                        }
                        "app switcher" => fixture.ui.switcher_open.set(true),
                        "notifications" => fixture.ui.notification_open.set(true),
                        "alarm" => crate::fire_alarm(&fixture.vm, fixture.alarm_id),
                        "host suspension" => crate::host_visibility(false),
                        _ => unreachable!(),
                    }
                    frame();
                    assert!(
                        !popup.is_open(),
                        "{boundary} closes the actual native popup"
                    );
                    assert!(
                        !phone_controls::has_open_phone_picker(),
                        "{boundary} clears popup ownership"
                    );
                    assert!(!part::<Grid>(&presenter, "PhonePickerSurface")
                        .is_attached_to_visual_tree());
                    assert_eq!(
                        picker.selected_date(),
                        Some(original_date()),
                        "{boundary} does not confirm staged input"
                    );
                    fixture.close();
                }
            },
            CancellationToken::none(),
        )
        .wait_timeout(Duration::from_secs(30))
        .expect("mounted editor picker takeover regressions complete");
    session.dispose();
}
