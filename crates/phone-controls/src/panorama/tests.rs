use super::*;
use ferroui_base::animation::{IClock, PlayState};
use ferroui_base::input::{
    InputElement, Key, KeyEventArgs, KeyboardNavigation, KeyboardNavigationMode,
    SwipeGestureEndedEventArgs, SwipeGestureEventArgs,
};
use ferroui_base::media::TranslateTransform;
use ferroui_base::reactive::{IDisposable, IObservable, IObserver, LightweightSubject};
use ferroui_base::threading::{CancellationToken, Dispatcher};
use ferroui_base::{Point, Ref, Vector};
use ferroui_controls::primitives::ScrollBarVisibility;
use ferroui_controls::{Application, Button, Control, Grid, ScrollViewer, TextBlock, Window};
use ferroui_headless::{FerroHeadlessPlatform, FerroTestAssembly, HeadlessUnitTestSession};
use std::cell::Cell;
use std::sync::Arc;
use std::time::Duration;

pub(super) fn session() -> Arc<HeadlessUnitTestSession> {
    // Native manipulation/material caches own compositor resources for the UI
    // thread's lifetime. Use the framework's assembly-scoped dispatcher while
    // retaining its default, fresh application isolation for every dispatch.
    // Closing each window/owner below remains mandatory, including on panic.
    static ASSEMBLY: FerroTestAssembly = FerroTestAssembly {
        name: "phone-controls.panorama",
        test_application: None,
        test_isolation: None,
    };
    HeadlessUnitTestSession::get_or_start_for_assembly(Some(&ASSEMBLY))
}

pub(super) struct MountedWindow {
    window: Ref<Window>,
    owner: Owner,
}

impl MountedWindow {
    pub(super) fn new(window: Ref<Window>, owner: Owner) -> Self {
        Self { window, owner }
    }
}

impl Drop for MountedWindow {
    fn drop(&mut self) {
        self.window.close();
        self.window.set_content(None);
        self.owner.dispose();
        frame();
        crate::set_motion_enabled(true);
    }
}

/// Drive the real inherited native animation clock without depending on the
/// headless compositor scheduling wall-clock animation ticks.
struct ManualClock {
    ticks: LightweightSubject<SignedDuration>,
    seconds: Cell<f64>,
    play: Cell<PlayState>,
}
impl ManualClock {
    fn new() -> Rc<Self> {
        Rc::new(Self {
            ticks: LightweightSubject::new(),
            seconds: Cell::new(0.0),
            play: Cell::new(PlayState::Run),
        })
    }
    fn advance(&self) {
        self.seconds.set(self.seconds.get() + 0.02);
        self.ticks
            .on_next(SignedDuration::from_seconds(self.seconds.get()));
    }
}
impl IObservable<SignedDuration> for ManualClock {
    fn subscribe(&self, observer: Rc<dyn IObserver<SignedDuration>>) -> Rc<dyn IDisposable> {
        self.ticks.subscribe(observer)
    }
}
impl IClock for ManualClock {
    fn play_state(&self) -> PlayState {
        self.play.get()
    }
    fn set_play_state(&self, value: PlayState) {
        self.play.set(value);
    }
}

fn frame() {
    Dispatcher::ui_thread().run_jobs(None);
    FerroHeadlessPlatform::force_render_timer_tick(1);
    Dispatcher::ui_thread().run_jobs(None);
}

fn settle(host: &Carousel, clock: &ManualClock) {
    for _ in 0..30 {
        clock.advance();
        frame();
        if super::admission::is_settled(host) {
            break;
        }
    }
    assert!(
        super::admission::is_settled(host),
        "native panorama settles at its selected logical offset"
    );
    for _ in 0..3 {
        frame();
    }
}

fn scene(
    motion: bool,
    run: impl FnOnce(Ref<Window>, Ref<Carousel>, Signal<usize>, Rc<ManualClock>) + Send + 'static,
) {
    let session = session();
    session.dispatch(move || {
        crate::set_motion_enabled(motion);
        crate::install_theme(&Application::current().unwrap());
        let selection=signal(0usize);
        let pages=["all","favorites","groups"].into_iter().enumerate().map(|(i,label)| {
            let mut rows=stack_panel();
            for n in 0..30 {
                rows=rows.child(rsx!{Button{name:format!("Contact-{i}-{n}"),content:format!("Contact {n}"),height:60.0}});
            }
            PanoramaPage::new(label,rsx!{ScrollViewer{
                name:format!("SectionScroll{i}"),
                horizontal_scroll_bar_visibility:ScrollBarVisibility::Disabled,
                vertical_scroll_bar_visibility:ScrollBarVisibility::Hidden,{rows}
            }})
        }).collect();
        let (control,owner)=Element::from(panorama_pages("PeoplePanorama","people",selection,pages)).mount_root();
        let panorama=control.cast::<Carousel>().unwrap();
        let clock=ManualClock::new();
        let window=Window::new();
        let _mounted=MountedWindow::new(window.clone(),owner);
        window.set_clock(Some(clock.clone()));window.set_width(400.0);window.set_height(720.0);
        window.set_content(Some(Control::boxed(control)));window.show();
        for _ in 0..5{frame();}
        run(window.clone(),panorama,selection,clock);
    },CancellationToken::none()).wait_timeout(Duration::from_secs(20)).expect("native panorama test finishes");
}

fn child<T: ferroui_base::ObjectType>(root: &Control, name: &str) -> Ref<T> {
    root.find_descendant_of_type_where::<Control>(false, |control| control.has_name(name))
        .and_then(|control| control.cast::<T>())
        .expect(name)
}

fn x(control: &Control) -> f64 {
    control
        .render_transform()
        .and_then(|transform| {
            transform
                .as_object()
                .and_then(|object| object.to_ref().cast::<TranslateTransform>())
        })
        .unwrap()
        .x()
}

#[test]
fn native_panorama_tracks_finger_across_layers_and_keeps_next_section_visible() {
    scene(true, |_, host, selection, clock| {
        let panel = host.items_panel_root().unwrap();
        let root = child::<Grid>(&host, "PanoramaRoot");
        let current = child::<TextBlock>(&host, "PeoplePanorama-section-0-header");
        let next = child::<TextBlock>(&host, "PeoplePanorama-section-1-header");
        let title = child::<TextBlock>(&host, "PanoramaTitle");
        let background = child::<ferroui_controls::Border>(&host, "PanoramaBackground0");
        let first = current.translate_point(Point::default(), &root).unwrap().x;
        let neighbor = next.translate_point(Point::default(), &root).unwrap().x;
        assert!(
            (first - 20.0).abs() < 0.01,
            "selected heading inset: {first}"
        );
        assert!(
            (neighbor - 380.0).abs() < 0.01,
            "next heading has visible preview: {neighbor}"
        );
        assert_eq!(x(&title), 0.0);
        panel.raise_event(&SwipeGestureEventArgs::new(
            31,
            Vector::new(108.0, 0.0),
            Vector::default(),
        ));
        frame();
        assert_eq!(
            selection.get_untracked(),
            0,
            "selection is committed after release"
        );
        assert!(host.is_swiping());
        assert!((panel.as_scrollable().unwrap().offset().x - 0.30).abs() < 0.01);
        assert!(x(&title) < 0.0 && x(&background) < x(&title));
        assert!(!host.container_from_index(0).unwrap().is_hit_test_visible());
        assert!(!host.container_from_index(1).unwrap().is_hit_test_visible());
        assert!(
            host.container_from_index(1).unwrap().is_enabled(),
            "preview keeps normal enabled ink"
        );
        assert_eq!(
            KeyboardNavigation::get_tab_navigation(&host.container_from_index(1).unwrap()),
            KeyboardNavigationMode::None
        );
        panel.raise_event(&SwipeGestureEndedEventArgs::new(31, Vector::default()));
        settle(&host, &clock);
        assert_eq!(selection.get_untracked(), 1);
        assert!(host.container_from_index(1).unwrap().is_hit_test_visible());
        assert_eq!(
            KeyboardNavigation::get_tab_navigation(&host.container_from_index(1).unwrap()),
            KeyboardNavigationMode::Continue
        );
        assert!(!host.container_from_index(2).unwrap().is_hit_test_visible());
    });
}

#[test]
fn native_keyboard_wrap_retains_section_scroll_and_theme_template_identity() {
    scene(true, |window, host, selection, clock| {
        let root = child::<Grid>(&host, "PanoramaRoot");
        let tag: ferroui_base::BoxedValue = Rc::new("application-owned state".to_owned());
        host.set_tag(Some(tag.clone()));
        frame();
        assert_eq!(
            child::<TextBlock>(&host, "PanoramaTitle").text().as_deref(),
            Some("people"),
            "the native Tag belongs to the application, not the title layer"
        );
        let scroll = child::<ScrollViewer>(&host, "SectionScroll0");
        scroll.set_offset(Vector::new(0.0, 240.0));
        for _ in 0..3 {
            frame();
        }
        let saved = scroll.offset().y;
        assert!(saved > 200.0);
        let key = |key| {
            let mut event = KeyEventArgs::new();
            event.set_routed_event(Some(InputElement::key_down_event()));
            event.key = key;
            host.raise_event(&event);
            settle(&host, &clock);
        };
        key(Key::Right);
        assert_eq!(selection.get_untracked(), 1);
        key(Key::Left);
        assert_eq!(selection.get_untracked(), 0);
        let restored = child::<ScrollViewer>(&host, "SectionScroll0");
        assert!(Ref::ptr_eq(&scroll, &restored));
        assert!((restored.offset().y - saved).abs() < 0.01);
        key(Key::Left);
        assert_eq!(selection.get_untracked(), 2);
        key(Key::Right);
        assert_eq!(selection.get_untracked(), 0);
        window.set_width(320.0);
        crate::set_palette(&Application::current().unwrap(), "#A200FF", true);
        for _ in 0..5 {
            frame();
        }
        assert!(Ref::ptr_eq(&root, &child::<Grid>(&host, "PanoramaRoot")));
        assert!(Rc::ptr_eq(&host.tag().unwrap(), &tag));
        assert_eq!(
            child::<TextBlock>(&host, "PanoramaTitle").text().as_deref(),
            Some("people")
        );
        let next = child::<TextBlock>(&host, "PeoplePanorama-section-1-header");
        assert!((next.translate_point(Point::default(), &root).unwrap().x - 300.0).abs() < 0.01);
        assert_eq!(selection.get_untracked(), 0);
    });
}

#[test]
fn retained_view_rerender_preserves_native_swipe_completion_and_selection() {
    session()
        .dispatch(
            || {
                crate::set_motion_enabled(true);
                crate::install_theme(&Application::current().unwrap());
                let selection = signal(0usize);
                let revision = signal(0usize);
                let (root, owner) = rsx! {Grid {{rerender(move || {
                    // App views observe their section to build commands, and
                    // ordinary model revisions may rebuild the same retained
                    // view while its native manipulation is still completing.
                    selection.get();
                    revision.get();
                    let pages = ["all", "favorites"].into_iter().map(|label| {
                        PanoramaPage::new(label, rsx! {TextBlock {text:label}})
                    }).collect();
                    Element::from(panorama_pages("RetainedPanorama", "people", selection, pages))
                })}}}.mount_root();
                let clock = ManualClock::new();
                let window = Window::new();
                let _mounted = MountedWindow::new(window.clone(), owner);
                window.set_clock(Some(clock.clone()));
                window.set_width(400.0);
                window.set_height(720.0);
                window.set_content(Some(Control::boxed(root.clone())));
                window.show();
                for _ in 0..5 {
                    frame();
                }
                let host = child::<Carousel>(&root, "RetainedPanorama");
                let panel = host.items_panel_root().unwrap();
                panel.raise_event(&SwipeGestureEventArgs::new(
                    47,
                    Vector::new(240.0, 0.0),
                    Vector::default(),
                ));
                frame();
                assert!(host.is_swiping());
                panel.raise_event(&SwipeGestureEndedEventArgs::new(47, Vector::default()));
                for _ in 0..3 {
                    clock.advance();
                    frame();
                }
                assert!(host.is_swiping(), "completion is still in flight");
                let before = panel.as_scrollable().unwrap().offset().x;
                revision.set(1);
                frame();
                assert!(Ref::ptr_eq(
                    &host,
                    &child::<Carousel>(&root, "RetainedPanorama")
                ));
                assert!(
                    host.is_swiping(),
                    "an ordinary view rebuild must not cancel native completion"
                );
                assert!(panel.as_scrollable().unwrap().offset().x >= before - 1e-6);
                settle(&host, &clock);
                assert_eq!(selection.get_untracked(), 1);
                assert_eq!(host.selected_index(), 1);
                assert!(Ref::ptr_eq(
                    &host,
                    &child::<Carousel>(&root, "RetainedPanorama")
                ));
                assert!((panel.as_scrollable().unwrap().offset().x - 1.0).abs() < 1e-6);
                assert!(host.container_from_index(1).unwrap().is_hit_test_visible());
            },
            CancellationToken::none(),
        )
        .wait_timeout(Duration::from_secs(20))
        .expect("retained panorama completion regression finishes");
}

#[test]
fn reduced_motion_keeps_native_keyboard_and_leaves_touch_to_the_host() {
    scene(false, |_, host, selection, _| {
        assert!(!host.is_swipe_enabled());
        assert!(host.page_transition().is_none());
        assert!(!host.classes().contains("phone-native-input"));
        let mut event = KeyEventArgs::new();
        event.set_routed_event(Some(InputElement::key_down_event()));
        event.key = Key::Right;
        host.raise_event(&event);
        frame();
        assert_eq!(selection.get_untracked(), 1);
        assert!(!host.is_swiping());
        assert_eq!(x(&child::<TextBlock>(&host, "PanoramaTitle")), 0.0);
    });
}

#[test]
fn inactive_sections_cannot_invoke_root_default_or_cancel_commands_and_keep_caller_values() {
    scene(true, |window, host, _, clock| {
        let first = child::<Button>(&host, "Contact-0-0");
        let second = child::<Button>(&host, "Contact-1-0");
        first.set_is_default(true);
        second.set_is_default(true);
        second.set_is_cancel(true);
        let first_calls = Rc::new(Cell::new(0));
        let second_calls = Rc::new(Cell::new(0));
        let calls = first_calls.clone();
        first.add_handler(Button::click_event(), move |_, _| {
            calls.set(calls.get() + 1)
        });
        let calls = second_calls.clone();
        second.add_handler(Button::click_event(), move |_, _| {
            calls.set(calls.get() + 1)
        });
        let root_key = |key| {
            let mut event = KeyEventArgs::new();
            event.set_routed_event(Some(InputElement::key_down_event()));
            event.key = key;
            window.raise_event(&event);
        };
        assert!(first.is_default());
        assert!(!second.is_default() && !second.is_cancel());
        assert!(
            second.is_effectively_enabled(),
            "inactive default buttons retain enabled ink"
        );
        root_key(Key::Enter);
        root_key(Key::Escape);
        assert_eq!(first_calls.get(), 1);
        assert_eq!(
            second_calls.get(),
            0,
            "the root cannot invoke inactive preview commands"
        );
        assert!(first.focus());
        host.set_focusable(false);
        host.set_selected_index(1);
        settle(&host, &clock);
        assert!(
            !first.is_focused(),
            "even a non-focusable host retires focus from its inactive row"
        );
        assert!(!first.is_default());
        assert!(second.is_default() && second.is_cancel());
        root_key(Key::Enter);
        root_key(Key::Escape);
        assert_eq!(first_calls.get(), 1);
        assert_eq!(second_calls.get(), 2);
        first.set_is_default(false);
        host.set_selected_index(0);
        settle(&host, &clock);
        assert!(
            !first.is_default(),
            "an inactive caller update survives removal of the scope lease"
        );
        root_key(Key::Enter);
        root_key(Key::Escape);
        assert_eq!(first_calls.get(), 1);
        assert_eq!(second_calls.get(), 2);
    });
}
