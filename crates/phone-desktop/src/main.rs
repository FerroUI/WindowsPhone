mod services;
mod storage;
use ferroui_base::threading::{DispatcherPriority, DispatcherTimer};
use ferroui_controls::{AppBuilder, Application, Control, Window};
use ferroui_desktop::AppBuilderDesktopExtensions;
use ferroui_fonts_inter::AppBuilderExtension;
use phone_state::{HostServices, PhoneViewModel};
use std::rc::Rc;

fn main() {
    AppBuilder::configure::<Application>()
        .use_platform_detect()
        .with_inter_font()
        .start(
            |app, _| {
                phone_controls::install_theme(app);
                let path = storage::data_path();
                let (data, warning, save_path) = storage::load(&path);
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
                let ticking = Rc::downgrade(&vm);
                let timer = DispatcherTimer::with_callback(
                    std::time::Duration::from_secs(1),
                    DispatcherPriority::BACKGROUND,
                    move |_| {
                        if let Some(vm) = ticking.upgrade() {
                            let date = chrono::Local::now();
                            vm.set_clock(date.timestamp());
                            vm.set_utc_offset(date.offset().local_minus_utc());
                        }
                    },
                );
                let host_vm = Rc::downgrade(&vm);
                vm.set_services(HostServices {
                    request: Rc::new(move |request| {
                        if let Some(vm) = host_vm.upgrade() {
                            if let Err(error) = services::handle(&vm, request) {
                                vm.set_status(error);
                            }
                        }
                    }),
                });
                let window = Window::new();
                window.set_title(Some("Windows Phone".into()));
                window.set_width(430.0);
                window.set_height(860.0);
                window.set_min_width(320.0);
                window.set_min_height(560.0);
                vm.set_viewport(430.0, 860.0);
                window.set_content(Some(Control::boxed(&phone_ui::create_view(vm.clone()))));
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
                timer.stop();
            },
            &std::env::args().collect::<Vec<_>>(),
        );
}
