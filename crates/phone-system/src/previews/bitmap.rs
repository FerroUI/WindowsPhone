use ferroui_base::media::{
    imaging::{IBitmap, RenderTargetBitmap},
    DrawingContext, IImage,
};
use ferroui_base::{PixelSize, Rect, Ref, Size, Vector, Visual};
use std::{any::Any, rc::Rc};

/// Image controls work in logical units; the drawing backend samples bitmap
/// source rectangles in pixels. Keep that conversion explicit for our reduced
/// DPI render targets rather than forwarding logical coordinates as pixels.
struct PageImage {
    bitmap: RenderTargetBitmap,
    logical_size: Size,
    scale: f64,
}

pub(crate) fn capture(
    visual: &Ref<Visual>,
    logical_size: Size,
    pixels: PixelSize,
    scale: f64,
) -> Rc<dyn IImage> {
    let bitmap = RenderTargetBitmap::with_dpi(pixels, Vector::new(96.0 * scale, 96.0 * scale));
    bitmap.render(visual);
    Rc::new(PageImage {
        bitmap,
        logical_size,
        scale,
    })
}

impl IImage for PageImage {
    fn size(&self) -> Size {
        self.logical_size
    }
    fn draw(&self, context: &mut DrawingContext<'_>, source: Rect, destination: Rect) {
        let pixels = Rect::new(
            source.x * self.scale,
            source.y * self.scale,
            source.width * self.scale,
            source.height * self.scale,
        );
        context.draw_bitmap(
            IBitmap::platform_impl(&self.bitmap),
            1.0,
            pixels,
            destination,
        );
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_bitmap(&self) -> Option<&dyn IBitmap> {
        Some(&self.bitmap)
    }
}
