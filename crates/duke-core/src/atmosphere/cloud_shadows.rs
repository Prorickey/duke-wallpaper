use crate::atmosphere::drawing::{
    blend_weight, multiply_pixel, periodic_fractal_noise, pixel_luma, prevailing_wind_direction, smoothstep,
    time_noise, wrap_noise_offset, FINE_DITHER, FULL_BLEND_WEIGHT,
};
use crate::atmosphere::Atmosphere;
use crate::{interpolate, Canvas, Environment, Weather};

const SIMULATION_TIME_TOLERANCE_SECONDS: f64 = 1e-6;
const SMALLEST_DECK_DIFFERENCE_WORTH_DRAWING: f32 = 0.004;
const GRADE_DARKENING_PER_OVERCAST: f32 = 0.38;
const GRADE_RED_LOSS_PER_OVERCAST: f32 = 0.06;
const GRADE_BLUE_GAIN_PER_OVERCAST: f32 = 0.05;
const GRADE_DESATURATION_PER_OVERCAST: f32 = 0.55;
const SHADOW_FADE_START_STRENGTH: f32 = 0.05;
const SHADOW_FADE_END_STRENGTH: f32 = 0.16;
const WEAKEST_VISIBLE_SHADOW_STRENGTH: f32 = 0.02;
const SCATTERED_CLOUD_NOISE_SECONDS: f64 = 400.0;
const SCATTERED_CLOUD_SEED_MIX: u32 = 0x77;
const SHADOW_NOISE_SEED_MIX: u32 = 0xC10D;
const LEAST_SCATTERED_COVER: f32 = 0.05;
const SCATTERED_COVER_RANGE: f32 = 0.42;
const CLOUDY_COVER: f32 = 0.8;
const RAINY_COVER: f32 = 1.0;
const SNOWY_COVER: f32 = 0.95;
const FOGGY_COVER: f32 = 0.7;
const NOISE_LEVEL_AT_HALF_COVER: f32 = 0.5;
const NOISE_LEVEL_CHANGE_PER_COVER: f32 = 0.38;
const SHADOW_EDGE_SOFTNESS: f32 = 0.05;
const SHADOW_WIDTH_PIXELS: f32 = 160.0;
const SHADOW_HEIGHT_PIXELS: f32 = 84.0;
const SHADOW_DRIFT_PIXELS_PER_SECOND: f64 = 4.0;
const SHADOW_DRIFT_PIXELS_PER_WIND_SECOND: f64 = 11.0;
const SHADOW_DRIFT_DOWN_PIXELS_PER_SECOND: f64 = 1.4;
const SHADOW_DEPTH_PER_STRENGTH: f32 = 0.8;
const DEEPEST_SHADOW_TINT: [f32; 3] = [0.60, 0.66, 0.80];
const SHADOW_LEVEL_COUNT: f32 = 2.0;
const FULL_SHADOW_LEVEL: i32 = 2;

const SHADOW_FALLOFF_WITH_OVERCAST: f32 = 1.5;
const NOISE_LEVEL_OF_LEAST_SCATTERED_CLOUD: f32 = 0.2;
const NOISE_LEVEL_OF_MOST_SCATTERED_CLOUD: f32 = 0.9;

impl Atmosphere {
    pub fn draw_cloud_shadows(&mut self, environment: &Environment, frame: &mut Canvas) {
        if frame.width <= 0 || frame.height <= 0 {
            return;
        }
        self.effect_state.follow_environment(environment);

        let came_from_another_simulation = (environment.animation_seconds - self.simulation_seconds).abs()
            > SIMULATION_TIME_TOLERANCE_SECONDS
            || environment.weather != self.current_weather;
        let overcast_in_environment = environment.overcast();
        let overcast_held_by_cloud_deck = if came_from_another_simulation {
            0.0
        } else {
            (self.cloud_deck_overcast - overcast_in_environment).max(0.0)
        };
        let overcast = (overcast_in_environment + overcast_held_by_cloud_deck).min(1.0);

        if overcast_held_by_cloud_deck > SMALLEST_DECK_DIFFERENCE_WORTH_DRAWING {
            darken_frame_to_overcast(frame, overcast_in_environment, overcast);
        }

        let intensity = environment.weather_intensity.clamp(0.0, 1.0);
        let sunlight_strength = environment.daylight() * (1.0 - overcast).powf(SHADOW_FALLOFF_WITH_OVERCAST);
        let strength_through_falling_weather = match environment.weather {
            Weather::Clear | Weather::Cloudy => sunlight_strength,
            _ => sunlight_strength * (1.0 - intensity),
        };
        let shadow_strength = strength_through_falling_weather
            * smoothstep(SHADOW_FADE_START_STRENGTH, SHADOW_FADE_END_STRENGTH, strength_through_falling_weather);
        if shadow_strength < WEAKEST_VISIBLE_SHADOW_STRENGTH {
            return;
        }

        let scattered_cloud_noise = time_noise(
            environment.animation_seconds / SCATTERED_CLOUD_NOISE_SECONDS,
            environment.seed as u32 ^ SCATTERED_CLOUD_SEED_MIX,
        );
        let scattered_cover = LEAST_SCATTERED_COVER
            + SCATTERED_COVER_RANGE
                * smoothstep(
                    NOISE_LEVEL_OF_LEAST_SCATTERED_CLOUD,
                    NOISE_LEVEL_OF_MOST_SCATTERED_CLOUD,
                    scattered_cloud_noise,
                );
        let cover_at_full_intensity = match environment.weather {
            Weather::Clear => scattered_cover,
            Weather::Cloudy => CLOUDY_COVER,
            Weather::Rain | Weather::Storm => RAINY_COVER,
            Weather::Snow => SNOWY_COVER,
            Weather::Fog => FOGGY_COVER,
        };
        let cover = interpolate(scattered_cover, cover_at_full_intensity, intensity);
        let noise_level_under_cloud = NOISE_LEVEL_AT_HALF_COVER - NOISE_LEVEL_CHANGE_PER_COVER * (cover - 0.5);

        let drift_right_pixels =
            environment.animation_seconds * SHADOW_DRIFT_PIXELS_PER_SECOND * prevailing_wind_direction(environment)
                + self.effect_state.wind_travel_seconds * SHADOW_DRIFT_PIXELS_PER_WIND_SECOND;
        let drift_down_pixels = environment.animation_seconds * SHADOW_DRIFT_DOWN_PIXELS_PER_SECOND;
        let noise_offset_x = wrap_noise_offset(-drift_right_pixels / SHADOW_WIDTH_PIXELS as f64);
        let noise_offset_y = wrap_noise_offset(-drift_down_pixels / SHADOW_HEIGHT_PIXELS as f64);
        let noise_seed = environment.seed as u32 ^ SHADOW_NOISE_SEED_MIX;
        self.effect_state.coarse_field.fill(frame.width, frame.height, |pixel_x, pixel_y| {
            let noise = periodic_fractal_noise(
                pixel_x / SHADOW_WIDTH_PIXELS + noise_offset_x,
                pixel_y / SHADOW_HEIGHT_PIXELS + noise_offset_y,
                noise_seed,
            );
            let shade = smoothstep(
                noise_level_under_cloud - SHADOW_EDGE_SOFTNESS,
                noise_level_under_cloud + SHADOW_EDGE_SOFTNESS,
                noise,
            );
            shade * SHADOW_LEVEL_COUNT
        });

        let shadow_depth = (shadow_strength * SHADOW_DEPTH_PER_STRENGTH).min(1.0);
        let full_shadow_tint = [
            interpolate(1.0, DEEPEST_SHADOW_TINT[0], shadow_depth),
            interpolate(1.0, DEEPEST_SHADOW_TINT[1], shadow_depth),
            interpolate(1.0, DEEPEST_SHADOW_TINT[2], shadow_depth),
        ];
        let full_shadow_factors =
            [blend_weight(full_shadow_tint[0]), blend_weight(full_shadow_tint[1]), blend_weight(full_shadow_tint[2])];
        let half_shadow_factors = [
            blend_weight(0.5 + 0.5 * full_shadow_tint[0]),
            blend_weight(0.5 + 0.5 * full_shadow_tint[1]),
            blend_weight(0.5 + 0.5 * full_shadow_tint[2]),
        ];
        self.effect_state.coarse_field.draw_dithered_levels(frame, &FINE_DITHER, |pixel, shadow_level| {
            let factors = if shadow_level >= FULL_SHADOW_LEVEL { full_shadow_factors } else { half_shadow_factors };
            *pixel = multiply_pixel(*pixel, factors);
        });
    }
}

fn darken_frame_to_overcast(frame: &mut Canvas, overcast_in_environment: f32, overcast: f32) {
    let missing_overcast = overcast - overcast_in_environment;
    let brightness_ratio = (1.0 - GRADE_DARKENING_PER_OVERCAST * overcast)
        / (1.0 - GRADE_DARKENING_PER_OVERCAST * overcast_in_environment);
    let red_factor = blend_weight(brightness_ratio * (1.0 - GRADE_RED_LOSS_PER_OVERCAST * missing_overcast));
    let green_factor = blend_weight(brightness_ratio);
    let blue_factor =
        blend_weight((brightness_ratio * (1.0 + GRADE_BLUE_GAIN_PER_OVERCAST * missing_overcast)).min(1.0));
    let grey_weight = blend_weight(GRADE_DESATURATION_PER_OVERCAST * missing_overcast);
    let colour_weight = FULL_BLEND_WEIGHT - grey_weight;
    for pixel in frame.pixels.iter_mut() {
        let luma = pixel_luma(*pixel);
        let greyed_red = (((*pixel >> 16) & 0xFF) * colour_weight + luma * grey_weight) >> 8;
        let greyed_green = (((*pixel >> 8) & 0xFF) * colour_weight + luma * grey_weight) >> 8;
        let greyed_blue = ((*pixel & 0xFF) * colour_weight + luma * grey_weight) >> 8;
        let red = (greyed_red * red_factor) >> 8;
        let green = (greyed_green * green_factor) >> 8;
        let blue = (greyed_blue * blue_factor) >> 8;
        *pixel = (red << 16) | (green << 8) | blue;
    }
}
