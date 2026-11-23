//! UI-thread runtime and replaceable service routing, independent of app crates.
pub mod activation;
mod gate;
#[cfg(test)]
mod tests;
use ferroui_base::{reactive::IDisposable, threading::Dispatcher};
use ferroui_reactive::ObservableExt;
use ferroui_reactiveui::IReactiveNotifyPropertyChanged;
use phone_host::{platform::*, HostEnvironment, HostPermission, HostRequest};
use phone_state::{HostServices, PhoneViewModel};
use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    rc::Rc,
};
type Completion = Box<dyn FnOnce(&Rc<PhoneViewModel>, PlatformResponse) -> Result<(), String>>;
pub struct PhoneRuntime {
    vm: Rc<PhoneViewModel>,
    platform: Rc<dyn PlatformPlugin>,
    services: Rc<ServiceRegistry>,
    gate: Rc<gate::Gate>,
    closed: Cell<bool>,
    closing: Cell<bool>,
    observing: Cell<bool>,
    serial: Cell<u64>,
    channels: RefCell<BTreeMap<u8, u64>>,
    notification_policy: RefCell<Option<DeviceNotificationPolicy>>,
    policy_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
}
impl PhoneRuntime {
    pub fn attach(
        vm: Rc<PhoneViewModel>,
        platform: Rc<dyn PlatformPlugin>,
        services: ServiceRegistry,
    ) -> Result<Rc<Self>, String> {
        let state = platform.state();
        state.validate()?;
        vm.set_platform_state(state)?;
        let runtime = Rc::new(Self {
            vm,
            platform,
            services: Rc::new(services),
            gate: Rc::default(),
            closed: Cell::new(false),
            closing: Cell::new(false),
            observing: Cell::new(false),
            serial: Cell::new(0),
            channels: RefCell::default(),
            notification_policy: RefCell::new(None),
            policy_subscription: RefCell::new(None),
        });
        let weak = Rc::downgrade(&runtime);
        runtime.vm.set_services(HostServices {
            request: Rc::new(move |request| {
                if let Some(runtime) = weak.upgrade() {
                    if let Err(error) = runtime.request(request) {
                        runtime.vm.set_status(error);
                    }
                }
            }),
        });
        let weak = Rc::downgrade(&runtime);
        *runtime.policy_subscription.borrow_mut() =
            Some(runtime.vm.changed().subscribe_next(move |_| {
                if let Some(runtime) = weak.upgrade() {
                    runtime.refresh_notification_policy();
                }
            }));
        runtime.refresh_notification_policy();
        runtime.observe();
        Ok(runtime)
    }
    pub fn vm(&self) -> Rc<PhoneViewModel> {
        self.vm.clone()
    }
    pub fn platform(&self) -> Rc<dyn PlatformPlugin> {
        self.platform.clone()
    }
    pub fn is_closed(&self) -> bool {
        self.closed.get()
    }
    fn refresh_notification_policy(&self) {
        if self.closed.get() || self.closing.get() {
            return;
        }
        let policy = {
            let data = self.vm.data();
            let settings = &data.settings;
            let messages = settings.notification_preference(phone_domain::AppId::Messaging);
            DeviceNotificationPolicy {
                enabled: settings.notifications,
                message_center: messages.action_center,
                message_banners: messages.banners,
                message_sound: settings.sounds && messages.sound,
                message_vibration: settings.vibration && messages.vibrate,
                lock_screen: settings.notifications_on_lock,
            }
        };
        if self.notification_policy.borrow().as_ref() == Some(&policy) {
            return;
        }
        *self.notification_policy.borrow_mut() = Some(policy);
        self.platform.set_notification_policy(&policy);
    }
    pub fn request(self: &Rc<Self>, request: HostRequest) -> Result<(), String> {
        if self.closed.get() {
            return Err("The phone host is closed.".into());
        }
        if let HostRequest::Platform { request } = request {
            let picker = match &request {
                PlatformRequest::Device {
                    request: DeviceRequest::PickMessageAttachments { client_id },
                } => Some(client_id.clone()),
                _ => None,
            };
            let client = match &request {
                PlatformRequest::Device {
                    request:
                        DeviceRequest::SendMessage { client_id, .. }
                        | DeviceRequest::SendMultimediaMessage { client_id, .. },
                } => Some(client_id.clone()),
                _ => None,
            };
            let result = self.dispatch_platform(request, None);
            if let (Err(message), Some(client_id)) = (&result, client) {
                self.vm.set_device_response(DeviceResponse::SendFailed {
                    client_id,
                    message: message.chars().take(1024).collect(),
                })?;
            }
            if let (Err(message), Some(client_id)) = (&result, picker) {
                self.vm
                    .set_device_response(DeviceResponse::DraftAttachments {
                        client_id,
                        attachments: None,
                        error: Some(message.chars().take(1024).collect()),
                    })?;
            }
            result
        } else {
            self.services.dispatch(request)
        }
    }
    pub fn dispatch_platform(
        self: &Rc<Self>,
        request: PlatformRequest,
        complete: Option<Completion>,
    ) -> Result<(), String> {
        if self.closed.get() {
            return Err("The phone host is closed.".into());
        }
        request.validate()?;
        {
            let data = self.vm.data();
            match &request {
                PlatformRequest::Locate if !data.settings.location => {
                    return Err("Turn on location in the phone's Settings first.".into());
                }
                PlatformRequest::Haptic { .. } if !data.settings.vibration => {
                    return Err("Turn on vibration in the phone's Settings first.".into());
                }
                PlatformRequest::Notify { .. } if !data.settings.notifications => {
                    return Err("Turn on notifications in the phone's Settings first.".into());
                }
                _ => {}
            }
        }
        let state = self.platform.state();
        state.validate()?;
        if request
            .feature()
            .is_some_and(|feature| !state.supports(feature))
        {
            return Err("This integration is unavailable on the current host.".into());
        }
        let reservation = self.gate.reserve()?;
        let serial = self
            .serial
            .get()
            .checked_add(1)
            .ok_or("Platform request identities exhausted.")?;
        self.serial.set(serial);
        let channel = match &request {
            PlatformRequest::Refresh => Some(0),
            PlatformRequest::ListApplications => Some(1),
            PlatformRequest::Locate => Some(11),
            PlatformRequest::Device { request } => request.observation_channel(),
            _ => None,
        };
        if let Some(channel) = channel {
            self.channels.borrow_mut().insert(channel, serial);
        }
        let epoch = self.vm.document_epoch();
        let send_client = match &request {
            PlatformRequest::Device {
                request:
                    DeviceRequest::SendMessage { client_id, .. }
                    | DeviceRequest::SendMultimediaMessage { client_id, .. },
            } => Some(client_id.clone()),
            _ => None,
        };
        let expected = request.clone();
        let picker_client = match &request {
            PlatformRequest::Device {
                request: DeviceRequest::PickMessageAttachments { client_id },
            } => Some(client_id.clone()),
            _ => None,
        };
        let task = reservation.bind(self.platform.dispatch(request));
        let weak = Rc::downgrade(self);
        drop(Dispatcher::current_dispatcher().spawn_local(async move {
            let result = match (task.await, send_client,picker_client) {
                (Err(message), Some(client_id),_) => {
                    Ok(PlatformResponse::Device(DeviceResponse::SendFailed {
                        client_id,
                        message:message.chars().take(1024).collect(),
                    }))
                }
                (Err(message),_,Some(client_id))=>Ok(PlatformResponse::Device(DeviceResponse::DraftAttachments{client_id,attachments:None,error:Some(message.chars().take(1024).collect())})),
                (result, _,_) => result,
            };
            let result=result.and_then(|response|{
                if let PlatformRequest::Device{request}=&expected {
                    if !matches!(&response,PlatformResponse::Device(value) if request.accepts_response(value)) {
                        return Err("The device provider returned a result for a different request.".into());
                    }
                }
                Ok(response)
            });
            if let Some(runtime) = weak.upgrade() {
                runtime.deliver_completion(epoch, channel, serial, result, complete);
            }
        }));
        Ok(())
    }

    /// All completion admission passes through this synchronous UI boundary.
    /// VM notifications and custom handlers may change ownership reentrantly.
    fn deliver_completion(
        &self,
        epoch: u64,
        channel: Option<u8>,
        serial: u64,
        result: Result<PlatformResponse, String>,
        complete: Option<Completion>,
    ) {
        if !self.current_completion(epoch, channel, serial) {
            return;
        }
        self.observe();
        if !self.current_completion(epoch, channel, serial) {
            return;
        }
        let result = result.and_then(|response| match complete {
            Some(complete) => complete(&self.vm, response),
            None => self.accept(response),
        });
        if let Err(error) = result {
            if self.current_completion(epoch, channel, serial) {
                self.vm.set_status(error.chars().take(2048).collect());
            }
        }
    }

    fn current_completion(&self, epoch: u64, channel: Option<u8>, serial: u64) -> bool {
        !self.closed.get()
            && !self.closing.get()
            && self.vm.document_epoch() == epoch
            && channel.is_none_or(|channel| self.channels.borrow().get(&channel) == Some(&serial))
    }
    fn accept(&self, response: PlatformResponse) -> Result<(), String> {
        match response {
            PlatformResponse::Device(response) => self.vm.set_device_response(response),
            PlatformResponse::State(state) => self.vm.set_platform_state(state),
            PlatformResponse::Applications(apps) => self.vm.set_device_applications(apps),
            PlatformResponse::Accepted { message } => {
                if !message.is_empty() {
                    self.vm.set_status(message.chars().take(2048).collect());
                }
                Ok(())
            }
            PlatformResponse::Location {
                latitude,
                longitude,
            } => {
                if !self.vm.data().settings.location
                    || matches!(
                        self.vm
                            .device_state()
                            .snapshot
                            .permission(DevicePermission::Location),
                        PermissionState::Denied | PermissionState::Restricted
                    )
                {
                    return Err("Location access changed before this request completed.".into());
                }
                if !latitude.is_finite()
                    || !longitude.is_finite()
                    || !(-90.0..=90.0).contains(&latitude)
                    || !(-180.0..=180.0).contains(&longitude)
                {
                    return Err("Invalid location from the platform provider.".into());
                }
                self.vm.set_device_location(latitude, longitude)
            }
        }
    }
    pub fn observe(&self) {
        if self.closed.get() || self.closing.get() || self.observing.replace(true) {
            return;
        }
        struct Reset<'a>(&'a Cell<bool>);
        impl Drop for Reset<'_> {
            fn drop(&mut self) {
                self.0.set(false);
            }
        }
        let _reset = Reset(&self.observing);
        let epoch = self.vm.document_epoch();
        let observation = self.platform.environment();
        let mut current = self.vm.environment();
        // An owning provider may revoke or become unable to observe permission.
        if self.platform.state().supports(HostFeature::Notifications) {
            current.notification_permission = observation.notification_permission;
        }
        merge_environment(&mut current, observation);
        self.vm.set_environment(current);
        if self.closed.get() || self.closing.get() || self.vm.document_epoch() != epoch {
            return;
        }
        if let Err(error) = self.vm.set_platform_state(self.platform.state()) {
            self.vm.set_status(error);
        }
        if self.closed.get() || self.closing.get() || self.vm.document_epoch() != epoch {
            return;
        }
        let events = self.platform.drain_events();
        if events.len() > 32 {
            self.vm
                .set_status("The device provider exceeded its event budget.".into());
            return;
        }
        for event in events {
            if self.closed.get() || self.closing.get() || self.vm.document_epoch() != epoch {
                break;
            }
            if let Err(error) = self.accept(event) {
                if !self.closed.get() && !self.closing.get() && self.vm.document_epoch() == epoch {
                    self.vm.set_status(error);
                }
            }
        }
    }
    pub fn lifecycle(&self, event: HostLifecycle) {
        if self.closed.get() || self.closing.get() {
            return;
        }
        if event == HostLifecycle::Closing {
            self.close_with(|| phone_system::host_closing(&self.vm));
            return;
        }
        phone_system::host_visibility(event == HostLifecycle::Resumed);
        self.platform.lifecycle(event);
        if event == HostLifecycle::Resumed {
            self.observe();
        }
    }

    fn close_with(&self, cleanup: impl FnOnce()) {
        if self.closed.get() || self.closing.replace(true) {
            return;
        }
        // Final app cleanup may dispatch camera/media stops, while nested
        // lifecycle callbacks cannot enter shutdown a second time.
        cleanup();
        self.closed.set(true);
        if let Some(subscription) = self.policy_subscription.borrow_mut().take() {
            subscription.dispose();
        }
        self.gate.close();
        self.channels.borrow_mut().clear();
        self.platform.lifecycle(HostLifecycle::Closing);
    }
    pub fn activate_uri(&self, uri: &str) -> Result<(), String> {
        if self.closed.get() {
            return Err("The phone host is closed.".into());
        }
        match activation::parse(uri)? {
            activation::Activation::Home => {
                phone_system::home(&self.vm);
                Ok(())
            }
            activation::Activation::App { id, path } => {
                if phone_system::launch_app(&self.vm, &id, &path) {
                    Ok(())
                } else {
                    Err("The activation target is unavailable or disabled.".into())
                }
            }
        }
    }
}
impl Drop for PhoneRuntime {
    fn drop(&mut self) {
        if let Some(subscription) = self.policy_subscription.get_mut().take() {
            subscription.dispose();
        }
        self.gate.close();
        if !self.closed.replace(true) {
            self.platform.lifecycle(HostLifecycle::Closing);
        }
    }
}
pub fn merge_environment(current: &mut HostEnvironment, observed: HostEnvironment) {
    current.platform = observed.platform;
    macro_rules! optional{($($field:ident),*)=>{$(if observed.$field.is_some(){current.$field=observed.$field;})*};}
    optional!(
        battery_percent,
        charging,
        external_power,
        power_saver,
        online,
        storage_usage_bytes,
        storage_quota_bytes,
        storage_persisted,
        persistent_storage_supported,
        media_session_supported
    );
    if observed.notification_permission != HostPermission::Unknown {
        current.notification_permission = observed.notification_permission;
    }
    current.normalize();
}
