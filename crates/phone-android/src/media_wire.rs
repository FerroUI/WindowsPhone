use phone_host::native::media_session::{Action, Command, PlaybackState, Snapshot};
use serde::Deserialize;

const MAX_SECONDS: f64 = 31_536_000.0;
fn milliseconds(seconds: f64) -> Option<u64> {
    (seconds.is_finite() && (0.0..=MAX_SECONDS).contains(&seconds))
        .then(|| (seconds * 1000.0).round() as u64)
}
fn label(value: &str) -> String {
    value
        .chars()
        .filter(|c| !c.is_control())
        .take(1024)
        .collect()
}

pub(crate) fn snapshot(value: &Snapshot) -> String {
    let active = value.active && value.generation != 0;
    let duration = active
        .then(|| value.duration_seconds.and_then(milliseconds))
        .flatten();
    let position = if active {
        milliseconds(value.position_seconds).unwrap_or(0)
    } else {
        0
    };
    let position = duration.map_or(position, |duration| position.min(duration));
    serde_json::json!({
        "generation": value.generation.to_string(), "active": active,
        "title": if active {label(&value.title)} else {String::new()},
        "artist": if active {label(&value.artist)} else {String::new()},
        "album": if active {label(&value.album)} else {String::new()},
        "state": if !active {"stopped"} else {match value.state {
            PlaybackState::Playing=>"playing",PlaybackState::Paused=>"paused",PlaybackState::Stopped=>"stopped"
        }},
        "position_ms": position, "duration_ms":duration,
        "seekable":active&&value.seekable,
        "can_next":active&&value.can_next,"can_previous":active&&value.can_previous,
    }).to_string()
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Incoming {
    generation: String,
    action: String,
    position_ms: Option<u64>,
}

pub(crate) fn commands(json: &str) -> Result<Vec<Command>, String> {
    if json.len() > 16384 {
        return Err("Android media command batch exceeds its limit.".into());
    }
    let incoming: Vec<Incoming> = serde_json::from_str(json)
        .map_err(|_| "Android returned invalid media commands.".to_string())?;
    if incoming.len() > 32 {
        return Err("Too many Android media commands.".into());
    }
    incoming
        .into_iter()
        .map(|value| {
            let generation = value
                .generation
                .parse::<u64>()
                .ok()
                .filter(|value| *value != 0)
                .ok_or("Invalid Android media source generation.")?;
            let action = match value.action.as_str() {
                "play" => Action::Play,
                "pause" => Action::Pause,
                "stop" => Action::Stop,
                "next" => Action::Next,
                "previous" => Action::Previous,
                "seek" => Action::Seek(
                    value
                        .position_ms
                        .filter(|value| *value <= 31_536_000_000)
                        .ok_or("Invalid Android seek position.")? as f64
                        / 1000.0,
                ),
                _ => return Err("Unknown Android media command.".into()),
            };
            if !matches!(action, Action::Seek(_)) && value.position_ms.is_some() {
                return Err("Unexpected Android media command argument.".into());
            }
            Ok(Command { generation, action })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn metadata_keeps_full_generation_and_bounds_progress() {
        let value = Snapshot {
            generation: u64::MAX,
            active: true,
            title: "A\0B".into(),
            position_seconds: 200.0,
            duration_seconds: Some(30.0),
            state: PlaybackState::Playing,
            ..Default::default()
        };
        let encoded: serde_json::Value = serde_json::from_str(&snapshot(&value)).unwrap();
        assert_eq!(encoded["generation"], u64::MAX.to_string());
        assert_eq!(encoded["position_ms"], 30000);
        assert_eq!(encoded["title"], "AB");
        let inactive = Snapshot {
            active: false,
            ..value
        };
        let encoded: serde_json::Value = serde_json::from_str(&snapshot(&inactive)).unwrap();
        assert_eq!(encoded["state"], "stopped");
        assert_eq!(encoded["title"], "");
    }
    #[test]
    fn commands_require_nonzero_generation_and_valid_seek() {
        let commands = commands(
            r#"[{"generation":"18446744073709551615","action":"seek","position_ms":1250}]"#,
        )
        .unwrap();
        assert_eq!(
            commands[0],
            Command {
                generation: u64::MAX,
                action: Action::Seek(1.25)
            }
        );
        for json in [
            r#"[{"generation":"0","action":"play"}]"#,
            r#"[{"generation":"2","action":"seek","position_ms":31536000001}]"#,
            r#"[{"generation":"2","action":"pause","position_ms":1}]"#,
            r#"[{"generation":"2","action":"open_uri"}]"#,
        ] {
            assert!(super::commands(json).is_err());
        }
        let excessive = format!(
            "[{}]",
            vec![r#"{"generation":"1","action":"play"}"#; 33].join(",")
        );
        assert!(super::commands(&excessive).is_err());
    }
}
