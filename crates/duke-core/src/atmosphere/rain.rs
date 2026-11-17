use crate::atmosphere::drawing::{
    blend_onto_canvas, blend_weight, hash_to_unit, periodic_noise, smoothstep, wrap_noise_offset, EffectState,
    PremultipliedColour, COARSE_DITHER, FULL_BLEND_WEIGHT, STANDARD_CANVAS_AREA,
};
use crate::hash_position;
use crate::{blend_colours, interpolate, rgb, scale_brightness, Canvas, Colour, Environment};

#[derive(Copy, Clone, PartialEq)]
pub(crate) enum RainLanding {
    PassesTheCamera,
    Speck,
    Ripple,
}

pub(crate) struct RainLayer {
    drop_slots_at_full_rain: f32,
    streak_length_range: (f32, f32),
    fall_speed_range: (f32, f32),
    visible_fall_range: (f32, f32),
    longest_pause_seconds: f32,
    opacity: f32,
    landing: RainLanding,
    seed: u32,
}

pub(crate) const FAR_RAIN: RainLayer = RainLayer {
    drop_slots_at_full_rain: 760.0,
    streak_length_range: (4.0, 7.0),
    fall_speed_range: (250.0, 330.0),
    visible_fall_range: (40.0, 90.0),
    longest_pause_seconds: 0.25,
    opacity: 0.5,
    landing: RainLanding::Speck,
    seed: 0xA1,
};

pub(crate) const MIDDLE_RAIN: RainLayer = RainLayer {
    drop_slots_at_full_rain: 400.0,
    streak_length_range: (7.0, 11.0),
    fall_speed_range: (400.0, 520.0),
    visible_fall_range: (110.0, 230.0),
    longest_pause_seconds: 0.35,
    opacity: 0.62,
    landing: RainLanding::Ripple,
    seed: 0xB2,
};

pub(crate) const NEAR_RAIN: RainLayer = RainLayer {
    drop_slots_at_full_rain: 100.0,
    streak_length_range: (14.0, 26.0),
    fall_speed_range: (740.0, 900.0),
    visible_fall_range: (0.0, 0.0),
    longest_pause_seconds: 1.0,
    opacity: 0.78,
    landing: RainLanding::PassesTheCamera,
    seed: 0xC3,
};

const SIDEWAYS_PIXELS_PER_PIXEL_FALLEN_AT_FULL_WIND: f32 = 0.6;
const GUST_WIDTH_PIXELS: f32 = 170.0;
const GUST_HEIGHT_PIXELS: f32 = 120.0;
const GUST_TRAVEL_PIXELS_PER_WIND_SECOND: f64 = 150.0;
const GUST_CHURN_CELLS_PER_SECOND: f64 = 0.07;
const GUST_SEED_MIX: u32 = 0x6057;
const LAYER_SEED_MULTIPLIER: u32 = 0x9E37_79B9;
const STREAK_COLOUR: Colour = rgb(206, 218, 238);
const LANDING_COLOUR: Colour = rgb(214, 226, 240);
const SHEET_COLOUR: Colour = rgb(186, 200, 226);
const WHITE: Colour = rgb(255, 255, 255);
const RAIN_BRIGHTNESS_AT_NIGHT: f32 = 0.5;
const RAIN_BRIGHTNESS_FROM_DAYLIGHT: f32 = 0.5;
const FLASH_SHARE_IN_RAIN_COLOUR: f32 = 0.8;
const EXTRA_OPACITY_PER_FLASH: f32 = 1.1;
const RIPPLE_BRIGHTNESS_ON_DRY_GROUND: f32 = 0.35;
const RIPPLE_BRIGHTNESS_FROM_WETNESS: f32 = 0.65;
const SPECK_SECONDS: f32 = 0.10;
const RIPPLE_SECONDS: f32 = 0.30;
const SHORTEST_PAUSE_SECONDS: f32 = 0.02;
const FALL_BEYOND_CANVAS_WHEN_PASSING_CAMERA: f32 = 70.0;
const LANDING_BELOW_CANVAS_WHEN_PASSING_CAMERA: f32 = 30.0;
const LANDING_SPREAD_BEYOND_WIDTH: f32 = 120.0;
const LANDING_SPREAD_LEFT_OF_CANVAS: f32 = 60.0;
const LANDING_SPREAD_BEYOND_HEIGHT: f32 = 40.0;
const LANDING_SPREAD_ABOVE_CANVAS: f32 = 8.0;
const CYCLE_HASH_MASK: i64 = 0x7FFF;
const CALM_GUST_STRENGTH: f32 = 0.5;
const LANDING_MARGIN_PIXELS: i32 = 6;
const SPECK_TAIL_SECONDS: f32 = 0.05;
const CROWN_SECONDS: f32 = 0.07;
const SMALL_RING_UNTIL_SECONDS: f32 = 0.17;
const SMALL_RING_PIXELS: [(i32, i32); 8] = [(-2, 0), (2, 0), (-1, -1), (0, -1), (1, -1), (-1, 1), (0, 1), (1, 1)];
const LARGE_RING_PIXELS: [(i32, i32); 12] =
    [(-4, 0), (4, 0), (-3, -1), (3, -1), (-3, 1), (3, 1), (-2, -2), (-1, -2), (1, -2), (2, -2), (-1, 2), (1, 2)];
const WEAKEST_SHEETS_WORTH_DRAWING: f32 = 0.05;
const GUST_STRENGTH_WHERE_SHEETS_BEGIN: f32 = 0.52;
const GUST_STRENGTH_WHERE_SHEETS_ARE_FULL: f32 = 0.8;
const SHEET_LEVEL_COUNT: i32 = 3;
const SHEET_OPACITY_BY_LEVEL: [f32; 4] = [0.0, 0.13, 0.21, 0.30];
const SHEET_LINE_SPACING_PIXELS: i32 = 5;
const SHEET_DASH_PIXELS: i32 = 12;
const SHEET_FALL_PIXELS_PER_SECOND: f64 = 560.0;
const SHEET_SCROLL_REPEAT_DASHES: i32 = 4096;
const SHEET_SEED_MIX: u32 = 0x5EE7;
const SHEET_DASH_OFFSET_MASK: u32 = 15;
const ONE_DASH_IN_FOUR_MASK: u32 = 3;

const BRIGHTNESS_IN_A_LULL: f32 = 0.3;
const BRIGHTNESS_GAINED_IN_A_GUST: f32 = 1.4;
const EXTRA_SLANT_IN_A_GUST: f32 = 0.8;
const SLANT_VARIATION_BETWEEN_DROPS: f32 = 0.05;
const EXTRA_STREAK_LENGTH_WHEN_SLANTED: f32 = 0.3;
const SPECK_OPACITY: f32 = 0.34;
const SPECK_TAIL_OPACITY: f32 = 0.2;
const CROWN_CENTRE_OPACITY: f32 = 0.7;
const CROWN_TOP_OPACITY: f32 = 0.5;
const CROWN_SIDE_OPACITY: f32 = 0.3;
const SMALL_RING_OPACITY: f32 = 0.36;
const LARGE_RING_OPACITY: f32 = 0.2;

struct GustField {
    noise_offset_x: f32,
    noise_offset_y: f32,
    seed: u32,
}

impl GustField {
    fn new(effect_state: &EffectState, environment: &Environment) -> GustField {
        let gust_travel_pixels = effect_state.wind_travel_seconds * GUST_TRAVEL_PIXELS_PER_WIND_SECOND;
        GustField {
            noise_offset_x: wrap_noise_offset(-gust_travel_pixels / GUST_WIDTH_PIXELS as f64),
            noise_offset_y: wrap_noise_offset(environment.animation_seconds * GUST_CHURN_CELLS_PER_SECOND),
            seed: environment.seed as u32 ^ GUST_SEED_MIX,
        }
    }

    #[inline]
    fn strength_at(&self, pixel_x: f32, pixel_y: f32) -> f32 {
        periodic_noise(
            pixel_x / GUST_WIDTH_PIXELS + self.noise_offset_x,
            pixel_y / GUST_HEIGHT_PIXELS + self.noise_offset_y,
            self.seed,
        )
    }
}

fn rain_colour_in_light(environment: &Environment, colour: Colour) -> Colour {
    let brightness = RAIN_BRIGHTNESS_AT_NIGHT + RAIN_BRIGHTNESS_FROM_DAYLIGHT * environment.daylight();
    blend_colours(
        scale_brightness(colour, brightness),
        WHITE,
        environment.lightning_flash.clamp(0.0, 1.0) * FLASH_SHARE_IN_RAIN_COLOUR,
    )
}

pub(crate) fn draw_rain_layer(
    effect_state: &EffectState,
    environment: &Environment,
    canvas: &mut Canvas,
    layer: &RainLayer,
    rain_amount: f32,
    storminess: f32,
) {
    let canvas_width = canvas.width as f32;
    let canvas_height = canvas.height as f32;
    let canvas_area_scale = (canvas_width * canvas_height) / STANDARD_CANVAS_AREA;
    let drop_count = (layer.drop_slots_at_full_rain * rain_amount.clamp(0.0, 1.0) * canvas_area_scale) as u32;
    if drop_count == 0 {
        return;
    }
    let layer_seed = (environment.seed as u32) ^ layer.seed.wrapping_mul(LAYER_SEED_MULTIPLIER);
    let wind_slant = environment.wind * SIDEWAYS_PIXELS_PER_PIXEL_FALLEN_AT_FULL_WIND;
    let flash = environment.lightning_flash.clamp(0.0, 1.0);
    let streak_colour = rain_colour_in_light(environment, STREAK_COLOUR);
    let landing_colour = rain_colour_in_light(environment, LANDING_COLOUR);
    let layer_opacity = layer.opacity * (1.0 + EXTRA_OPACITY_PER_FLASH * flash);
    let ripple_brightness =
        RIPPLE_BRIGHTNESS_ON_DRY_GROUND + RIPPLE_BRIGHTNESS_FROM_WETNESS * environment.wetness.clamp(0.0, 1.0);
    let gusts = GustField::new(effect_state, environment);
    let passes_the_camera = layer.landing == RainLanding::PassesTheCamera;
    let landing_seconds = match layer.landing {
        RainLanding::Ripple => RIPPLE_SECONDS,
        RainLanding::Speck => SPECK_SECONDS,
        RainLanding::PassesTheCamera => 0.0,
    };

    for drop_index in 0..drop_count {
        let speed_roll = hash_to_unit(drop_index, 0, layer_seed);
        let fall_roll = hash_to_unit(drop_index, 1, layer_seed);
        let length_roll = hash_to_unit(drop_index, 2, layer_seed);
        let timing_roll = hash_to_unit(drop_index, 3, layer_seed);
        let fall_speed = interpolate(layer.fall_speed_range.0, layer.fall_speed_range.1, speed_roll);
        let fall_distance = if passes_the_camera {
            canvas_height + FALL_BEYOND_CANVAS_WHEN_PASSING_CAMERA
        } else {
            interpolate(layer.visible_fall_range.0, layer.visible_fall_range.1, fall_roll)
        };
        let fall_seconds = fall_distance / fall_speed;
        let cycle_seconds =
            fall_seconds + landing_seconds + layer.longest_pause_seconds * length_roll + SHORTEST_PAUSE_SECONDS;
        let cycles_so_far = environment.animation_seconds / cycle_seconds as f64 + timing_roll as f64;
        let whole_cycles = cycles_so_far.floor();
        let seconds_into_cycle = (cycles_so_far - whole_cycles) as f32 * cycle_seconds;
        let cycle_hash_index = (whole_cycles as i64 & CYCLE_HASH_MASK) as u32;

        let landing_x = hash_to_unit(drop_index, cycle_hash_index * 2 + 10, layer_seed)
            * (canvas_width + LANDING_SPREAD_BEYOND_WIDTH)
            - LANDING_SPREAD_LEFT_OF_CANVAS;
        let landing_y = if passes_the_camera {
            canvas_height + LANDING_BELOW_CANVAS_WHEN_PASSING_CAMERA
        } else {
            hash_to_unit(drop_index, cycle_hash_index * 2 + 11, layer_seed)
                * (canvas_height + LANDING_SPREAD_BEYOND_HEIGHT)
                - LANDING_SPREAD_ABOVE_CANVAS
        };
        let gust_strength = if storminess > 0.0 { gusts.strength_at(landing_x, landing_y) } else { CALM_GUST_STRENGTH };
        let brightness_in_gust =
            interpolate(1.0, BRIGHTNESS_IN_A_LULL + BRIGHTNESS_GAINED_IN_A_GUST * gust_strength, storminess);
        let slant = wind_slant * (1.0 + storminess * EXTRA_SLANT_IN_A_GUST * (gust_strength - 0.5))
            + (fall_roll - 0.5) * SLANT_VARIATION_BETWEEN_DROPS;

        let is_still_falling = seconds_into_cycle < fall_seconds;
        if !is_still_falling {
            let seconds_since_landing = seconds_into_cycle - fall_seconds;
            if seconds_since_landing < landing_seconds {
                draw_rain_landing(
                    canvas,
                    landing_x,
                    landing_y,
                    seconds_since_landing,
                    layer.landing,
                    landing_colour,
                    brightness_in_gust,
                    ripple_brightness,
                );
            }
            continue;
        }

        let distance_still_to_fall = fall_distance - fall_speed * seconds_into_cycle;
        let streak_length = interpolate(layer.streak_length_range.0, layer.streak_length_range.1, length_roll);
        let streak_pixels = (streak_length * (1.0 + EXTRA_STREAK_LENGTH_WHEN_SLANTED * slant.abs())) as i32;
        let head_x = landing_x - slant * distance_still_to_fall;
        let head_y = (landing_y - distance_still_to_fall).floor() as i32;
        let streak_is_off_canvas = head_y < 0 || head_y - streak_pixels >= canvas.height;
        if streak_is_off_canvas {
            continue;
        }
        let head_weight = blend_weight(layer_opacity * brightness_in_gust).min(FULL_BLEND_WEIGHT);
        for pixels_behind_head in 0..streak_pixels {
            let pixel_y = head_y - pixels_behind_head;
            if pixel_y >= canvas.height {
                continue;
            }
            let pixel_x = (head_x - slant * pixels_behind_head as f32 + 0.5).floor() as i32;
            let weight = if pixels_behind_head * 3 < streak_pixels {
                head_weight
            } else if pixels_behind_head * 3 < streak_pixels * 2 {
                head_weight * 5 / 8
            } else {
                head_weight * 5 / 16
            };
            blend_onto_canvas(canvas, pixel_x, pixel_y, streak_colour, weight);
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn draw_rain_landing(
    canvas: &mut Canvas,
    landing_x: f32,
    landing_y: f32,
    seconds_since_landing: f32,
    landing: RainLanding,
    colour: Colour,
    brightness: f32,
    ripple_brightness: f32,
) {
    let pixel_x = landing_x.floor() as i32;
    let pixel_y = landing_y.floor() as i32;
    let is_far_off_canvas = pixel_x < -LANDING_MARGIN_PIXELS
        || pixel_y < -LANDING_MARGIN_PIXELS
        || pixel_x > canvas.width + LANDING_MARGIN_PIXELS
        || pixel_y > canvas.height + LANDING_MARGIN_PIXELS;
    if is_far_off_canvas {
        return;
    }
    if landing == RainLanding::Speck {
        blend_onto_canvas(canvas, pixel_x, pixel_y, colour, blend_weight(SPECK_OPACITY * brightness));
        if seconds_since_landing < SPECK_TAIL_SECONDS {
            blend_onto_canvas(canvas, pixel_x, pixel_y - 1, colour, blend_weight(SPECK_TAIL_OPACITY * brightness));
        }
        return;
    }
    if seconds_since_landing < CROWN_SECONDS {
        blend_onto_canvas(canvas, pixel_x, pixel_y, colour, blend_weight(CROWN_CENTRE_OPACITY * brightness));
        blend_onto_canvas(canvas, pixel_x, pixel_y - 1, colour, blend_weight(CROWN_TOP_OPACITY * brightness));
        blend_onto_canvas(canvas, pixel_x - 1, pixel_y, colour, blend_weight(CROWN_SIDE_OPACITY * brightness));
        blend_onto_canvas(canvas, pixel_x + 1, pixel_y, colour, blend_weight(CROWN_SIDE_OPACITY * brightness));
        return;
    }
    if seconds_since_landing < SMALL_RING_UNTIL_SECONDS {
        let ring_weight = blend_weight(SMALL_RING_OPACITY * brightness * ripple_brightness);
        for (offset_x, offset_y) in SMALL_RING_PIXELS {
            blend_onto_canvas(canvas, pixel_x + offset_x, pixel_y + offset_y, colour, ring_weight);
        }
        return;
    }
    let ring_weight = blend_weight(LARGE_RING_OPACITY * brightness * ripple_brightness);
    for (offset_x, offset_y) in LARGE_RING_PIXELS {
        blend_onto_canvas(canvas, pixel_x + offset_x, pixel_y + offset_y, colour, ring_weight);
    }
}

pub(crate) fn draw_storm_sheets(
    effect_state: &mut EffectState,
    environment: &Environment,
    canvas: &mut Canvas,
    rain_amount: f32,
    storminess: f32,
) {
    let sheet_strength = rain_amount.clamp(0.0, 1.0) * storminess.clamp(0.0, 1.0);
    if sheet_strength < WEAKEST_SHEETS_WORTH_DRAWING {
        return;
    }
    let gusts = GustField::new(effect_state, environment);
    let canvas_width = canvas.width;
    let canvas_height = canvas.height;
    effect_state.coarse_field.fill(canvas_width, canvas_height, |pixel_x, pixel_y| {
        let gust_strength = gusts.strength_at(pixel_x, pixel_y);
        let sheet_density =
            smoothstep(GUST_STRENGTH_WHERE_SHEETS_BEGIN, GUST_STRENGTH_WHERE_SHEETS_ARE_FULL, gust_strength);
        sheet_strength * sheet_density * SHEET_LEVEL_COUNT as f32
    });

    let sheet_colour = rain_colour_in_light(environment, SHEET_COLOUR);
    let flash_boost = 1.0 + environment.lightning_flash.clamp(0.0, 1.0);
    let mut sheet_at_each_level = [PremultipliedColour::default(); 4];
    for level in 1..=SHEET_LEVEL_COUNT as usize {
        sheet_at_each_level[level] =
            PremultipliedColour::new(sheet_colour, blend_weight(SHEET_OPACITY_BY_LEVEL[level] * flash_boost));
    }
    let slant = environment.wind * SIDEWAYS_PIXELS_PER_PIXEL_FALLEN_AT_FULL_WIND;
    let scroll_repeat_pixels = (SHEET_DASH_PIXELS * SHEET_SCROLL_REPEAT_DASHES) as f64;
    let scrolled_pixels =
        (environment.animation_seconds * SHEET_FALL_PIXELS_PER_SECOND).rem_euclid(scroll_repeat_pixels) as i32;
    let sheet_seed = environment.seed as u32 ^ SHEET_SEED_MIX;

    let sideways_reach = (slant.abs() * canvas_height as f32).ceil() as i32 + SHEET_LINE_SPACING_PIXELS;
    let first_line = (-sideways_reach).div_euclid(SHEET_LINE_SPACING_PIXELS) * SHEET_LINE_SPACING_PIXELS;
    let mut line_x_at_top = first_line;
    while line_x_at_top < canvas_width + sideways_reach {
        let dash_offset = (hash_position(line_x_at_top, 0, sheet_seed) & SHEET_DASH_OFFSET_MASK) as i32;
        let mut dash_top = 0;
        while dash_top < canvas_height {
            let distance_along_line = dash_top - scrolled_pixels + dash_offset;
            let dash_index = distance_along_line.div_euclid(SHEET_DASH_PIXELS);
            let dash_bottom =
                (dash_top + SHEET_DASH_PIXELS - distance_along_line.rem_euclid(SHEET_DASH_PIXELS)).min(canvas_height);
            let dash_is_lit = hash_position(line_x_at_top, dash_index, sheet_seed) & ONE_DASH_IN_FOUR_MASK == 0;
            if dash_is_lit {
                for pixel_y in dash_top..dash_bottom {
                    let pixel_x = line_x_at_top + (slant * pixel_y as f32).floor() as i32;
                    if pixel_x < 0 || pixel_x >= canvas_width {
                        continue;
                    }
                    let density_level = effect_state.coarse_field.dithered_level_at(&COARSE_DITHER, pixel_x, pixel_y);
                    if density_level <= 0 {
                        continue;
                    }
                    let distance_into_dash = (pixel_y - scrolled_pixels + dash_offset).rem_euclid(SHEET_DASH_PIXELS);
                    let brightening_toward_leading_end = distance_into_dash * 3 / SHEET_DASH_PIXELS;
                    let level = (density_level.min(SHEET_LEVEL_COUNT) - 2 + brightening_toward_leading_end)
                        .clamp(1, SHEET_LEVEL_COUNT);
                    let pixel = &mut canvas.pixels[(pixel_y * canvas_width + pixel_x) as usize];
                    *pixel = sheet_at_each_level[level as usize].over(*pixel);
                }
            }
            dash_top = dash_bottom;
        }
        line_x_at_top += SHEET_LINE_SPACING_PIXELS;
    }
}
