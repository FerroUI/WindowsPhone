//! Host-owned native media sessions; app crates only request shared services.
mod alarm;
mod camera;
mod photo;
mod playback;
mod providers;
mod surfaces;

use ferroui_base::{
    reactive::Subscription,
    rendering::composition::{frames::FrameBudget, ElementComposition, ICompositionGpuInterop},
    threading::{Dispatcher, DispatcherPriority, DispatcherTimer},
    Ref, WeakRef,
};
use ferroui_controls::Window;
use ferroui_media_camera::{CameraCapture, CameraSession};
use ferroui_media_controls::TexturePresenter;
use ferroui_media_video::MediaPlayer;
use phone_state::{HostRequest, PhoneViewModel};
use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
    time::Duration,
};

thread_local! { static CURRENT: RefCell<Weak<NativeMedia>> = const { RefCell::new(Weak::new()) }; }

pub struct NativeMedia {
    vm: Weak<PhoneViewModel>,
    window: WeakRef<Window>,
    budget: FrameBudget,
    player: RefCell<Option<MediaPlayer>>,
    alarm_player: RefCell<Option<MediaPlayer>>,
    camera_capture: RefCell<Option<CameraCapture>>,
    camera_session: RefCell<Option<CameraSession>>,
    video_views: RefCell<Vec<surfaces::Surface>>,
    camera_views: RefCell<Vec<surfaces::Surface>>,
    camera_generation: Cell<u64>,
    camera_pending: Cell<bool>,
    photo_pending: Cell<bool>,
    media_generation: Cell<u64>,
    alarm_generation: Cell<u64>,
    document_epoch: Cell<u64>,
    source: RefCell<String>,
    title: RefCell<String>,
    playing: Cell<bool>,
    audio_options: Cell<Option<(u8, bool)>>,
    timer: RefCell<Option<Subscription>>,
}

impl NativeMedia {
    pub fn install(vm: &Rc<PhoneViewModel>, window: &Ref<Window>) -> Rc<Self> {
        let state = Rc::new(Self {
            vm: Rc::downgrade(vm),
            window: window.downgrade(),
            budget: FrameBudget::new(Default::default()),
            player: RefCell::new(None),
            alarm_player: RefCell::new(None),
            camera_capture: RefCell::new(None),
            camera_session: RefCell::new(None),
            video_views: RefCell::new(Vec::new()),
            camera_views: RefCell::new(Vec::new()),
            camera_generation: Cell::new(0),
            camera_pending: Cell::new(false),
            photo_pending: Cell::new(false),
            media_generation: Cell::new(0),
            alarm_generation: Cell::new(0),
            document_epoch: Cell::new(vm.document_epoch()),
            source: RefCell::new(String::new()),
            title: RefCell::new(String::new()),
            playing: Cell::new(false),
            audio_options: Cell::new(None),
            timer: RefCell::new(None),
        });
        CURRENT.with(|current| *current.borrow_mut() = Rc::downgrade(&state));
        let weak = Rc::downgrade(&state);
        phone_ui::set_host_surface_factory(Rc::new(move |kind| {
            weak.upgrade().map(|state| state.surface(kind))
        }));
        let weak = Rc::downgrade(&state);
        *state.timer.borrow_mut() = Some(DispatcherTimer::run(
            move || {
                let Some(state) = weak.upgrade() else {
                    return false;
                };
                state.tick();
                true
            },
            Duration::from_millis(33),
            DispatcherPriority::BACKGROUND,
        ));
        state
    }

    async fn interop(&self) -> Result<Rc<dyn ICompositionGpuInterop>, String> {
        let window = self.window.upgrade().ok_or("The phone window is closed.")?;
        let visual = ElementComposition::get_element_visual(&window)
            .ok_or("The native renderer is not ready.")?;
        visual
            .compositor()
            .try_get_composition_gpu_interop()
            .await
            .ok_or_else(|| "This renderer does not provide native media textures.".into())
    }

    fn status(&self, error: impl Into<String>) {
        if let Some(vm) = self.vm.upgrade() {
            vm.set_status(error.into());
        }
    }

    fn tick(self: &Rc<Self>) {
        let Some(vm) = self.vm.upgrade() else {
            return;
        };
        if self.document_epoch.replace(vm.document_epoch()) != vm.document_epoch() {
            self.stop_all();
        }
        if (self.camera_pending.get()
            || self.camera_capture.borrow().is_some()
            || self.camera_session.borrow().is_some())
            && !self.has_camera_surface()
        {
            self.stop_camera();
        }
        self.poll_playback(&vm);
        self.poll_camera();
        self.poll_alarm();
    }

    pub fn stop_all(&self) {
        self.stop_camera();
        self.media_generation
            .set(self.media_generation.get().wrapping_add(1));
        self.alarm_generation
            .set(self.alarm_generation.get().wrapping_add(1));
        if let Some(player) = self.player.borrow_mut().take() {
            player.clear_source();
        }
        if let Some(player) = self.alarm_player.borrow_mut().take() {
            player.clear_source();
        }
        self.set_playing(false);
        self.refresh_video_surfaces();
    }
}

impl Drop for NativeMedia {
    fn drop(&mut self) {
        self.stop_all();
        if let Some(timer) = self.timer.get_mut().take() {
            timer.dispose();
        }
    }
}

pub fn handle(request: HostRequest) -> Result<(), String> {
    let state = CURRENT
        .with(|current| current.borrow().upgrade())
        .ok_or("Native media is not initialized.")?;
    match request {
        HostRequest::PlayMedia { source } => state.play(source),
        HostRequest::MediaControl { action } => state.control(&action),
        HostRequest::StartCamera => {
            state.start_camera();
            Ok(())
        }
        HostRequest::CapturePhoto => state.capture_photo(),
        HostRequest::StopCamera => {
            state.stop_camera();
            Ok(())
        }
        HostRequest::AlarmAlert {
            active,
            volume,
            vibration: _,
        } => {
            state.alarm(active, volume);
            Ok(())
        }
        _ => Err("This is not a native media request.".into()),
    }
}
