//! All transparent tiles sample one viewport-relative image plane. Scrolling
//! changes a shared native transform, never the reactive tree or phone data.
use crate::{images, session::Session};
use ferroui_controls::ScrollViewer;
use ferroui_view::prelude::*;

pub(super) const TOP_INSET: f64 = 18.0;
const PAN_FRACTION: f64 = 0.25;

pub(super) fn view(
    ui: &Session,
    tile_id: phone_domain::Id,
    source: &str,
    width: f64,
    viewport: f64,
    tile_x: f64,
    tile_y: f64,
) -> Element {
    rsx! {Canvas{is_hit_test_visible:false,
        Border{name:format!("StartWallpaper-{tile_id}"),Canvas.left:-tile_x,Canvas.top:-tile_y-TOP_INSET,
            width:width,height:plane_height(viewport),
            render_transform:Some(ui.start_wallpaper.clone().into()),
            {images::photo(source)}
        }
    }}
}

pub(super) fn update(ui: &Session, scroll: &ScrollViewer, motion: bool) {
    let y = translation(
        scroll.offset().y,
        scroll.extent().height,
        scroll.viewport().height,
        motion,
    );
    if (ui.start_wallpaper.y() - y).abs() > 0.001 {
        ui.start_wallpaper.set_y(y);
    }
}

fn plane_height(viewport: f64) -> f64 {
    finite_positive(viewport) * (1.0 + PAN_FRACTION)
}

/// The image travels at most a quarter viewport from the first to last row.
/// This bounds cropping even with hundreds of rows. With decorative motion
/// disabled, the image remains fixed on screen as its tile windows scroll.
fn translation(offset: f64, extent: f64, viewport: f64, motion: bool) -> f64 {
    let viewport = finite_positive(viewport);
    let maximum = (finite_positive(extent) - viewport).max(0.0);
    let offset = if offset.is_finite() {
        offset.clamp(0.0, maximum)
    } else {
        0.0
    };
    let travel = if motion {
        viewport.min(maximum) * PAN_FRACTION
    } else {
        0.0
    };
    offset
        - if maximum > 0.0 {
            travel * offset / maximum
        } else {
            0.0
        }
}

fn finite_positive(value: f64) -> f64 {
    if value.is_finite() {
        value.max(1.0)
    } else {
        1.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn visible_tiles_never_sample_beyond_the_shared_image() {
        for viewport in [200.0_f64, 460.0, 740.0, 1600.0] {
            for extent in [100.0, 950.0, 1600.0, 30_000.0] {
                let maximum = (extent - viewport).max(0.0);
                for step in 0..=20 {
                    let scroll = maximum * f64::from(step) / 20.0;
                    for motion in [false, true] {
                        let shift = translation(scroll, extent, viewport, motion);
                        // A visible screen y samples y + scroll - shift.
                        assert!(scroll - shift >= -0.001);
                        assert!(viewport + scroll - shift <= plane_height(viewport) + 0.001);
                    }
                }
            }
        }
    }

    #[test]
    fn sampling_is_continuous_and_reduced_motion_is_stationary() {
        let (scroll, extent, viewport) = (1234.0, 6000.0, 740.0);
        let shift = translation(scroll, extent, viewport, true);
        for tile_y in [0.0, 190.0, 975.0, 3000.0] {
            let tile_screen_y = TOP_INSET + tile_y - scroll;
            let image_local_y = -tile_y - TOP_INSET + shift;
            assert!((tile_screen_y + image_local_y - (shift - scroll)).abs() < 0.001);
        }
        assert_eq!(translation(scroll, extent, viewport, false), scroll);
        assert!(translation(5000.0, 6000.0, viewport, true).is_finite());
        assert_eq!(translation(f64::NAN, f64::INFINITY, 0.0, true), 0.0);
    }
}
