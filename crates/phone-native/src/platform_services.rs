use phone_host::{platform::*, HostRequest};
use phone_runtime::PhoneRuntime;
use std::rc::Rc;
/// Use a real OS provider when offered; absence preserves the existing native
/// desktop service. A provider error never falls through to another mechanism.
pub(super) fn handle(runtime: &Rc<PhoneRuntime>, request: &HostRequest) -> Result<bool, String> {
    let state = runtime.platform().state();
    let platform = match request {
        HostRequest::Share { title, text, url }
            if !url.starts_with("media:") && state.supports(HostFeature::NativeSharing) =>
        {
            Some(PlatformRequest::ShareText {
                title: title.clone(),
                text: text.clone(),
                url: url.clone(),
            })
        }
        HostRequest::RequestNotificationPermission
            if state.supports(HostFeature::Notifications) =>
        {
            Some(PlatformRequest::RequestNotifications)
        }
        HostRequest::Notify { title, body } if state.supports(HostFeature::Notifications) => {
            Some(PlatformRequest::Notify {
                title: title.clone(),
                body: body.clone(),
            })
        }
        HostRequest::Locate | HostRequest::FetchWeather
            if state.supports(HostFeature::Location) =>
        {
            let weather = matches!(request, HostRequest::FetchWeather);
            let weak = Rc::downgrade(runtime);
            runtime.dispatch_platform(
                PlatformRequest::Locate,
                Some(Box::new(move |vm, response| {
                    let PlatformResponse::Location {
                        latitude,
                        longitude,
                    } = response
                    else {
                        return Err("The location provider returned an unexpected response.".into());
                    };
                    let position = phone_host::native::location::Position {
                        latitude,
                        longitude,
                        accuracy_metres: 0.0,
                    }
                    .validate()?;
                    if weather {
                        let epoch = vm.document_epoch();
                        let model = Rc::downgrade(vm);
                        let owner = weak.clone();
                        drop(
                            ferroui_base::threading::Dispatcher::current_dispatcher().spawn_local(
                                async move {
                                    let result = crate::background::run_system(move || {
                                        phone_host::native::weather::current(position)
                                    })
                                    .await;
                                    if owner.upgrade().is_none_or(|runtime| runtime.is_closed()) {
                                        return;
                                    }
                                    if let Some(vm) =
                                        model.upgrade().filter(|vm| vm.document_epoch() == epoch)
                                    {
                                        match result {
                                            Ok(reading) => phone_ui::set_weather(
                                                "current location".into(),
                                                reading.temperature_celsius,
                                                reading.condition.into(),
                                            ),
                                            Err(error) => vm.set_status(error),
                                        }
                                    }
                                },
                            ),
                        );
                    } else if let Some(runtime) = weak.upgrade() {
                        runtime.request(HostRequest::OpenUrl {
                            url: position.map_url()?,
                        })?;
                    }
                    Ok(())
                })),
            )?;
            return Ok(true);
        }
        HostRequest::RefreshEnvironment => {
            runtime.observe();
            if matches!(
                state.platform,
                phone_host::HostPlatform::Android | phone_host::HostPlatform::Ios
            ) {
                runtime
                    .vm()
                    .set_status("Device information refreshed.".into());
                return Ok(true);
            }
            return Ok(false);
        }
        _ => None,
    };
    if let Some(request) = platform {
        runtime.dispatch_platform(request, None)?;
        Ok(true)
    } else {
        Ok(false)
    }
}
