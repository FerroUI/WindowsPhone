use ferroui_view::prelude::*;
use phone_controls::{action, app_bar, command, Icon};
use phone_domain::AppId;
use phone_sdk::{common::*, session::Session, AppManifest, AppStorage};
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
    let description=app.builtin.map(crate::description::description)
        .unwrap_or("This independently registered app is included in your phone build. Its data is stored in its own app namespace and included in phone backups.");
    let mut body=stack_panel().margin(20.0).spacing(24.0)
        .child(rsx!{Grid{column_definitions:"84,*",column_spacing:18.0,
            {crate::square(&vm,&app,84.0)}
            StackPanel{Grid.column:1,spacing:7.0,{text(app.title,31.0)}{muted("included with this phone",16.0)}}
        }})
        .child(primary).child(text("overview",38.0)).child(phone_controls::paragraph(description))
        .child(muted(&format!("version {}",app.version),16.0))
        .child(muted("This collection lists apps included in this build. The retired Windows Phone Store is not connected.",16.0));
    if app.builtin.is_none() {
        body = body
            .child(text("app information", 28.0))
            .child(muted(app.id, 15.0));
        let services = if app.capabilities.is_empty() {
            "none".into()
        } else {
            app.capabilities
                .iter()
                .map(|service| format!("{service:?}").to_lowercase())
                .collect::<Vec<_>>()
                .join(", ")
        };
        body = body.child(muted(
            &format!("declared device services: {services}"),
            15.0,
        ));
        let bytes = vm
            .app_data(app.id)
            .and_then(|value| serde_json::to_vec(&value).ok())
            .map_or(0, |bytes| bytes.len());
        body = body.child(muted(&format!("saved app data: {bytes} bytes"), 15.0));
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
                    "Disable {}? Its Start tiles will be removed. Saved app data will be kept so you can enable it again.",
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
