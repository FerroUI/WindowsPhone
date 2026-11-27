use super::*;
use ferroui_base::animation::{IClock, PlayState};
use ferroui_base::input::{SwipeGestureEndedEventArgs, SwipeGestureEventArgs};
use ferroui_base::media::Brushes;
use ferroui_base::reactive::{IDisposable, IObservable, IObserver, LightweightSubject};
use ferroui_base::rendering::composition::server::{
    IServerCompositionSurface, ServerCompositionVisual, ServerRedirectVisual,
};
use ferroui_base::rendering::composition::{ElementComposition, RedirectVisual};
use ferroui_base::threading::{CancellationToken, Dispatcher};
use ferroui_base::{Point, Ref, Vector};
use ferroui_controls::{Application, Border, Canvas, Control, Window};
use ferroui_headless::FerroHeadlessPlatform;
use std::{cell::Cell, time::Duration};

struct Clock {
    ticks: LightweightSubject<SignedDuration>,
    seconds: Cell<f64>,
    play: Cell<PlayState>,
}

impl Clock {
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

impl IObservable<SignedDuration> for Clock {
    fn subscribe(&self, observer: Rc<dyn IObserver<SignedDuration>>) -> Rc<dyn IDisposable> {
        self.ticks.subscribe(observer)
    }
}

impl IClock for Clock {
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

fn scene(
    selected: usize,
    run: impl FnOnce(Ref<Window>, Ref<Carousel>, Signal<usize>, Rc<Clock>) + Send + 'static,
) {
    let session = super::tests::session();
    session
        .dispatch(
            move || {
                struct RestoreMotion;
                impl Drop for RestoreMotion {
                    fn drop(&mut self) {
                        crate::set_motion_enabled(true);
                    }
                }
                let _restore = RestoreMotion;
                crate::set_motion_enabled(true);
                crate::install_theme(&Application::current().unwrap());
                let selection = signal(selected);
                let pages = ["first", "second"]
                    .into_iter()
                    .enumerate()
                    .map(|(index, title)| {
                        PanoramaPage::new(
                            title,
                            rsx! {Border {
                                name:format!("Swatch{index}"),
                                background:"Blue",
                                {rsx!{Button {content:format!("Action {index}")}}}
                            }},
                        )
                    })
                    .collect();
                let (control, owner) = Element::from(panorama_pages(
                    "TwoSections",
                    "two sections",
                    selection,
                    pages,
                ))
                .mount_root();
                let host = control.cast::<Carousel>().unwrap();
                let clock = Clock::new();
                let window = Window::new();
                let _mounted = super::tests::MountedWindow::new(window.clone(), owner);
                window.set_clock(Some(clock.clone()));
                window.set_width(400.0);
                window.set_height(720.0);
                window.set_content(Some(Control::boxed(control)));
                window.show();
                for _ in 0..5 {
                    frame();
                }
                run(window.clone(), host, selection, clock);
            },
            CancellationToken::none(),
        )
        .wait_timeout(Duration::from_secs(20))
        .expect("native two-section panorama test finishes");
}

fn overlay(host: &Carousel) -> Ref<Canvas> {
    host.find_descendant_of_type_where::<Canvas>(false, |part| part.has_name("PanoramaWrapPreview"))
        .unwrap()
}

fn ghost(host: &Carousel) -> Option<RedirectVisual> {
    ElementComposition::get_element_child_visual(&overlay(host))
        .and_then(|visual| RedirectVisual::from_visual(&visual))
}

fn geometry(host: &Carousel) -> String {
    let layer = overlay(host);
    let containers = host
        .get_realized_containers()
        .into_iter()
        .map(|part| {
            (
                host.index_from_container(&part),
                part.bounds(),
                part.translate_point(Point::default(), &layer),
            )
        })
        .collect::<Vec<_>>();
    format!(
        "selected={}, panel={:?}, containers={containers:?}, ghost={:?}, overlay={:?}",
        host.selected_index(),
        host.items_panel_root()
            .unwrap()
            .as_scrollable()
            .unwrap()
            .offset(),
        ghost(host).map(|ghost| (ghost.offset(), ghost.size())),
        layer.bounds()
    )
}

fn held(host: &Carousel, id: i32, delta: f64) {
    host.items_panel_root()
        .unwrap()
        .raise_event(&SwipeGestureEventArgs::new(
            id,
            Vector::new(delta, 0.0),
            Vector::default(),
        ));
    for _ in 0..2 {
        frame();
    }
    assert!(host.is_swiping());
}

fn release(host: &Carousel, id: i32, clock: &Clock) {
    host.items_panel_root()
        .unwrap()
        .raise_event(&SwipeGestureEndedEventArgs::new(id, Vector::default()));
    for _ in 0..25 {
        clock.advance();
        frame();
    }
    assert!(!host.is_swiping());
}

fn assert_duplicate(host: &Carousel, index: i32, both_edges: bool) -> RedirectVisual {
    let overlay = overlay(host);
    let source = host.container_from_index(index).unwrap();
    let origin = source.translate_point(Point::default(), &overlay).unwrap();
    let duplicate =
        ghost(host).expect("second visible cyclic slot has a live native render instance");
    let actual_source = ElementComposition::get_element_visual(&source).unwrap();
    assert!(Rc::ptr_eq(&duplicate.source().unwrap(), &actual_source));
    assert!(!overlay.is_hit_test_visible());
    assert!(duplicate.clip_to_bounds());
    let adjacent = host.container_from_index(1 - index).unwrap();
    let adjacent_x = adjacent
        .translate_point(Point::default(), &overlay)
        .unwrap()
        .x;
    let extent = (adjacent_x - origin.x).abs();
    assert!(
        (duplicate.offset().x - origin.x - 2.0 * extent).abs() < 0.01,
        "source={origin:?}, source_bounds={:?}, adjacent_x={adjacent_x}, duplicate={:?}",
        source.bounds(),
        duplicate.offset()
    );
    assert!(duplicate.offset().x < overlay.bounds().width);
    assert!(duplicate.offset().x + duplicate.size().x > 0.0);
    if both_edges {
        assert!(origin.x < 0.0 && origin.x + source.bounds().width > 0.0);
        assert!(duplicate.offset().x > 0.0);
    }
    duplicate
}

#[test]
fn two_sections_keep_live_cyclic_preview_on_both_swipe_directions() {
    scene(0, |_, host, selection, clock| {
        assert!(
            ghost(&host).is_none(),
            "the initial next section is already live on the right"
        );

        held(&host, 101, 180.0);
        assert!(
            ghost(&host).is_none(),
            "midpoint needs each section only once"
        );
        held(&host, 101, 162.0);
        assert_duplicate(&host, 0, true);
        assert_eq!(selection.get_untracked(), 0);
        release(&host, 101, &clock);
        assert_eq!(selection.get_untracked(), 1);
        assert_duplicate(&host, 0, false);

        // Forward across the last-to-first seam, still with exactly two
        // native containers. The source remains visible at both viewport ends.
        held(&host, 102, 180.0);
        assert!(ghost(&host).is_none(), "{}", geometry(&host));
        held(&host, 102, 162.0);
        assert_duplicate(&host, 1, true);
        release(&host, 102, &clock);
        assert_eq!(selection.get_untracked(), 0);

        // In reverse, the duplicate is needed near the beginning of the drag,
        // before the previous section has moved fully onto the left edge.
        held(&host, 103, -18.0);
        assert_duplicate(&host, 1, true);
        held(&host, 103, -162.0);
        assert!(ghost(&host).is_none());
        held(&host, 103, -162.0);
        assert!(ghost(&host).is_none());
        release(&host, 103, &clock);
        assert_eq!(selection.get_untracked(), 1);
        assert_duplicate(&host, 0, false);

        held(&host, 104, -18.0);
        assert_duplicate(&host, 0, true);
        held(&host, 104, -324.0);
        release(&host, 104, &clock);
        assert_eq!(selection.get_untracked(), 0);
        assert!(ghost(&host).is_none());
        assert_eq!(host.get_realized_containers().len(), 2);
    });
}

#[test]
fn two_section_preview_follows_native_render_changes_and_releases_its_source() {
    scene(1, |window, host, _, _| {
        let first = assert_duplicate(&host, 0, false);
        let server = first
            .compositor()
            .server()
            .get::<ServerCompositionVisual>(first.server())
            .unwrap();
        let surface = server
            .content_as::<ServerRedirectVisual>()
            .unwrap()
            .source_surface()
            .unwrap();
        assert!(
            !surface.is_frozen(),
            "the cyclic preview is a live source, not a bitmap"
        );
        assert!(surface.source_visual().is_some());
        let changes = Rc::new(Cell::new(0));
        let observed = changes.clone();
        let key = Rc::as_ptr(&changes) as usize;
        surface
            .changed()
            .add(key, Rc::new(move || observed.set(observed.get() + 1)));
        let swatch = host
            .find_descendant_of_type_where::<Border>(false, |part| part.has_name("Swatch0"))
            .unwrap();
        swatch.set_background(Some(Brushes::red()));
        for _ in 0..2 {
            frame();
        }
        assert!(
            changes.get() > 0,
            "paint-only native content invalidates the live preview"
        );
        let before = changes.get();
        let swatch_visual = ElementComposition::get_element_visual(&swatch).unwrap();
        swatch_visual.set_opacity(0.4);
        for _ in 0..2 {
            frame();
        }
        assert!(
            changes.get() > before,
            "compositor-only child changes invalidate the live preview"
        );
        assert!(Rc::ptr_eq(first.visual(), ghost(&host).unwrap().visual()));
        surface.changed().remove(key);

        window.set_width(320.0);
        for _ in 0..5 {
            frame();
        }
        let resized = assert_duplicate(&host, 0, false);
        assert!(
            (resized.offset().x - 280.0).abs() < 0.01,
            "{}",
            geometry(&host)
        );
        assert!(Rc::ptr_eq(first.visual(), resized.visual()));

        host.set_is_visible(false);
        frame();
        assert!(ghost(&host).is_none());
        assert!(
            first.source().is_none(),
            "hiding releases the old live subtree lease"
        );
        host.set_is_visible(true);
        for _ in 0..5 {
            frame();
        }
        let reentered = assert_duplicate(&host, 0, false);
        window.set_content(None);
        assert!(
            ghost(&host).is_none(),
            "detach immediately unparents the hand-in visual"
        );
        assert!(reentered.source().is_none());
        window.set_content(Some(Control::boxed(host.clone())));
        for _ in 0..5 {
            frame();
        }
        let reattached = assert_duplicate(&host, 0, false);
        assert!(!Rc::ptr_eq(reentered.visual(), reattached.visual()));
    });
}
