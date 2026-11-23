use super::Position;
use std::ffi::{c_char, c_int, CStr};

unsafe extern "C" {
    fn wp_locate(
        latitude: *mut f64,
        longitude: *mut f64,
        accuracy: *mut f64,
        error: *mut c_char,
        length: usize,
    ) -> c_int;
}

pub(super) fn locate() -> Result<Position, String> {
    let mut position = Position {
        latitude: 0.0,
        longitude: 0.0,
        accuracy_metres: 0.0,
    };
    let mut error = [0; 512];
    let result = unsafe {
        wp_locate(
            &mut position.latitude,
            &mut position.longitude,
            &mut position.accuracy_metres,
            error.as_mut_ptr(),
            error.len(),
        )
    };
    if result == 1 {
        Ok(position)
    } else {
        Err(unsafe {
            CStr::from_ptr(error.as_ptr())
                .to_string_lossy()
                .into_owned()
        })
    }
}
