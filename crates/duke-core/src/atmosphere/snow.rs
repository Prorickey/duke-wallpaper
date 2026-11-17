use crate::atmosphere::drawing::{
    blend_pixels, blend_weight, hash_to_unit, multiply_pixel, pixel_luma, sine_wave, EffectState, STANDARD_CANVAS_AREA,
};
use crate::{blend_colours, colour_channels, interpolate, rgb, Canvas, Colour, Environment};

#[derive(Copy, Clone, PartialEq)]
enum FlakeShape {
    SinglePixel,
    SinglePixelSometimesPaired,
    SmallClump,
    SoftBlob,
}

pub(crate) struct SnowLayer {
    depth_index: u32,
    flakes_at_full_snowfall: f32,
    fall_speed_range: (f32, f32),
    sideways_pixels_per_wind_second: f32,
    flutter_pixels: f32,
    shape: FlakeShape,
    opacity: f32,
    colour: Colour,
    brightness_kept_in_the_dark: f32,
    seed: u32,
}

const DISTANT_FLAKE_COLOUR: Colour = rgb(226, 232, 244);
const NEAR_FLAKE_COLOUR: Colour = rgb(252, 253, 255);

pub(crate) const FAR_SNOW: SnowLayer = SnowLayer {
    depth_index: 0,
    flakes_at_full_snowfall: 900.0,
    fall_speed_range: (13.0, 22.0),
    sideways_pixels_per_wind_second: 14.0,
    flutter_pixels: 1.5,
    shape: FlakeShape::SinglePixel,
    opacity: 0.8,
    colour: DISTANT_FLAKE_COLOUR,
    brightness_kept_in_the_dark: 0.5,
    seed: 0x51,
};

pub(crate) const MIDDLE_SNOW: SnowLayer = SnowLayer {
    depth_index: 1,
    flakes_at_full_snowfall: 520.0,
    fall_speed_range: (24.0, 38.0),
    sideways_pixels_per_wind_second: 24.0,
    flutter_pixels: 3.0,
    shape: FlakeShape::SinglePixelSometimesPaired,
    opacity: 0.92,
    colour: NEAR_FLAKE_COLOUR,
    brightness_kept_in_the_dark: 0.55,
    seed: 0x52,
};

pub(crate) const NEAR_SNOW: SnowLayer = SnowLayer {
    depth_index: 2,
    flakes_at_full_snowfall: 230.0,
    fall_speed_range: (40.0, 62.0),
    sideways_pixels_per_wind_second: 38.0,
    flutter_pixels: 5.0,
    shape: FlakeShape::SmallClump,
    opacity: 0.95,
    colour: NEAR_FLAKE_COLOUR,
    brightness_kept_in_the_dark: 0.55,
    seed: 0x53,
};

pub(crate) const FOREGROUND_SNOW: SnowLayer = SnowLayer {
    depth_index: 3,
    flakes_at_full_snowfall: 9.0,
    fall_speed_range: (58.0, 84.0),
    sideways_pixels_per_wind_second: 55.0,
    flutter_pixels: 9.0,
    shape: FlakeShape::SoftBlob,
    opacity: 0.62,
    colour: NEAR_FLAKE_COLOUR,
    brightness_kept_in_the_dark: 0.55,
    seed: 0x54,
};

const LAYER_SEED_MULTIPLIER: u32 = 0x85EB_CA6B;
const WHITE: Colour = rgb(255, 255, 255);
const FLASH_SHARE_IN_FLAKE_COLOUR: f32 = 0.8;
const MARGIN_ABOVE_AND_BELOW_PIXELS: f32 = 8.0;
const MARGIN_LEFT_AND_RIGHT_PIXELS: f32 = 40.0;
const CYCLE_HASH_MASK: i64 = 0x7FFF;
const SWIRL_HEIGHT_PIXELS: f32 = 110.0;
const SWIRL_CYCLES_PER_SECOND: f32 = 0.09;
const SWIRL_CYCLE_OFFSET_PER_LAYER: f32 = 0.31;
const SWIRL_SHARE_OF_FLUTTER: f32 = 1.3;
const SHARE_OF_FLAKES_THAT_TWINKLE: f32 = 0.35;
const SHARE_OF_FLAKES_THAT_ARE_PAIRS: f32 = 0.35;
const SHARE_OF_CLUMPS_THAT_ARE_SQUARES: f32 = 0.4;
const SHARE_OF_CLUMPS_THAT_ARE_SQUARES_OR_CROSSES: f32 = 0.65;
const SHARE_OF_BLOBS_THAT_ARE_LARGE: f32 = 0.4;
const LARGE_BLOB_PIXELS: i32 = 5;
const SMALL_BLOB_PIXELS: i32 = 4;
const LUMA_MARGIN_FOR_A_VISIBLE_FLAKE: u32 = 20;
const SHADOW_OF_FLAKE_ON_BRIGHT_GROUND: [u32; 3] = [188, 200, 226];

const SLOWEST_SHARE_OF_WIND_DRIFT: f32 = 0.8;
const WIND_DRIFT_SHARE_RANGE: f32 = 0.4;
const SLOWEST_SLOW_FLUTTER_CYCLES_PER_SECOND: f32 = 0.19;
const SLOW_FLUTTER_RATE_RANGE: f32 = 0.3;
const SLOWEST_QUICK_FLUTTER_CYCLES_PER_SECOND: f32 = 0.53;
const QUICK_FLUTTER_RATE_RANGE: f32 = 0.4;
const QUICK_FLUTTER_SHARE: f32 = 0.5;
const DIMMEST_TWINKLE: f32 = 0.72;
const TWINKLE_RANGE: f32 = 0.28;
const SLOWEST_TWINKLE_CYCLES_PER_SECOND: f32 = 0.9;

#[inline]
fn draw_flake_pixel(
    canvas: &mut Canvas,
    pixel_x: i32,
    pixel_y: i32,
    flake_colour: Colour,
    flake_luma: u32,
    weight: u32,
) {
    if !canvas.in_bounds(pixel_x, pixel_y) {
        return;
    }
    let pixel_index = (pixel_y * canvas.width + pixel_x) as usize;
    let pixel = canvas.pixels[pixel_index];
    let ground_is_as_bright_as_the_flake = pixel_luma(pixel) + LUMA_MARGIN_FOR_A_VISIBLE_FLAKE > flake_luma;
    let colour = if ground_is_as_bright_as_the_flake {
        multiply_pixel(pixel, SHADOW_OF_FLAKE_ON_BRIGHT_GROUND)
    } else {
        flake_colour
    };
    canvas.pixels[pixel_index] = blend_pixels(pixel, colour, weight);
}

pub(crate) fn draw_snow_layer(
    effect_state: &EffectState,
    environment: &Environment,
    canvas: &mut Canvas,
    layer: &SnowLayer,
    snowfall: f32,
) {
    let canvas_width = canvas.width as f32;
    let canvas_height = canvas.height as f32;
    let canvas_area_scale = (canvas_width * canvas_height) / STANDARD_CANVAS_AREA;
    let exact_flake_count = layer.flakes_at_full_snowfall * snowfall.clamp(0.0, 1.0) * canvas_area_scale;
    let flake_count = exact_flake_count.ceil() as u32;
    if flake_count == 0 {
        return;
    }
    let layer_seed = (environment.seed as u32) ^ layer.seed.wrapping_mul(LAYER_SEED_MULTIPLIER);

    let scene_light = environment.lighting().tint;
    let (flake_red, flake_green, flake_blue) = colour_channels(layer.colour);
    let red_light = interpolate(scene_light[0].min(1.0), 1.0, layer.brightness_kept_in_the_dark);
    let green_light = interpolate(scene_light[1].min(1.0), 1.0, layer.brightness_kept_in_the_dark);
    let blue_light = interpolate(scene_light[2].min(1.0), 1.0, layer.brightness_kept_in_the_dark);
    let lit_colour = rgb(
        (flake_red as f32 * red_light).clamp(0.0, 255.0) as u8,
        (flake_green as f32 * green_light).clamp(0.0, 255.0) as u8,
        (flake_blue as f32 * blue_light).clamp(0.0, 255.0) as u8,
    );
    let flake_colour = blend_colours(lit_colour, WHITE, environment.lightning_flash * FLASH_SHARE_IN_FLAKE_COLOUR);
    let flake_luma = pixel_luma(flake_colour);

    let fall_span = (canvas_height + 2.0 * MARGIN_ABOVE_AND_BELOW_PIXELS) as f64;
    let drift_span = (canvas_width + 2.0 * MARGIN_LEFT_AND_RIGHT_PIXELS) as f64;
    let swirl_cycle_offset_of_layer = layer.depth_index as f32 * SWIRL_CYCLE_OFFSET_PER_LAYER;
    for flake_index in 0..flake_count {
        let speed_roll = hash_to_unit(flake_index, 0, layer_seed);
        let height_roll = hash_to_unit(flake_index, 1, layer_seed);
        let drift_roll = hash_to_unit(flake_index, 2, layer_seed);
        let flutter_roll = hash_to_unit(flake_index, 3, layer_seed);

        let fall_speed = interpolate(layer.fall_speed_range.0, layer.fall_speed_range.1, speed_roll);
        let distance_fallen = height_roll as f64 * fall_span + environment.animation_seconds * fall_speed as f64;
        let whole_falls = (distance_fallen / fall_span).floor();
        let flake_y = (distance_fallen - whole_falls * fall_span) as f32 - MARGIN_ABOVE_AND_BELOW_PIXELS;
        let fall_hash_index = (whole_falls as i64 & CYCLE_HASH_MASK) as u32;

        let starting_x = hash_to_unit(flake_index, fall_hash_index + 20, layer_seed) as f64 * drift_span;
        let wind_drift = effect_state.wind_travel_seconds
            * (layer.sideways_pixels_per_wind_second
                * (SLOWEST_SHARE_OF_WIND_DRIFT + WIND_DRIFT_SHARE_RANGE * drift_roll)) as f64;
        let slow_flutter = sine_wave(
            environment.animation_seconds,
            SLOWEST_SLOW_FLUTTER_CYCLES_PER_SECOND + SLOW_FLUTTER_RATE_RANGE * drift_roll,
            flutter_roll,
        );
        let quick_flutter = sine_wave(
            environment.animation_seconds,
            SLOWEST_QUICK_FLUTTER_CYCLES_PER_SECOND + QUICK_FLUTTER_RATE_RANGE * flutter_roll,
            drift_roll,
        );
        let flutter = layer.flutter_pixels * (slow_flutter + QUICK_FLUTTER_SHARE * quick_flutter);
        let swirl = layer.flutter_pixels
            * SWIRL_SHARE_OF_FLUTTER
            * sine_wave(
                environment.animation_seconds,
                SWIRL_CYCLES_PER_SECOND,
                flake_y / SWIRL_HEIGHT_PIXELS + swirl_cycle_offset_of_layer,
            );
        let flake_x =
            (starting_x + wind_drift).rem_euclid(drift_span) as f32 - MARGIN_LEFT_AND_RIGHT_PIXELS + flutter + swirl;

        let fade_of_last_flake = (exact_flake_count - flake_index as f32).clamp(0.0, 1.0);
        let twinkle = if flutter_roll < SHARE_OF_FLAKES_THAT_TWINKLE {
            DIMMEST_TWINKLE
                + TWINKLE_RANGE
                    * sine_wave(
                        environment.animation_seconds,
                        SLOWEST_TWINKLE_CYCLES_PER_SECOND + speed_roll,
                        height_roll,
                    )
        } else {
            1.0
        };
        let weight = blend_weight(layer.opacity * fade_of_last_flake * twinkle);
        let softer_weight = weight * 5 / 8;
        let pixel_x = flake_x.floor() as i32;
        let pixel_y = flake_y.floor() as i32;

        if layer.shape == FlakeShape::SinglePixel {
            draw_flake_pixel(canvas, pixel_x, pixel_y, flake_colour, flake_luma, weight);
            continue;
        }
        if layer.shape == FlakeShape::SinglePixelSometimesPaired {
            draw_flake_pixel(canvas, pixel_x, pixel_y, flake_colour, flake_luma, weight);
            if height_roll < SHARE_OF_FLAKES_THAT_ARE_PAIRS {
                let (partner_x, partner_y) =
                    if drift_roll < 0.5 { (pixel_x + 1, pixel_y) } else { (pixel_x, pixel_y + 1) };
                draw_flake_pixel(canvas, partner_x, partner_y, flake_colour, flake_luma, weight * 3 / 4);
            }
            continue;
        }
        if layer.shape == FlakeShape::SmallClump {
            if height_roll < SHARE_OF_CLUMPS_THAT_ARE_SQUARES {
                draw_flake_pixel(canvas, pixel_x, pixel_y, flake_colour, flake_luma, weight);
                draw_flake_pixel(canvas, pixel_x + 1, pixel_y, flake_colour, flake_luma, weight);
                draw_flake_pixel(canvas, pixel_x, pixel_y + 1, flake_colour, flake_luma, weight);
                draw_flake_pixel(canvas, pixel_x + 1, pixel_y + 1, flake_colour, flake_luma, weight);
            } else if height_roll < SHARE_OF_CLUMPS_THAT_ARE_SQUARES_OR_CROSSES {
                draw_flake_pixel(canvas, pixel_x, pixel_y, flake_colour, flake_luma, weight);
                draw_flake_pixel(canvas, pixel_x - 1, pixel_y, flake_colour, flake_luma, softer_weight);
                draw_flake_pixel(canvas, pixel_x + 1, pixel_y, flake_colour, flake_luma, softer_weight);
                draw_flake_pixel(canvas, pixel_x, pixel_y - 1, flake_colour, flake_luma, softer_weight);
                draw_flake_pixel(canvas, pixel_x, pixel_y + 1, flake_colour, flake_luma, softer_weight);
            } else {
                draw_flake_pixel(canvas, pixel_x, pixel_y, flake_colour, flake_luma, weight);
                draw_flake_pixel(canvas, pixel_x + 1, pixel_y, flake_colour, flake_luma, weight);
            }
            continue;
        }
        let blob_pixels =
            if height_roll < SHARE_OF_BLOBS_THAT_ARE_LARGE { LARGE_BLOB_PIXELS } else { SMALL_BLOB_PIXELS };
        let last_blob_pixel = blob_pixels - 1;
        for blob_row in 0..blob_pixels {
            for blob_column in 0..blob_pixels {
                let is_on_left_or_right_edge = blob_column == 0 || blob_column == last_blob_pixel;
                let is_on_top_or_bottom_edge = blob_row == 0 || blob_row == last_blob_pixel;
                if is_on_left_or_right_edge && is_on_top_or_bottom_edge {
                    continue;
                }
                let is_on_an_edge = is_on_left_or_right_edge || is_on_top_or_bottom_edge;
                let blob_weight = if is_on_an_edge { softer_weight } else { weight };
                draw_flake_pixel(
                    canvas,
                    pixel_x + blob_column,
                    pixel_y + blob_row,
                    flake_colour,
                    flake_luma,
                    blob_weight,
                );
            }
        }
    }
}
