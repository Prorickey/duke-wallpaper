use crate::host::win32::top_down_bitmap_description;
use duke_core::wallpaper::fit::Placement;
use duke_core::{Canvas, CANVAS_HEIGHT, CANVAS_WIDTH};
use windows_sys::Win32::Foundation::RECT;
use windows_sys::Win32::Graphics::Gdi::{
    CreateSolidBrush, DeleteObject, FillRect, SetStretchBltMode, StretchDIBits, COLORONCOLOR, DIB_RGB_COLORS, HDC,
    SRCCOPY,
};

pub fn draw_canvas_scaled_by_whole_pixels(
    device_context: HDC,
    canvas: &Canvas,
    placement: &Placement,
    target_width: i32,
    target_height: i32,
    letterbox_colour: u32,
) -> bool {
    if canvas.pixels.len() != (CANVAS_WIDTH * CANVAS_HEIGHT) as usize {
        return false;
    }

    let image_left = placement.left.max(0);
    let image_top = placement.top.max(0);
    let image_right = (placement.left + placement.width).min(target_width);
    let image_bottom = (placement.top + placement.height).min(target_height);
    let bars_around_the_image = [
        RECT { left: 0, top: 0, right: target_width, bottom: image_top },
        RECT { left: 0, top: image_bottom, right: target_width, bottom: target_height },
        RECT { left: 0, top: image_top, right: image_left, bottom: image_bottom },
        RECT { left: image_right, top: image_top, right: target_width, bottom: image_bottom },
    ];
    let image_leaves_room_for_bars =
        image_left > 0 || image_top > 0 || image_right < target_width || image_bottom < target_height;
    if image_leaves_room_for_bars {
        let red = (letterbox_colour >> 16) & 0xFF;
        let green = (letterbox_colour >> 8) & 0xFF;
        let blue = letterbox_colour & 0xFF;
        let colour_in_gdi_byte_order = blue << 16 | green << 8 | red;
        unsafe {
            let brush = CreateSolidBrush(colour_in_gdi_byte_order);
            for bar in &bars_around_the_image {
                if bar.right > bar.left && bar.bottom > bar.top {
                    FillRect(device_context, bar, brush);
                }
            }
            DeleteObject(brush);
        }
    }

    let canvas_as_bitmap = top_down_bitmap_description(CANVAS_WIDTH, CANVAS_HEIGHT);
    let scan_lines_drawn = unsafe {
        SetStretchBltMode(device_context, COLORONCOLOR);
        StretchDIBits(
            device_context,
            placement.left,
            placement.top,
            placement.width,
            placement.height,
            0,
            0,
            CANVAS_WIDTH,
            CANVAS_HEIGHT,
            canvas.pixels.as_ptr().cast(),
            &canvas_as_bitmap,
            DIB_RGB_COLORS,
            SRCCOPY,
        )
    };
    scan_lines_drawn > 0
}
#[cfg(test)]
mod tests {
    use super::*;
    use duke_core::wallpaper::config::Fit;
    use duke_core::wallpaper::fit::place_canvas;
    use std::ptr::null_mut;
    use windows_sys::Win32::Graphics::Gdi::{
        CreateCompatibleDC, CreateDIBSection, DeleteDC, GdiFlush, GetDC, ReleaseDC, SelectObject,
    };

    const ONLY_RED_GREEN_BLUE: u32 = 0x00FF_FFFF;

    fn device_pixel_a_correct_blit_shows(
        canvas: &Canvas,
        placement: &Placement,
        letterbox_colour: u32,
        device_column: i32,
        device_row: i32,
    ) -> u32 {
        let column_within_image = device_column - placement.left;
        let row_within_image = device_row - placement.top;
        let is_outside_the_image = column_within_image < 0
            || row_within_image < 0
            || column_within_image >= placement.width
            || row_within_image >= placement.height;
        if is_outside_the_image {
            return letterbox_colour & ONLY_RED_GREEN_BLUE;
        }
        let art_column = column_within_image / placement.device_pixels_per_art_pixel;
        let art_row = row_within_image / placement.device_pixels_per_art_pixel;
        canvas.pixel_at(art_column, art_row) & ONLY_RED_GREEN_BLUE
    }

    fn test_pattern_where_every_art_pixel_differs_from_its_neighbours() -> Canvas {
        const GRID_LINE_SPACING: i32 = 40;
        const WHITE: u32 = 0x00FF_FFFF;
        let mut pattern = Canvas::full_size();
        for row in 0..CANVAS_HEIGHT {
            for column in 0..CANVAS_WIDTH {
                let red_rises_left_to_right = (column * 255 / (CANVAS_WIDTH - 1)) as u32;
                let green_rises_top_to_bottom = (row * 255 / (CANVAS_HEIGHT - 1)) as u32;
                let blue_checkerboard = if (column + row) % 2 == 0 { 32 } else { 224 };
                let is_on_a_grid_line = column % GRID_LINE_SPACING == 0 || row % GRID_LINE_SPACING == 0;
                let colour = if is_on_a_grid_line {
                    WHITE
                } else {
                    red_rises_left_to_right << 16 | green_rises_top_to_bottom << 8 | blue_checkerboard
                };
                pattern.set_pixel(column, row, colour);
            }
        }
        pattern
    }

    fn pixels_after_drawing_off_screen(
        canvas: &Canvas,
        placement: &Placement,
        width: i32,
        height: i32,
        letterbox_colour: u32,
    ) -> Vec<u32> {
        let surface_description = top_down_bitmap_description(width, height);
        unsafe {
            let screen = GetDC(null_mut());
            let memory_device_context = CreateCompatibleDC(screen);
            let mut surface_pixels: *mut core::ffi::c_void = null_mut();
            let surface =
                CreateDIBSection(screen, &surface_description, DIB_RGB_COLORS, &mut surface_pixels, null_mut(), 0);
            ReleaseDC(null_mut(), screen);
            assert!(!memory_device_context.is_null() && !surface.is_null() && !surface_pixels.is_null());
            let previously_selected = SelectObject(memory_device_context, surface);

            let drawn = draw_canvas_scaled_by_whole_pixels(
                memory_device_context,
                canvas,
                placement,
                width,
                height,
                letterbox_colour,
            );
            assert!(drawn, "GDI refused the blit");
            GdiFlush();
            let pixel_count = (width * height) as usize;
            let mut pixels = Vec::with_capacity(pixel_count);
            for &pixel in std::slice::from_raw_parts(surface_pixels as *const u32, pixel_count) {
                pixels.push(pixel & ONLY_RED_GREEN_BLUE);
            }

            SelectObject(memory_device_context, previously_selected);
            DeleteObject(surface);
            DeleteDC(memory_device_context);
            pixels
        }
    }

    #[test]
    fn every_device_pixel_shows_the_right_art_pixel_on_any_monitor_shape() {
        let canvas = test_pattern_where_every_art_pixel_differs_from_its_neighbours();
        let letterbox_colour = 0x0B0E14;
        let monitor_sizes = [
            (1920, 1080),
            (1920, 1200),
            (1200, 1920),
            (1366, 768),
            (2560, 1440),
            (3440, 1440),
            (800, 1280),
            (641, 361),
            (300, 200),
        ];
        for (width, height) in monitor_sizes {
            for fit in [Fit::Cover, Fit::Contain] {
                let placement = place_canvas(fit, width, height);
                let pixels = pixels_after_drawing_off_screen(&canvas, &placement, width, height, letterbox_colour);
                for device_row in 0..height {
                    for device_column in 0..width {
                        let shown = pixels[(device_row * width + device_column) as usize];
                        let correct = device_pixel_a_correct_blit_shows(
                            &canvas,
                            &placement,
                            letterbox_colour,
                            device_column,
                            device_row,
                        );
                        assert_eq!(
                            shown, correct,
                            "{fit:?} {width}x{height} at ({device_column},{device_row}), {placement:?}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn a_canvas_of_the_wrong_size_is_refused_instead_of_read_out_of_bounds() {
        let too_small = Canvas::new(10, 10);
        let placement = place_canvas(Fit::Cover, 100, 100);
        assert!(!draw_canvas_scaled_by_whole_pixels(null_mut(), &too_small, &placement, 100, 100, 0));
    }
}
