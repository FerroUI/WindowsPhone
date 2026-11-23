use super::PowerSnapshot;
use std::{fs::File, io::Read, path::Path};

fn attribute(path: &Path, name: &str) -> Option<String> {
    let mut value = String::new();
    File::open(path.join(name))
        .ok()?
        .take(256)
        .read_to_string(&mut value)
        .ok()?;
    Some(value.trim().to_owned())
}

pub(super) fn read(root: &Path) -> PowerSnapshot {
    let Ok(entries) = root.read_dir() else {
        return PowerSnapshot::default();
    };
    let mut batteries = Vec::new();
    let mut power_sources = Vec::new();
    for entry in entries.take(64).flatten() {
        let path = entry.path();
        let kind = attribute(&path, "type").unwrap_or_default();
        if kind == "Battery"
            && attribute(&path, "scope").as_deref() != Some("Device")
            && attribute(&path, "present").as_deref() != Some("0")
        {
            let percent = attribute(&path, "capacity")
                .and_then(|v| v.parse::<u8>().ok())
                .filter(|v| *v <= 100);
            let charging = match attribute(&path, "status").as_deref() {
                Some("Charging") => Some(true),
                Some("Discharging" | "Full" | "Not charging") => Some(false),
                _ => None,
            };
            let energy = attribute(&path, "energy_now")
                .and_then(|v| v.parse::<u64>().ok())
                .zip(attribute(&path, "energy_full").and_then(|v| v.parse::<u64>().ok()))
                .filter(|(now, full)| *full > 0 && now <= full);
            batteries.push((percent, charging, energy));
        } else if matches!(
            kind.as_str(),
            "Mains" | "USB" | "USB_C" | "USB_PD" | "USB_PD_DRP" | "Wireless"
        ) {
            power_sources.push(match attribute(&path, "online").as_deref() {
                Some("1") => Some(true),
                Some("0") => Some(false),
                _ => None,
            });
        }
    }
    let external = if power_sources.iter().any(|value| *value == Some(true)) {
        Some(true)
    } else if !power_sources.is_empty() && power_sources.iter().all(|value| *value == Some(false)) {
        Some(false)
    } else {
        None
    };
    let percent = if batteries.len() == 1 {
        batteries[0].0.or_else(|| {
            batteries[0].2.map(|(now, full)| {
                ((u128::from(now) * 100 + u128::from(full) / 2) / u128::from(full)) as u8
            })
        })
    } else if !batteries.is_empty() && batteries.iter().all(|b| b.2.is_some()) {
        let (now, full) = batteries
            .iter()
            .filter_map(|b| b.2)
            .fold((0_u128, 0_u128), |(n, f), (a, b)| {
                (n + u128::from(a), f + u128::from(b))
            });
        Some(((now * 100 + full / 2) / full).min(100) as u8)
    } else {
        None
    };
    let charging = if batteries.iter().any(|b| b.1 == Some(true)) {
        Some(true)
    } else if !batteries.is_empty() && batteries.iter().all(|b| b.1 == Some(false)) {
        Some(false)
    } else {
        None
    };
    PowerSnapshot {
        battery_percent: percent,
        charging,
        external_power: external,
        power_saver: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sysfs_handles_weighted_batteries_and_unknown_values() {
        let root = std::env::temp_dir().join(format!("phone-power-test-{}", std::process::id()));
        for (name, values) in [
            (
                "BAT0",
                vec![
                    ("type", "Battery"),
                    ("capacity", "50"),
                    ("status", "Charging"),
                    ("energy_now", "50"),
                    ("energy_full", "100"),
                ],
            ),
            (
                "BAT1",
                vec![
                    ("type", "Battery"),
                    ("capacity", "100"),
                    ("status", "Full"),
                    ("energy_now", "300"),
                    ("energy_full", "300"),
                ],
            ),
            ("AC", vec![("type", "Mains"), ("online", "1")]),
        ] {
            let path = root.join(name);
            std::fs::create_dir_all(&path).unwrap();
            for (key, value) in values {
                std::fs::write(path.join(key), value).unwrap();
            }
        }
        let value = read(&root);
        assert_eq!(value.battery_percent, Some(88));
        assert_eq!(value.charging, Some(true));
        assert_eq!(value.external_power, Some(true));
        std::fs::remove_file(root.join("BAT1/energy_full")).unwrap();
        assert_eq!(read(&root).battery_percent, None);
        std::fs::remove_dir_all(root.join("BAT1")).unwrap();
        std::fs::remove_file(root.join("BAT0/capacity")).unwrap();
        assert_eq!(read(&root).battery_percent, Some(50));
        std::fs::create_dir_all(root.join("AC2")).unwrap();
        std::fs::write(root.join("AC2/type"), "Mains").unwrap();
        assert_eq!(read(&root).external_power, Some(true));
        std::fs::write(root.join("AC/online"), "0").unwrap();
        assert_eq!(read(&root).external_power, None);
        std::fs::write(root.join("AC2/online"), "0").unwrap();
        assert_eq!(read(&root).external_power, Some(false));
        std::fs::write(root.join("AC2/online"), "1").unwrap();
        assert_eq!(read(&root).external_power, Some(true));
        std::fs::remove_dir_all(&root).unwrap();
        assert_eq!(read(&root), PowerSnapshot::default());
    }
}
