use ferroui_base::media::{imaging::Bitmap, IImage, Stretch};
use ferroui_view::prelude::*;
use std::{cell::RefCell, collections::HashMap, io::Cursor, rc::Rc};
ferroui_base::ferro_thread_local! {
    static CACHE: RefCell<HashMap<String,Rc<dyn IImage>>> = RefCell::new(HashMap::new());
    static REVISION: RefCell<Option<Signal<u64>>> = const {RefCell::new(None)};
}
pub fn initialize() {
    REVISION.with(|r| *r.borrow_mut() = Some(signal(0)));
}
pub fn register(key: String, bytes: Vec<u8>) -> Result<(), String> {
    if bytes.len() > 16 * 1024 * 1024 {
        return Err("Image exceeds 16 MB.".into());
    }
    let bitmap = Bitmap::decode_to_width(
        &mut Cursor::new(bytes),
        1200,
        ferroui_base::media::imaging::BitmapInterpolationMode::HighQuality,
    )
    .map_err(|e| e.to_string())?;
    CACHE.with(|c| {
        c.borrow_mut().insert(key, Rc::new(bitmap));
    });
    REVISION.with(|r| {
        if let Some(rev) = *r.borrow() {
            rev.update(|n| *n += 1)
        }
    });
    Ok(())
}
fn source(key: &str) -> Option<Rc<dyn IImage>> {
    CACHE.with(|cache| {
        if let Some(image) = cache.borrow().get(key) {
            return Some(image.clone());
        }
        let bytes: &[u8] = match key {
            "mountains" => include_bytes!(
                "../../../FerroUI/samples/WinUIGallery/Assets/SampleMedia/rainier.jpg"
            ),
            "coast" => include_bytes!(
                "../../../FerroUI/samples/WinUIGallery/Assets/SampleMedia/sunset.jpg"
            ),
            "forest" => include_bytes!(
                "../../../FerroUI/samples/WinUIGallery/Assets/SampleMedia/valley.jpg"
            ),
            "city" => {
                include_bytes!("../../../FerroUI/samples/WinUIGallery/Assets/SampleMedia/cliff.jpg")
            }
            _ => return None,
        };
        let image: Rc<dyn IImage> = Rc::new(Bitmap::from_stream(&mut Cursor::new(bytes)).ok()?);
        cache.borrow_mut().insert(key.into(), image.clone());
        Some(image)
    })
}
pub fn photo(key: &str) -> Element {
    REVISION.with(|r| {
        if let Some(rev) = *r.borrow() {
            rev.get();
        }
    });
    if let Some(image) = source(key) {
        rsx! {Image{source:Some(image),stretch:Stretch::UniformToFill,is_hit_test_visible:false}}
    } else {
        rsx! {Border{background:"#25353F",{phone_controls::icon(phone_controls::Icon::Photos,42.0,"#8BA0AC")}}}
    }
}
pub fn photo_fit(key: &str) -> Element {
    REVISION.with(|r| {
        if let Some(rev) = *r.borrow() {
            rev.get();
        }
    });
    if let Some(image) = source(key) {
        rsx! {Image{source:Some(image),stretch:Stretch::Uniform,is_hit_test_visible:false}}
    } else {
        photo(key)
    }
}
