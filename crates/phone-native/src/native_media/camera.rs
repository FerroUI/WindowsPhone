use super::*;
use crate::asset_store::{AssetStore, Kind};

impl NativeMedia {
    pub(super) fn start_camera(self: &Rc<Self>) {
        self.stop_camera();
        self.camera_pending.set(true);
        let generation = self.camera_generation.get();
        let Some(vm) = self.vm.upgrade() else {
            return;
        };
        let epoch = vm.document_epoch();
        let weak = Rc::downgrade(self);
        drop(Dispatcher::current_dispatcher().spawn_local(async move {
            let Some(state) = weak.upgrade() else {
                return;
            };
            let result = async {
                let interop = state.interop().await?;
                if state.camera_generation.get() != generation || vm.document_epoch() != epoch {
                    return Ok(());
                }
                let provider = providers::camera(&*interop, state.budget.clone())
                    .map_err(|e| e.to_string())?;
                let capture = CameraCapture::new(provider, state.budget.clone());
                let pending = capture.start(Default::default());
                *state.camera_capture.borrow_mut() = Some(capture);
                let session = pending.await.map_err(|e| e.to_string())?;
                if state.camera_generation.get() != generation || vm.document_epoch() != epoch {
                    session.stop();
                    return Ok(());
                }
                *state.camera_session.borrow_mut() = Some(session);
                state.refresh_camera_surfaces();
                Ok::<_, String>(())
            }
            .await;
            if state.camera_generation.get() == generation {
                state.camera_pending.set(false);
                if let Err(error) = result {
                    state.stop_camera();
                    state.status(format!("Camera: {error}"));
                }
            }
        }));
    }

    pub(super) fn stop_camera(&self) {
        self.camera_pending.set(false);
        self.photo_pending.set(false);
        self.camera_generation
            .set(self.camera_generation.get().wrapping_add(1));
        let capture = self.camera_capture.borrow_mut().take();
        if let Some(capture) = capture {
            capture.cancel_pending_start();
        }
        let session = self.camera_session.borrow_mut().take();
        if let Some(session) = session {
            session.stop();
        }
        self.refresh_camera_surfaces();
    }

    pub(super) fn capture_photo(self: &Rc<Self>) -> Result<(), String> {
        let session = self
            .camera_session
            .borrow()
            .clone()
            .ok_or("Turn on the camera before taking a photo.")?;
        let vm = self.vm.upgrade().ok_or("The phone has closed.")?;
        if self.photo_pending.replace(true) {
            return Err("A photo is still being captured or saved.".into());
        }
        let epoch = vm.document_epoch();
        let generation = self.camera_generation.get();
        let weak = Rc::downgrade(self);
        let pending = session.photo();
        let store = AssetStore::default();
        drop(Dispatcher::current_dispatcher().spawn_local(async move {
            let result = pending.await.map_err(|e| e.to_string());
            let Some(state) = weak
                .upgrade()
                .filter(|s| s.camera_generation.get() == generation)
            else {
                return;
            };
            if vm.document_epoch() != epoch {
                return;
            }
            let result = async {
                let photo = result?;
                if photo.content_type != "image/jpeg"
                    || photo.encoded.len() > Kind::Image.limit()
                    || u64::from(photo.width) * u64::from(photo.height) > 16 * 1024 * 1024
                {
                    return Err("The camera output exceeds the supported photo bounds.".into());
                }
                let name = format!(
                    "Camera-{}.jpg",
                    chrono::Local::now().format("%Y%m%d-%H%M%S")
                );
                let writer = store.clone();
                let (key, name, bytes) = crate::background::run(move || {
                    let bytes = super::photo::oriented_jpeg(
                        &photo.encoded,
                        photo.rotation_degrees,
                        photo.mirrored,
                    )?;
                    let key = writer.put(&name, "image/jpeg", Kind::Image, &bytes)?;
                    Ok((key, name, bytes))
                })
                .await?;
                if state.camera_generation.get() != generation || vm.document_epoch() != epoch {
                    let _ = crate::background::run(move || store.remove(&key)).await;
                    return Ok(());
                }
                if let Err(error) = phone_ui::register_image(key.clone(), bytes) {
                    let _ = crate::background::run(move || store.remove(&key)).await;
                    return Err(error);
                }
                vm.add_photo(name, key);
                vm.set_status("Photo saved to your Camera Roll.".into());
                Ok::<_, String>(())
            }
            .await;
            if state.camera_generation.get() == generation {
                state.photo_pending.set(false);
                if let Err(error) = result {
                    state.status(error);
                }
            }
        }));
        Ok(())
    }

    pub(super) fn poll_camera(&self) {
        let session = self.camera_session.borrow().clone();
        if let Some(session) = session {
            if let Err(error) = session.poll() {
                self.stop_camera();
                self.status(error.to_string());
            } else {
                Self::present(&self.camera_views);
            }
        }
    }
}
