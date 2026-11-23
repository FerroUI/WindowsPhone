//! The same provider contracts on every desktop, with no permission fallback.
use ferroui_base::rendering::composition::{frames::*, ICompositionGpuInterop};
use ferroui_media::MediaError;
#[cfg(any(target_os = "macos", target_os = "ios"))]
use ferroui_media::MediaErrorKind;
use ferroui_media_camera::CameraProvider;
use ferroui_media_controls::TextureFrameImporter;
use ferroui_media_video::MediaPlayerProvider;
use std::rc::Rc;

pub fn importer() -> Rc<dyn TextureFrameImporter> {
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    {
        Rc::new(ferroui_media_apple::AppleTextureImporter)
    }
    #[cfg(target_os = "android")]
    {
        Rc::new(ferroui_media_android::NativeGlTextureImporter)
    }
    #[cfg(windows)]
    {
        Rc::new(ferroui_media_windows::NativeGlTextureImporter)
    }
    #[cfg(target_os = "linux")]
    {
        Rc::new(ferroui_media_linux::NativeGlTextureImporter)
    }
}

pub fn video(
    interop: &dyn ICompositionGpuInterop,
    budget: FrameBudget,
) -> Result<Rc<dyn MediaPlayerProvider>, MediaError> {
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    {
        Ok(Rc::new(ferroui_media_video_apple::AppleVideoProvider {
            device: DeviceIdentity::Luid(interop.device_luid().ok_or_else(|| {
                MediaError::new(
                    MediaErrorKind::Unsupported,
                    "No compatible Apple GPU device",
                )
            })?),
            device_generation: interop.shared_context_generation(),
            budget,
        }))
    }
    #[cfg(target_os = "android")]
    {
        Ok(Rc::new(ferroui_media_video_android::AndroidVideoProvider {
            frames: ferroui_media_android::NativeGlFrameProducer::from_interop(interop, budget)?,
        }))
    }
    #[cfg(windows)]
    {
        Ok(Rc::new(ferroui_media_video_windows::WindowsVideoProvider {
            frames: ferroui_media_opengl::NativeGlFrameProducer::from_interop(interop, budget)?,
        }))
    }
    #[cfg(target_os = "linux")]
    {
        Ok(Rc::new(ferroui_media_video_linux::LinuxVideoProvider {
            frames: ferroui_media_opengl::NativeGlFrameProducer::from_interop(interop, budget)?,
        }))
    }
}

pub fn camera(
    interop: &dyn ICompositionGpuInterop,
    budget: FrameBudget,
) -> Result<Rc<dyn CameraProvider>, MediaError> {
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    {
        let _ = budget;
        Ok(Rc::new(ferroui_media_camera_apple::AppleCameraProvider {
            device: DeviceIdentity::Luid(interop.device_luid().ok_or_else(|| {
                MediaError::new(
                    MediaErrorKind::Unsupported,
                    "No compatible Apple GPU device",
                )
            })?),
            device_generation: interop.shared_context_generation(),
        }))
    }
    #[cfg(target_os = "android")]
    {
        Ok(Rc::new(
            ferroui_media_camera_android::AndroidCameraProvider {
                frames: Some(ferroui_media_android::NativeGlFrameProducer::from_interop(
                    interop, budget,
                )?),
                untagged_sdr_color: None,
            },
        ))
    }
    #[cfg(windows)]
    {
        Ok(Rc::new(
            ferroui_media_camera_windows::WindowsCameraProvider::new(
                ferroui_media_opengl::NativeGlFrameProducer::from_interop(interop, budget)?,
            ),
        ))
    }
    #[cfg(target_os = "linux")]
    {
        Ok(Rc::new(
            ferroui_media_camera_linux::LinuxCameraProvider::new(
                ferroui_media_opengl::NativeGlFrameProducer::from_interop(interop, budget)?,
                ferroui_media_camera_linux::LinuxCameraProfile::Portal,
            ),
        ))
    }
}
