//! Desktop composition root. The phone and OS providers are reusable libraries.
mod platform;
use ferroui_controls::{AppBuilder, Application, Control, Window, WindowState};
use ferroui_fonts_inter::AppBuilderExtension;
use phone_fonts::PhoneFontBuilderExt;
use phone_host::platform::{HostLifecycle, HostMode};
use phone_native::NativeHost;
use phone_platform_desktop::DesktopPlatform;
use std::rc::Rc;

#[derive(Default)]
struct Options {
    mode: HostMode,
    backend: platform::Backend,
    activation: Option<String>,
}
impl Options {
    fn parse(args: impl IntoIterator<Item = String>) -> Result<Option<Self>, String> {
        let mut args = args.into_iter();
        let mut options = Self::default();
        while let Some(argument) = args.next() {
            match argument.as_str() {
                "--help" | "-h" => return Ok(None),
                "--mode" => {
                    options.mode = HostMode::parse(&args.next().ok_or("--mode requires a value.")?)?
                }
                "--backend" => {
                    options.backend = platform::Backend::parse(
                        &args.next().ok_or("--backend requires a value.")?,
                    )?
                }
                "--activate" => {
                    let uri = args
                        .next()
                        .ok_or("--activate requires a ferroui-phone URI.")?;
                    NativeHost::validate_activation(&uri)?;
                    options.activation = Some(uri);
                }
                _ => return Err(format!("Unknown option: {argument}. Use --help.")),
            }
        }
        Ok(Some(options))
    }
}
fn main() {
    if let Err(error) = run() {
        eprintln!("Windows Phone: {error}");
        std::process::exit(2);
    }
}
fn run() -> Result<(), String> {
    let Some(options) = Options::parse(std::env::args().skip(1))? else {
        println!("Windows Phone\n  --mode app|launcher|shell|embedded\n  --backend auto|x11|wayland (Linux)\n  --activate ferroui-phone://app/APP_ID?path=PATH\n\nShell mode requires an already configured supported OS session.\nEmbedding applications should use phone_native::NativeHost directly.");
        return Ok(());
    };
    let platform = Rc::new(DesktopPlatform::new(options.mode)?);
    let builder = platform::configure(AppBuilder::configure::<Application>(), options.backend)?;
    builder.with_inter_font().with_phone_font().start(
        move |app, _| {
            phone_controls::install_theme(app);
            let host = match NativeHost::new(phone_native::storage::data_path(), platform.clone()) {
                Ok(host) => host,
                Err(error) => {
                    eprintln!("Could not start Windows Phone: {error}");
                    return;
                }
            };
            let window = Window::new();
            window.set_title(Some("Windows Phone".into()));
            window.set_width(430.0);
            window.set_height(860.0);
            window.set_min_width(320.0);
            window.set_min_height(560.0);
            host.vm().set_viewport(430.0, 860.0);
            if matches!(options.mode, HostMode::Launcher | HostMode::Shell) {
                window.set_window_state(WindowState::FullScreen);
            }
            window.set_content(Some(Control::boxed(&host.view())));
            let activated = Rc::downgrade(&host);
            window
                .activated(move || {
                    if let Some(host) = activated.upgrade() {
                        host.lifecycle(HostLifecycle::Resumed);
                    }
                })
                .detach();
            let closing = Rc::downgrade(&host);
            window
                .closing(move |_| {
                    if let Some(host) = closing.upgrade() {
                        host.lifecycle(HostLifecycle::Closing);
                    }
                })
                .detach();
            let visibility = platform::follow_visibility(&window, Rc::downgrade(&host));
            if let Some(uri) = &options.activation {
                if let Err(error) = host.activate_uri(uri) {
                    host.vm().set_status(error);
                }
            }
            app.run_window(window);
            visibility.dispose();
            host.lifecycle(HostLifecycle::Closing);
        },
        &std::env::args().collect::<Vec<_>>(),
    );
    Ok(())
}
