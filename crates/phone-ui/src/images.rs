mod cache;
mod info;

use cache::{BUDGET, Cache, ImageState, Key, Lease, Loader};
use ferroui_base::media::{
    IImage, Stretch,
    imaging::{Bitmap, BitmapInterpolationMode},
};
use ferroui_view::prelude::*;
use std::{cell::RefCell, io::Cursor, rc::Rc};

const FULL_EDGE: u32 = 1200;
const THUMBNAIL_EDGE: u32 = 320;

ferroui_base::ferro_thread_local! {
    static CACHE: RefCell<Cache> = RefCell::new(Cache::default());
}

pub fn initialize() {
    // The host installs its loader before create_view. Keep that callback intact.
    CACHE.with(|cache| cache.borrow_mut().revision = Some(signal(0)));
}

pub fn set_loader(loader: Loader) {
    let revision = CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        cache.loader = Some(loader);
        cache.revision
    });
    notify(revision);
}

pub fn failed(source: String) {
    let revision = CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        cache.fail(&source);
        cache.revision
    });
    notify(revision);
}

pub fn forget(source: &str) {
    let revision = CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        cache.forget(source);
        cache.revision
    });
    notify(revision);
}

pub fn clear() {
    let revision = CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        cache.clear();
        cache.revision
    });
    notify(revision);
}

/// Complete a requested source on the UI thread, decoding only currently requested sizes.
pub fn register(source: String, bytes: Vec<u8>) -> Result<(), String> {
    let keys = CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        if cache.forgotten.contains(&source) {
            return Vec::new();
        }
        let pending = cache.pending.remove(&source);
        let mut keys = cache.requested_keys(&source);
        // A view may disappear before its IDB request completes. Avoid decoding unused data.
        // Explicit registration before a view mounts still supports the original public API.
        if keys.is_empty() && !pending {
            keys.push(Key::new(&source, FULL_EDGE));
        }
        keys
    });
    if keys.is_empty() {
        return Ok(());
    }
    let mut decoded = Vec::with_capacity(keys.len());
    for key in keys {
        match decode(&bytes, key.edge) {
            Ok((bitmap, size)) => decoded.push((key, Rc::new(bitmap) as Rc<dyn IImage>, size)),
            Err(error) => {
                failed(source);
                return Err(error);
            }
        }
    }
    let revision = CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        cache.failed.remove(&source);
        for (key, image, bytes) in decoded {
            cache.insert(key, image, bytes);
        }
        cache.trim();
        cache.revision
    });
    notify(revision);
    Ok(())
}

fn decode(bytes: &[u8], edge: u32) -> Result<(Bitmap, usize), String> {
    let size = info::dimensions(bytes)?;
    let mut stream = Cursor::new(bytes);
    let bitmap = if size.height > size.width {
        Bitmap::decode_to_height(
            &mut stream,
            size.height.min(edge) as i32,
            BitmapInterpolationMode::HighQuality,
        )
    } else {
        Bitmap::decode_to_width(
            &mut stream,
            size.width.min(edge) as i32,
            BitmapInterpolationMode::HighQuality,
        )
    }
    .map_err(|error| error.to_string())?;
    let pixels = bitmap.pixel_size();
    let bytes = (pixels.width as usize)
        .checked_mul(pixels.height as usize)
        .and_then(|size| size.checked_mul(4))
        .filter(|&size| size > 0 && size <= BUDGET)
        .ok_or("Decoded image exceeds the image cache budget.")?;
    Ok((bitmap, bytes))
}

pub fn photo(source: &str) -> Element {
    view(source, FULL_EDGE, Stretch::UniformToFill)
}
pub fn photo_fit(source: &str) -> Element {
    view(source, FULL_EDGE, Stretch::Uniform)
}
pub fn thumbnail(source: &str) -> Element {
    view(source, THUMBNAIL_EDGE, Stretch::UniformToFill)
}

fn view(source: &str, edge: u32, stretch: Stretch) -> Element {
    let key = Key::new(source, edge);
    let lease: Lease = Rc::new(RefCell::new(ImageState::Waiting));
    CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        cache
            .leases
            .entry(key.clone())
            .or_default()
            .push(Rc::downgrade(&lease));
    });
    let cleanup_key = key.clone();
    let cleanup_lease = Rc::downgrade(&lease);
    on_cleanup(move || {
        CACHE.with(|cache| {
            let mut cache = cache.borrow_mut();
            if let Some(leases) = cache.leases.get_mut(&cleanup_key) {
                leases.retain(|weak| weak.strong_count() > 0 && !weak.ptr_eq(&cleanup_lease));
                if leases.is_empty() {
                    cache.leases.remove(&cleanup_key);
                }
            }
            cache.trim();
        });
    });
    rerender(move || {
        let (source, revision, loader) = CACHE.with(|cache| {
            let mut cache = cache.borrow_mut();
            if !cache.images.contains_key(&key)
                && !cache.failed.contains(&key.source)
                && !cache.forgotten.contains(&key.source)
            {
                if let Some(bytes) = builtin(&key.source) {
                    match decode(bytes, key.edge) {
                        Ok((bitmap, bytes)) => cache.insert(key.clone(), Rc::new(bitmap), bytes),
                        Err(_) => cache.fail(&key.source),
                    }
                    cache.trim();
                }
            }
            if let Some(image) = cache.get(&key) {
                *lease.borrow_mut() = ImageState::Ready(image);
            }
            let source = match &*lease.borrow() {
                ImageState::Ready(image) => Some(image.clone()),
                _ => None,
            };
            let blocked =
                cache.failed.contains(&key.source) || cache.forgotten.contains(&key.source);
            let loader = if source.is_none()
                && !blocked
                && cache.loader.is_some()
                && cache.pending.insert(key.source.clone())
            {
                cache.loader.clone()
            } else {
                None
            };
            (source, cache.revision, loader)
        });
        if let Some(revision) = revision {
            revision.get();
        }
        if let Some(loader) = loader {
            loader(key.source.clone());
        }
        if let Some(source) = source {
            rsx! {Image{source:Some(source),stretch:stretch,is_hit_test_visible:false}}
        } else {
            rsx! {Border{background:"#25353F",{phone_controls::icon(phone_controls::Icon::Photos,42.0,"#8BA0AC")}}}
        }
    })
}

fn notify(revision: Option<Signal<u64>>) {
    if let Some(revision) = revision {
        if revision.is_alive() {
            revision.update(|value| *value = value.wrapping_add(1));
        }
    }
}

fn builtin(source: &str) -> Option<&'static [u8]> {
    Some(match source {
        "mountains" => {
            include_bytes!("../../../FerroUI/samples/WinUIGallery/Assets/SampleMedia/rainier.jpg")
        }
        "coast" => {
            include_bytes!("../../../FerroUI/samples/WinUIGallery/Assets/SampleMedia/sunset.jpg")
        }
        "forest" => {
            include_bytes!("../../../FerroUI/samples/WinUIGallery/Assets/SampleMedia/valley.jpg")
        }
        "city" => {
            include_bytes!("../../../FerroUI/samples/WinUIGallery/Assets/SampleMedia/cliff.jpg")
        }
        _ => return None,
    })
}
