//! Renderer selection and visibility. Role policy belongs to the OS provider.
use ferroui_controls::AppBuilder;
use ferroui_desktop::AppBuilderDesktopExtensions;
use phone_host::platform::HostLifecycle;

#[derive(Clone, Copy, Default)]
pub enum Backend {
    #[default]
    Auto,
    X11,
    Wayland,
}
impl Backend {
    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "auto" => Ok(Self::Auto),
            "x11" => Ok(Self::X11),
            "wayland" => Ok(Self::Wayland),
            _ => Err("Choose auto, x11 or wayland for --backend.".into()),
        }
    }
}
/// Focus loss is excluded: OS permission dialogs may temporarily take focus.
pub fn follow_visibility(
    window: &ferroui_base::Ref<ferroui_controls::Window>,
    host: std::rc::Weak<phone_native::NativeHost>,
) -> std::rc::Rc<dyn ferroui_base::reactive::IDisposable> {
    use ferroui_controls::{Window, WindowState};
    let weak = window.downgrade();
    let previous = std::cell::Cell::new(true);
    window.property_changed(move |change| {
        if change.property() == Window::window_state_property().as_property()
            || change.property() == ferroui_base::Visual::is_visible_property().as_property()
        {
            if let (Some(window), Some(host)) = (weak.upgrade(), host.upgrade()) {
                let visible =
                    window.is_visible() && window.window_state() != WindowState::Minimized;
                if previous.replace(visible) != visible {
                    host.lifecycle(if visible {
                        HostLifecycle::Resumed
                    } else {
                        HostLifecycle::Suspended
                    });
                }
            }
        }
    })
}
pub fn configure(builder: AppBuilder, backend: Backend) -> Result<AppBuilder, String> {
    let builder = builder.use_platform_detect();
    #[cfg(target_os = "linux")]
    {
        use ferroui_wayland::FerroWaylandPlatformExtensions;
        use ferroui_x11::FerroX11PlatformExtensions;
        let profiles = vec![ferroui_opengl::GlVersion::new(
            ferroui_opengl::GlProfileType::OpenGLES,
            3,
            0,
        )];
        let builder = builder
            .with(std::rc::Rc::new(ferroui_x11::X11PlatformOptions {
                rendering_mode: vec![
                    ferroui_x11::X11RenderingMode::Egl,
                    ferroui_x11::X11RenderingMode::Software,
                ],
                gl_profiles: profiles.clone(),
                wm_class: Some("com.ferroui.WindowsPhone".into()),
                ..Default::default()
            }))
            .with(std::rc::Rc::new(ferroui_wayland::WaylandPlatformOptions {
                app_id: Some("com.ferroui.WindowsPhone".into()),
                gl_profiles: profiles,
                ..Default::default()
            }));
        Ok(match backend {
            Backend::Auto => builder.use_wayland_with_fallback(),
            Backend::X11 => builder.use_x11(),
            Backend::Wayland => builder.use_wayland(),
        })
    }
    #[cfg(not(target_os = "linux"))]
    match backend {
        Backend::Auto => Ok(builder),
        _ => Err("Explicit X11/Wayland backends are available on Linux only.".into()),
    }
}
