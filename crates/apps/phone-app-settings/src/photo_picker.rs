//! Bounded native photo picker; large Camera Rolls retain only visible thumbnails.
use ferroui_base::{
    layout::{HorizontalAlignment as H, VerticalAlignment as V},
    BoxedValue,
};
use ferroui_controls::ItemsSource;
use ferroui_view::prelude::*;
use phone_domain::Photo;
use std::{cell::RefCell, rc::Rc};

struct Rows {
    photos: Vec<Photo>,
    source: ItemsSource,
}
impl Rows {
    fn new(photos: Vec<Photo>) -> Self {
        let source = ItemsSource::from_items(
            photos
                .iter()
                .cloned()
                .map(|photo| Some(Rc::new(photo) as BoxedValue)),
        );
        Self { photos, source }
    }
}

pub(super) fn view(photos: Vec<Photo>, height: f64, choose: Rc<dyn Fn(String)>) -> Element {
    rerender(move || {
        let cached = use_hook(|| Rc::new(RefCell::new(Rows::new(photos.clone()))));
        let source = {
            let mut cached = cached.borrow_mut();
            if cached.photos != photos {
                *cached = Rows::new(photos.clone());
            }
            cached.source.clone()
        };
        let template = use_hook({
            let choose = choose.clone();
            move || {
                template(move |photo: Photo| {
                    let select = choose.clone();
                    let source = photo.source.clone();
                    rsx! {Button{class:"phone-flat",horizontal_alignment:H::Stretch,horizontal_content_alignment:H::Stretch,
                        automation_properties_name:format!("Choose background photo {}",photo.name),padding:(0.0,4.0),min_height:78.0,
                        on_click:move |_|select(source.clone()),
                        Grid {column_definitions:"70,*",column_spacing:14.0,
                            Border {width:70.0,height:70.0,clip_to_bounds:true,{phone_sdk::images::thumbnail(&photo.source)}}
                            TextBlock {Grid.column:1,text:photo.name,font_size:23.0,vertical_alignment:V::Center,text_wrapping:ferroui_base::media::TextWrapping::Wrap}
                        }
                    }}
                })
            }
        });
        rsx! {ListBox{class:"phone-list",height:height.clamp(220.0,430.0),items_source:source,item_template:template}}
    })
}
