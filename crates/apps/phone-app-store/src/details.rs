use ferroui_view::prelude::*;
use phone_controls::{Icon, action, app_bar, command};
use phone_domain::AppId;
use phone_sdk::{AppCategory, AppManifest, AppStorage, common::*, session::Session};
use phone_state::PhoneViewModel;
use std::rc::Rc;

pub(crate) fn view(vm: Rc<PhoneViewModel>, ui: Rc<Session>, app: AppManifest) -> Element {
    let installed = crate::enabled(&vm, &app);
    let model = vm.clone();
    let open_ui = ui.clone();
    let primary = command(
        if installed {
            "open"
        } else if app.builtin.is_some() {
            "install"
        } else {
            "enable"
        },
        move || {
            if let Some(id) = app.builtin {
                if installed {
                    open_ui.open_app(&model, id);
                } else {
                    model.install(id);
                }
            } else if installed {
                model.resume_extension(app.id);
            } else if let Err(error) = model.set_extension_enabled(app.id, true) {
                model.set_status(error);
            }
        },
    );
    let description = app.builtin.map(crate::description::description).unwrap_or(
        if app.category == AppCategory::Game {
            "Your progress is saved on this device and included in phone backups."
        } else {
            "Your data is saved on this device and included in phone backups."
        },
    );
    let mut body=stack_panel().margin(20.0).spacing(24.0)
        .child(rsx!{Grid{column_definitions:"84,*",column_spacing:18.0,
            {crate::square(&vm,&app,84.0)}
            StackPanel{Grid.column:1,spacing:7.0,{text(app.title,31.0)}{muted("included with this phone",16.0)}}
        }})
        .child(primary).child(text("overview",38.0)).child(phone_controls::paragraph(description))
        .child(muted(&format!("version {}",app.version),16.0))
        .child(muted("Manage the apps on this phone. The original Windows Phone Store is not connected.",16.0));
    if app.builtin.is_none() {
        body = body.child(text("app information", 28.0));
        let services = if app.capabilities.is_empty() {
            "none".into()
        } else {
            app.capabilities
                .iter()
                .map(crate::description::device_feature)
                .collect::<Vec<_>>()
                .join(", ")
        };
        body = body.child(muted(&format!("requested device access: {services}"), 15.0));
        let bytes = vm
            .app_data(app.id)
            .and_then(|value| serde_json::to_vec(&value).ok())
            .map_or(0, |bytes| bytes.len());
        body = body.child(muted(&format!("saved data: {bytes} bytes"), 15.0));
        if bytes > 0 {
            body = body.child(reset_data(vm.clone(), ui.clone(), app));
        }
    }
    let removable = app.builtin.is_none_or(|id| {
        !matches!(
            id,
            AppId::Phone | AppId::Messaging | AppId::People | AppId::Settings | AppId::Store
        )
    });
    if installed && removable {
        body = body.child(remove(vm.clone(), ui.clone(), app));
    }
    let mut actions = Vec::new();
    if installed {
        let model = vm.clone();
        actions.push(action(Icon::Pin, "pin to start", move || {
            if let Some(id) = app.builtin {
                model.pin(id);
            } else if let Err(error) = model.pin_app(app.id, "", app.title) {
                model.set_status(error);
            }
        }));
    }
    page(
        "store",
        scroll(body.into()),
        app_bar(actions, false, vec![]),
    )
}

fn remove(vm: Rc<PhoneViewModel>, ui: Rc<Session>, app: AppManifest) -> Element {
    command(
        if app.builtin.is_some() {
            "uninstall"
        } else {
            "disable"
        },
        move || {
            let model = vm.clone();
            let state = ui.clone();
            let prompt = if app.builtin.is_some() {
                format!("Uninstall {}?", app.title)
            } else {
                format!(
                    "Disable {}? Its Start tiles will be removed. Your saved data will be kept so you can pick up where you left off.",
                    app.title
                )
            };
            ui.confirm(
                &prompt,
                Rc::new(move || {
                    if let Some(id) = app.builtin {
                        model.uninstall(id);
                        state.registry().close(&model, &state, app.id);
                    } else {
                        match model.set_extension_enabled(app.id, false) {
                            Ok(()) => state.registry().close(&model, &state, app.id),
                            Err(error) => model.set_status(error),
                        }
                    }
                    state.dialog.set(None);
                }),
            );
        },
    )
}

fn reset_data(vm: Rc<PhoneViewModel>, ui: Rc<Session>, app: AppManifest) -> Element {
    command("reset app data", move || {
        let model = vm.clone();
        let state = ui.clone();
        ui.confirm(
            &format!(
                "Delete all saved data for {}? This cannot be undone without a phone backup.",
                app.title
            ),
            Rc::new(move || {
                match AppStorage::new(model.clone(), app.id).clear() {
                    Ok(()) => {
                        state.registry().reset_app(&model, &state, app.id);
                        model.set_status("App data was reset.".into());
                    }
                    Err(error) => model.set_status(error),
                }
                state.dialog.set(None);
            }),
        );
    })
}
