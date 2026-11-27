//! Pure layout and wrap arithmetic, independent of controls and animation.

pub(super) const PREVIEW: f64 = crate::reference_pixels(24.0);
pub(super) const TITLE_HEIGHT: f64 = crate::reference_pixels(174.0);
pub(super) const TITLE_FONT_SIZE: f64 = crate::reference_pixels(170.0);
pub(super) const TITLE_TOP: f64 = -crate::reference_pixels(34.0);
const TITLE_RATE: f64 = 0.16;
const BACKGROUND_RATE: f64 = 0.40;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Geometry {
    pub width: f64,
    pub inset: f64,
    pub item_extent: f64,
    pub viewport_fraction: f64,
    pub background_extent: f64,
}

impl Geometry {
    pub fn new(width: f64, count: usize) -> Self {
        let width = if width.is_finite() {
            width.max(1.0)
        } else {
            1.0
        };
        // Section headings use the standard content inset; leave the documented
        // preview beyond that inset, rather than revealing only empty padding.
        let inset = if count > 1 {
            (PREVIEW + crate::PAGE_MARGIN).min(width * 0.25)
        } else {
            0.0
        };
        let item_extent = width - inset;
        let viewport_fraction = item_extent / (width + inset);
        let background_extent = (item_extent * count as f64 * BACKGROUND_RATE).max(width);
        Self {
            width,
            inset,
            item_extent,
            viewport_fraction,
            background_extent,
        }
    }

    /// One title instance leaves the viewport before crossing the wrap seam;
    /// it reenters on the other side. No duplicate title is exposed to input or
    /// accessibility. Between ordinary sections it travels slower than art.
    pub fn title_x(self, offset: f64, count: usize, title_width: f64) -> f64 {
        if count < 2 || !offset.is_finite() {
            return 0.0;
        }
        let phase = offset.rem_euclid(count as f64);
        let last = count as f64 - 1.0;
        let travel = self.item_extent * TITLE_RATE;
        if phase <= last {
            return -phase * travel;
        }
        let wrap = phase - last;
        if wrap < 0.5 {
            let start = -last * travel;
            let end = -title_width.max(self.width) - crate::PAGE_MARGIN;
            start + (end - start) * (wrap * 2.0)
        } else {
            (self.width + crate::PAGE_MARGIN) * (2.0 - wrap * 2.0)
        }
    }

    pub fn background_x(self, offset: f64, count: usize) -> f64 {
        if count < 2 || !offset.is_finite() {
            return 0.0;
        }
        -(offset * self.background_extent / count as f64).rem_euclid(self.background_extent)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_starts_at_zero_and_next_header_has_a_twenty_unit_preview() {
        for width in [320.0, 400.0, 480.0] {
            let g = Geometry::new(width, 3);
            let native_width = width + g.inset;
            let item = native_width * g.viewport_fraction;
            let centered_inset = (native_width - item) * 0.5;
            assert!((centered_inset - g.inset).abs() < 1e-9);
            assert!((width - item - crate::PAGE_MARGIN - PREVIEW).abs() < 1e-9);
        }
    }

    #[test]
    fn title_and_background_are_cyclic_and_have_distinct_rates() {
        let g = Geometry::new(400.0, 4);
        let title = g.title_x(0.5, 4, 620.0);
        let art = g.background_x(0.5, 4);
        assert!(title.abs() < art.abs());
        assert!(art.abs() < g.item_extent * 0.5);
        assert_eq!(g.title_x(0.0, 4, 620.0), 0.0);
        assert_eq!(g.title_x(4.0, 4, 620.0), 0.0);
        assert!((g.title_x(-0.2, 4, 620.0) - g.title_x(3.8, 4, 620.0)).abs() < 1e-9);
        assert!((g.background_x(-0.2, 4) - g.background_x(3.8, 4)).abs() < 1e-9);
        assert!(g.title_x(3.4999, 4, 620.0) + 620.0 + crate::PAGE_MARGIN < 0.2);
        assert!(g.title_x(3.5, 4, 620.0) >= 400.0);
    }

    #[test]
    fn empty_single_and_non_finite_input_have_bounded_layout() {
        for width in [0.0, -20.0, f64::INFINITY, f64::NAN] {
            let g = Geometry::new(width, 0);
            assert!(g.width.is_finite() && g.width > 0.0);
            assert_eq!(g.viewport_fraction, 1.0);
            assert_eq!(g.title_x(f64::NAN, 0, 0.0), 0.0);
            assert_eq!(g.background_x(f64::INFINITY, 0), 0.0);
        }
        let g = Geometry::new(400.0, 1);
        assert_eq!(g.inset, 0.0);
        assert_eq!(g.item_extent, 400.0);
    }
}
