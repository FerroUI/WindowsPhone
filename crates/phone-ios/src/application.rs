use ferroui_base::{
    FerroObjectImpl, Ref, ferro_class, ferro_impl_classes, instantiate, reactive::IDisposable,
};
use ferroui_controls::{
    AppBuilder, Application, ApplicationImpl, ApplicationImplExt, Control, NewApplication,
    TextBlock, TopLevel, UserControl,
};
use ferroui_fonts_inter::AppBuilderExtension;
use ferroui_ios::{FerroApplicationDelegate, IFerroAppDelegate, IosApplicationExtensions};
use phone_fonts::PhoneFontBuilderExt;
use phone_host::platform::HostLifecycle;
use phone_native::NativeHost;
use std::{cell::RefCell, collections::VecDeque, rc::Rc};

thread_local! {
    static HOST: RefCell<Option<Rc<NativeHost>>> = const { RefCell::new(None) };
    static PENDING_LINKS: RefCell<VecDeque<String>> = const { RefCell::new(VecDeque::new()) };
}

/// Called only by the retained notification delegate, on the UIKit thread,
/// after the user opens one of this provider's carrier notifications.
#[unsafe(no_mangle)]
unsafe extern "C" fn wp_ios_native_activation(uri: *const std::ffi::c_char) {
    if uri.is_null() {
        return;
    }
    // Objective-C owns this valid NUL-terminated UTF-8 view until we return.
    let Ok(uri) = (unsafe { std::ffi::CStr::from_ptr(uri) }).to_str() else {
        return;
    };
    if uri.len() > 4096 || !uri.starts_with("ferroui-phone://app/phone.messaging?path=thread/") {
        return;
    }
    if let Some(host) = HOST.with(|slot| slot.borrow().clone()) {
        if let Err(error) = host.activate_uri(uri) {
            host.vm().set_status(error);
        }
    } else {
        PENDING_LINKS.with(|links| {
            let mut links = links.borrow_mut();
            if links.len() < 16 {
                links.push_back(uri.into());
            }
        });
    }
}

#[repr(C)]
struct PhoneApplication {
    base: Application,
}
ferro_class!(PhoneApplication: Application);
ferro_impl_classes!(PhoneApplication: FerroObjectImpl);

impl NewApplication for PhoneApplication {
    fn new_application() -> Ref<Self> {
        instantiate(Self {
            base: Application::construct(),
        })
    }
}

impl ApplicationImpl for PhoneApplication {
    fn initialize(this: &Self) {
        crate::smoke::phase("application.initialize.begin");
        Self::parent_initialize(this);
        crate::smoke::phase("application.initialize.theme");
        phone_controls::install_theme(&this.base);
        crate::smoke::phase("application.initialize.end");
    }

    fn on_framework_initialization_completed(this: &Self) {
        crate::smoke::phase("application.framework_init.begin");
        if let Some(lifetime) = this.application_lifetime() {
            if let Some(lifetime) = lifetime.as_single_view_application_lifetime() {
                let result = crate::data_path().and_then(|path| {
                    crate::smoke::phase("application.platform.begin");
                    let platform = Rc::new(crate::IosPlatform::new());
                    crate::smoke::phase("application.host.begin");
                    let result = NativeHost::new(path, platform);
                    crate::smoke::phase("application.host.end");
                    result
                });
                let view = match result {
                    Ok(host) => {
                        let view = host.view();
                        HOST.with(|slot| *slot.borrow_mut() = Some(host.clone()));
                        crate::smoke::phase("application.resumed.begin");
                        host.lifecycle(HostLifecycle::Resumed);
                        crate::smoke::phase("application.resumed.end");
                        crate::smoke::install(&host);
                        let links =
                            PENDING_LINKS.with(|links| std::mem::take(&mut *links.borrow_mut()));
                        for url in links {
                            if let Err(error) = host.activate_uri(&url) {
                                host.vm().set_status(error);
                            }
                        }
                        view
                    }
                    Err(error) => {
                        let label = TextBlock::new();
                        let message = format!("Could not open Windows Phone: {error}");
                        label.set_text(Some(&message));
                        label.set_text_wrapping(ferroui_base::media::TextWrapping::Wrap);
                        label.upcast::<Control>()
                    }
                };
                // The native host measures its own bounds for the phone's
                // viewport. Apply UIKit insets outside that measured view.
                let safe_host = UserControl::new();
                TopLevel::set_auto_safe_area_padding(&view, false);
                safe_host.set_content(Some(Control::boxed(view)));
                TopLevel::set_auto_safe_area_padding(&safe_host, true);
                crate::smoke::phase("application.main_view.begin");
                lifetime.set_main_view(Some(safe_host.upcast()));
                crate::smoke::phase("application.main_view.end");
            }
        }
        Self::parent_on_framework_initialization_completed(this);
        crate::smoke::phase("application.framework_init.end");
    }
}

#[derive(Default)]
pub struct AppDelegate {
    subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,
}

impl FerroApplicationDelegate for AppDelegate {
    fn create_app_builder(&self, delegate: Rc<dyn IFerroAppDelegate>) -> AppBuilder {
        crate::smoke::phase("delegate.create_builder.begin");
        self.subscriptions
            .borrow_mut()
            .push(delegate.activated(Rc::new(|args| {
                let host = HOST.with(|slot| slot.borrow().clone());
                if let Some(protocol) = args.as_protocol_activated() {
                    crate::smoke::activation(host.as_ref(), protocol.uri().as_str(), "received");
                }
                if let Some(host) = &host {
                    host.lifecycle(HostLifecycle::Resumed);
                }
                if let Some(protocol) = args.as_protocol_activated() {
                    let uri = protocol.uri().as_str();
                    if let Some(host) = host {
                        let result = host.activate_uri(uri);
                        crate::smoke::activation(
                            Some(&host),
                            uri,
                            result
                                .as_ref()
                                .map(|_| "accepted")
                                .unwrap_or_else(|error| error.as_str()),
                        );
                        if let Err(error) = result {
                            host.vm().set_status(error);
                        }
                    } else if uri.len() <= 4096 {
                        PENDING_LINKS.with(|links| {
                            let mut links = links.borrow_mut();
                            if links.len() < 16 {
                                links.push_back(uri.into());
                            }
                        });
                    }
                }
            })));
        self.subscriptions
            .borrow_mut()
            .push(delegate.deactivated(Rc::new(|_| {
                if let Some(host) = HOST.with(|slot| slot.borrow().clone()) {
                    host.lifecycle(HostLifecycle::Suspended);
                }
            })));
        let builder = AppBuilder::configure::<PhoneApplication>()
            .use_ios_with_delegate(delegate)
            .with_inter_font()
            .with_phone_font();
        crate::smoke::phase("delegate.create_builder.end");
        if std::env::var("PHONE_IOS_SMOKE").as_deref() == Ok("1") {
            builder
                .after_platform_services_setup(|_| {
                    crate::smoke::phase("builder.platform_services.end")
                })
                .after_application_setup(|_| crate::smoke::phase("builder.application_setup.end"))
                .after_setup(|_| crate::smoke::phase("builder.fonts_and_setup.end"))
        } else {
            builder
        }
    }
}

impl Drop for AppDelegate {
    fn drop(&mut self) {
        for subscription in self.subscriptions.get_mut().drain(..) {
            subscription.dispose();
        }
        if let Some(host) = HOST.with(|slot| slot.borrow_mut().take()) {
            host.lifecycle(HostLifecycle::Closing);
        }
    }
}
