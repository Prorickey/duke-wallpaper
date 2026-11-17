use crate::atmosphere::lightning::BoltSegment;
use crate::{hash_fraction, hash_position};
use crate::{Canvas, Colour, Environment};

pub(crate) const FULL_BLEND_WEIGHT: u32 = 256;
pub(crate) const STANDARD_CANVAS_AREA: f32 = 640.0 * 360.0;
const RED_AND_BLUE_CHANNELS: u32 = 0x00FF_00FF;
const GREEN_CHANNEL: u32 = 0x0000_FF00;
const ONE_IN_FIXED_POINT: f32 = 65536.0;
const FIXED_POINT_SHIFT: u32 = 16;
const NOISE_PERIOD_IN_CELLS: i32 = 256;
const NOISE_OCTAVE_COUNT: u32 = 3;
const NOISE_OCTAVE_SEED_STEP: u32 = 131;
const TIME_NOISE_PERIOD_IN_CELLS: i64 = 1 << 20;
const TIME_NOISE_HASH_ROW: i32 = 7;
const DITHER_TILE_PIXELS: i32 = 8;
const DITHER_TILE_MASK: i32 = DITHER_TILE_PIXELS - 1;
const FIELD_CELL_PIXELS: i32 = DITHER_TILE_PIXELS;
const LONGEST_FRAME_GAP_SECONDS: f64 = 2.0;
const RED_LUMA_WEIGHT: u32 = 77;
const GREEN_LUMA_WEIGHT: u32 = 150;
const BLUE_LUMA_WEIGHT: u32 = 29;

#[inline]
pub(crate) fn smoothstep(edge_start: f32, edge_end: f32, value: f32) -> f32 {
    let progress = ((value - edge_start) / (edge_end - edge_start)).clamp(0.0, 1.0);
    progress * progress * (3.0 - 2.0 * progress)
}

#[inline]
pub(crate) fn triangle_bump(value: f32, centre: f32, half_width: f32) -> f32 {
    (1.0 - ((value - centre).abs() / half_width)).clamp(0.0, 1.0)
}

#[inline]
pub(crate) fn hash_to_unit(first_index: u32, second_index: u32, seed: u32) -> f32 {
    hash_fraction(first_index as i32, second_index as i32, seed)
}

#[inline]
pub(crate) fn sine_wave(time: f64, cycles_per_second: f32, cycle_offset: f32) -> f32 {
    let cycle_position = (time * cycles_per_second as f64 + cycle_offset as f64).rem_euclid(1.0) as f32;
    (cycle_position * std::f32::consts::TAU).sin()
}

#[inline]
pub(crate) fn periodic_noise(noise_x: f32, noise_y: f32, seed: u32) -> f32 {
    let cell_x = noise_x.floor();
    let cell_y = noise_y.floor();
    let within_cell_x = noise_x - cell_x;
    let within_cell_y = noise_y - cell_y;
    let lattice_x = cell_x as i32;
    let lattice_y = cell_y as i32;
    let eased_x = within_cell_x * within_cell_x * (3.0 - 2.0 * within_cell_x);
    let eased_y = within_cell_y * within_cell_y * (3.0 - 2.0 * within_cell_y);
    let lattice_mask = NOISE_PERIOD_IN_CELLS - 1;
    let left = lattice_x & lattice_mask;
    let right = lattice_x.wrapping_add(1) & lattice_mask;
    let top = lattice_y & lattice_mask;
    let bottom = lattice_y.wrapping_add(1) & lattice_mask;
    let top_left = hash_fraction(left, top, seed);
    let top_right = hash_fraction(right, top, seed);
    let bottom_left = hash_fraction(left, bottom, seed);
    let bottom_right = hash_fraction(right, bottom, seed);
    let top_edge = top_left + (top_right - top_left) * eased_x;
    let bottom_edge = bottom_left + (bottom_right - bottom_left) * eased_x;
    top_edge + (bottom_edge - top_edge) * eased_y
}

#[inline]
pub(crate) fn periodic_fractal_noise(noise_x: f32, noise_y: f32, seed: u32) -> f32 {
    let mut weighted_sum = 0.0;
    let mut total_weight = 0.0;
    let mut octave_weight = 0.5;
    let mut octave_frequency = 1.0;
    for octave in 0..NOISE_OCTAVE_COUNT {
        let octave_seed = seed.wrapping_add(octave.wrapping_mul(NOISE_OCTAVE_SEED_STEP));
        weighted_sum +=
            octave_weight * periodic_noise(noise_x * octave_frequency, noise_y * octave_frequency, octave_seed);
        total_weight += octave_weight;
        octave_weight *= 0.5;
        octave_frequency *= 2.0;
    }
    weighted_sum / total_weight
}

#[inline]
pub(crate) fn wrap_noise_offset(offset_in_cells: f64) -> f32 {
    offset_in_cells.rem_euclid(NOISE_PERIOD_IN_CELLS as f64) as f32
}

pub(crate) fn time_noise(time_in_cells: f64, seed: u32) -> f32 {
    let whole_cells = time_in_cells.floor();
    let within_cell = (time_in_cells - whole_cells) as f32;
    let this_cell = (whole_cells as i64).rem_euclid(TIME_NOISE_PERIOD_IN_CELLS) as i32;
    let next_cell = (this_cell + 1) & (TIME_NOISE_PERIOD_IN_CELLS as i32 - 1);
    let eased = within_cell * within_cell * (3.0 - 2.0 * within_cell);
    let value_here = hash_fraction(this_cell, TIME_NOISE_HASH_ROW, seed);
    let value_next = hash_fraction(next_cell, TIME_NOISE_HASH_ROW, seed);
    value_here + (value_next - value_here) * eased
}

pub(crate) fn prevailing_wind_direction(environment: &Environment) -> f64 {
    let world_hash = hash_position(17, 4, environment.seed as u32);
    if world_hash & 1 == 0 {
        1.0
    } else {
        -1.0
    }
}

#[inline]
pub(crate) fn blend_weight(alpha: f32) -> u32 {
    (alpha.clamp(0.0, 1.0) * FULL_BLEND_WEIGHT as f32) as u32
}

#[derive(Copy, Clone, Default)]
pub(crate) struct PremultipliedColour {
    weighted_red_and_blue: u32,
    weighted_green: u32,
    weight_of_pixel_underneath: u32,
}

impl PremultipliedColour {
    pub(crate) fn new(colour: Colour, weight: u32) -> PremultipliedColour {
        let weight = weight.min(FULL_BLEND_WEIGHT);
        PremultipliedColour {
            weighted_red_and_blue: (colour & RED_AND_BLUE_CHANNELS) * weight,
            weighted_green: (colour & GREEN_CHANNEL) * weight,
            weight_of_pixel_underneath: FULL_BLEND_WEIGHT - weight,
        }
    }

    #[inline]
    pub(crate) fn over(&self, pixel: Colour) -> Colour {
        let red_and_blue_sum = (pixel & RED_AND_BLUE_CHANNELS)
            .wrapping_mul(self.weight_of_pixel_underneath)
            .wrapping_add(self.weighted_red_and_blue);
        let green_sum =
            (pixel & GREEN_CHANNEL).wrapping_mul(self.weight_of_pixel_underneath).wrapping_add(self.weighted_green);
        let red_and_blue = (red_and_blue_sum >> 8) & RED_AND_BLUE_CHANNELS;
        let green = (green_sum >> 8) & GREEN_CHANNEL;
        red_and_blue | green
    }
}

#[inline]
pub(crate) fn blend_pixels(pixel: Colour, colour: Colour, weight: u32) -> Colour {
    PremultipliedColour::new(colour, weight).over(pixel)
}

#[inline]
pub(crate) fn multiply_pixel(pixel: Colour, channel_factors: [u32; 3]) -> Colour {
    let red = ((pixel >> 16) & 0xFF).wrapping_mul(channel_factors[0]) >> 8;
    let green = ((pixel >> 8) & 0xFF).wrapping_mul(channel_factors[1]) >> 8;
    let blue = (pixel & 0xFF).wrapping_mul(channel_factors[2]) >> 8;
    (red << 16) | (green << 8) | blue
}

#[inline]
pub(crate) fn add_to_pixel(pixel: Colour, colour: Colour, weight: u32) -> Colour {
    let red = (((pixel >> 16) & 0xFF) + ((((colour >> 16) & 0xFF) * weight) >> 8)).min(255);
    let green = (((pixel >> 8) & 0xFF) + ((((colour >> 8) & 0xFF) * weight) >> 8)).min(255);
    let blue = ((pixel & 0xFF) + (((colour & 0xFF) * weight) >> 8)).min(255);
    (red << 16) | (green << 8) | blue
}

#[inline]
pub(crate) fn pixel_luma(pixel: Colour) -> u32 {
    let red = (pixel >> 16) & 0xFF;
    let green = (pixel >> 8) & 0xFF;
    let blue = pixel & 0xFF;
    (red * RED_LUMA_WEIGHT + green * GREEN_LUMA_WEIGHT + blue * BLUE_LUMA_WEIGHT) >> 8
}

#[inline]
pub(crate) fn blend_onto_canvas(canvas: &mut Canvas, pixel_x: i32, pixel_y: i32, colour: Colour, weight: u32) {
    if !canvas.in_bounds(pixel_x, pixel_y) {
        return;
    }
    let pixel_index = (pixel_y * canvas.width + pixel_x) as usize;
    canvas.pixels[pixel_index] = blend_pixels(canvas.pixels[pixel_index], colour, weight);
}

#[inline]
pub(crate) fn glow_onto_canvas(canvas: &mut Canvas, pixel_x: i32, pixel_y: i32, colour: Colour, weight: u32) {
    if !canvas.in_bounds(pixel_x, pixel_y) {
        return;
    }
    let pixel_index = (pixel_y * canvas.width + pixel_x) as usize;
    canvas.pixels[pixel_index] = add_to_pixel(canvas.pixels[pixel_index], colour, weight.min(FULL_BLEND_WEIGHT));
}

pub(crate) struct DitherPattern {
    rank_of_each_pixel: [[u8; 8]; 8],
    half_threshold_step_in_fixed_point: i32,
}

pub(crate) const COARSE_DITHER: DitherPattern = DitherPattern {
    rank_of_each_pixel: [
        [0, 8, 2, 10, 0, 8, 2, 10],
        [12, 4, 14, 6, 12, 4, 14, 6],
        [3, 11, 1, 9, 3, 11, 1, 9],
        [15, 7, 13, 5, 15, 7, 13, 5],
        [0, 8, 2, 10, 0, 8, 2, 10],
        [12, 4, 14, 6, 12, 4, 14, 6],
        [3, 11, 1, 9, 3, 11, 1, 9],
        [15, 7, 13, 5, 15, 7, 13, 5],
    ],
    half_threshold_step_in_fixed_point: 65536 / (2 * 16),
};

pub(crate) const FINE_DITHER: DitherPattern = DitherPattern {
    rank_of_each_pixel: [
        [0, 32, 8, 40, 2, 34, 10, 42],
        [48, 16, 56, 24, 50, 18, 58, 26],
        [12, 44, 4, 36, 14, 46, 6, 38],
        [60, 28, 52, 20, 62, 30, 54, 22],
        [3, 35, 11, 43, 1, 33, 9, 41],
        [51, 19, 59, 27, 49, 17, 57, 25],
        [15, 47, 7, 39, 13, 45, 5, 37],
        [63, 31, 55, 23, 61, 29, 53, 21],
    ],
    half_threshold_step_in_fixed_point: 65536 / (2 * 64),
};

impl DitherPattern {
    #[inline]
    pub(crate) fn threshold_in_fixed_point(&self, pixel_x: i32, pixel_y: i32) -> i32 {
        let rank =
            self.rank_of_each_pixel[(pixel_y & DITHER_TILE_MASK) as usize][(pixel_x & DITHER_TILE_MASK) as usize];
        (rank as i32 * 2 + 1) * self.half_threshold_step_in_fixed_point
    }

    #[inline]
    pub(crate) fn threshold(&self, pixel_x: i32, pixel_y: i32) -> f32 {
        self.threshold_in_fixed_point(pixel_x, pixel_y) as f32 / ONE_IN_FIXED_POINT
    }

    fn whole_tile_of_thresholds_in_fixed_point(&self) -> [[i32; 8]; 8] {
        let mut thresholds = [[0; 8]; 8];
        for tile_y in 0..8 {
            for tile_x in 0..8 {
                thresholds[tile_y as usize][tile_x as usize] = self.threshold_in_fixed_point(tile_x, tile_y);
            }
        }
        thresholds
    }
}

#[derive(Default)]
pub(crate) struct CoarseField {
    nodes_across: i32,
    nodes_down: i32,
    node_values: Vec<f32>,
}

impl CoarseField {
    pub(crate) fn fill(
        &mut self,
        canvas_width: i32,
        canvas_height: i32,
        mut value_at_pixel: impl FnMut(f32, f32) -> f32,
    ) {
        self.nodes_across = (canvas_width + FIELD_CELL_PIXELS - 1) / FIELD_CELL_PIXELS + 1;
        self.nodes_down = (canvas_height + FIELD_CELL_PIXELS - 1) / FIELD_CELL_PIXELS + 1;
        self.node_values.resize((self.nodes_across * self.nodes_down) as usize, 0.0);
        let mut node_index = 0;
        for node_row in 0..self.nodes_down {
            let pixel_y = (node_row * FIELD_CELL_PIXELS) as f32;
            for node_column in 0..self.nodes_across {
                let pixel_x = (node_column * FIELD_CELL_PIXELS) as f32;
                self.node_values[node_index] = value_at_pixel(pixel_x, pixel_y);
                node_index += 1;
            }
        }
    }

    #[inline]
    pub(crate) fn draw_dithered_levels(
        &self,
        canvas: &mut Canvas,
        dither: &DitherPattern,
        mut paint_level: impl FnMut(&mut Colour, i32),
    ) {
        let pixels_per_cell = 1.0 / FIELD_CELL_PIXELS as f32;
        let canvas_width = canvas.width;
        let canvas_height = canvas.height;
        let thresholds = dither.whole_tile_of_thresholds_in_fixed_point();
        for cell_row in 0..self.nodes_down - 1 {
            let cell_top = cell_row * FIELD_CELL_PIXELS;
            let upper_node_row_start = (cell_row * self.nodes_across) as usize;
            let lower_node_row_start = upper_node_row_start + self.nodes_across as usize;
            for cell_column in 0..self.nodes_across - 1 {
                let top_left = self.node_values[upper_node_row_start + cell_column as usize];
                let top_right = self.node_values[upper_node_row_start + cell_column as usize + 1];
                let bottom_left = self.node_values[lower_node_row_start + cell_column as usize];
                let bottom_right = self.node_values[lower_node_row_start + cell_column as usize + 1];
                let cell_is_empty = top_left <= 0.0 && top_right <= 0.0 && bottom_left <= 0.0 && bottom_right <= 0.0;
                if cell_is_empty {
                    continue;
                }
                let cell_left = cell_column * FIELD_CELL_PIXELS;
                let cell_right = (cell_left + FIELD_CELL_PIXELS).min(canvas_width);
                if cell_left >= cell_right {
                    continue;
                }
                for row_in_cell in 0..FIELD_CELL_PIXELS {
                    let pixel_y = cell_top + row_in_cell;
                    if pixel_y >= canvas_height {
                        break;
                    }
                    let down_the_cell = (row_in_cell as f32 + 0.5) * pixels_per_cell;
                    let left_edge_value = top_left + (bottom_left - top_left) * down_the_cell;
                    let right_edge_value = top_right + (bottom_right - top_right) * down_the_cell;
                    let value_step = (right_edge_value - left_edge_value) * pixels_per_cell;
                    let mut value_in_fixed_point = ((left_edge_value + value_step * 0.5) * ONE_IN_FIXED_POINT) as i32;
                    let value_step_in_fixed_point = (value_step * ONE_IN_FIXED_POINT) as i32;
                    let row_start = (pixel_y * canvas_width + cell_left) as usize;
                    let row_end = (pixel_y * canvas_width + cell_right) as usize;
                    let thresholds_for_this_row = &thresholds[(pixel_y & DITHER_TILE_MASK) as usize];
                    let pixels_in_row = &mut canvas.pixels[row_start..row_end];
                    for (pixels_into_cell, pixel) in pixels_in_row.iter_mut().enumerate() {
                        let threshold = thresholds_for_this_row[pixels_into_cell & DITHER_TILE_MASK as usize];
                        let level = value_in_fixed_point.wrapping_add(threshold) >> FIXED_POINT_SHIFT;
                        if level > 0 {
                            paint_level(pixel, level);
                        }
                        value_in_fixed_point = value_in_fixed_point.wrapping_add(value_step_in_fixed_point);
                    }
                }
            }
        }
    }

    #[inline]
    pub(crate) fn dithered_level_at(&self, dither: &DitherPattern, pixel_x: i32, pixel_y: i32) -> i32 {
        let cell_column = pixel_x / FIELD_CELL_PIXELS;
        let cell_row = pixel_y / FIELD_CELL_PIXELS;
        let pixels_per_cell = 1.0 / FIELD_CELL_PIXELS as f32;
        let across_the_cell = ((pixel_x - cell_column * FIELD_CELL_PIXELS) as f32 + 0.5) * pixels_per_cell;
        let down_the_cell = ((pixel_y - cell_row * FIELD_CELL_PIXELS) as f32 + 0.5) * pixels_per_cell;
        let upper_node = (cell_row * self.nodes_across + cell_column) as usize;
        let lower_node = upper_node + self.nodes_across as usize;
        let top_left = self.node_values[upper_node];
        let top_right = self.node_values[upper_node + 1];
        let bottom_left = self.node_values[lower_node];
        let bottom_right = self.node_values[lower_node + 1];
        let left_edge_value = top_left + (bottom_left - top_left) * down_the_cell;
        let right_edge_value = top_right + (bottom_right - top_right) * down_the_cell;
        let value = left_edge_value + (right_edge_value - left_edge_value) * across_the_cell;
        let value_in_fixed_point = (value * ONE_IN_FIXED_POINT) as i32;
        (value_in_fixed_point + dither.threshold_in_fixed_point(pixel_x, pixel_y)) >> FIXED_POINT_SHIFT
    }
}

#[derive(Default)]
pub(crate) struct EffectState {
    has_seen_a_frame: bool,
    time_of_last_frame: f64,
    pub(crate) wind_travel_seconds: f64,
    pub(crate) coarse_field: CoarseField,
    pub(crate) bolt_mask: Vec<u8>,
    pub(crate) bolt_segments: Vec<BoltSegment>,
}

impl EffectState {
    pub(crate) fn follow_environment(&mut self, environment: &Environment) {
        if !self.has_seen_a_frame {
            self.has_seen_a_frame = true;
            self.time_of_last_frame = environment.animation_seconds;
            let travel_if_wind_had_always_blown = environment.wind as f64 * environment.animation_seconds;
            self.wind_travel_seconds =
                if travel_if_wind_had_always_blown.is_finite() { travel_if_wind_had_always_blown } else { 0.0 };
            return;
        }
        let seconds_since_last_frame = environment.animation_seconds - self.time_of_last_frame;
        if seconds_since_last_frame == 0.0 {
            return;
        }
        self.time_of_last_frame = environment.animation_seconds;
        let usable_seconds = if seconds_since_last_frame.is_finite() {
            seconds_since_last_frame.clamp(0.0, LONGEST_FRAME_GAP_SECONDS)
        } else {
            0.0
        };
        if environment.wind.is_finite() {
            self.wind_travel_seconds += environment.wind as f64 * usable_seconds;
        }
    }
}
