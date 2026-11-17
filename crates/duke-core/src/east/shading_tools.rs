use crate::{blend_colours, hash_position, tint_colour, Colour};

const ORDERED_DITHER_MATRIX: [[u8; 4]; 4] = [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]];
const RAIN_SOAKED_TINT: [f32; 3] = [0.66, 0.69, 0.78];

#[inline]
pub fn ordered_dither_threshold(screen_x: i32, screen_y: i32) -> f32 {
    let matrix_entry = ORDERED_DITHER_MATRIX[(screen_y & 3) as usize][(screen_x & 3) as usize];
    (matrix_entry as f32 + 0.5) / 16.0
}

#[inline]
pub fn dithered_ramp_colour(ramp: &[Colour], level: f32, screen_x: i32, screen_y: i32) -> Colour {
    let last_index = ramp.len() as f32 - 1.0;
    let position = level.clamp(0.0, 1.0) * last_index;
    let lower_index = position.floor();
    let fraction_toward_next = position - lower_index;
    let mut index = lower_index as usize;
    if fraction_toward_next > ordered_dither_threshold(screen_x, screen_y) && index + 1 < ramp.len() {
        index += 1;
    }
    ramp[index]
}

#[inline]
pub fn banded_ramp_colour(ramp: &[Colour], level: f32, screen_x: i32, screen_y: i32, dither_band_width: f32) -> Colour {
    let last_index = ramp.len() as f32 - 1.0;
    let position = level.clamp(0.0, 1.0) * last_index;
    let lower_index = position.floor();
    let fraction_toward_next = position - lower_index;
    let mut index = lower_index as usize;
    let fraction_within_band = ((fraction_toward_next - 0.5) / (2.0 * dither_band_width) + 0.5).clamp(0.0, 1.0);
    if fraction_within_band > ordered_dither_threshold(screen_x, screen_y) && index + 1 < ramp.len() {
        index += 1;
    }
    ramp[index]
}

#[inline]
pub fn nearest_ramp_colour(ramp: &[Colour], level: f32) -> Colour {
    let last_index = ramp.len() as f32 - 1.0;
    ramp[(level.clamp(0.0, 1.0) * last_index + 0.5) as usize]
}

#[inline]
pub fn smoothstep(start: f32, end: f32, value: f32) -> f32 {
    let fraction = ((value - start) / (end - start)).clamp(0.0, 1.0);
    fraction * fraction * (3.0 - 2.0 * fraction)
}

#[inline]
pub fn rain_darkened(colour: Colour, wetness: f32) -> Colour {
    if wetness <= 0.0 {
        return colour;
    }
    blend_colours(colour, tint_colour(colour, RAIN_SOAKED_TINT), wetness)
}

#[inline]
pub fn unit_hash(first: i32, second: i32, seed: u32) -> f32 {
    (hash_position(first, second, seed) >> 8) as f32 / 16_777_216.0
}
