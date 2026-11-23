use super::*;
use crate::{AppContext, AppManifest, AppRegistration, AppRegistry, Capability};
use phone_controls::Icon;
use phone_domain::{AppId, MediaTrack, PhoneData, Route};
use phone_state::HostServices;
use std::{cell::RefCell, rc::Rc};

fn fixture() -> (
    Rc<PhoneViewModel>,
    Rc<Session>,
    Rc<RefCell<Vec<HostRequest>>>,
) {
    fixture_with(vec![AppRegistration::builtin(AppId::Music, |_, _| {
        Element::Empty
    })])
}
fn fixture_with(
    entries: Vec<AppRegistration>,
) -> (
    Rc<PhoneViewModel>,
    Rc<Session>,
    Rc<RefCell<Vec<HostRequest>>>,
) {
    let mut data = PhoneData::default();
    data.tracks = [(1, "media:a"), (2, "media:b")]
        .into_iter()
        .map(|(id, source)| MediaTrack {
            id,
            title: source.into(),
            source: source.into(),
            artist: String::new(),
            duration_seconds: 0.0,
            media_kind: "audio".into(),
        })
        .collect();
    let vm = PhoneViewModel::new(data, Rc::new(|_| {}));
    let calls = Rc::new(RefCell::new(Vec::new()));
    let recorded = calls.clone();
    vm.set_services(HostServices {
        request: Rc::new(move |request| recorded.borrow_mut().push(request)),
    });
    vm.set_registered_apps(
        entries
            .iter()
            .filter(|app| app.manifest.builtin.is_none())
            .map(|app| app.manifest.id.into()),
    );
    let registry = Rc::new(AppRegistry::new(entries).unwrap());
    let ui = Rc::new(Session::new(0, registry));
    (vm, ui, calls)
}
#[test]
fn queue_routes_actual_host_requests_without_optimistic_playback_success() {
    let (vm, ui, calls) = fixture();
    start(
        &vm,
        &ui,
        vec!["media:a".into(), "media:b".into()],
        "media:a",
        RepeatMode::Off,
        false,
    )
    .unwrap();
    assert!(!ui.media_playing.get_untracked());
    assert!(
        matches!(calls.borrow().last(),Some(HostRequest::PlayMedia{source})if source=="media:a")
    );
    publish_state(&vm, &ui, "a".into(), "media:a".into(), true);
    assert_eq!(capabilities(&vm, &ui), (true, true));
    assert!(control(&vm, &ui, "ended"));
    assert!(
        matches!(calls.borrow().last(),Some(HostRequest::PlayMedia{source})if source=="media:b")
    );
    publish_state(&vm, &ui, "b".into(), "media:b".into(), true);
    ui.state::<PlaybackSession>()
        .queue
        .update(|queue| queue.repeat = RepeatMode::One);
    assert!(control(&vm, &ui, "ended"));
    assert!(
        matches!(calls.borrow().last(),Some(HostRequest::MediaControl{action})if action=="restart")
    );
    assert!(!control(&vm, &ui, "play"));
}
#[test]
fn deleted_tracks_and_document_replacement_cannot_reactivate_old_queue_sources() {
    let (vm, ui, calls) = fixture();
    start(
        &vm,
        &ui,
        vec!["media:a".into(), "media:b".into()],
        "media:a",
        RepeatMode::All,
        false,
    )
    .unwrap();
    publish_state(&vm, &ui, "a".into(), "media:a".into(), true);
    vm.delete_track(2);
    control(&vm, &ui, "next");
    assert_eq!(
        ui.state::<PlaybackSession>()
            .queue
            .get_untracked()
            .sources()
            .collect::<Vec<_>>(),
        vec!["media:a"]
    );
    vm.replace_data_after_save(PhoneData::default(), |_| Ok(()))
        .unwrap();
    calls.borrow_mut().clear();
    control(&vm, &ui, "ended");
    assert!(calls.borrow().is_empty());
    assert_eq!(capabilities(&vm, &ui), (false, false));
    assert!(ui
        .state::<PlaybackSession>()
        .queue
        .get_untracked()
        .is_empty());
}
#[test]
fn host_progress_is_finite_bounded_and_reset_with_the_session() {
    let (_, ui, _) = fixture();
    progress(&ui, 999.0, 30.0);
    assert_eq!(ui.state::<PlaybackSession>().position.get_untracked(), 30.0);
    progress(&ui, f64::NAN, f64::INFINITY);
    assert_eq!(ui.state::<PlaybackSession>().position.get_untracked(), 0.0);
    assert_eq!(ui.state::<PlaybackSession>().duration.get_untracked(), 0.0);
    progress(&ui, 14.0, 90.0);
    clear(&ui);
    assert_eq!(ui.state::<PlaybackSession>().position.get_untracked(), 0.0);
}

#[derive(Default)]
struct Events(RefCell<Vec<(&'static str, PlaybackEvent, Option<String>)>>);
fn record(cx: AppContext, event: &PlaybackEvent) {
    let current = cx
        .session
        .state::<PlaybackSession>()
        .queue
        .with_untracked(|queue| queue.current().map(str::to_owned));
    cx.session
        .state::<Events>()
        .0
        .borrow_mut()
        .push((cx.id, event.clone(), current));
}
fn observer(id: &'static str, capabilities: &'static [Capability]) -> AppRegistration {
    AppRegistration::new(
        AppManifest::new(id, id, Icon::Headphones).capabilities(capabilities),
        |_| Element::Empty,
    )
    .with_media_event(record)
}
fn start_two(vm: &Rc<PhoneViewModel>, ui: &Rc<Session>) {
    start(
        vm,
        ui,
        vec!["media:a".into(), "media:b".into()],
        "media:a",
        RepeatMode::Off,
        false,
    )
    .unwrap();
    publish_state(vm, ui, "a".into(), "media:a".into(), true);
}

#[test]
fn observers_receive_progress_off_route_only_with_capability_and_availability() {
    let (vm, ui, _) = fixture_with(vec![
        observer("tests.media", &[Capability::Media]),
        observer("tests.none", &[]),
        observer("tests.disabled", &[Capability::Media]),
        AppRegistration::builtin(AppId::Music, |_, _| Element::Empty).with_media_event(record),
        AppRegistration::builtin(AppId::Video, |_, _| Element::Empty).with_media_event(record),
    ]);
    vm.set_extension_enabled("tests.disabled", false).unwrap();
    let mut data = vm.data().clone();
    data.installed.retain(|app| *app != AppId::Video);
    vm.replace_data_after_save(data, |_| Ok(())).unwrap();
    assert_eq!(vm.route(), Route::Start);
    start_two(&vm, &ui);
    publish_progress(&vm, &ui, 23.0, 60.0);
    let events = ui.state::<Events>();
    assert_eq!(
        events
            .0
            .borrow()
            .iter()
            .map(|event| event.0)
            .collect::<Vec<_>>(),
        vec!["tests.media", "phone.music"]
    );
    assert!(events.0.borrow().iter().all(|(_, event, _)| matches!(event, PlaybackEvent::Progress(value) if value.source == "media:a" && value.position == 23.0 && value.document_epoch() == vm.document_epoch())));
}

#[test]
fn completion_is_delivered_before_advancement_and_only_once() {
    let (vm, ui, calls) = fixture_with(vec![observer("tests.media", &[Capability::Media])]);
    start_two(&vm, &ui);
    publish_progress(&vm, &ui, 60.0, 60.0);
    ui.state::<Events>().0.borrow_mut().clear();
    calls.borrow_mut().clear();
    assert!(control(&vm, &ui, "ended"));
    let events = ui.state::<Events>();
    assert!(
        matches!(&events.0.borrow()[0], (_, PlaybackEvent::Ended(value), Some(current)) if value.source == "media:a" && value.position == 60.0 && current == "media:a")
    );
    assert_eq!(ui.state::<PlaybackSession>().position.get_untracked(), 0.0);
    assert!(matches!(&calls.borrow()[0], HostRequest::PlayMedia { source } if source == "media:b"));
    control(&vm, &ui, "ended");
    publish_progress(&vm, &ui, 60.0, 60.0);
    publish_state(&vm, &ui, "a".into(), "media:a".into(), false);
    assert_eq!(events.0.borrow().len(), 1);
    assert_eq!(calls.borrow().len(), 1);
}

#[test]
fn source_switch_flushes_observed_position_even_after_pending_queue_reset() {
    let (vm, ui, _) = fixture_with(vec![observer("tests.media", &[Capability::Media])]);
    start_two(&vm, &ui);
    publish_progress(&vm, &ui, 37.0, 90.0);
    ui.state::<Events>().0.borrow_mut().clear();
    start(
        &vm,
        &ui,
        vec!["media:b".into()],
        "media:b",
        RepeatMode::Off,
        false,
    )
    .unwrap();
    assert_eq!(ui.state::<PlaybackSession>().position.get_untracked(), 0.0);
    publish_state(&vm, &ui, "b".into(), "media:b".into(), true);
    let events = ui.state::<Events>();
    assert!(
        matches!(&events.0.borrow()[0].1, PlaybackEvent::Paused(value) if value.source == "media:a" && value.position == 37.0 && value.duration == 90.0)
    );
    publish_progress(&vm, &ui, 5.0, 100.0);
    assert!(
        matches!(&events.0.borrow()[1].1, PlaybackEvent::Progress(value) if value.source == "media:b" && value.position == 5.0)
    );
}

#[test]
fn restoring_during_completion_blocks_later_subscribers_and_old_queue_actions() {
    let restoring =
        observer("tests.restore", &[Capability::Media]).with_media_event(|cx, event| {
            if matches!(event, PlaybackEvent::Ended(_)) {
                let same_sources = cx.vm.data().clone();
                cx.vm
                    .replace_data_after_save(same_sources, |_| Ok(()))
                    .unwrap();
            }
        });
    let (vm, ui, calls) = fixture_with(vec![
        restoring,
        observer("tests.later", &[Capability::Media]),
    ]);
    start_two(&vm, &ui);
    publish_progress(&vm, &ui, 42.0, 60.0);
    ui.state::<Events>().0.borrow_mut().clear();
    calls.borrow_mut().clear();
    control(&vm, &ui, "ended");
    publish_progress(&vm, &ui, 50.0, 60.0);
    control(&vm, &ui, "ended");
    assert!(ui.state::<Events>().0.borrow().is_empty());
    assert!(calls.borrow().is_empty());
}

#[test]
fn source_changes_and_disabling_during_delivery_are_revalidated_for_each_subscriber() {
    let switching = observer("tests.switch", &[Capability::Media]).with_media_event(|cx, event| {
        if matches!(event, PlaybackEvent::Progress(_)) {
            cx.vm
                .set_extension_enabled("tests.disabled", false)
                .unwrap();
            publish_state(&cx.vm, &cx.session, "b".into(), "media:b".into(), true);
        }
    });
    let (vm, ui, _) = fixture_with(vec![
        switching,
        observer("tests.later", &[Capability::Media]),
        observer("tests.disabled", &[Capability::Media]),
    ]);
    start_two(&vm, &ui);
    publish_progress(&vm, &ui, 23.0, 60.0);
    let events = ui.state::<Events>();
    assert_eq!(events.0.borrow().len(), 1);
    assert_eq!(events.0.borrow()[0].0, "tests.later");
    assert!(
        matches!(&events.0.borrow()[0].1, PlaybackEvent::Paused(value) if value.source == "media:a")
    );
    assert_eq!(ui.media_source.get_untracked(), "media:b");
}

#[test]
fn reentrant_same_source_queue_replacement_survives_old_completion() {
    let replacing =
        observer("tests.replace", &[Capability::Media]).with_media_event(|cx, event| {
            if matches!(event, PlaybackEvent::Ended(_)) {
                start(
                    &cx.vm,
                    &cx.session,
                    vec!["media:a".into()],
                    "media:a",
                    RepeatMode::One,
                    false,
                )
                .unwrap();
            }
        });
    let (vm, ui, calls) = fixture_with(vec![replacing]);
    start_two(&vm, &ui);
    calls.borrow_mut().clear();
    control(&vm, &ui, "ended");
    assert_eq!(calls.borrow().len(), 1);
    assert!(matches!(&calls.borrow()[0], HostRequest::PlayMedia{source} if source == "media:a"));
    assert_eq!(
        ui.state::<PlaybackSession>()
            .queue
            .with_untracked(|queue| queue.current().map(str::to_owned)),
        Some("media:a".into())
    );
}
