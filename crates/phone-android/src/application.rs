use crate::{bridge, AndroidPlatformPlugin};
use ferroui_android::{
    log::{self, LogPriority},
    AndroidApplicationExtensions,
};
use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate, logging::LogEventLevel, reactive::Subscription,
    FerroObjectImpl, Ref,
};
use ferroui_controls::{
    application_lifetimes::IActivatableLifetime, AppBuilder, Application, ApplicationImpl,
    ApplicationImplExt, Control, NewApplication, TextBlock, TopLevel, UserControl,
};
use ferroui_fonts_inter::AppBuilderExtension;
use phone_fonts::PhoneFontBuilderExt;
use phone_host::platform::HostLifecycle;
use phone_native::NativeHost;
use std::{cell::RefCell, rc::Rc};

#[repr(C)]
pub struct PhoneApplication {
    base: Application,
    host: RefCell<Option<Rc<NativeHost>>>,
    subscriptions: RefCell<Vec<Subscription>>,
}
ferro_class!(PhoneApplication: Application);
ferro_impl_classes!(PhoneApplication: FerroObjectImpl);

impl NewApplication for PhoneApplication {
    fn new_application() -> Ref<Self> {
        instantiate(Self {
            base: Application::construct(),
            host: RefCell::new(None),
            subscriptions: RefCell::new(Vec::new()),
        })
    }
}

impl ApplicationImpl for PhoneApplication {
    fn initialize(this: &Self) {
        Self::parent_initialize(this);
        phone_controls::install_theme(this);
    }

    fn on_framework_initialization_completed(this: &Self) {
        // Resolve application classes while the Android application class loader is current.
        let _ = bridge::class();
        crate::media::prepare();
        let host = match bridge::data_path()
            .and_then(|path| NativeHost::new(path, Rc::new(AndroidPlatformPlugin::default())))
        {
            Ok(host) => host,
            Err(error) => {
                if let Some(lifetime) = this
                    .application_lifetime()
                    .as_ref()
                    .and_then(|lifetime| lifetime.as_activity_application_lifetime())
                {
                    let message = format!("Could not open Windows Phone: {error}");
                    let label = TextBlock::new();
                    label.set_text(Some(&message));
                    label.set_text_wrapping(ferroui_base::media::TextWrapping::Wrap);
                    let root = safe_area_root(label.upcast());
                    lifetime.set_main_view_factory(Some(Rc::new(move || root.clone())));
                }
                Self::parent_on_framework_initialization_completed(this);
                return;
            }
        };
        let root = safe_area_root(host.view());
        #[cfg(debug_assertions)]
        crate::smoke::install(&host, &root);
        this.host.replace(Some(host.clone()));
        if let Some(lifetime) = this
            .application_lifetime()
            .as_ref()
            .and_then(|lifetime| lifetime.as_activity_application_lifetime())
        {
            // The manifest uses singleTask. FerroActivity::on_destroy clears the old
            // content before recreation, so the retained wrapper and inner view can
            // be reattached together without reparenting the live NativeHost view.
            lifetime.set_main_view_factory(Some(Rc::new(move || root.clone())));
        }
        if let Some(activation) = this.try_get::<dyn IActivatableLifetime>() {
            let active = Rc::downgrade(&host);
            let resumed = activation.activated(Rc::new(move |args| {
                if let Some(host) = active.upgrade() {
                    host.lifecycle(HostLifecycle::Resumed);
                    if let Some(protocol) = args.as_protocol_activated() {
                        let uri = protocol.uri().as_str();
                        let result = host.activate_uri(uri);
                        #[cfg(debug_assertions)]
                        crate::smoke::activation(uri, &result);
                        if let Err(error) = result {
                            host.vm().set_status(error);
                        }
                    }
                }
            }));
            let inactive = Rc::downgrade(&host);
            let suspended = activation.deactivated(Rc::new(move |_| {
                if let Some(host) = inactive.upgrade() {
                    host.lifecycle(HostLifecycle::Suspended);
                }
            }));
            this.subscriptions
                .borrow_mut()
                .extend([Subscription::new(resumed), Subscription::new(suspended)]);
        }
        Self::parent_on_framework_initialization_completed(this);
    }
}

fn safe_area_root(view: Ref<Control>) -> Ref<Control> {
    // NativeHost measures its own view to size PhoneScreen. Apply system insets to
    // an outer container so those measurements describe the inset content area.
    TopLevel::set_auto_safe_area_padding(&view, false);
    let root = UserControl::new();
    root.set_content(Some(Control::boxed(view)));
    TopLevel::set_auto_safe_area_padding(&root, true);
    root.upcast()
}

pub fn build() -> AppBuilder {
    AppBuilder::configure::<PhoneApplication>()
        .use_android()
        .with_inter_font()
        .with_phone_font()
        .log_to_delegate(
            |line| log::write(LogPriority::Warn, "windows-phone", line),
            LogEventLevel::Warning,
            &[],
        )
}
