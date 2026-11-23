use super::PowerSnapshot;

#[repr(C)]
#[derive(Default)]
struct SystemPowerStatus {
    ac_line_status: u8,
    battery_flag: u8,
    battery_life_percent: u8,
    system_status_flag: u8,
    battery_life_time: u32,
    battery_full_life_time: u32,
}

fn snapshot(value: SystemPowerStatus) -> PowerSnapshot {
    let present = value.battery_flag != 255 && value.battery_flag & 128 == 0;
    PowerSnapshot {
        battery_percent: (present && value.battery_life_percent <= 100)
            .then_some(value.battery_life_percent),
        charging: present.then_some(value.battery_flag & 8 != 0),
        external_power: match value.ac_line_status {
            0 => Some(false),
            1 => Some(true),
            _ => None,
        },
        power_saver: match value.system_status_flag {
            0 => Some(false),
            1 => Some(true),
            _ => None,
        },
    }
}

#[cfg(target_os = "windows")]
pub(super) fn read() -> PowerSnapshot {
    #[link(name = "kernel32")]
    extern "system" {
        fn GetSystemPowerStatus(status: *mut SystemPowerStatus) -> i32;
    }
    let mut value = SystemPowerStatus::default();
    // repr(C) matches SYSTEM_POWER_STATUS and the initialized writable value
    // outlives this synchronous call. A failed observation remains unknown.
    if unsafe { GetSystemPowerStatus(&mut value) } == 0 {
        PowerSnapshot::default()
    } else {
        snapshot(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unknown_and_absent_batteries_are_not_reported_as_full_or_charging() {
        let value = snapshot(SystemPowerStatus {
            ac_line_status: 255,
            battery_flag: 255,
            battery_life_percent: 255,
            ..Default::default()
        });
        assert_eq!(value.battery_percent, None);
        assert_eq!(value.charging, None);
        assert_eq!(value.external_power, None);
        let absent = snapshot(SystemPowerStatus {
            battery_flag: 128,
            battery_life_percent: 100,
            ..Default::default()
        });
        assert_eq!(absent.battery_percent, None);
        let value = snapshot(SystemPowerStatus {
            ac_line_status: 1,
            battery_flag: 8,
            battery_life_percent: 47,
            system_status_flag: 1,
            ..Default::default()
        });
        assert_eq!(value.battery_percent, Some(47));
        assert_eq!(value.charging, Some(true));
        assert_eq!(value.power_saver, Some(true));
    }
}
