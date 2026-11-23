//! Read the built-in battery through IOKit. Copy/Create results are released;
//! dictionary values remain borrowed for the lifetime of the power snapshot.
use super::PowerSnapshot;
use std::ffi::c_void;

type Cf = *const c_void;
#[link(name = "IOKit", kind = "framework")]
extern "C" {
    fn IOPSCopyPowerSourcesInfo() -> Cf;
    fn IOPSCopyPowerSourcesList(info: Cf) -> Cf;
    fn IOPSGetPowerSourceDescription(info: Cf, source: Cf) -> Cf;
    fn IOPSGetProvidingPowerSourceType(info: Cf) -> Cf;
}
#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    fn CFRelease(value: Cf);
    fn CFGetTypeID(value: Cf) -> usize;
    fn CFArrayGetCount(array: Cf) -> isize;
    fn CFArrayGetValueAtIndex(array: Cf, index: isize) -> Cf;
    fn CFDictionaryGetValue(dictionary: Cf, key: Cf) -> Cf;
    fn CFStringCreateWithBytes(
        allocator: Cf,
        bytes: *const u8,
        count: isize,
        encoding: u32,
        external: u8,
    ) -> Cf;
    fn CFEqual(a: Cf, b: Cf) -> u8;
    fn CFNumberGetTypeID() -> usize;
    fn CFNumberGetValue(number: Cf, kind: isize, value: *mut c_void) -> u8;
    fn CFBooleanGetTypeID() -> usize;
    fn CFBooleanGetValue(value: Cf) -> u8;
}

struct Owned(Cf);
impl Owned {
    fn new(value: Cf) -> Option<Self> {
        (!value.is_null()).then_some(Self(value))
    }
    fn string(value: &str) -> Option<Self> {
        // UTF-8 bytes are borrowed only for the duration of this copying call.
        Self::new(unsafe {
            CFStringCreateWithBytes(
                std::ptr::null(),
                value.as_ptr(),
                value.len() as isize,
                0x0800_0100,
                0,
            )
        })
    }
}
impl Drop for Owned {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                CFRelease(self.0);
            }
        }
    }
}

fn property(dictionary: Cf, name: &str) -> Option<Cf> {
    let key = Owned::string(name)?;
    let value = unsafe { CFDictionaryGetValue(dictionary, key.0) };
    (!value.is_null()).then_some(value)
}
fn equals(value: Cf, text: &str) -> bool {
    if value.is_null() {
        return false;
    }
    Owned::string(text).is_some_and(|text| unsafe { CFEqual(value, text.0) != 0 })
}
fn number(dictionary: Cf, name: &str) -> Option<i64> {
    let value = property(dictionary, name)?;
    if unsafe { CFGetTypeID(value) != CFNumberGetTypeID() } {
        return None;
    }
    let mut result = 0_i64;
    // kCFNumberSInt64Type=4: the output has the exact matching size/alignment.
    (unsafe { CFNumberGetValue(value, 4, (&mut result as *mut i64).cast()) } != 0).then_some(result)
}
fn boolean(dictionary: Cf, name: &str) -> Option<bool> {
    let value = property(dictionary, name)?;
    if unsafe { CFGetTypeID(value) != CFBooleanGetTypeID() } {
        return None;
    }
    Some(unsafe { CFBooleanGetValue(value) != 0 })
}

pub(super) fn read() -> PowerSnapshot {
    let Some(info) = Owned::new(unsafe { IOPSCopyPowerSourcesInfo() }) else {
        return PowerSnapshot::default();
    };
    let source = unsafe { IOPSGetProvidingPowerSourceType(info.0) };
    let mut result = PowerSnapshot {
        external_power: if equals(source, "AC Power") {
            Some(true)
        } else if equals(source, "Battery Power") {
            Some(false)
        } else {
            None
        },
        ..Default::default()
    };
    let Some(list) = Owned::new(unsafe { IOPSCopyPowerSourcesList(info.0) }) else {
        return result;
    };
    for index in 0..unsafe { CFArrayGetCount(list.0) }.clamp(0, 64) {
        let source = unsafe { CFArrayGetValueAtIndex(list.0, index) };
        let dictionary = unsafe { IOPSGetPowerSourceDescription(info.0, source) };
        if dictionary.is_null()
            || !property(dictionary, "Type").is_some_and(|value| equals(value, "InternalBattery"))
            || boolean(dictionary, "Is Present") == Some(false)
        {
            continue;
        }
        result.charging = boolean(dictionary, "Is Charging");
        result.battery_percent = number(dictionary, "Current Capacity")
            .zip(number(dictionary, "Max Capacity"))
            .filter(|(now, full)| *now >= 0 && *full > 0 && now <= full)
            .map(|(now, full)| {
                ((i128::from(now) * 100 + i128::from(full) / 2) / i128::from(full)) as u8
            });
        break;
    }
    result
}
