use ferroui_base::layout::HorizontalAlignment as H;
use ferroui_view::prelude::*;
use phone_controls::flat;
use phone_sdk::{
    common::{empty, muted, page, row, scroll, text},
    device,
    host::platform::*,
    session::Session,
};
use phone_state::PhoneViewModel;
use std::rc::Rc;

pub fn active(vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Element {
    let mut content = stack_panel().margin(20.0).spacing(20.0);
    let can_control = vm
        .device_state()
        .snapshot
        .supports(DeviceOperation::CallControl);
    for call in vm
        .device_state()
        .calls
        .into_iter()
        .filter(|v| v.state != CallState::Disconnected)
    {
        let name = if call.name.is_empty() {
            call.number.clone()
        } else {
            call.name.clone()
        };
        let status = match call.state {
            CallState::Ringing => "incoming call",
            CallState::Dialing => "dialing",
            CallState::Connecting => "connecting",
            CallState::Active => "connected",
            CallState::Holding => "on hold",
            CallState::Disconnected => "ended",
        };
        let elapsed = call.connected_at.map(|value| {
            ui.state::<crate::PhoneSession>()
                .clock
                .get()
                .saturating_sub(value.div_euclid(1000))
                .max(0)
        });
        let mut actions = stack_panel().spacing(12.0);
        let options = if !can_control {
            Vec::new()
        } else if call.state == CallState::Ringing {
            vec![
                ("answer", CallCommand::Answer),
                ("reject", CallCommand::Reject),
            ]
        } else {
            let mut options = vec![
                ("end call", CallCommand::End),
                (
                    if call.muted { "unmute" } else { "mute" },
                    CallCommand::SetMuted { muted: !call.muted },
                ),
                (
                    if call.speaker {
                        "use earpiece"
                    } else {
                        "speaker"
                    },
                    CallCommand::SetSpeaker {
                        enabled: !call.speaker,
                    },
                ),
            ];
            if call.can_hold {
                options.push((
                    if call.state == CallState::Holding {
                        "resume"
                    } else {
                        "hold"
                    },
                    if call.state == CallState::Holding {
                        CallCommand::Resume
                    } else {
                        CallCommand::Hold
                    },
                ));
            }
            options
        };
        for (label, action) in options {
            let model = vm.clone();
            let id = call.id.clone();
            actions = actions.child(flat(label, move || {
                model.request_device(DeviceRequest::CallAction {
                    id: id.clone(),
                    action: action.clone(),
                })
            }));
        }
        if can_control && call.state == CallState::Active {
            let mut tones = grid()
                .column_definitions("*,*,*")
                .row_definitions("48,48,48,48")
                .row_spacing(5.0)
                .column_spacing(5.0);
            for (index, tone) in ["1", "2", "3", "4", "5", "6", "7", "8", "9", "*", "0", "#"]
                .into_iter()
                .enumerate()
            {
                let model = vm.clone();
                let id = call.id.clone();
                tones=tones.child(rsx!{Button{Grid.row:(index/3)as i32,Grid.column:(index%3)as i32,class:"phone-button",content:tone,font_size:25.0,
                    automation_properties_name:format!("Call keypad {tone}"),on_click:move |_|model.request_device(DeviceRequest::CallAction{id:id.clone(),action:CallCommand::Dtmf{tone:tone.into()}})}});
            }
            actions = actions.child(tones);
        }
        content=content.child(rsx!{StackPanel{spacing:12.0,name:"ActiveDeviceCall",
            {muted(status,20.0)}{text(&name,40.0)}if !call.number.is_empty()&&call.number!=name{{muted(&call.number,22.0)}}
            if let Some(seconds)=elapsed{{muted(&format!("{}:{:02}",seconds/60,seconds%60),20.0)}}{actions}
        }});
    }
    page("phone", scroll(content.into()), Element::Empty)
}
pub fn history(vm: Rc<PhoneViewModel>) -> Element {
    let state = vm.device_state();
    let refresh = vm.clone();
    let mut body = stack_panel().spacing(12.0);
    if !state.snapshot.supports(DeviceOperation::CallHistory) {
        body = body.child(device::permission(
            vm.clone(),
            DevicePermission::CallLog,
            "call history",
        ));
        body = body.child(device::role(vm.clone(), DeviceRole::Dialer));
    } else {
        if state.call_history.is_empty() {
            body = body.child(empty(
                "No device calls",
                "Use refresh to read your device's recent call history.",
            ));
        }
        for call in state.call_history {
            let model = vm.clone();
            let number = call.number.clone();
            let name = call.name.clone();
            let direction = match call.direction {
                CallDirection::Incoming => "incoming",
                CallDirection::Outgoing => "outgoing",
                CallDirection::Missed => "missed",
                CallDirection::Rejected => "rejected",
                CallDirection::Blocked => "blocked",
                CallDirection::Voicemail => "voicemail",
                CallDirection::Unknown => "call",
            };
            body = body.child(row(
                if call.name.is_empty() {
                    &call.number
                } else {
                    &call.name
                },
                &format!(
                    "{direction} · {} · {}:{:02}",
                    device::timestamp(&vm, call.timestamp),
                    call.duration / 60,
                    call.duration % 60
                ),
                move || model.record_call(name.clone(), number.clone()),
            ));
        }
    }
    body.child(flat("refresh call history", move || {
        refresh.refresh_device();
        refresh.request_device(DeviceRequest::CallHistory { limit: 200 });
    }))
    .into()
}
pub fn accounts(vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Element {
    let accounts = vm.device_state().snapshot.phone_accounts;
    if accounts.len() < 2 {
        return Element::Empty;
    }
    let selected = ui.state::<crate::PhoneSession>().account;
    let mut rows = stack_panel().spacing(4.0).child(muted("call using", 16.0));
    let default = selected;
    rows = rows.child(flat("system default", move || default.set(None)));
    for account in accounts {
        let current = selected.get().as_deref() == Some(&account.id);
        rows=rows.child(rsx!{Button{class:"phone-flat",horizontal_alignment:H::Left,content:format!("{}{}",if current{"✓ "}else{""},account.label),
        on_click:move |_|selected.set(Some(account.id.clone()))}});
    }
    rows.into()
}
pub fn refresh(vm: &PhoneViewModel) {
    if !vm.platform_state().supports(HostFeature::Telephony) {
        return;
    }
    vm.refresh_device();
    let snapshot = vm.device_state().snapshot;
    if snapshot.supports(DeviceOperation::ActiveCalls) {
        vm.request_device(DeviceRequest::Calls);
    }
    if snapshot.supports(DeviceOperation::CallHistory) {
        vm.request_device(DeviceRequest::CallHistory { limit: 200 });
    }
}
