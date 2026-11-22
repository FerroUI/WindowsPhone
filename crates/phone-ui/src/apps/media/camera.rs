use crate::{common::*, session::Session};
use ferroui_base::layout::{HorizontalAlignment as H, VerticalAlignment as V};
use ferroui_view::prelude::*;
use phone_controls::{action, app_bar, icon, Icon};
use phone_domain::{AppId, Route};
use phone_state::{HostRequest, PhoneViewModel};
use std::rc::Rc;

pub fn view(vm: Rc<PhoneViewModel>, _ui: Rc<Session>) -> Element {
    let start = vm.clone();
    let capture = vm.clone();
    let stop = vm.clone();
    let photos = vm.clone();
    let body = rsx! {Grid{background:"#090909",row_definitions:"*,Auto",
        Border{name:"CameraViewport",background:"#090909",margin:(0.0,0.0,0.0,0.0),
            StackPanel{spacing:22.0,horizontal_alignment:H::Center,vertical_alignment:V::Center,
                {icon(Icon::Camera,66.0,"#666666")}
                {phone_controls::command("turn on camera",move||start.request(HostRequest::StartCamera))}
            }
        }
        Border{Grid.row:1,padding:(20.0,12.0),{muted("Keep your moments.",16.0)}}
    }};
    page(
        "camera",
        body,
        app_bar(
            vec![
                action(Icon::Photos, "camera roll", move || {
                    photos.navigate(Route::App(AppId::Photos))
                }),
                action(Icon::Camera, "take photo", move || {
                    capture.request(HostRequest::CapturePhoto)
                }),
                action(Icon::Close, "close camera", move || {
                    stop.request(HostRequest::StopCamera)
                }),
            ],
            false,
            vec![],
        ),
    )
}
