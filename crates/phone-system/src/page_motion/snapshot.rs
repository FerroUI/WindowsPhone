//! Outgoing images are short-lived render targets, never retained app owners.
use crate::previews::Preview;
use ferroui_base::platform::IPlatformRenderInterface;
use ferroui_base::{FerroLocator, LocatorExtensions, PixelSize, Ref, Size, Visual};
use ferroui_controls::{Control, TopLevel};

const MAX_WIDTH: i32 = 1536;
const MAX_HEIGHT: i32 = 2048;
const MAX_PIXELS: i32 = 2_097_152; // At most 8 MiB for an RGBA target.

pub(super) fn capture(page: &Ref<Control>) -> Option<Preview> {
    if !crate::previews::capture_enabled()
        || !page.is_attached_to_visual_tree()
        || !page.is_effectively_visible()
        || !page.is_measure_valid()
        || !page.is_arrange_valid()
        || page.render_transform().is_some()
        || page.opacity() < 1.0
    {
        return None;
    }
    let visual = page.clone().upcast::<Visual>();
    let top = TopLevel::get_top_level(Some(&visual))?;
    if top.layout_manager().is_in_layout_pass()
        || FerroLocator::current()
            .get_service::<dyn IPlatformRenderInterface>()
            .is_none()
    {
        return None;
    }
    let size = page.bounds().size();
    let (pixels, scale) = capture_size(size, top.render_scaling())?;
    Some(Preview {
        image: crate::previews::bitmap::capture(&visual, size, pixels, scale),
        logical_size: size,
    })
}

fn capture_size(size: Size, density: f64) -> Option<(PixelSize, f64)> {
    if !size.width.is_finite()
        || !size.height.is_finite()
        || size.width <= 0.0
        || size.height <= 0.0
        || !density.is_finite()
        || density <= 0.0
    {
        return None;
    }
    let scale = density
        .min(2.0)
        .min(MAX_WIDTH as f64 / size.width)
        .min(MAX_HEIGHT as f64 / size.height)
        .min((MAX_PIXELS as f64 / (size.width * size.height)).sqrt());
    if !scale.is_finite() || scale <= 0.0 {
        return None;
    }
    let width = (size.width * scale).ceil().clamp(1.0, MAX_WIDTH as f64) as i32;
    let height = (size.height * scale)
        .ceil()
        .clamp(1.0, MAX_HEIGHT.min(MAX_PIXELS / width) as f64) as i32;
    let scale = scale
        .min(width as f64 / size.width)
        .min(height as f64 / size.height);
    Some((PixelSize::new(width, height), scale))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_targets_preserve_normal_density_and_bound_untrusted_viewports() {
        for (size, density) in [
            (Size::new(320.0, 660.0), 2.0),
            (Size::new(400.0, 740.0), 1.0),
            (Size::new(10000.0, 10000.0), 8.0),
        ] {
            let (pixels, scale) = capture_size(size, density).unwrap();
            assert!(pixels.width <= MAX_WIDTH && pixels.height <= MAX_HEIGHT);
            assert!(pixels.width * pixels.height <= MAX_PIXELS);
            assert!(size.width * scale <= pixels.width as f64);
            assert!(size.height * scale <= pixels.height as f64);
            if size.width < 500.0 {
                assert_eq!(scale, density);
            }
        }
        for size in [
            Size::new(0.0, 100.0),
            Size::new(f64::INFINITY, 10.0),
            Size::new(100.0, f64::NAN),
        ] {
            assert!(capture_size(size, 1.0).is_none());
        }
        assert!(capture_size(Size::new(320.0, 660.0), f64::NAN).is_none());
    }
}
