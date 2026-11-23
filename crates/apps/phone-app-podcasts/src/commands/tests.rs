use super::*;
use phone_sdk::{media, AppRegistry};
use phone_state::HostServices;
use std::cell::{Cell, RefCell};

const FIRST: &str = "https://example.com/episode-one.mp3";
const SECOND: &str = "https://example.com/episode-two.mp3";

fn episode(source: &str, position: u32) -> Episode {
    Episode {
        title: source.into(),
        source: source.into(),
        description: String::new(),
        published: 0,
        duration: 100,
        video: false,
        played: false,
        position,
    }
}
fn fixture(position: u32) -> (AppContext, Rc<Cell<usize>>, Rc<RefCell<Vec<HostRequest>>>) {
    let saved = Saved {
        schema: 1,
        next_id: 3,
        shows: vec![
            Show {
                id: 1,
                title: "First show".into(),
                description: String::new(),
                feed: None,
                episodes: vec![episode(FIRST, position), episode(SECOND, 0)],
            },
            Show {
                id: 2,
                title: "Second show".into(),
                description: String::new(),
                feed: None,
                episodes: vec![episode(FIRST, position)],
            },
        ],
    };
    saved.validate().unwrap();
    let mut data = phone_domain::PhoneData::default();
    data.app_data
        .insert(crate::APP_ID.into(), serde_json::to_value(saved).unwrap());
    data.tracks = [FIRST, SECOND]
        .into_iter()
        .enumerate()
        .map(|(index, source)| phone_domain::MediaTrack {
            id: (index + 1) as u64,
            title: source.into(),
            source: source.into(),
            artist: String::new(),
            duration_seconds: 100.0,
            media_kind: "audio".into(),
        })
        .collect();
    let writes = Rc::new(Cell::new(0));
    let recorded = writes.clone();
    let vm = PhoneViewModel::new(data, Rc::new(move |_| recorded.set(recorded.get() + 1)));
    vm.set_clock(100);
    vm.set_registered_apps([crate::APP_ID.into()]);
    let calls = Rc::new(RefCell::new(Vec::new()));
    let recorded = calls.clone();
    vm.set_services(HostServices {
        request: Rc::new(move |value| recorded.borrow_mut().push(value)),
    });
    let registry = Rc::new(AppRegistry::new(vec![crate::registration()]).unwrap());
    let session = Rc::new(Session::new(100, registry));
    let cx = AppContext {
        vm,
        session,
        id: crate::APP_ID,
    };
    media::start(
        &cx.vm,
        &cx.session,
        vec![FIRST.into(), SECOND.into()],
        FIRST,
        RepeatMode::Off,
        false,
    )
    .unwrap();
    media::publish_state(&cx.vm, &cx.session, "first".into(), FIRST.into(), true);
    assert_eq!(cx.vm.route(), Route::Start);
    (cx, writes, calls)
}
fn matching(cx: &AppContext) -> Vec<Episode> {
    load(cx)
        .unwrap()
        .shows
        .into_iter()
        .flat_map(|show| show.episodes)
        .filter(|episode| episode.source == FIRST)
        .collect()
}

#[test]
fn progress_persists_off_route_and_pause_flushes_exact_position_without_repeated_writes() {
    let (cx, writes, _) = fixture(0);
    media::publish_progress(&cx.vm, &cx.session, 42.0, 100.0);
    assert!(matching(&cx)
        .iter()
        .all(|episode| episode.position == 42 && !episode.played));
    assert_eq!(writes.get(), 1);
    cx.vm.set_clock(101);
    media::publish_progress(&cx.vm, &cx.session, 43.0, 100.0);
    assert_eq!(writes.get(), 1);
    media::publish_state(&cx.vm, &cx.session, "first".into(), FIRST.into(), false);
    assert!(matching(&cx)
        .iter()
        .all(|episode| episode.position == 43 && !episode.played));
    assert_eq!(writes.get(), 2);
    media::flush(&cx.vm, &cx.session);
    media::publish_state(&cx.vm, &cx.session, String::new(), String::new(), false);
    assert_eq!(writes.get(), 2);
}

#[test]
fn end_marks_every_matching_episode_before_queue_switch_and_ignores_duplicate_end() {
    let (cx, writes, calls) = fixture(0);
    media::publish_progress(&cx.vm, &cx.session, 98.0, 100.0);
    // Near the end is still progress, not proof that the stream completed.
    assert!(matching(&cx).iter().all(|episode| !episode.played));
    calls.borrow_mut().clear();
    media::control(&cx.vm, &cx.session, "ended");
    assert!(matching(&cx)
        .iter()
        .all(|episode| episode.played && episode.position == 0));
    assert_eq!(writes.get(), 2);
    assert!(matches!(&calls.borrow()[0], HostRequest::PlayMedia { source } if source == SECOND));
    media::control(&cx.vm, &cx.session, "ended");
    media::publish_progress(&cx.vm, &cx.session, 100.0, 100.0);
    media::publish_state(&cx.vm, &cx.session, "first".into(), FIRST.into(), false);
    assert_eq!(writes.get(), 2);
    assert_eq!(calls.borrow().len(), 1);
}

#[test]
fn resume_waits_for_metadata_without_clobbering_the_saved_position() {
    let (cx, writes, calls) = fixture(80);
    cx.session
        .state::<State>()
        .pending_seek
        .replace(Some((FIRST.into(), 80)));
    calls.borrow_mut().clear();
    media::publish_progress(&cx.vm, &cx.session, 0.0, 0.0);
    media::flush(&cx.vm, &cx.session);
    assert_eq!(writes.get(), 0);
    assert!(matching(&cx).iter().all(|episode| episode.position == 80));
    media::publish_progress(&cx.vm, &cx.session, 0.0, 100.0);
    assert!(
        matches!(&calls.borrow()[0], HostRequest::MediaControl { action } if action == "seek:80")
    );
    assert_eq!(writes.get(), 0);
    media::publish_progress(&cx.vm, &cx.session, 80.0, 100.0);
    assert_eq!(writes.get(), 0);
    cx.vm.set_clock(115);
    media::publish_progress(&cx.vm, &cx.session, 85.0, 100.0);
    assert_eq!(writes.get(), 1);
    assert!(matching(&cx).iter().all(|episode| episode.position == 85));
}

#[test]
fn same_source_after_restore_rejects_old_progress_and_completion() {
    let (cx, writes, _) = fixture(55);
    media::publish_progress(&cx.vm, &cx.session, 60.0, 100.0);
    let replacement = cx.vm.data().clone();
    cx.vm
        .replace_data_after_save(replacement, |_| Ok(()))
        .unwrap();
    let before = writes.get();
    media::publish_progress(&cx.vm, &cx.session, 90.0, 100.0);
    media::control(&cx.vm, &cx.session, "ended");
    assert_eq!(writes.get(), before);
    assert!(matching(&cx)
        .iter()
        .all(|episode| episode.position == 60 && !episode.played));
}

#[test]
fn feed_completion_commits_without_replacing_a_changed_route_or_episode() {
    for destination in [
        Route::Start,
        Route::Extension {
            app: crate::APP_ID.into(),
            path: "show/2".into(),
        },
    ] {
        let (cx, _, _) = fixture(0);
        cx.navigate("show/1");
        let origin = FeedOrigin::capture(&cx);
        cx.vm.navigate(destination.clone());
        let mut show = load(&cx).unwrap().shows[0].clone();
        show.title = "Refreshed in the background".into();
        save_show(&cx, show, &origin);
        assert_eq!(cx.vm.route(), destination);
        assert_eq!(
            load(&cx).unwrap().shows[0].title,
            "Refreshed in the background"
        );
    }
    let (cx, _, _) = fixture(0);
    cx.navigate("show/1");
    let origin = FeedOrigin::capture(&cx);
    cx.session.state::<State>().episode.set(Some(FIRST.into()));
    let show = load(&cx).unwrap().shows[0].clone();
    save_show(&cx, show, &origin);
    assert_eq!(
        cx.session
            .state::<State>()
            .episode
            .get_untracked()
            .as_deref(),
        Some(FIRST)
    );
}

#[test]
fn feed_completion_opens_the_saved_show_when_the_origin_is_still_current() {
    let (cx, _, _) = fixture(0);
    cx.navigate("");
    let origin = FeedOrigin::capture(&cx);
    let mut show = load(&cx).unwrap().shows[0].clone();
    show.id = 0;
    show.title = "New subscription".into();
    save_show(&cx, show, &origin);
    assert_eq!(
        cx.vm.route(),
        Route::Extension {
            app: crate::APP_ID.into(),
            path: "show/3".into()
        }
    );
    assert_eq!(load(&cx).unwrap().shows.len(), 3);
}
