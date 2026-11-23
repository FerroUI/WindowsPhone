//! Platform renderer prerequisites for the pinned native media providers.
use ferroui_controls::AppBuilder;

/// Minimized/hidden windows suspend apps. Ordinary focus loss is deliberately
/// excluded: an OS permission dialog may take focus while camera starts.
pub fn follow_visibility(
    window: &ferroui_base::Ref<ferroui_controls::Window>,
) -> std::rc::Rc<dyn ferroui_base::reactive::IDisposable> {
    use ferroui_controls::{Window, WindowState};
    let weak = window.downgrade();
    let previous = std::cell::Cell::new(true);
    window.property_changed(move |change| {
        if change.property() == Window::window_state_property().as_property()
            || change.property() == ferroui_base::Visual::is_visible_property().as_property()
        {
            if let Some(window) = weak.upgrade() {
                let visible =
                    window.is_visible() && window.window_state() != WindowState::Minimized;
                if previous.replace(visible) != visible {
                    phone_ui::host_visibility(visible);
                }
            }
        }
    })
}

pub fn configure(builder: AppBuilder) -> AppBuilder {
    #[cfg(target_os = "linux")]
    {
        // The media bridge imports DMA-BUF with EGL and its shaders use GLES 3.
        // The framework default is GLX, which cannot provide that transport.
        builder.with(std::rc::Rc::new(ferroui_x11::X11PlatformOptions {
            rendering_mode: vec![
                ferroui_x11::X11RenderingMode::Egl,
                ferroui_x11::X11RenderingMode::Software,
            ],
            gl_profiles: vec![ferroui_opengl::GlVersion::new(
                ferroui_opengl::GlProfileType::OpenGLES,
                3,
                0,
            )],
            wm_class: Some("com.ferroui.WindowsPhone".into()),
            ..Default::default()
        }))
    }
    #[cfg(not(target_os = "linux"))]
    {
        builder
    }
}
