//! Board symbols are geometry, independent of the host's font fallback.
use ferroui_base::{
    Ref,
    layout::{HorizontalAlignment as H, VerticalAlignment as V},
    media::Geometry,
};
use ferroui_view::prelude::*;

#[derive(Clone, Copy)]
pub enum Symbol {
    Flag,
    Mine,
}

ferroui_base::ferro_thread_local! {
    static GEOMETRY: [Ref<Geometry>; 2] = [
        Geometry::parse("M4,2 L6,2 L6,3 L21,3 L16,8 L21,13 L6,13 L6,21 L12,21 L12,23 L0,23 L0,21 L4,21 Z")
            .expect("embedded minefield flag"),
        Geometry::parse(concat!(
            "M11,0 L13,0 L13,5.1 A7,7 0 0 1 16.2,6.4 L19.5,3.1 L20.9,4.5 L17.6,7.8 ",
            "A7,7 0 0 1 18.9,11 L24,11 L24,13 L18.9,13 A7,7 0 0 1 17.6,16.2 ",
            "L20.9,19.5 L19.5,20.9 L16.2,17.6 A7,7 0 0 1 13,18.9 L13,24 L11,24 L11,18.9 ",
            "A7,7 0 0 1 7.8,17.6 L4.5,20.9 L3.1,19.5 L6.4,16.2 A7,7 0 0 1 5.1,13 ",
            "L0,13 L0,11 L5.1,11 A7,7 0 0 1 6.4,7.8 L3.1,4.5 L4.5,3.1 L7.8,6.4 A7,7 0 0 1 11,5.1 Z"
        )).expect("embedded minefield mine"),
    ];
}

pub fn view(symbol: Symbol, size: f64, color: &str) -> Element {
    let data = GEOMETRY.with(|geometry| geometry[symbol as usize].clone());
    rsx! {PathIcon {
        data:Some(data), width:size, height:size, foreground:color,
        horizontal_alignment:H::Center, vertical_alignment:V::Center,
        is_hit_test_visible:false, automation_properties_is_control_element_override:Some(false)
    }}
}
