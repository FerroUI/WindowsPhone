use super::*;
use ferroui_controls::Control;
use ferroui_media::MediaSubscription;
use phone_ui::HostSurface;

pub(super) struct Surface {
    view: WeakRef<TexturePresenter>,
    _errors: MediaSubscription,
}

impl NativeMedia {
    pub(super) fn has_camera_surface(&self) -> bool {
        self.camera_views.borrow().iter().any(|entry| {
            entry
                .view
                .upgrade()
                .is_some_and(|view| view.is_attached_to_visual_tree())
        })
    }

    pub(super) fn surface(self: &Rc<Self>, kind: HostSurface) -> Ref<Control> {
        let view = TexturePresenter::new();
        view.set_is_hit_test_visible(false);
        view.set_stretch(ferroui_base::media::Stretch::Uniform);
        let weak = Rc::downgrade(self);
        let errors = view.errors().subscribe(move |error| {
            if let Some(state) = weak.upgrade() {
                match kind {
                    HostSurface::Camera => state.stop_camera(),
                    HostSurface::Video => {
                        if let Some(player) = state.player.borrow().as_ref() {
                            player.clear_source();
                        }
                        state.set_playing(false);
                    }
                }
                state.status(format!(
                    "Native media: {error}. Restart playback or the camera after renderer recovery."
                ));
            }
        });
        let entry = Surface {
            view: view.downgrade(),
            _errors: errors,
        };
        let source = match kind {
            HostSurface::Camera => {
                self.camera_views.borrow_mut().push(entry);
                self.camera_session
                    .borrow()
                    .as_ref()
                    .map(CameraSession::frames)
            }
            HostSurface::Video => {
                self.video_views.borrow_mut().push(entry);
                self.player.borrow().as_ref().map(MediaPlayer::frames)
            }
        };
        view.set_frame_source(source, Some(providers::importer()));
        view.upcast()
    }

    pub(super) fn refresh_video_surfaces(&self) {
        let source = self.player.borrow().as_ref().map(MediaPlayer::frames);
        self.video_views.borrow_mut().retain(|entry| {
            if let Some(view) = entry.view.upgrade() {
                view.set_frame_source(source.clone(), Some(providers::importer()));
                true
            } else {
                false
            }
        });
    }

    pub(super) fn refresh_camera_surfaces(&self) {
        let source = self
            .camera_session
            .borrow()
            .as_ref()
            .map(CameraSession::frames);
        self.camera_views.borrow_mut().retain(|entry| {
            if let Some(view) = entry.view.upgrade() {
                view.set_frame_source(source.clone(), Some(providers::importer()));
                true
            } else {
                false
            }
        });
    }

    pub(super) fn present(surfaces: &RefCell<Vec<Surface>>) {
        // Release the list borrow before invoking presenter callbacks.
        let views = {
            let mut entries = surfaces.borrow_mut();
            entries.retain(|entry| entry.view.upgrade().is_some());
            entries
                .iter()
                .filter_map(|entry| entry.view.upgrade())
                .collect::<Vec<_>>()
        };
        for view in views {
            view.present_latest();
        }
    }
}
