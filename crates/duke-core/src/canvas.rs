use crate::colour::{add_colours, blend_colours, Colour};
use crate::{CANVAS_HEIGHT, CANVAS_WIDTH};

const BLACK: Colour = 0;
const PIXEL_CENTRE_OFFSET: f32 = 0.5;

#[derive(Clone)]
pub struct Canvas {
    pub width: i32,
    pub height: i32,
    pub pixels: Vec<Colour>,
}

impl Canvas {
    pub fn new(width: i32, height: i32) -> Self {
        let width = width.max(0);
        let height = height.max(0);
        Canvas { width, height, pixels: vec![BLACK; (width * height) as usize] }
    }

    pub fn full_size() -> Self {
        Self::new(CANVAS_WIDTH, CANVAS_HEIGHT)
    }

    #[inline]
    pub fn in_bounds(&self, x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && x < self.width && y < self.height
    }

    pub fn clear(&mut self, colour: Colour) {
        self.pixels.fill(colour);
    }

    #[inline]
    pub fn set_pixel(&mut self, x: i32, y: i32, colour: Colour) {
        if self.in_bounds(x, y) {
            self.pixels[(y * self.width + x) as usize] = colour;
        }
    }

    #[inline]
    pub fn pixel_at(&self, x: i32, y: i32) -> Colour {
        if self.in_bounds(x, y) {
            self.pixels[(y * self.width + x) as usize]
        } else {
            BLACK
        }
    }

    #[inline]
    pub fn blend_pixel(&mut self, x: i32, y: i32, colour: Colour, opacity: f32) {
        if self.in_bounds(x, y) {
            let index = (y * self.width + x) as usize;
            self.pixels[index] = blend_colours(self.pixels[index], colour, opacity);
        }
    }

    #[inline]
    pub fn add_glow(&mut self, x: i32, y: i32, colour: Colour, amount: f32) {
        if self.in_bounds(x, y) {
            let index = (y * self.width + x) as usize;
            self.pixels[index] = add_colours(self.pixels[index], colour, amount);
        }
    }

    pub fn horizontal_line(&mut self, first_x: i32, last_x: i32, y: i32, colour: Colour) {
        if y < 0 || y >= self.height {
            return;
        }
        let left = first_x.min(last_x).max(0);
        let right = first_x.max(last_x).min(self.width - 1);
        if left > right {
            return;
        }
        let row_start = (y * self.width) as usize;
        self.pixels[row_start + left as usize..=row_start + right as usize].fill(colour);
    }

    pub fn fill_rectangle(&mut self, left: i32, top: i32, width: i32, height: i32, colour: Colour) {
        let right = left.saturating_add(width).saturating_sub(1);
        let bottom_exclusive = top.saturating_add(height).min(self.height);
        for y in top.max(0)..bottom_exclusive {
            self.horizontal_line(left, right, y, colour);
        }
    }

    pub fn draw_line(&mut self, start_x: i32, start_y: i32, end_x: i32, end_y: i32, colour: Colour) {
        let horizontal_distance = (end_x as i64 - start_x as i64).abs();
        let negative_vertical_distance = -(end_y as i64 - start_y as i64).abs();
        let step_x = if start_x < end_x { 1 } else { -1 };
        let step_y = if start_y < end_y { 1 } else { -1 };
        let mut error = horizontal_distance + negative_vertical_distance;
        let (mut x, mut y) = (start_x, start_y);
        loop {
            self.set_pixel(x, y, colour);
            if x == end_x && y == end_y {
                break;
            }
            let doubled_error = 2 * error;
            if doubled_error >= negative_vertical_distance {
                error += negative_vertical_distance;
                x += step_x;
            }
            if doubled_error <= horizontal_distance {
                error += horizontal_distance;
                y += step_y;
            }
        }
    }

    pub fn fill_ellipse(&mut self, centre_x: i32, centre_y: i32, radius_x: i32, radius_y: i32, colour: Colour) {
        if radius_x <= 0 || radius_y <= 0 {
            self.set_pixel(centre_x, centre_y, colour);
            return;
        }
        for offset_y in -radius_y..=radius_y {
            let remaining_width_squared =
                1.0 - (offset_y as f32 * offset_y as f32) / (radius_y as f32 * radius_y as f32);
            let half_width = (radius_x as f32 * remaining_width_squared.max(0.0).sqrt()).round() as i32;
            self.horizontal_line(centre_x - half_width, centre_x + half_width, centre_y + offset_y, colour);
        }
    }

    pub fn fill_polygon(&mut self, corners: &[(i32, i32)], colour: Colour) {
        if corners.len() < 3 {
            return;
        }
        let top = corners.iter().map(|corner| corner.1).min().unwrap().max(0);
        let bottom = corners.iter().map(|corner| corner.1).max().unwrap().min(self.height - 1);
        let mut crossings: Vec<i32> = Vec::with_capacity(8);
        for y in top..=bottom {
            crossings.clear();
            let scanline_y = y as f32 + PIXEL_CENTRE_OFFSET;
            for (corner_index, &(start_x, start_y)) in corners.iter().enumerate() {
                let (end_x, end_y) = corners[(corner_index + 1) % corners.len()];
                let (start_y, end_y) = (start_y as f32, end_y as f32);
                let edge_crosses_scanline =
                    (start_y <= scanline_y && end_y > scanline_y) || (end_y <= scanline_y && start_y > scanline_y);
                if edge_crosses_scanline {
                    let fraction_along_edge = (scanline_y - start_y) / (end_y - start_y);
                    crossings.push((start_x as f32 + fraction_along_edge * (end_x - start_x) as f32).round() as i32);
                }
            }
            crossings.sort_unstable();
            for span in crossings.chunks_exact(2) {
                self.horizontal_line(span[0], span[1], y, colour);
            }
        }
    }

    pub fn draw_canvas(&mut self, source: &Canvas, left: i32, top: i32, transparent_colour: Option<Colour>) {
        for y in 0..source.height {
            for x in 0..source.width {
                let colour = source.pixels[(y * source.width + x) as usize];
                if Some(colour) != transparent_colour {
                    self.set_pixel(left + x, top + y, colour);
                }
            }
        }
    }
}
