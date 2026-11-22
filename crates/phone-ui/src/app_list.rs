use crate::{common, session::Session};
use ferroui_base::layout::{HorizontalAlignment as H, VerticalAlignment as V};
use ferroui_base::media::FontWeight;
use ferroui_controls::primitives::ScrollBarVisibility;
use ferroui_view::prelude::*;
use phone_controls::{Icon, icon};
use phone_domain::AppId;
use phone_state::PhoneViewModel;
use std::{collections::BTreeMap, rc::Rc};

pub fn view(vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Element {
    let query = ui.query.get();
    let apps = vm
        .data()
        .installed
        .iter()
        .copied()
        .filter(|app| app.label().to_lowercase().contains(&query.to_lowercase()))
        .collect::<Vec<_>>();
    let mut groups: BTreeMap<char, Vec<AppId>> = BTreeMap::new();
    for app in apps {
        groups.entry(app.initial()).or_default().push(app);
    }
    let mut rows = stack_panel().spacing(0.0).margin((14.0, 0.0, 12.0, 24.0));
    for (letter, mut apps) in groups.clone() {
        apps.sort_by_key(|a| a.label());
        let alphabet = ui.clone();
        rows=rows.child(rsx!{Button{name:format!("AppGroup-{letter}"),class:"phone-flat",width:42.0,height:43.0,horizontal_alignment:H::Left,margin:(0.0,10.0,0.0,8.0),automation_properties_name:format!("Jump to letter {letter}"),on_click:move |_|alphabet.alphabet.set(true),
            Border{width:32.0,height:32.0,border_brush:common::accent(&vm).as_str(),border_thickness:2.0,
                TextBlock{text:letter.to_string(),font_size:23.0,foreground:common::accent(&vm).as_str(),horizontal_alignment:H::Center,vertical_alignment:V::Center}}
        }});
        for app in apps {
            let open_vm = vm.clone();
            let open_ui = ui.clone();
            let pin_vm = vm.clone();
            let pin_ui = ui.clone();
            let label = app.label();
            rows=rows.child(rsx!{Button{name:format!("App-{}",app.icon()),class:"phone-flat",height:59.0,horizontal_alignment:H::Stretch,horizontal_content_alignment:H::Stretch,
                automation_properties_name:label,on_click:move |_|open_ui.open_app(&open_vm,app),
                on_context_requested:move |event|{let vm=pin_vm.clone();pin_ui.confirm(&format!("pin {label} to start?"),Rc::new(move||{vm.pin(app);vm.home();}));event.set_handled(true);},
                Grid{column_definitions:"45,*",column_spacing:14.0,
                    {common::app_square(app,45.0,&common::accent(&vm))}
                    TextBlock{Grid.column:1,text:label,font_size:26.0,font_weight:FontWeight::Light,vertical_alignment:V::Center}
                }
            }});
        }
    }
    if groups.is_empty() {
        rows = rows.child(common::empty("no results", "Try a different app name."));
    }
    let scroll_node = ui.app_scroll;
    let clear = ui.clone();
    let home = vm.clone();
    let remember = ui.clone();
    let restore = ui.clone();
    let mut result=grid().row_definitions("Auto,*").child(rsx!{Grid{column_definitions:"48,*,40",margin:(10.0,12.0,14.0,8.0),
        Button{class:"phone-flat",automation_properties_name:"Back to Start",on_click:move |_|home.home(),{icon(Icon::Back,23.0,&common::fg(&vm))}},
        TextBox{Grid.column:1,name:"AppSearch",class:"phone-input",text:ui.query.two_way(),placeholder_text:"search",automation_properties_name:"Search apps",font_size:21.0,min_height:43.0},
        Button{Grid.column:2,class:"phone-flat",automation_properties_name:"Clear app search",on_click:move |_|clear.query.set(String::new()),{icon(Icon::Close,17.0,&common::fg(&vm))}}
    }}).child(rsx!{ScrollViewer{Grid.row:1,name:"AppsScroll",key:"apps-scroll",node_ref:scroll_node,horizontal_scroll_bar_visibility:ScrollBarVisibility::Disabled,vertical_scroll_bar_visibility:ScrollBarVisibility::Hidden,
        on_scroll_changed:move |event|if let Some(scroll)=event.source().and_then(|s|s.cast::<ScrollViewer>()){remember.scrolls.borrow_mut().insert("apps".into(),scroll.offset().y);},
        on_loaded:move |event|if let Some(scroll)=event.source().and_then(|s|s.cast::<ScrollViewer>()){let offset=restore.scrolls.borrow().get("apps").copied().unwrap_or(0.0);scroll.set_offset(ferroui_base::Vector::new(0.0,offset));},
        {rows}
    }});
    if ui.alphabet.get() {
        let mut alphabet = uniform_grid()
            .columns(4)
            .margin((20.0, 24.0))
            .horizontal_alignment(H::Stretch);
        for letter in "#abcdefghijklmnopqrstuvwxyz".chars() {
            let palette = common::accent(&vm);
            let enabled = groups.contains_key(&letter);
            let jump = ui.clone();
            let count_before = groups
                .iter()
                .take_while(|(group, _)| **group < letter)
                .map(|(_, apps)| 61.0 + apps.len() as f64 * 59.0)
                .sum::<f64>();
            alphabet=alphabet.child(rsx!{Button{class:"phone-flat",height:70.0,margin:5.0,is_enabled:enabled,automation_properties_name:format!("Jump to {letter}"),background:if enabled{palette.as_str()}else{"#333333"},horizontal_content_alignment:H::Center,
                on_click:move |_|{jump.alphabet.set(false);if let Some(scroll)=scroll_node.get_untracked(){scroll.set_offset(ferroui_base::Vector::new(0.0,count_before));}},
                TextBlock{text:letter.to_string(),font_size:37.0,font_weight:FontWeight::Light,foreground:if enabled{"White"}else{"#666666"}}
            }});
        }
        result=result.child(rsx!{Border{Grid.row_span:2,background:common::bg(&vm).as_str(),{common::scroll(alphabet.into())}}});
    }
    result.into()
}
