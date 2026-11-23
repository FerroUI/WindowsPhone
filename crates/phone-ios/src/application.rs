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
        Self::parent_initialize(this);
        phone_controls::install_theme(&this.base);
    }

    fn on_framework_initialization_completed(this: &Self) {
        if let Some(lifetime) = this.application_lifetime() {
            if let Some(lifetime) = lifetime.as_single_view_application_lifetime() {
                let result = crate::data_path()
                    .and_then(|path| NativeHost::new(path, Rc::new(crate::IosPlatform::new())));
                let view = match result {
                    Ok(host) => {
                        let view = host.view();
                        HOST.with(|slot| *slot.borrow_mut() = Some(host.clone()));
                        host.lifecycle(HostLifecycle::Resumed);
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
                lifetime.set_main_view(Some(safe_host.upcast()));
            }
        }
        Self::parent_on_framework_initialization_completed(this);
    }
}

#[derive(Default)]
pub struct AppDelegate {
    subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,
}

impl FerroApplicationDelegate for AppDelegate {
    fn create_app_builder(&self, delegate: Rc<dyn IFerroAppDelegate>) -> AppBuilder {
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
        AppBuilder::configure::<PhoneApplication>()
            .use_ios_with_delegate(delegate)
            .with_inter_font()
            .with_phone_font()
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
