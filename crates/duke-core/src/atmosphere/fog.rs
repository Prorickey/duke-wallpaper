use crate::atmosphere::drawing::{
    blend_weight, periodic_fractal_noise, prevailing_wind_direction, smoothstep, triangle_bump, wrap_noise_offset,
    EffectState, PremultipliedColour, COARSE_DITHER,
};
use crate::{blend_colours, colour_channels, interpolate, rgb, Canvas, Colour, Environment, Season, Weather};

const HOURS_PER_DAY: f32 = 24.0;
const OPACITY_LEVEL_COUNT: f32 = 5.0;
const SMALLEST_AMOUNT_WORTH_DRAWING: f32 = 0.004;
const PLAIN_FOG_COLOUR: Colour = rgb(208, 216, 224);
const LOW_CLOUD_COLOUR: Colour = rgb(226, 230, 236);
const STORM_SCUD_COLOUR: Colour = rgb(188, 196, 210);
const LIGHTNING_LIT_FOG_COLOUR: Colour = rgb(240, 244, 255);
const THICKEST_BANK_OPACITY_IN_LIGHT_MIST: f32 = 0.58;
const THICKEST_BANK_OPACITY_IN_FULL_FOG: f32 = 0.74;
const VEIL_OPACITY_IN_FULL_FOG: f32 = 0.07;
const WISP_AMOUNT_IN_FULL_FOG: f32 = 0.8;
const WISP_AMOUNT_WHEN_CLOUDY: f32 = 0.42;
const WISP_AMOUNT_IN_RAIN: f32 = 0.34;
const BANK_AMOUNT_IN_RAIN: f32 = 0.15;
const WISP_AMOUNT_IN_STORM: f32 = 0.5;
const DRIFT_SPEED_IN_FULL_STORM: f32 = 2.6;
const DAWN_MIST_AMOUNT: f32 = 0.42;
const DAWN_MIST_HALF_WIDTH_HOURS: f32 = 1.6;
const DAWN_MIST_PEAK_HOURS_AFTER_SUNRISE: f32 = 0.1;
const WISP_SHARE_OF_DAWN_MIST: f32 = 0.5;
const WIND_THAT_STARTS_CLEARING_MIST: f32 = 0.25;
const WIND_THAT_CLEARS_MIST: f32 = 0.6;
const FOG_BRIGHTNESS_LIFT_AT_NIGHT: f32 = 0.22;
const FOG_BRIGHTNESS_LIFT_FROM_DAYLIGHT: f32 = 0.38;
const FOG_GREYNESS: f32 = 0.4;
const FLASH_SHARE_IN_FOG_COLOUR: f32 = 0.7;
const RED_LUMA_SHARE: f32 = 0.299;
const GREEN_LUMA_SHARE: f32 = 0.587;
const BLUE_LUMA_SHARE: f32 = 0.114;
const BANK_WIDTH_PIXELS: f32 = 240.0;
const BANK_HEIGHT_PIXELS: f32 = 110.0;
const WISP_WIDTH_PIXELS: f32 = 150.0;
const WISP_HEIGHT_PIXELS: f32 = 44.0;
const BANK_DRIFT_PIXELS_PER_SECOND: f64 = 3.0;
const BANK_DRIFT_PIXELS_PER_WIND_SECOND: f64 = 9.0;
const BANK_DRIFT_DOWN_PIXELS_PER_SECOND: f64 = 0.7;
const WISP_DRIFT_PIXELS_PER_SECOND: f64 = 8.0;
const WISP_DRIFT_PIXELS_PER_WIND_SECOND: f64 = 24.0;
const WISP_DRIFT_DOWN_PIXELS_PER_SECOND: f64 = 2.2;
const NOISE_LEVEL_UNDER_FAINTEST_FOG: f32 = 0.78;
const NOISE_LEVEL_UNDER_FULL_BANKS: f32 = 0.42;
const NOISE_LEVEL_UNDER_FULL_WISPS: f32 = 0.5;
const BANK_EDGE_SOFTNESS: f32 = 0.3;
const WISP_EDGE_SOFTNESS: f32 = 0.16;
const AMOUNT_AT_WHICH_FOG_IS_FULLY_PRESENT: f32 = 0.1;
const WISP_OPACITY_SHARE: f32 = 0.7;
const FOG_NOISE_SEED_MIX: u32 = 0xF06;
const WISP_NOISE_SEED_MIX: u32 = 0x5EED;

const COLDNESS_AT_START_OF_AUTUMN: f32 = 0.55;
const COLDNESS_GAINED_THROUGH_AUTUMN: f32 = 0.45;
const COLDNESS_AT_START_OF_SPRING: f32 = 0.6;

pub(crate) fn draw_fog(effect_state: &mut EffectState, environment: &Environment, canvas: &mut Canvas) {
    let intensity = environment.weather_intensity.clamp(0.0, 1.0);
    let mut bank_amount = 0.0f32;
    let mut wisp_amount = 0.0f32;
    let mut veil_opacity = 0.0f32;
    let mut drift_speed = 1.0f32;
    let mut thickest_bank_opacity = THICKEST_BANK_OPACITY_IN_LIGHT_MIST;
    let mut weather_fog_colour = PLAIN_FOG_COLOUR;
    match environment.weather {
        Weather::Fog => {
            bank_amount = intensity;
            wisp_amount = WISP_AMOUNT_IN_FULL_FOG * intensity;
            veil_opacity = VEIL_OPACITY_IN_FULL_FOG * intensity;
            thickest_bank_opacity =
                interpolate(THICKEST_BANK_OPACITY_IN_LIGHT_MIST, THICKEST_BANK_OPACITY_IN_FULL_FOG, intensity);
        }
        Weather::Cloudy => {
            wisp_amount = WISP_AMOUNT_WHEN_CLOUDY * intensity;
            weather_fog_colour = LOW_CLOUD_COLOUR;
        }
        Weather::Rain => {
            wisp_amount = WISP_AMOUNT_IN_RAIN * intensity;
            bank_amount = BANK_AMOUNT_IN_RAIN * intensity;
        }
        Weather::Storm => {
            wisp_amount = WISP_AMOUNT_IN_STORM * intensity;
            drift_speed = interpolate(1.0, DRIFT_SPEED_IN_FULL_STORM, intensity);
            weather_fog_colour = STORM_SCUD_COLOUR;
        }
        Weather::Snow | Weather::Clear => {}
    }

    let (sunrise_hour, _) = environment.season.sunrise_and_sunset_hours();
    let nearness_to_dawn = triangle_bump(
        environment.hour.rem_euclid(HOURS_PER_DAY),
        sunrise_hour + DAWN_MIST_PEAK_HOURS_AFTER_SUNRISE,
        DAWN_MIST_HALF_WIDTH_HOURS,
    );
    let coldness_of_season = match environment.season {
        Season::Autumn => COLDNESS_AT_START_OF_AUTUMN + COLDNESS_GAINED_THROUGH_AUTUMN * environment.season_progress,
        Season::Winter => 1.0,
        Season::Spring => COLDNESS_AT_START_OF_SPRING * (1.0 - environment.season_progress),
        Season::Summer => 0.0,
    };
    let stillness_of_air =
        1.0 - smoothstep(WIND_THAT_STARTS_CLEARING_MIST, WIND_THAT_CLEARS_MIST, environment.wind.abs());
    let dawn_mist = DAWN_MIST_AMOUNT * nearness_to_dawn * coldness_of_season * stillness_of_air;
    let bank_amount = bank_amount.max(dawn_mist).clamp(0.0, 1.0);
    let wisp_amount = wisp_amount.max(WISP_SHARE_OF_DAWN_MIST * dawn_mist).clamp(0.0, 1.0);
    let veil_opacity = veil_opacity.clamp(0.0, 1.0);
    let nothing_to_draw = bank_amount < SMALLEST_AMOUNT_WORTH_DRAWING
        && wisp_amount < SMALLEST_AMOUNT_WORTH_DRAWING
        && veil_opacity < SMALLEST_AMOUNT_WORTH_DRAWING;
    if nothing_to_draw {
        return;
    }

    let scene_light = environment.lighting().tint;
    let (base_red, base_green, base_blue) =
        colour_channels(blend_colours(PLAIN_FOG_COLOUR, weather_fog_colour, intensity));
    let brightness_lift = FOG_BRIGHTNESS_LIFT_AT_NIGHT + FOG_BRIGHTNESS_LIFT_FROM_DAYLIGHT * environment.daylight();
    let lit_red = base_red as f32 * interpolate(scene_light[0].min(1.0), 1.0, brightness_lift);
    let lit_green = base_green as f32 * interpolate(scene_light[1].min(1.0), 1.0, brightness_lift);
    let lit_blue = base_blue as f32 * interpolate(scene_light[2].min(1.0), 1.0, brightness_lift);
    let lit_luma = RED_LUMA_SHARE * lit_red + GREEN_LUMA_SHARE * lit_green + BLUE_LUMA_SHARE * lit_blue;
    let greyed_colour = rgb(
        interpolate(lit_red, lit_luma, FOG_GREYNESS).clamp(0.0, 255.0) as u8,
        interpolate(lit_green, lit_luma, FOG_GREYNESS).clamp(0.0, 255.0) as u8,
        interpolate(lit_blue, lit_luma, FOG_GREYNESS).clamp(0.0, 255.0) as u8,
    );
    let fog_colour =
        blend_colours(greyed_colour, LIGHTNING_LIT_FOG_COLOUR, environment.lightning_flash * FLASH_SHARE_IN_FOG_COLOUR);

    let prevailing_direction = prevailing_wind_direction(environment);
    let wind_travel_seconds = effect_state.wind_travel_seconds;
    let drift_speed = drift_speed as f64;
    let bank_drift_right_pixels = environment.animation_seconds * BANK_DRIFT_PIXELS_PER_SECOND * prevailing_direction
        + wind_travel_seconds * BANK_DRIFT_PIXELS_PER_WIND_SECOND;
    let bank_drift_down_pixels = environment.animation_seconds * BANK_DRIFT_DOWN_PIXELS_PER_SECOND;
    let wisp_drift_right_pixels = environment.animation_seconds * WISP_DRIFT_PIXELS_PER_SECOND * prevailing_direction
        + wind_travel_seconds * WISP_DRIFT_PIXELS_PER_WIND_SECOND;
    let wisp_drift_down_pixels = environment.animation_seconds * WISP_DRIFT_DOWN_PIXELS_PER_SECOND;
    let bank_offset_x = wrap_noise_offset(-bank_drift_right_pixels * drift_speed / BANK_WIDTH_PIXELS as f64);
    let bank_offset_y = wrap_noise_offset(-bank_drift_down_pixels * drift_speed / BANK_HEIGHT_PIXELS as f64);
    let wisp_offset_x = wrap_noise_offset(-wisp_drift_right_pixels * drift_speed / WISP_WIDTH_PIXELS as f64);
    let wisp_offset_y = wrap_noise_offset(-wisp_drift_down_pixels * drift_speed / WISP_HEIGHT_PIXELS as f64);

    let noise_level_under_banks =
        interpolate(NOISE_LEVEL_UNDER_FAINTEST_FOG, NOISE_LEVEL_UNDER_FULL_BANKS, bank_amount);
    let bank_presence = smoothstep(0.0, AMOUNT_AT_WHICH_FOG_IS_FULLY_PRESENT, bank_amount);
    let noise_level_under_wisps =
        interpolate(NOISE_LEVEL_UNDER_FAINTEST_FOG, NOISE_LEVEL_UNDER_FULL_WISPS, wisp_amount);
    let wisp_presence = smoothstep(0.0, AMOUNT_AT_WHICH_FOG_IS_FULLY_PRESENT, wisp_amount) * WISP_OPACITY_SHARE;
    let bank_seed = environment.seed as u32 ^ FOG_NOISE_SEED_MIX;
    let wisp_seed = bank_seed ^ WISP_NOISE_SEED_MIX;

    effect_state.coarse_field.fill(canvas.width, canvas.height, |pixel_x, pixel_y| {
        let mut clear_air = 1.0;
        if bank_presence > 0.0 {
            let bank_noise = periodic_fractal_noise(
                pixel_x / BANK_WIDTH_PIXELS + bank_offset_x,
                pixel_y / BANK_HEIGHT_PIXELS + bank_offset_y,
                bank_seed,
            );
            let bank_thickness =
                smoothstep(noise_level_under_banks, noise_level_under_banks + BANK_EDGE_SOFTNESS, bank_noise);
            clear_air *= 1.0 - bank_presence * bank_thickness;
        }
        if wisp_presence > 0.0 {
            let wisp_noise = periodic_fractal_noise(
                pixel_x / WISP_WIDTH_PIXELS + wisp_offset_x,
                pixel_y / WISP_HEIGHT_PIXELS + wisp_offset_y,
                wisp_seed,
            );
            let wisp_thickness =
                smoothstep(noise_level_under_wisps, noise_level_under_wisps + WISP_EDGE_SOFTNESS, wisp_noise);
            clear_air *= 1.0 - wisp_presence * wisp_thickness;
        }
        let opacity = veil_opacity + (1.0 - veil_opacity) * thickest_bank_opacity * (1.0 - clear_air);
        opacity * OPACITY_LEVEL_COUNT
    });

    let highest_level = OPACITY_LEVEL_COUNT as usize;
    let mut fog_at_each_level = [PremultipliedColour::default(); 6];
    for (level, fog_at_level) in fog_at_each_level.iter_mut().enumerate() {
        *fog_at_level = PremultipliedColour::new(fog_colour, blend_weight(level as f32 / OPACITY_LEVEL_COUNT));
    }
    effect_state.coarse_field.draw_dithered_levels(canvas, &COARSE_DITHER, |pixel, level| {
        *pixel = fog_at_each_level[(level as usize).min(highest_level)].over(*pixel);
    });
}
