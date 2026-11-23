use super::{Action, PlaybackState, Shared, Snapshot, Status};
use std::{
    ffi::{CStr, CString, c_char, c_int, c_void},
    ptr::NonNull,
    sync::Arc,
};

#[repr(C)]
struct NativeSnapshot {
    generation: u64,
    active: c_int,
    state: c_int,
    seekable: c_int,
    can_next: c_int,
    can_previous: c_int,
    position: f64,
    duration: f64,
    title: *const c_char,
    artist: *const c_char,
    album: *const c_char,
}
#[repr(C)]
#[derive(Default)]
struct NativeCommand {
    generation: u64,
    kind: c_int,
    value: f64,
}

unsafe extern "C" {
    fn wp_media_create(error: *mut c_char, length: usize) -> *mut c_void;
    fn wp_media_publish(
        session: *mut c_void,
        snapshot: *const NativeSnapshot,
        error: *mut c_char,
        length: usize,
    ) -> c_int;
    fn wp_media_pop(session: *mut c_void, command: *mut NativeCommand) -> c_int;
    fn wp_media_close(session: *mut c_void);
}

pub(super) struct Session(NonNull<c_void>);
impl Session {
    pub(super) fn new(_: Option<isize>, shared: Arc<Shared>) -> Result<Self, String> {
        let mut error = [0; 512];
        let session = NonNull::new(unsafe { wp_media_create(error.as_mut_ptr(), error.len()) });
        let session = session.ok_or_else(|| unsafe {
            CStr::from_ptr(error.as_ptr())
                .to_string_lossy()
                .into_owned()
        })?;
        shared.status(Status::Ready);
        Ok(Self(session))
    }
    pub(super) fn publish(&self, s: &Snapshot) -> Result<(), String> {
        let title = CString::new(s.title.as_str()).map_err(|e| e.to_string())?;
        let artist = CString::new(s.artist.as_str()).map_err(|e| e.to_string())?;
        let album = CString::new(s.album.as_str()).map_err(|e| e.to_string())?;
        let snapshot = NativeSnapshot {
            generation: s.generation,
            active: i32::from(s.active),
            state: match s.state {
                PlaybackState::Stopped => 0,
                PlaybackState::Paused => 1,
                PlaybackState::Playing => 2,
            },
            seekable: i32::from(s.seekable),
            can_next: i32::from(s.can_next),
            can_previous: i32::from(s.can_previous),
            position: s.position_seconds,
            duration: s.duration_seconds.unwrap_or(-1.0),
            title: title.as_ptr(),
            artist: artist.as_ptr(),
            album: album.as_ptr(),
        };
        // The bridge copies each string before returning; no Rust stack address
        // is retained by a native command callback.
        let mut error = [0; 512];
        if unsafe { wp_media_publish(self.0.as_ptr(), &snapshot, error.as_mut_ptr(), error.len()) }
            != 1
        {
            return Err(unsafe { CStr::from_ptr(error.as_ptr()) }
                .to_string_lossy()
                .into_owned());
        }
        Ok(())
    }
    pub(super) fn drain(&self, shared: &Shared) {
        for _ in 0..32 {
            let mut command = NativeCommand::default();
            if unsafe { wp_media_pop(self.0.as_ptr(), &mut command) } != 1 {
                break;
            }
            let action = match command.kind {
                1 => Action::Play,
                2 => Action::Pause,
                3 => Action::Toggle,
                4 => Action::Stop,
                5 => Action::Next,
                6 => Action::Previous,
                7 => Action::Seek(command.value),
                _ => continue,
            };
            shared.enqueue_for(command.generation, action);
        }
    }
}
impl Drop for Session {
    fn drop(&mut self) {
        unsafe {
            wp_media_close(self.0.as_ptr());
        }
    }
}
