use ferroui_view::prelude::*;
use phone_controls::Icon;
use phone_sdk::{session::Session, AppManifest, AppRegistration, AppRegistry};
use std::{cell::RefCell, rc::Rc};

#[test]
fn activation_wakes_an_already_active_view_without_a_route_change() {
    let registry = Rc::new(
        AppRegistry::new(vec![AppRegistration::new(
            AppManifest::new("tests.activation", "Activation", Icon::Note),
            |_| Element::Empty,
        )])
        .unwrap(),
    );
    let session = Rc::new(Session::new(0, registry));
    let received = Rc::new(RefCell::new(Vec::new()));
    let render = Owner::new_root();
    render.with(|| {
        let session = session.clone();
        let received = received.clone();
        effect(move || {
            if let Some(payload) = session.take_activation("tests.activation") {
                received.borrow_mut().push(payload);
            }
        });
    });
    session
        .queue_activation("tests.activation", serde_json::json!({"note": 1}))
        .unwrap();
    session
        .queue_activation("tests.activation", serde_json::json!({"note": 2}))
        .unwrap();
    assert_eq!(
        *received.borrow(),
        vec![
            serde_json::json!({"note": 1}),
            serde_json::json!({"note": 2})
        ]
    );
    assert!(session.take_activation("tests.activation").is_none());
    assert!(session
        .queue_activation("tests.missing", serde_json::Value::Null)
        .is_err());
    assert_eq!(received.borrow().len(), 2);
    render.dispose();
}
