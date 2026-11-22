//! Native browser chrome. Cross-origin web content stays in the host surface;
//! tabs, address-entry journals, favorites and history remain phone state.
mod chrome;
mod collections;
mod navigation;
mod session;

use crate::{common::*, session::Session};
use ferroui_base::layout::{HorizontalAlignment as H, VerticalAlignment as V};
use ferroui_view::prelude::*;
pub use navigation::{browser_back, browser_navigated};
use phone_controls::{Icon, icon};
use phone_state::PhoneViewModel;
pub use session::BrowserSession;
use session::BrowserView;
use std::rc::Rc;

pub fn view(vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Element {
    let url = vm.data().browser.url().to_owned();
    let view = ui.browser.view.get();
    let content = match view {
        BrowserView::Tabs => collections::tabs(vm.clone(), ui.clone()),
        BrowserView::Favorites => collections::favorites(vm.clone(), ui.clone()),
        BrowserView::History => collections::history(vm.clone(), ui.clone()),
        BrowserView::Settings => collections::settings(vm.clone(), ui.clone()),
        BrowserView::Page if url.is_empty() => {
            scroll(rsx! {StackPanel{margin:(20.0,14.0),spacing:18.0,
                TextBlock{text:"frequent",font_size:48.0,font_weight:ferroui_base::media::FontWeight::Light}
                {collections::frequent(vm.clone(),ui.clone())}
            }})
        }
        BrowserView::Page => {
            rsx! {Border{name:"BrowserViewport",background:"White",min_height:100.0,
                StackPanel{horizontal_alignment:H::Center,vertical_alignment:V::Center,spacing:14.0,
                    {icon(Icon::Explorer,56.0,"#777777")}
                    TextBlock{text:phone_domain::page_title(&url),font_size:20.0,foreground:"#555555"}
                }
            }}
        }
    };
    let attach_vm = vm.clone();
    let attach_ui = ui.clone();
    rsx! {Grid{name:"InternetExplorer",row_definitions:"*,Auto",clip_to_bounds:true,
        on_loaded:move|_|navigation::restore(&attach_vm,&attach_ui),
        Border{clip_to_bounds:true,{content}}
        Border{Grid.row:1,{chrome::view(vm,ui)}}
    }}
}
