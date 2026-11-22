//! Live native control geometry for end-to-end input verification.
use ferroui_base::{input::InputElement, Point, Visual};
use ferroui_controls::{automation::AutomationProperties, Application, Button, TextBlock, TextBox};
use serde_json::json;
use wasm_bindgen::prelude::*;
#[wasm_bindgen(js_name=inspectView)]
pub fn inspect_view() -> String {
    let Some(root) = Application::current()
        .and_then(|a| a.application_lifetime())
        .and_then(|l| {
            l.as_single_top_level_application_lifetime()
                .and_then(|l| l.top_level())
        })
    else {
        return "null".into();
    };
    let viewport = root.bounds();
    let elements=root.get_visual_descendants().filter_map(|visual|{
        let name=visual.name();
        let text=if let Some(t)=visual.cast::<TextBlock>(){t.text()}else if let Some(t)=visual.cast::<TextBox>(){t.text()}else if visual.cast::<Button>().is_some()||name.is_some(){None}else{return None;};
        let b=visual.bounds();
        if !visual.is_effectively_visible()||b.width<=0.0||b.height<=0.0{return None;}
        let p=visual.translate_point(Point::new(0.0,0.0),&root)?;
        if p.x>=viewport.width||p.y>=viewport.height||p.x+b.width<=0.0||p.y+b.height<=0.0{return None;}
        let hit=root.input_hit_test_with(Point::new(p.x+b.width/2.0,p.y+b.height/2.0),false).is_some_and(|h|{let h:&Visual=&h;std::ptr::eq(h,&*visual)||visual.is_visual_ancestor_of(h)||h.is_visual_ancestor_of(&visual)});
        Some(json!({"type":visual.get_type().name(),"name":name,"text":text,"automationName":AutomationProperties::get_name(&visual),"focused":visual.cast::<InputElement>().is_some_and(|i|i.is_focused()),"x":p.x,"y":p.y,"width":b.width,"height":b.height,"hit":hit}))
    }).collect::<Vec<_>>();
    json!({"width":viewport.width,"height":viewport.height,"elements":elements}).to_string()
}
