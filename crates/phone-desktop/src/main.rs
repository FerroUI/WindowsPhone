mod asset_store;
use phone_host::native::background;
mod backup;
mod documents;
mod environment;
mod file_picker;
mod location;
mod media_files;
mod native_media;
mod notifications;
mod picker_io;
mod pim;
mod platform;
mod services;
mod storage;
use ferroui_base::threading::{DispatcherPriority, DispatcherTimer};
use ferroui_controls::{AppBuilder, Application, Control, Window};
use ferroui_desktop::AppBuilderDesktopExtensions;
use ferroui_fonts_inter::AppBuilderExtension;
use phone_fonts::PhoneFontBuilderExt;
use phone_state::{HostServices, PhoneViewModel};
use std::rc::Rc;

fn main() {
    let builder = AppBuilder::configure::<Application>().use_platform_detect();
    platform::configure(builder)
        .with_inter_font()
        .with_phone_font()
        .start(
            |app, _| {
                phone_controls::install_theme(app);
                phone_ui::set_preview_capture_enabled(true);
                let path = storage::data_path();
                let (data, warning, save_path) = storage::load(&path);
                storage::set_active_data_path(save_path.clone());
                let vm = PhoneViewModel::new(
                    data,
                    Rc::new(move |data| {
                        if let Err(error) = storage::save(&save_path, data) {
                            eprintln!("Could not save Windows Phone data: {error}");
                        }
                    }),
                );
                let date = chrono::Local::now();
                vm.set_clock(date.timestamp());
                vm.set_utc_offset(date.offset().local_minus_utc());
                if let Some(warning) = warning {
                    vm.set_status(warning);
                }
                let native_environment = environment::NativeEnvironment::new(&vm);
                let ticking = Rc::downgrade(&vm);
                let timer = DispatcherTimer::with_callback(
                    std::time::Duration::from_secs(1),
                    DispatcherPriority::BACKGROUND,
                    move |_| {
                        if let Some(vm) = ticking.upgrade() {
                            native_environment.tick(&vm);
                            let date = chrono::Local::now();
                            vm.set_clock(date.timestamp());
                            vm.set_utc_offset(date.offset().local_minus_utc());
                            phone_ui::host_tick(date.timestamp());
                            phone_ui::alarm_tick(&vm);
                        }
                    },
                );
                let window = Window::new();
                let activated_vm = Rc::downgrade(&vm);
                window
                    .activated(move || {
                        if let Some(vm) = activated_vm.upgrade() {
                            notifications::refresh(&vm, false);
                        }
                    })
                    .detach();
                let native_media = native_media::NativeMedia::install(&vm, &window);
                let closing_media = Rc::downgrade(&native_media);
                let closing_window = window.downgrade();
                window
                    .closing(move |_| {
                        if let Some(media) = closing_media.upgrade() {
                            media.stop_all();
                        }
                        if let Some(window) = closing_window.upgrade() {
                            phone_host::native::notifications::close(notifications::window_handle(
                                &window,
                            ));
                        }
                    })
                    .detach();
                let host_vm = Rc::downgrade(&vm);
                let host_window = window.downgrade();
                vm.set_services(HostServices {
                    request: Rc::new(move |request| {
                        if let (Some(vm), Some(window)) = (host_vm.upgrade(), host_window.upgrade())
                        {
                            if let Err(error) = services::handle(&vm, &window, request) {
                                vm.set_status(error);
                            }
                        }
                    }),
                });
                window.set_title(Some("Windows Phone".into()));
                window.set_width(430.0);
                window.set_height(860.0);
                window.set_min_width(320.0);
                window.set_min_height(560.0);
                vm.set_viewport(430.0, 860.0);
                media_files::install_image_loader();
                window.set_content(Some(Control::boxed(&phone_ui::create_view(vm.clone()))));
                let visibility = platform::follow_visibility(&window);
                let weak_window = window.downgrade();
                let weak_vm = Rc::downgrade(&vm);
                window
                    .layout_updated(move || {
                        if let (Some(window), Some(vm)) = (weak_window.upgrade(), weak_vm.upgrade())
                        {
                            let size = window.client_size();
                            vm.set_viewport(size.width, size.height);
                        }
                    })
                    .detach();
                app.run_window(window);
                visibility.dispose();
                native_media.stop_all();
                timer.stop();
            },
            &std::env::args().collect::<Vec<_>>(),
        );
}
