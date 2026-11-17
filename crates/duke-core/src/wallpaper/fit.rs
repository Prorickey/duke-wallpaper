use crate::wallpaper::config::Fit;
use crate::{CANVAS_HEIGHT, CANVAS_WIDTH};

pub const LARGEST_SUPPORTED_TARGET_SIDE: i32 = 1 << 20;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Placement {
    pub device_pixels_per_art_pixel: i32,
    pub left: i32,
    pub top: i32,
    pub width: i32,
    pub height: i32,
}

pub struct VisibleCanvas {
    pub first_column: i32,
    pub column_after_last: i32,
    pub first_row: i32,
    pub row_after_last: i32,
}

pub fn place_canvas(fit: Fit, target_width: i32, target_height: i32) -> Placement {
    let target_width = target_width.clamp(1, LARGEST_SUPPORTED_TARGET_SIDE);
    let target_height = target_height.clamp(1, LARGEST_SUPPORTED_TARGET_SIDE);
    let whole_number_scale = match fit {
        Fit::Cover => {
            let scale_that_fills_the_width = (target_width + CANVAS_WIDTH - 1) / CANVAS_WIDTH;
            let scale_that_fills_the_height = (target_height + CANVAS_HEIGHT - 1) / CANVAS_HEIGHT;
            scale_that_fills_the_width.max(scale_that_fills_the_height)
        }
        Fit::Contain => {
            let largest_scale_that_fits_the_width = target_width / CANVAS_WIDTH;
            let largest_scale_that_fits_the_height = target_height / CANVAS_HEIGHT;
            largest_scale_that_fits_the_width.min(largest_scale_that_fits_the_height).max(1)
        }
    };
    let width = CANVAS_WIDTH * whole_number_scale;
    let height = CANVAS_HEIGHT * whole_number_scale;
    Placement {
        device_pixels_per_art_pixel: whole_number_scale,
        left: (target_width - width).div_euclid(2),
        top: (target_height - height).div_euclid(2),
        width,
        height,
    }
}

impl Placement {
    pub fn visible_canvas(&self, target_width: i32, target_height: i32) -> VisibleCanvas {
        let scale = self.device_pixels_per_art_pixel;
        let right_edge_in_art_pixels = (target_width - self.left + scale - 1).div_euclid(scale);
        let bottom_edge_in_art_pixels = (target_height - self.top + scale - 1).div_euclid(scale);
        VisibleCanvas {
            first_column: (-self.left).div_euclid(scale).max(0),
            column_after_last: right_edge_in_art_pixels.min(CANVAS_WIDTH),
            first_row: (-self.top).div_euclid(scale).max(0),
            row_after_last: bottom_edge_in_art_pixels.min(CANVAS_HEIGHT),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn placement(scale: i32, left: i32, top: i32) -> Placement {
        Placement {
            device_pixels_per_art_pixel: scale,
            left,
            top,
            width: CANVAS_WIDTH * scale,
            height: CANVAS_HEIGHT * scale,
        }
    }

    #[test]
    fn cover_is_exact_on_sixteen_by_nine_monitors() {
        assert_eq!(place_canvas(Fit::Cover, 1920, 1080), placement(3, 0, 0));
        assert_eq!(place_canvas(Fit::Cover, 2560, 1440), placement(4, 0, 0));
        assert_eq!(place_canvas(Fit::Cover, 3840, 2160), placement(6, 0, 0));
    }

    #[test]
    fn cover_crops_evenly_on_other_shapes() {
        assert_eq!(place_canvas(Fit::Cover, 1920, 1200), placement(4, -320, -120));
        assert_eq!(place_canvas(Fit::Cover, 3440, 1440), placement(6, -200, -360));
        assert_eq!(place_canvas(Fit::Cover, 1200, 1920), placement(6, -1320, -120));
        assert_eq!(place_canvas(Fit::Cover, 1366, 768), placement(3, -277, -156));
    }

    #[test]
    fn contain_letterboxes() {
        assert_eq!(place_canvas(Fit::Contain, 1920, 1200), placement(3, 0, 60));
        assert_eq!(place_canvas(Fit::Contain, 1366, 768), placement(2, 43, 24));
        assert_eq!(place_canvas(Fit::Contain, 320, 200).device_pixels_per_art_pixel, 1);
    }

    #[test]
    fn visible_canvas_is_the_part_inside_the_target() {
        let visible = place_canvas(Fit::Cover, 1920, 1200).visible_canvas(1920, 1200);
        assert_eq!(
            (visible.first_column, visible.column_after_last, visible.first_row, visible.row_after_last),
            (80, 560, 30, 330)
        );
        let visible = place_canvas(Fit::Contain, 1920, 1200).visible_canvas(1920, 1200);
        assert_eq!(
            (visible.first_column, visible.column_after_last, visible.first_row, visible.row_after_last),
            (0, CANVAS_WIDTH, 0, CANVAS_HEIGHT)
        );
        let partly_cut_art_pixels = place_canvas(Fit::Cover, 1366, 768);
        let visible = partly_cut_art_pixels.visible_canvas(1366, 768);
        assert!(visible.first_column * 3 + partly_cut_art_pixels.left <= 0);
        assert!(visible.column_after_last * 3 + partly_cut_art_pixels.left >= 1366);
        assert!(visible.first_row * 3 + partly_cut_art_pixels.top <= 0);
        assert!(visible.row_after_last * 3 + partly_cut_art_pixels.top >= 768);
    }

    #[test]
    fn extreme_and_odd_sizes_never_overflow() {
        let extreme_sizes =
            [(i32::MAX, i32::MAX), (i32::MAX, 1), (1, i32::MAX), (0, 0), (-5, -5), (i32::MIN, i32::MIN), (1, 1)];
        for fit in [Fit::Cover, Fit::Contain] {
            for (width, height) in extreme_sizes {
                let placed = place_canvas(fit, width, height);
                assert!(placed.device_pixels_per_art_pixel >= 1, "{fit:?} {width}x{height}: {placed:?}");
                assert_eq!(placed.width, CANVAS_WIDTH * placed.device_pixels_per_art_pixel);
                assert_eq!(placed.height, CANVAS_HEIGHT * placed.device_pixels_per_art_pixel);
            }
        }
        let odd_real_world_sizes = [
            (1366, 768),
            (1200, 1920),
            (1920, 1200),
            (3440, 1440),
            (5120, 1440),
            (1024, 768),
            (800, 1280),
            (2256, 1504),
            (641, 361),
        ];
        for (width, height) in odd_real_world_sizes {
            let covering = place_canvas(Fit::Cover, width, height);
            assert!(covering.left <= 0 && covering.top <= 0, "cover {width}x{height}: {covering:?}");
            assert!(covering.left + covering.width >= width && covering.top + covering.height >= height);
            assert!(covering.width - CANVAS_WIDTH < width || covering.height - CANVAS_HEIGHT < height);
            assert!(((width - covering.width) - 2 * covering.left).abs() <= 1);
            assert!(((height - covering.height) - 2 * covering.top).abs() <= 1);
            let contained = place_canvas(Fit::Contain, width, height);
            assert!(contained.left >= 0 && contained.top >= 0, "contain {width}x{height}: {contained:?}");
            assert!(contained.width <= width && contained.height <= height);
            assert!(contained.width + CANVAS_WIDTH > width || contained.height + CANVAS_HEIGHT > height);
        }
    }
}
