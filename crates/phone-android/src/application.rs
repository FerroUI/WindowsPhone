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
    ApplicationImplExt, Control, NewApplication, TextBlock, TopLevel,
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
                    lifetime.set_main_view_factory(Some(Rc::new(move || {
                        let label = TextBlock::new();
                        label.set_text(Some(&message));
                        label.set_text_wrapping(ferroui_base::media::TextWrapping::Wrap);
                        let view = label.upcast::<Control>();
                        TopLevel::set_auto_safe_area_padding(&view, true);
                        view
                    })));
                }
                Self::parent_on_framework_initialization_completed(this);
                return;
            }
        };
        TopLevel::set_auto_safe_area_padding(&host.view(), true);
        this.host.replace(Some(host.clone()));
        if let Some(lifetime) = this
            .application_lifetime()
            .as_ref()
            .and_then(|lifetime| lifetime.as_activity_application_lifetime())
        {
            // The manifest uses singleTask. FerroActivity::on_destroy clears the old
            // content before recreation, so the retained root can be reattached and
            // unsaved app drafts remain in the same NativeHost.
            let host = host.clone();
            lifetime.set_main_view_factory(Some(Rc::new(move || host.view())));
        }
        if let Some(activation) = this.try_get::<dyn IActivatableLifetime>() {
            let active = Rc::downgrade(&host);
            let resumed = activation.activated(Rc::new(move |args| {
                if let Some(host) = active.upgrade() {
                    host.lifecycle(HostLifecycle::Resumed);
                    if let Some(protocol) = args.as_protocol_activated() {
                        if let Err(error) = host.activate_uri(protocol.uri().as_str()) {
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
