use super::{Action, PlaybackState, Shared, Snapshot, Status};
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU64, Ordering},
        mpsc::{sync_channel, SyncSender},
        Arc,
    },
    time::Duration,
};
use zbus::{Connection, ObjectPath, OwnedVariant, Str, Variant};

const PATH: &str = "/org/mpris/MediaPlayer2";
const PLAYER: &str = "org.mpris.MediaPlayer2.Player";
static NEXT_SESSION: AtomicU64 = AtomicU64::new(1);

pub(super) struct Session {
    wake: SyncSender<()>,
}

impl Session {
    pub(super) fn new(_: Option<isize>, shared: Arc<Shared>) -> Result<Self, String> {
        let (wake, changes) = sync_channel(1);
        std::thread::Builder::new()
            .name("phone-mpris".into())
            .spawn(move || {
                let result = (|| {
                    let name = format!(
                        "org.mpris.MediaPlayer2.WindowsPhone.instance{}_{}",
                        std::process::id(),
                        NEXT_SESSION.fetch_add(1, Ordering::Relaxed)
                    );
                    let connection = zbus::block_on(
                        zbus::connection::Builder::session()
                            .method_timeout(Duration::from_secs(5))
                            .name(name)
                            .serve_at(PATH, Root(shared.clone()))
                            .serve_at(PATH, Player(shared.clone()))
                            .build(),
                    )?;
                    shared.status(Status::Ready);
                    let mut previous = Snapshot::default();
                    loop {
                        if shared
                            .state
                            .lock()
                            .unwrap_or_else(|e| e.into_inner())
                            .closed
                        {
                            break;
                        }
                        if connection.is_closed() {
                            shared.status(Status::Unavailable(
                                "The desktop media session has disconnected.".into(),
                            ));
                            break;
                        }
                        let current = shared.snapshot();
                        zbus::block_on(publish(&connection, &previous, &current))?;
                        previous = current;
                        // Wait OUTSIDE block_on: zbus 6's default runtime gives
                        // idle connections to its helper only when block_on is
                        // no longer active on this thread. Blocking within the
                        // async future would prevent incoming method dispatch.
                        if matches!(
                            changes.recv_timeout(Duration::from_secs(5)),
                            Err(std::sync::mpsc::RecvTimeoutError::Disconnected)
                        ) {
                            break;
                        }
                    }
                    Ok::<_, zbus::Error>(())
                })();
                if let Err(error) = result {
                    shared.status(Status::Unavailable(format!(
                        "Desktop media controls: {error}"
                    )));
                }
            })
            .map_err(|e| e.to_string())?;
        Ok(Self { wake })
    }
    pub(super) fn publish(&self, _: &Snapshot) -> Result<(), String> {
        match self.wake.try_send(()) {
            Ok(()) | Err(std::sync::mpsc::TrySendError::Full(_)) => Ok(()),
            Err(std::sync::mpsc::TrySendError::Disconnected(_)) => {
                Err("The desktop media session has disconnected.".into())
            }
        }
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        // The owner sets closed first. A full channel already wakes the worker;
        // dropping the last sender wakes it when the channel is empty.
        let _ = self.wake.try_send(());
    }
}

struct Root(Arc<Shared>);
#[zbus::interface(name = "org.mpris.MediaPlayer2")]
impl Root {
    fn raise(&self) {
        self.0.enqueue(Action::Raise);
    }
    fn quit(&self) {}
    #[zbus(property)]
    fn can_quit(&self) -> bool {
        false
    }
    #[zbus(property)]
    fn can_raise(&self) -> bool {
        true
    }
    #[zbus(property)]
    fn has_track_list(&self) -> bool {
        false
    }
    #[zbus(property)]
    fn identity(&self) -> &str {
        "Windows Phone"
    }
    #[zbus(property)]
    fn desktop_entry(&self) -> &str {
        "com.ferroui.WindowsPhone"
    }
    #[zbus(property)]
    fn supported_uri_schemes(&self) -> Vec<String> {
        Vec::new()
    }
    #[zbus(property)]
    fn supported_mime_types(&self) -> Vec<String> {
        Vec::new()
    }
}

struct Player(Arc<Shared>);
#[zbus::interface(name = "org.mpris.MediaPlayer2.Player")]
impl Player {
    fn play(&self) {
        self.0.enqueue(Action::Play);
    }
    fn pause(&self) {
        self.0.enqueue(Action::Pause);
    }
    fn play_pause(&self) -> zbus::fdo::Result<()> {
        if self.0.enqueue(Action::Toggle) {
            Ok(())
        } else {
            Err(zbus::fdo::Error::Failed(
                "No active media player or command queue is busy.".into(),
            ))
        }
    }
    fn stop(&self) {
        self.0.enqueue(Action::Stop);
    }
    fn next(&self) {
        self.0.enqueue(Action::Next);
    }
    fn previous(&self) {
        self.0.enqueue(Action::Previous);
    }
    fn seek(&self, offset: i64) {
        self.0.enqueue(Action::SeekBy(offset as f64 / 1_000_000.0));
    }
    fn set_position(&self, track_id: ObjectPath<'_>, position: i64) {
        let snapshot = self.0.snapshot();
        if position >= 0 && track_id.as_str() == track_path(snapshot.generation) {
            self.0.enqueue_for(
                snapshot.generation,
                Action::Seek(position as f64 / 1_000_000.0),
            );
        }
    }
    fn open_uri(&self, _uri: &str) -> zbus::fdo::Result<()> {
        Err(zbus::fdo::Error::NotSupported(
            "Choose media in the phone application.".into(),
        ))
    }
    #[zbus(property)]
    fn playback_status(&self) -> &'static str {
        playback_state(self.0.snapshot().state)
    }
    #[zbus(property)]
    fn metadata(&self) -> HashMap<String, OwnedVariant> {
        metadata(&self.0.snapshot())
    }
    #[zbus(property(emits_changed_signal = "false"))]
    fn position(&self) -> i64 {
        micros(self.0.snapshot().position_seconds)
    }
    #[zbus(property)]
    fn volume(&self) -> f64 {
        self.0.snapshot().volume
    }
    #[zbus(property)]
    fn set_volume(&self, volume: f64) -> zbus::fdo::Result<()> {
        // MPRIS defines negative volume as silence. This player has no gain
        // amplification, so values above unity use its maximum native level.
        if volume.is_finite() && self.0.enqueue(Action::Volume(volume.clamp(0.0, 1.0))) {
            Ok(())
        } else {
            Err(zbus::fdo::Error::InvalidArgs(
                "Volume must be between zero and one for an active player.".into(),
            ))
        }
    }
    #[zbus(property)]
    fn rate(&self) -> f64 {
        1.0
    }
    #[zbus(property)]
    fn set_rate(&self, rate: f64) -> zbus::fdo::Result<()> {
        if rate == 0.0 {
            self.0.enqueue(Action::Pause);
            Ok(())
        } else if rate == 1.0 {
            Ok(())
        } else {
            Err(zbus::fdo::Error::NotSupported(
                "Only normal playback rate is supported.".into(),
            ))
        }
    }
    #[zbus(property)]
    fn minimum_rate(&self) -> f64 {
        1.0
    }
    #[zbus(property)]
    fn maximum_rate(&self) -> f64 {
        1.0
    }
    #[zbus(property)]
    fn can_go_next(&self) -> bool {
        self.0.snapshot().can_next
    }
    #[zbus(property)]
    fn can_go_previous(&self) -> bool {
        self.0.snapshot().can_previous
    }
    #[zbus(property)]
    fn can_play(&self) -> bool {
        self.0.snapshot().active
    }
    #[zbus(property)]
    fn can_pause(&self) -> bool {
        self.0.snapshot().active
    }
    #[zbus(property)]
    fn can_seek(&self) -> bool {
        self.0.snapshot().seekable
    }
    #[zbus(property)]
    fn can_control(&self) -> bool {
        true
    }
}

fn playback_state(state: PlaybackState) -> &'static str {
    match state {
        PlaybackState::Playing => "Playing",
        PlaybackState::Paused => "Paused",
        PlaybackState::Stopped => "Stopped",
    }
}
fn track_path(generation: u64) -> String {
    format!("/com/ferroui/WindowsPhone/track/{generation}")
}
fn micros(value: f64) -> i64 {
    (value * 1_000_000.0) as i64
}
fn metadata(s: &Snapshot) -> HashMap<String, OwnedVariant> {
    if !s.active {
        return HashMap::new();
    }
    let mut data = HashMap::from([
        (
            "mpris:trackid".into(),
            OwnedVariant::from(
                ObjectPath::try_from(track_path(s.generation)).expect("numeric track path"),
            ),
        ),
        (
            "xesam:title".into(),
            OwnedVariant::from(Str::from(s.title.clone())),
        ),
    ]);
    if !s.artist.is_empty() {
        data.insert(
            "xesam:artist".into(),
            Variant::from(vec![s.artist.clone()])
                .try_into_owned()
                .expect("string array owns no descriptors"),
        );
    }
    if !s.album.is_empty() {
        data.insert(
            "xesam:album".into(),
            OwnedVariant::from(Str::from(s.album.clone())),
        );
    }
    if let Some(duration) = s.duration_seconds {
        data.insert("mpris:length".into(), OwnedVariant::from(micros(duration)));
    }
    data
}

async fn publish(
    connection: &Connection,
    previous: &Snapshot,
    current: &Snapshot,
) -> zbus::Result<()> {
    let mut changed = HashMap::<String, OwnedVariant>::new();
    if (
        previous.generation,
        previous.active,
        &previous.title,
        &previous.artist,
        &previous.album,
        previous.duration_seconds,
    ) != (
        current.generation,
        current.active,
        &current.title,
        &current.artist,
        &current.album,
        current.duration_seconds,
    ) {
        changed.insert("Metadata".into(), metadata(current).into());
    }
    if previous.state != current.state {
        changed.insert(
            "PlaybackStatus".into(),
            Str::from(playback_state(current.state)).into(),
        );
    }
    if previous.volume != current.volume {
        changed.insert("Volume".into(), current.volume.into());
    }
    for (name, old, new) in [
        ("CanGoNext", previous.can_next, current.can_next),
        ("CanGoPrevious", previous.can_previous, current.can_previous),
        ("CanSeek", previous.seekable, current.seekable),
        ("CanPlay", previous.active, current.active),
        ("CanPause", previous.active, current.active),
    ] {
        if old != new {
            changed.insert(name.into(), new.into());
        }
    }
    if !changed.is_empty() {
        connection
            .emit_signal(
                None::<&str>,
                PATH,
                "org.freedesktop.DBus.Properties",
                "PropertiesChanged",
                &(PLAYER, changed, Vec::<String>::new()),
            )
            .await?;
    }
    if current.active
        && previous.generation == current.generation
        && previous.seek_serial != current.seek_serial
    {
        connection
            .emit_signal(
                None::<&str>,
                PATH,
                PLAYER,
                "Seeked",
                &(micros(current.position_seconds),),
            )
            .await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mpris_rejects_stale_track_positions_and_arbitrary_urls() {
        let shared = Arc::new(super::super::tests::shared());
        let player = Player(shared.clone());
        player.set_position(ObjectPath::try_from(track_path(6)).unwrap(), 10_000_000);
        assert!(shared.state.lock().unwrap().commands.is_empty());
        player.set_position(ObjectPath::try_from(track_path(7)).unwrap(), 10_000_000);
        assert_eq!(
            shared
                .state
                .lock()
                .unwrap()
                .commands
                .pop_front()
                .unwrap()
                .action,
            Action::Seek(10.0)
        );
        assert!(player.open_uri("file:///etc/passwd").is_err());
        assert_eq!(player.rate(), 1.0);
        assert!(player.set_rate(f64::NAN).is_err());
    }
    #[test]
    fn metadata_uses_a_typed_opaque_track_and_never_a_source_url() {
        let mut snapshot = Snapshot {
            active: true,
            generation: 4,
            title: "Track".into(),
            artist: "Artist".into(),
            duration_seconds: Some(2.5),
            ..Default::default()
        };
        let values = metadata(&snapshot);
        assert_eq!(values.len(), 4);
        assert!(!values.contains_key("xesam:url"));
        assert_eq!(
            i64::try_from(values.get("mpris:length").unwrap()).unwrap(),
            2_500_000
        );
        snapshot.active = false;
        assert!(metadata(&snapshot).is_empty());
    }

    /// Run with `dbus-run-session -- env PHONE_MPRIS_TEST_SESSION=1 cargo test
    /// -p phone-host private_bus_media_controls -- --ignored`. Never attaches
    /// a test player to the person's existing desktop session.
    #[test]
    #[ignore = "requires a dedicated dbus-run-session"]
    fn private_bus_media_controls() {
        assert_eq!(
            std::env::var("PHONE_MPRIS_TEST_SESSION").as_deref(),
            Ok("1")
        );
        let session = super::super::MediaSession::new(None).unwrap();
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        while session.status() == Status::Pending && std::time::Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(session.status(), Status::Ready);
        let snapshot = Snapshot {
            generation: 9,
            active: true,
            title: "Private test track".into(),
            artist: "Private artist".into(),
            state: PlaybackState::Playing,
            seekable: true,
            can_next: true,
            duration_seconds: Some(120.0),
            volume: 0.5,
            ..Default::default()
        };
        session.publish(snapshot).unwrap();
        zbus::block_on(async {
            let connection = Connection::session().await.unwrap();
            let bus = zbus::Proxy::new(
                &connection,
                "org.freedesktop.DBus",
                "/org/freedesktop/DBus",
                "org.freedesktop.DBus",
            )
            .await
            .unwrap();
            let names: Vec<String> = bus.call("ListNames", &()).await.unwrap();
            let prefix = format!(
                "org.mpris.MediaPlayer2.WindowsPhone.instance{}_",
                std::process::id()
            );
            let name = names.iter().find(|name| name.starts_with(&prefix)).unwrap();
            let player = zbus::Proxy::new(&connection, name.as_str(), PATH, PLAYER)
                .await
                .unwrap();
            let status: String = player.get_property("PlaybackStatus").await.unwrap();
            assert_eq!(status, "Playing");
            let mut meta: HashMap<String, OwnedVariant> =
                player.get_property("Metadata").await.unwrap();
            assert_eq!(
                String::try_from(meta.remove("xesam:title").unwrap()).unwrap(),
                "Private test track"
            );
            let _: () = player.call("Pause", &()).await.unwrap();
            assert_eq!(
                session.take_commands(),
                vec![super::super::Command {
                    generation: 9,
                    action: Action::Pause
                }]
            );
            let _: () = player
                .call(
                    "SetPosition",
                    &(ObjectPath::try_from(track_path(8)).unwrap(), 20_000_000i64),
                )
                .await
                .unwrap();
            assert!(session.take_commands().is_empty());
            let _: () = player
                .call(
                    "SetPosition",
                    &(ObjectPath::try_from(track_path(9)).unwrap(), 20_000_000i64),
                )
                .await
                .unwrap();
            assert_eq!(
                session.take_commands(),
                vec![super::super::Command {
                    generation: 9,
                    action: Action::Seek(20.0)
                }]
            );
            let _: () = player.call("Next", &()).await.unwrap();
            assert_eq!(
                session.take_commands(),
                vec![super::super::Command {
                    generation: 9,
                    action: Action::Next
                }]
            );
            player.set_property("Volume", &0.25f64).await.unwrap();
            assert_eq!(
                session.take_commands(),
                vec![super::super::Command {
                    generation: 9,
                    action: Action::Volume(0.25)
                }]
            );
            // The OS cannot open untrusted paths or URLs through this adapter.
            assert!(player
                .call::<_, _, ()>("OpenUri", &("file:///etc/passwd",))
                .await
                .is_err());
            drop(session);
            let deadline = std::time::Instant::now() + Duration::from_secs(3);
            loop {
                let owned: bool = bus.call("NameHasOwner", &(name.as_str(),)).await.unwrap();
                if !owned {
                    break;
                }
                assert!(
                    std::time::Instant::now() < deadline,
                    "MPRIS session did not release its bus name"
                );
                std::thread::sleep(Duration::from_millis(10));
            }
        });
    }
}
