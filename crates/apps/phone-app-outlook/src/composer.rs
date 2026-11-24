use crate::drafts::{self, Draft, Fields};
use ferroui_base::media::TextWrapping;
use ferroui_view::prelude::*;
use phone_controls::{action, app_bar, Icon};
use phone_sdk::{
    common::{muted, page, scroll, text, title},
    session::Session,
};
use phone_state::PhoneViewModel;
use std::{cell::RefCell, rc::Rc};

pub struct State {
    pub active: Signal<bool>,
    id: Signal<Option<u32>>,
    to: Signal<String>,
    subject: Signal<String>,
    body: Signal<String>,
    error: Signal<String>,
    notice: Signal<String>,
    saved: RefCell<Fields>,
    feedback_fields: RefCell<Fields>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            active: signal(false),
            id: signal(None),
            to: signal(String::new()),
            subject: signal(String::new()),
            body: signal(String::new()),
            error: signal(String::new()),
            notice: signal(String::new()),
            saved: RefCell::new(Fields::default()),
            feedback_fields: RefCell::new(Fields::default()),
        }
    }
}

impl State {
    fn fields(&self) -> Fields {
        Fields {
            to: self.to.get_untracked(),
            subject: self.subject.get_untracked(),
            body: self.body.get_untracked(),
        }
    }

    fn dirty(&self) -> bool {
        self.fields() != *self.saved.borrow()
    }

    pub fn close(&self) {
        self.active.set(false);
        self.id.set(None);
        self.to.set(String::new());
        self.subject.set(String::new());
        self.body.set(String::new());
        self.error.set(String::new());
        self.notice.set(String::new());
        *self.saved.borrow_mut() = Fields::default();
        *self.feedback_fields.borrow_mut() = Fields::default();
    }
}

pub fn begin(ui: &Rc<Session>, fields: Fields, draft: Option<Draft>) {
    let state = ui.state::<State>();
    ui.menu.set(false);
    ui.state::<crate::MailState>().reset();
    state.id.set(draft.as_ref().map(|draft| draft.id));
    *state.saved.borrow_mut() = draft.map(|draft| draft.fields).unwrap_or_default();
    state.to.set(fields.to);
    state.subject.set(fields.subject);
    state.body.set(fields.body);
    state.error.set(String::new());
    state.notice.set(String::new());
    state.active.set(true);
}

fn save(vm: &Rc<PhoneViewModel>, ui: &Rc<Session>) {
    let state = ui.state::<State>();
    let fields = state.fields();
    let result = drafts::save(vm, state.id.get_untracked(), fields.clone());
    *state.feedback_fields.borrow_mut() = fields;
    match result {
        Ok(draft) => {
            state.id.set(Some(draft.id));
            *state.saved.borrow_mut() = draft.fields;
            state.error.set(String::new());
            state.notice.set("Draft saved on this phone.".into());
        }
        Err(error) => {
            state.notice.set(String::new());
            state.error.set(error);
        }
    }
}

fn open_email(vm: &Rc<PhoneViewModel>, ui: &Rc<Session>) {
    let state = ui.state::<State>();
    let fields = state.fields();
    *state.feedback_fields.borrow_mut() = fields.clone();
    match fields.email_request() {
        Ok(request) => {
            state.error.set(String::new());
            // HostRequest::Email is a handoff, with no delivery acknowledgement.
            // Keep both the local draft and the current editor until user action.
            state
                .notice
                .set("Requested your email app. Finish sending there.".into());
            vm.request(request);
        }
        Err(error) => {
            state.notice.set(String::new());
            state.error.set(error);
        }
    }
}

pub fn back(ui: &Rc<Session>) -> bool {
    let state = ui.state::<State>();
    if !state.active.get_untracked() {
        return false;
    }
    if state.dirty() {
        let close = ui.clone();
        ui.form_with_action(
            "discard unsaved changes?",
            vec![],
            "discard",
            Rc::new(move |_| {
                close.state::<State>().close();
            }),
        );
    } else {
        state.close();
    }
    true
}

fn discard(vm: &Rc<PhoneViewModel>, ui: &Rc<Session>) {
    let state = ui.state::<State>();
    if let Some(id) = state.id.get_untracked() {
        let model = vm.clone();
        let close = ui.clone();
        ui.form_with_action(
            "delete this draft?",
            vec![],
            "delete",
            Rc::new(move |_| match drafts::delete(&model, id) {
                Ok(()) => close.state::<State>().close(),
                Err(error) => close.form_error.set(error),
            }),
        );
    } else {
        back(ui);
    }
}

pub fn view(vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Element {
    let state = ui.state::<State>();
    let error = state.error.get();
    let notice = state.notice.get();
    // Derive feedback visibility from the completed two-way model update.
    // A TextChanged handler that mutates render state could reconcile an old
    // model value before the native property's binding has published its edit.
    let feedback_current = *state.feedback_fields.borrow()
        == Fields {
            to: state.to.get(),
            subject: state.subject.get(),
            body: state.body.get(),
        };
    let save_vm = vm.clone();
    let save_ui = ui.clone();
    let email_vm = vm.clone();
    let email_ui = ui.clone();
    let delete_vm = vm.clone();
    let delete_ui = ui.clone();
    let heading = if state.id.get().is_some() {
        "draft"
    } else {
        "new email"
    };
    page(
        "Outlook",
        scroll(rsx! {StackPanel {
            {title(heading)}
            StackPanel {margin:(20.0,0.0,20.0,18.0),spacing:8.0,
                {muted("Save drafts on this phone. Use your email app to send.",16.0)}
                {text("to",18.0)}
                TextBox {name:"MailComposeTo",class:"phone-input",text:state.to.two_way(),placeholder_text:"email address",automation_properties_name:"Email recipients"}
                {text("subject",18.0)}
                TextBox {name:"MailComposeSubject",class:"phone-input",text:state.subject.two_way(),placeholder_text:"subject",automation_properties_name:"Email subject"}
                {text("message",18.0)}
                TextBox {name:"MailComposeBody",class:"phone-input",text:state.body.two_way(),accepts_return:true,text_wrapping:TextWrapping::Wrap,min_height:220.0,placeholder_text:"type a message",automation_properties_name:"Email message"}
                if feedback_current && !error.is_empty() {{text(&error,17.0)}}
                if feedback_current && !notice.is_empty() {{muted(&notice,17.0)}}
            }
        }}),
        app_bar(
            vec![
                action(Icon::Mail, "open email", move || {
                    open_email(&email_vm, &email_ui)
                }),
                action(Icon::Save, "save draft", move || save(&save_vm, &save_ui)),
                action(
                    Icon::Delete,
                    if state.id.get().is_some() {
                        "delete draft"
                    } else {
                        "discard"
                    },
                    move || discard(&delete_vm, &delete_ui),
                ),
            ],
            ui.menu.get(),
            vec![],
        ),
    )
}

#[cfg(test)]
mod tests;
