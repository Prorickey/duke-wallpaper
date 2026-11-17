use crate::atmosphere::drawing::{
    blend_onto_canvas, blend_weight, glow_onto_canvas, hash_to_unit, pixel_luma, sine_wave, smoothstep, triangle_bump,
    EffectState, FULL_BLEND_WEIGHT, STANDARD_CANVAS_AREA,
};
use crate::{blend_colours, interpolate, rgb, tint_colour, Canvas, Colour, Environment, Season, Weather};

const WHITE: Colour = rgb(255, 255, 255);
const FLASH_SHARE_IN_PARTICLE_COLOUR: f32 = 0.6;
const MURK_UNDER_FULL_CLOUD: f32 = 0.15;
const CYCLE_HASH_MASK: i64 = 0x7FFF;

const DRIFT_MARGIN_PIXELS: f32 = 12.0;
const DRIFT_SEED_MULTIPLIER: u32 = 0xC2B2_AE35;
const TUMBLE_FRAME_REPEAT: f64 = 4096.0;
const TUMBLE_POSE_MASK: u32 = 3;

const LEAVES_AT_FULL_FALL: f32 = 44.0;
const SMALLEST_AMOUNT_WORTH_DRAWING: f32 = 0.01;
const STORM_DEBRIS_AMOUNT: f32 = 0.3;
const SHARE_OF_LEAVES_THAT_ARE_LARGE: f32 = 0.36;
const LEAF_SHADOW_COLOUR: Colour = 0x10_1820;
const LEAF_SHADOW_OPACITY: f32 = 0.2;
const FAINTEST_LEAF_SHADOW_WEIGHT: u32 = 6;
const LEAF_HEIGHT_ABOVE_GROUND_PIXELS: f32 = 7.0;
const LEAF_UNDERSIDE_COLOUR: Colour = rgb(255, 240, 200);
const LEAF_UNDERSIDE_SHARE: f32 = 0.22;
const SEASON_PROGRESS_SPREAD_BETWEEN_LEAVES: f32 = 0.4;
const SEASON_PROGRESS_WHEN_LEAVES_PEAK: f32 = 0.33;
const SEASON_PROGRESS_WHEN_LEAVES_TURN_BROWN: f32 = 0.82;
const EARLY_AUTUMN_LEAF_COLOURS: [Colour; 4] =
    [rgb(168, 178, 62), rgb(214, 188, 64), rgb(226, 170, 52), rgb(140, 164, 60)];
const PEAK_AUTUMN_LEAF_COLOURS: [Colour; 6] =
    [rgb(226, 122, 42), rgb(198, 64, 40), rgb(234, 178, 58), rgb(172, 46, 48), rgb(212, 142, 46), rgb(236, 96, 44)];
const LATE_AUTUMN_LEAF_COLOURS: [Colour; 4] =
    [rgb(142, 92, 52), rgb(164, 86, 46), rgb(178, 132, 82), rgb(190, 150, 70)];
const GREEN_LEAF_COLOURS: [Colour; 3] = [rgb(92, 132, 60), rgb(116, 150, 66), rgb(78, 112, 54)];
const SMALL_LEAF_POSES: [[(i32, i32); 2]; 4] = [[(0, 0), (1, 0)], [(0, 0), (1, 1)], [(0, 0), (0, 1)], [(1, 0), (0, 1)]];
const LARGE_LEAF_POSES: [[(i32, i32); 3]; 4] =
    [[(0, 0), (1, 0), (0, 1)], [(0, 0), (1, 0), (1, 1)], [(1, 0), (0, 1), (1, 1)], [(0, 0), (0, 1), (1, 1)]];

const PETAL_COLOURS: [Colour; 5] =
    [rgb(255, 196, 214), rgb(255, 224, 234), rgb(255, 250, 246), rgb(246, 160, 196), rgb(255, 210, 222)];
const PETAL_OPACITY: f32 = 0.95;
const SHARE_OF_PETALS_WITH_TWO_PIXELS: f32 = 0.6;
const SECOND_PETAL_PIXEL_POSES: [(i32, i32); 4] = [(1, 0), (0, 1), (1, 1), (-1, 1)];
const POLLEN_COLOUR: Colour = rgb(236, 226, 120);
const POLLEN_GLINT_OPACITY: f32 = 0.6;
const POLLEN_DULL_OPACITY: f32 = 0.3;

const FIREFLIES_ON_A_PERFECT_NIGHT: f32 = 44.0;
const FEWEST_FIREFLIES_WORTH_DRAWING: f32 = 0.02;
const FIREFLY_SEED_MIX: u32 = 0xF1EF;
const FIREFLY_COLOUR: Colour = rgb(222, 255, 128);
const FIREFLY_INNER_GLOW_COLOUR: Colour = rgb(140, 190, 44);
const FIREFLY_OUTER_GLOW_COLOUR: Colour = rgb(120, 170, 40);
const FIREFLY_BRIGHTENING_SECONDS: f32 = 0.08;
const FIREFLY_FULLY_LIT_UNTIL_SECONDS: f32 = 0.3;
const FIREFLY_DARK_AFTER_SECONDS: f32 = 0.72;
const FIREFLY_FADING_SECONDS: f32 = 0.42;
const FIREFLY_INNER_GLOW_OFFSETS: [(i32, i32); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];
const FIREFLY_OUTER_GLOW_OFFSETS: [(i32, i32); 8] =
    [(-1, -1), (1, -1), (-1, 1), (1, 1), (-2, 0), (2, 0), (0, -2), (0, 2)];

const SECONDS_BETWEEN_FLOCK_CHANCES: f64 = 38.0;
const CHANCE_OF_A_FLOCK: f32 = 0.55;
const FLOCK_SEED_MIX: u32 = 0xB12D;
const FLOCK_SEED_MULTIPLIER: u32 = 977;
const FLOCK_MARGIN_PIXELS: f32 = 70.0;
const BIRD_MARGIN_PIXELS: i32 = 8;
const WEAKEST_SUN_THAT_CASTS_BIRD_SHADOWS: f32 = 0.12;
const BIRD_SHADOW_COLOUR: Colour = 0x0C_1420;
const BIRD_SHADOW_OPACITY: f32 = 0.34;
const SMALL_BIRD_HEIGHT_ABOVE_GROUND_PIXELS: f32 = 15.0;
const LARGE_BIRD_HEIGHT_ABOVE_GROUND_PIXELS: f32 = 24.0;
const GOOSE_BODY_COLOUR: Colour = rgb(52, 46, 44);
const GOOSE_WING_COLOUR: Colour = rgb(92, 82, 74);
const SONGBIRD_BODY_COLOUR: Colour = rgb(36, 38, 50);
const SONGBIRD_WING_COLOUR: Colour = rgb(66, 70, 88);
const PALE_BIRD_COLOUR: Colour = rgb(214, 216, 224);
const DARKEST_GROUND_THAT_SHOWS_A_DARK_BIRD: u32 = 52;
const WINGS_LEVEL: usize = 1;
const WING_POSE_THROUGH_A_BEAT: [usize; 4] = [0, 1, 2, 1];
const LARGE_BIRD_WING_POSES: [[(i32, i32); 4]; 3] =
    [[(-2, -1), (-1, 0), (1, 0), (2, -1)], [(-2, 0), (-1, 0), (1, 0), (2, 0)], [(-2, 1), (-1, 0), (1, 0), (2, 1)]];
const SMALL_BIRD_WING_POSES: [[(i32, i32); 2]; 3] = [[(-1, -1), (1, -1)], [(-1, 0), (1, 0)], [(-1, 1), (1, 1)]];

const SLOWEST_SHARE_OF_WIND_DRIFT: f32 = 0.65;
const WIND_DRIFT_SHARE_RANGE: f32 = 0.7;
const SLOWEST_SWAY_RATE_SHARE: f32 = 0.7;
const SWAY_RATE_SHARE_RANGE: f32 = 0.6;
const QUICK_SWAY_RATE_MULTIPLE: f32 = 2.7;
const QUICK_SWAY_SHARE: f32 = 0.4;
const VERTICAL_SWAY_SHARE: f32 = 0.45;
const SLOWEST_VERTICAL_SWAY_RATE_SHARE: f32 = 0.9;
const VERTICAL_SWAY_RATE_SHARE_RANGE: f32 = 0.5;
const VERTICAL_SWAY_CYCLE_OFFSET: f32 = 0.25;
const TUMBLE_FRAME_OFFSET_RANGE: f64 = 8.0;
const LEAF_FALL_AT_START_OF_AUTUMN: f32 = 0.06;
const LEAF_FALL_GAINED_BY_PEAK: f32 = 0.94;
const LEAF_FALL_BEGINS_AT_AUTUMN_PROGRESS: f32 = 0.08;
const LEAF_FALL_PEAKS_AT_AUTUMN_PROGRESS: f32 = 0.55;
const SHARE_OF_LEAF_FALL_LOST_BY_END_OF_AUTUMN: f32 = 0.55;
const LEAF_FALL_WANES_FROM_AUTUMN_PROGRESS: f32 = 0.86;
const LEAF_FALL_AT_START_OF_WINTER: f32 = 0.3;
const LEAF_FALL_OVER_BY_WINTER_PROGRESS: f32 = 0.22;
const LEAF_FALL_LOST_IN_FULL_SNOW_OR_FOG: f32 = 0.7;
const SHARE_OF_LEAVES_FALLING_IN_STILL_AIR: f32 = 0.4;
const EXTRA_SHARE_OF_LEAVES_FALLING_IN_FULL_WIND: f32 = 1.1;
const CALM_LOST_IN_FOUL_AIR: f32 = 0.8;
const BLOSSOM_BEGINS_AT_SPRING_PROGRESS: f32 = 0.03;
const BLOSSOM_PEAKS_AT_SPRING_PROGRESS: f32 = 0.16;
const BLOSSOM_WANES_FROM_SPRING_PROGRESS: f32 = 0.36;
const BLOSSOM_OVER_BY_SPRING_PROGRESS: f32 = 0.62;
const SHARE_OF_PETALS_FALLING_IN_STILL_AIR: f32 = 0.5;
const EXTRA_SHARE_OF_PETALS_FALLING_IN_FULL_WIND: f32 = 0.8;
const POLLEN_PEAKS_AT_SPRING_PROGRESS: f32 = 0.42;
const POLLEN_SEASON_HALF_WIDTH: f32 = 0.24;
const SHARE_OF_FIREFLIES_GONE_BY_END_OF_SUMMER: f32 = 0.5;
const FIREFLIES_WANE_FROM_SUMMER_PROGRESS: f32 = 0.75;
const FIREFLY_SHARE_IN_LATE_SPRING: f32 = 0.8;
const FIREFLIES_APPEAR_FROM_SPRING_PROGRESS: f32 = 0.72;
const FIREFLIES_ESTABLISHED_BY_SPRING_PROGRESS: f32 = 0.95;
const FIREFLY_SHARE_AT_START_OF_AUTUMN: f32 = 0.5;
const FIREFLIES_GONE_BY_AUTUMN_PROGRESS: f32 = 0.12;
const NIGHT_AT_WHICH_FIREFLIES_APPEAR: f32 = 0.72;
const NIGHT_AT_WHICH_ALL_FIREFLIES_ARE_OUT: f32 = 0.95;
const FIREFLIES_LOST_PER_MURKINESS: f32 = 1.4;
const WETNESS_THAT_STARTS_HIDING_FIREFLIES: f32 = 0.35;
const WETNESS_THAT_HIDES_ALL_FIREFLIES: f32 = 0.7;
const SNOW_COVER_THAT_STARTS_HIDING_FIREFLIES: f32 = 0.02;
const SNOW_COVER_THAT_HIDES_ALL_FIREFLIES: f32 = 0.2;
const SHORTEST_BLINK_CYCLE_SECONDS: f32 = 2.6;
const BLINK_CYCLE_SECONDS_RANGE: f32 = 3.4;
const HIGHEST_FIREFLY_HOME_SHARE_OF_HEIGHT: f32 = 0.1;
const FIREFLY_HOME_SPAN_SHARE_OF_HEIGHT: f32 = 0.88;
const FIREFLY_WANDER_WIDTH_PIXELS: f32 = 16.0;
const SLOWEST_SIDEWAYS_WANDER_CYCLES_PER_SECOND: f32 = 0.031;
const SIDEWAYS_WANDER_RATE_RANGE: f32 = 0.02;
const FIREFLY_WOBBLE_WIDTH_PIXELS: f32 = 6.0;
const SLOWEST_SIDEWAYS_WOBBLE_CYCLES_PER_SECOND: f32 = 0.11;
const SIDEWAYS_WOBBLE_RATE_RANGE: f32 = 0.05;
const FIREFLY_WANDER_HEIGHT_PIXELS: f32 = 9.0;
const SLOWEST_VERTICAL_WANDER_CYCLES_PER_SECOND: f32 = 0.027;
const VERTICAL_WANDER_RATE_RANGE: f32 = 0.02;
const FIREFLY_WOBBLE_HEIGHT_PIXELS: f32 = 3.0;
const VERTICAL_WOBBLE_CYCLES_PER_SECOND: f32 = 0.13;
const FIREFLY_RISE_PIXELS: f32 = 4.0;
const FIREFLY_RISE_SECONDS: f32 = 0.6;
const FIREFLY_INNER_GLOW_STRENGTH: f32 = 0.5;
const FIREFLY_OUTER_GLOW_STRENGTH: f32 = 0.2;
const DAYLIGHT_AT_WHICH_BIRDS_START_FLYING: f32 = 0.3;
const DAYLIGHT_AT_WHICH_BIRDS_FLY_FREELY: f32 = 0.6;
const WORST_WEATHER_BIRDS_WILL_FLY_IN: f32 = 0.5;
const SHORTEST_CROSSING_SECONDS: f32 = 10.0;
const CROSSING_SECONDS_RANGE: f32 = 5.0;
const CHANCE_OF_A_VEE_IN_GOOSE_SEASON: f32 = 0.6;
const CHANCE_OF_A_VEE_OTHERWISE: f32 = 0.15;
const EXTRA_BIRDS_RANGE: f32 = 6.0;
const FEWEST_GEESE: u32 = 7;
const FEWEST_SONGBIRDS: u32 = 3;
const CHANCE_SONGBIRDS_ARE_SMALL: f32 = 0.45;
const ENTRY_MARGIN_SHARE_OF_WIDTH: f32 = 0.12;
const ENTRY_SPAN_SHARE_OF_WIDTH: f32 = 0.76;
const LARGEST_SIDEWAYS_LEAN_SHARE_OF_WIDTH: f32 = 0.7;
const FLOCK_WEAVE_PIXELS: f32 = 6.0;
const FLOCK_WEAVE_RADIANS_PER_CROSSING: f32 = 7.0;
const VEE_SPACING_ACROSS_PIXELS: f32 = 6.5;
const VEE_SPACING_BEHIND_PIXELS: f32 = 5.0;
const VEE_RAGGEDNESS_PIXELS: f32 = 2.5;
const LOOSE_FLOCK_WIDTH_PIXELS: f32 = 70.0;
const LOOSE_FLOCK_LENGTH_PIXELS: f32 = 44.0;
const BIRD_BOB_WIDTH_PIXELS: f32 = 2.2;
const SLOWEST_SIDEWAYS_BOB_CYCLES_PER_SECOND: f32 = 0.21;
const BIRD_BOB_HEIGHT_PIXELS: f32 = 2.0;
const SLOWEST_VERTICAL_BOB_CYCLES_PER_SECOND: f32 = 0.17;
const BOB_RATE_RANGE: f32 = 0.1;
const GROUND_PIXELS_BELOW_BIRD_PER_PIXEL_OF_HEIGHT: f32 = 0.9;
const SLOWEST_GOOSE_WINGBEATS_PER_SECOND: f64 = 3.2;
const SLOWEST_SONGBIRD_WINGBEATS_PER_SECOND: f64 = 4.6;
const WINGBEAT_RATE_RANGE: f64 = 1.2;
const SLOWEST_GLIDE_CYCLES_PER_SECOND: f32 = 0.16;
const GLIDE_RATE_RANGE: f32 = 0.07;
const SHARE_OF_GLIDE_CYCLE_SPENT_FLAPPING: f32 = 0.5;
const QUARTER_BEATS_PER_WINGBEAT: f64 = 4.0;

fn colour_in_scene_light(environment: &Environment, colour: Colour) -> Colour {
    let scene_light = environment.lighting().tint;
    blend_colours(tint_colour(colour, scene_light), WHITE, environment.lightning_flash * FLASH_SHARE_IN_PARTICLE_COLOUR)
}

fn murkiness_of_air(environment: &Environment) -> f32 {
    let intensity = environment.weather_intensity.clamp(0.0, 1.0);
    match environment.weather {
        Weather::Clear => 0.0,
        Weather::Cloudy => MURK_UNDER_FULL_CLOUD * intensity,
        Weather::Rain | Weather::Snow | Weather::Fog | Weather::Storm => intensity,
    }
}

struct DriftingParticles {
    count_at_full_amount: f32,
    seed: u32,
    fall_speed_range: (f32, f32),
    sideways_pixels_per_wind_second: f32,
    sway_pixels: f32,
    sway_cycles_per_second: f32,
    tumble_frames_per_second_range: (f32, f32),
}

struct DriftingParticle {
    index: u32,
    pixel_x: i32,
    pixel_y: i32,
    first_roll: f32,
    second_roll: f32,
    opacity: f32,
    tumble_frame: u32,
}

fn for_each_drifting_particle(
    effect_state: &EffectState,
    environment: &Environment,
    canvas: &mut Canvas,
    particles: &DriftingParticles,
    amount: f32,
    mut draw_particle: impl FnMut(&mut Canvas, &DriftingParticle),
) {
    let canvas_width = canvas.width as f32;
    let canvas_height = canvas.height as f32;
    let canvas_area_scale = (canvas_width * canvas_height) / STANDARD_CANVAS_AREA;
    let exact_count = particles.count_at_full_amount * amount.clamp(0.0, 1.0) * canvas_area_scale;
    let particle_count = exact_count.ceil() as u32;
    let seed = (environment.seed as u32) ^ particles.seed.wrapping_mul(DRIFT_SEED_MULTIPLIER);
    let fall_span = (canvas_height + 2.0 * DRIFT_MARGIN_PIXELS) as f64;
    let drift_span = (canvas_width + 2.0 * DRIFT_MARGIN_PIXELS) as f64;
    let sway_pixels = particles.sway_pixels;
    let sway_rate = particles.sway_cycles_per_second;
    for index in 0..particle_count {
        let first_roll = hash_to_unit(index, 0, seed);
        let height_roll = hash_to_unit(index, 1, seed);
        let second_roll = hash_to_unit(index, 2, seed);
        let third_roll = hash_to_unit(index, 3, seed);

        let fall_speed = interpolate(particles.fall_speed_range.0, particles.fall_speed_range.1, first_roll);
        let distance_fallen = height_roll as f64 * fall_span + environment.animation_seconds * fall_speed as f64;
        let whole_falls = (distance_fallen / fall_span).floor();
        let mut drift_y = (distance_fallen - whole_falls * fall_span) as f32 - DRIFT_MARGIN_PIXELS;
        let fall_hash_index = (whole_falls as i64 & CYCLE_HASH_MASK) as u32;

        let starting_x = hash_to_unit(index, fall_hash_index + 40, seed) as f64 * drift_span;
        let wind_drift = effect_state.wind_travel_seconds
            * (particles.sideways_pixels_per_wind_second
                * (SLOWEST_SHARE_OF_WIND_DRIFT + WIND_DRIFT_SHARE_RANGE * second_roll)) as f64;
        let mut drift_x = (starting_x + wind_drift).rem_euclid(drift_span) as f32 - DRIFT_MARGIN_PIXELS;
        let slow_sway = sine_wave(
            environment.animation_seconds,
            sway_rate * (SLOWEST_SWAY_RATE_SHARE + SWAY_RATE_SHARE_RANGE * second_roll),
            third_roll,
        );
        let quick_sway = sine_wave(environment.animation_seconds, sway_rate * QUICK_SWAY_RATE_MULTIPLE, first_roll);
        drift_x += sway_pixels * (slow_sway + QUICK_SWAY_SHARE * quick_sway);
        drift_y += sway_pixels
            * VERTICAL_SWAY_SHARE
            * sine_wave(
                environment.animation_seconds,
                sway_rate * (SLOWEST_VERTICAL_SWAY_RATE_SHARE + VERTICAL_SWAY_RATE_SHARE_RANGE * third_roll),
                second_roll + VERTICAL_SWAY_CYCLE_OFFSET,
            );

        let tumble_rate = interpolate(
            particles.tumble_frames_per_second_range.0,
            particles.tumble_frames_per_second_range.1,
            third_roll,
        );
        let tumble_frame = ((environment.animation_seconds * tumble_rate as f64
            + height_roll as f64 * TUMBLE_FRAME_OFFSET_RANGE)
            .rem_euclid(TUMBLE_FRAME_REPEAT)) as u32;
        let opacity = (exact_count - index as f32).clamp(0.0, 1.0);
        let particle = DriftingParticle {
            index,
            pixel_x: drift_x.floor() as i32,
            pixel_y: drift_y.floor() as i32,
            first_roll,
            second_roll,
            opacity,
            tumble_frame,
        };
        draw_particle(canvas, &particle);
    }
}

pub(crate) fn draw_leaves(effect_state: &EffectState, environment: &Environment, canvas: &mut Canvas) {
    let season_progress = environment.season_progress.clamp(0.0, 1.0);
    let intensity = environment.weather_intensity.clamp(0.0, 1.0);
    let leaves_the_season_drops = match environment.season {
        Season::Autumn => {
            LEAF_FALL_AT_START_OF_AUTUMN
                + LEAF_FALL_GAINED_BY_PEAK
                    * smoothstep(
                        LEAF_FALL_BEGINS_AT_AUTUMN_PROGRESS,
                        LEAF_FALL_PEAKS_AT_AUTUMN_PROGRESS,
                        season_progress,
                    )
                    * (1.0
                        - SHARE_OF_LEAF_FALL_LOST_BY_END_OF_AUTUMN
                            * smoothstep(LEAF_FALL_WANES_FROM_AUTUMN_PROGRESS, 1.0, season_progress))
        }
        Season::Winter => {
            LEAF_FALL_AT_START_OF_WINTER * (1.0 - smoothstep(0.0, LEAF_FALL_OVER_BY_WINTER_PROGRESS, season_progress))
        }
        _ => 0.0,
    };
    let stillness_of_air = match environment.weather {
        Weather::Snow | Weather::Fog => 1.0 - LEAF_FALL_LOST_IN_FULL_SNOW_OR_FOG * intensity,
        _ => 1.0,
    };
    let seasonal_amount = leaves_the_season_drops
        * (SHARE_OF_LEAVES_FALLING_IN_STILL_AIR + EXTRA_SHARE_OF_LEAVES_FALLING_IN_FULL_WIND * environment.wind.abs())
            .min(1.0)
        * stillness_of_air;
    let storm_tears_leaves_off = environment.weather == Weather::Storm && environment.season != Season::Winter;
    let debris_amount = if storm_tears_leaves_off { STORM_DEBRIS_AMOUNT * intensity } else { 0.0 };
    let amount = seasonal_amount.max(debris_amount);
    if amount < SMALLEST_AMOUNT_WORTH_DRAWING {
        return;
    }
    let canvas_area_scale = (canvas.width as f32 * canvas.height as f32) / STANDARD_CANVAS_AREA;
    let seasonal_leaf_count = LEAVES_AT_FULL_FALL * seasonal_amount * canvas_area_scale;
    let leaves = DriftingParticles {
        count_at_full_amount: LEAVES_AT_FULL_FALL,
        seed: 0x1EAF,
        fall_speed_range: (9.0, 24.0),
        sideways_pixels_per_wind_second: 46.0,
        sway_pixels: 5.0,
        sway_cycles_per_second: 0.23,
        tumble_frames_per_second_range: (2.0, 5.5),
    };
    let season_progress_for_colour = match environment.season {
        Season::Autumn => season_progress,
        Season::Winter => 1.0,
        _ => 0.0,
    };
    let (shadow_right_per_height, shadow_down_per_height) = environment.shadow_direction();
    let shadow_weight = blend_weight(LEAF_SHADOW_OPACITY * environment.shadow_strength());
    for_each_drifting_particle(effect_state, environment, canvas, &leaves, amount, |canvas, leaf| {
        let own_season_progress =
            season_progress_for_colour + (leaf.first_roll - 0.5) * SEASON_PROGRESS_SPREAD_BETWEEN_LEAVES;
        let is_storm_debris = leaf.index as f32 >= seasonal_leaf_count;
        let leaf_colour = if is_storm_debris {
            GREEN_LEAF_COLOURS[(leaf.second_roll * GREEN_LEAF_COLOURS.len() as f32) as usize % GREEN_LEAF_COLOURS.len()]
        } else if own_season_progress < SEASON_PROGRESS_WHEN_LEAVES_PEAK {
            EARLY_AUTUMN_LEAF_COLOURS
                [(leaf.second_roll * EARLY_AUTUMN_LEAF_COLOURS.len() as f32) as usize % EARLY_AUTUMN_LEAF_COLOURS.len()]
        } else if own_season_progress < SEASON_PROGRESS_WHEN_LEAVES_TURN_BROWN {
            PEAK_AUTUMN_LEAF_COLOURS
                [(leaf.second_roll * PEAK_AUTUMN_LEAF_COLOURS.len() as f32) as usize % PEAK_AUTUMN_LEAF_COLOURS.len()]
        } else {
            LATE_AUTUMN_LEAF_COLOURS
                [(leaf.second_roll * LATE_AUTUMN_LEAF_COLOURS.len() as f32) as usize % LATE_AUTUMN_LEAF_COLOURS.len()]
        };
        let shows_its_underside = leaf.tumble_frame & 1 == 1;
        let side_colour = if shows_its_underside {
            blend_colours(leaf_colour, LEAF_UNDERSIDE_COLOUR, LEAF_UNDERSIDE_SHARE)
        } else {
            leaf_colour
        };
        let lit_colour = colour_in_scene_light(environment, side_colour);
        let weight = blend_weight(leaf.opacity);
        let pose = (leaf.tumble_frame & TUMBLE_POSE_MASK) as usize;
        let is_large = leaf.first_roll < SHARE_OF_LEAVES_THAT_ARE_LARGE;
        if !is_large {
            for (offset_x, offset_y) in SMALL_LEAF_POSES[pose] {
                blend_onto_canvas(canvas, leaf.pixel_x + offset_x, leaf.pixel_y + offset_y, lit_colour, weight);
            }
            return;
        }
        if shadow_weight > FAINTEST_LEAF_SHADOW_WEIGHT {
            let shadow_x = leaf.pixel_x + (shadow_right_per_height * LEAF_HEIGHT_ABOVE_GROUND_PIXELS) as i32;
            let shadow_y = leaf.pixel_y + 7 + (shadow_down_per_height * LEAF_HEIGHT_ABOVE_GROUND_PIXELS) as i32;
            blend_onto_canvas(canvas, shadow_x, shadow_y, LEAF_SHADOW_COLOUR, shadow_weight);
            blend_onto_canvas(canvas, shadow_x + 1, shadow_y, LEAF_SHADOW_COLOUR, shadow_weight);
        }
        for (offset_x, offset_y) in LARGE_LEAF_POSES[pose] {
            blend_onto_canvas(canvas, leaf.pixel_x + offset_x, leaf.pixel_y + offset_y, lit_colour, weight);
        }
    });
}

pub(crate) fn draw_spring_petals_and_pollen(
    effect_state: &EffectState,
    environment: &Environment,
    canvas: &mut Canvas,
) {
    if environment.season != Season::Spring {
        return;
    }
    let season_progress = environment.season_progress.clamp(0.0, 1.0);
    let calmness = 1.0 - CALM_LOST_IN_FOUL_AIR * murkiness_of_air(environment);

    let blossom = smoothstep(BLOSSOM_BEGINS_AT_SPRING_PROGRESS, BLOSSOM_PEAKS_AT_SPRING_PROGRESS, season_progress)
        * (1.0 - smoothstep(BLOSSOM_WANES_FROM_SPRING_PROGRESS, BLOSSOM_OVER_BY_SPRING_PROGRESS, season_progress))
        * calmness;
    if blossom > SMALLEST_AMOUNT_WORTH_DRAWING {
        let petals = DriftingParticles {
            count_at_full_amount: 46.0,
            seed: 0x9E7A,
            fall_speed_range: (7.0, 17.0),
            sideways_pixels_per_wind_second: 34.0,
            sway_pixels: 6.0,
            sway_cycles_per_second: 0.27,
            tumble_frames_per_second_range: (1.5, 4.0),
        };
        let petal_amount = blossom
            * (SHARE_OF_PETALS_FALLING_IN_STILL_AIR
                + EXTRA_SHARE_OF_PETALS_FALLING_IN_FULL_WIND * environment.wind.abs())
            .min(1.0);
        for_each_drifting_particle(effect_state, environment, canvas, &petals, petal_amount, |canvas, petal| {
            let petal_colour = colour_in_scene_light(
                environment,
                PETAL_COLOURS[(petal.second_roll * PETAL_COLOURS.len() as f32) as usize % PETAL_COLOURS.len()],
            );
            let weight = blend_weight(petal.opacity * PETAL_OPACITY);
            blend_onto_canvas(canvas, petal.pixel_x, petal.pixel_y, petal_colour, weight);
            if petal.first_roll < SHARE_OF_PETALS_WITH_TWO_PIXELS {
                let (offset_x, offset_y) = SECOND_PETAL_PIXEL_POSES[(petal.tumble_frame & TUMBLE_POSE_MASK) as usize];
                blend_onto_canvas(
                    canvas,
                    petal.pixel_x + offset_x,
                    petal.pixel_y + offset_y,
                    petal_colour,
                    weight * 3 / 4,
                );
            }
        });
    }

    let pollen_amount = triangle_bump(season_progress, POLLEN_PEAKS_AT_SPRING_PROGRESS, POLLEN_SEASON_HALF_WIDTH)
        * environment.daylight()
        * calmness;
    if pollen_amount > SMALLEST_AMOUNT_WORTH_DRAWING {
        let pollen = DriftingParticles {
            count_at_full_amount: 70.0,
            seed: 0x9011,
            fall_speed_range: (-3.0, 6.0),
            sideways_pixels_per_wind_second: 20.0,
            sway_pixels: 7.0,
            sway_cycles_per_second: 0.12,
            tumble_frames_per_second_range: (0.6, 1.4),
        };
        let pollen_colour = colour_in_scene_light(environment, POLLEN_COLOUR);
        for_each_drifting_particle(effect_state, environment, canvas, &pollen, pollen_amount, |canvas, mote| {
            let catches_the_light = mote.tumble_frame & TUMBLE_POSE_MASK == 0;
            let glint = if catches_the_light { POLLEN_GLINT_OPACITY } else { POLLEN_DULL_OPACITY };
            blend_onto_canvas(canvas, mote.pixel_x, mote.pixel_y, pollen_colour, blend_weight(mote.opacity * glint));
        });
    }
}

pub(crate) fn draw_fireflies(environment: &Environment, canvas: &mut Canvas) {
    let season_progress = environment.season_progress;
    let seasonal_share = match environment.season {
        Season::Summer => {
            1.0 - SHARE_OF_FIREFLIES_GONE_BY_END_OF_SUMMER
                * smoothstep(FIREFLIES_WANE_FROM_SUMMER_PROGRESS, 1.0, season_progress)
        }
        Season::Spring => {
            FIREFLY_SHARE_IN_LATE_SPRING
                * smoothstep(
                    FIREFLIES_APPEAR_FROM_SPRING_PROGRESS,
                    FIREFLIES_ESTABLISHED_BY_SPRING_PROGRESS,
                    season_progress,
                )
        }
        Season::Autumn => {
            FIREFLY_SHARE_AT_START_OF_AUTUMN
                * (1.0 - smoothstep(0.0, FIREFLIES_GONE_BY_AUTUMN_PROGRESS, season_progress))
        }
        Season::Winter => 0.0,
    };
    let darkness =
        smoothstep(NIGHT_AT_WHICH_FIREFLIES_APPEAR, NIGHT_AT_WHICH_ALL_FIREFLIES_ARE_OUT, environment.darkness());
    let dryness = (1.0 - FIREFLIES_LOST_PER_MURKINESS * murkiness_of_air(environment)).max(0.0)
        * (1.0
            - smoothstep(WETNESS_THAT_STARTS_HIDING_FIREFLIES, WETNESS_THAT_HIDES_ALL_FIREFLIES, environment.wetness));
    let amount = seasonal_share
        * darkness
        * dryness
        * (1.0
            - smoothstep(
                SNOW_COVER_THAT_STARTS_HIDING_FIREFLIES,
                SNOW_COVER_THAT_HIDES_ALL_FIREFLIES,
                environment.snow_cover,
            ));
    if amount < FEWEST_FIREFLIES_WORTH_DRAWING {
        return;
    }
    let canvas_width = canvas.width as f32;
    let canvas_height = canvas.height as f32;
    let exact_count = FIREFLIES_ON_A_PERFECT_NIGHT * amount * (canvas_width * canvas_height) / STANDARD_CANVAS_AREA;
    let firefly_count = exact_count.ceil() as u32;
    let seed = environment.seed as u32 ^ FIREFLY_SEED_MIX;
    let time = environment.animation_seconds;
    for firefly_index in 0..firefly_count {
        let first_roll = hash_to_unit(firefly_index, 0, seed);
        let second_roll = hash_to_unit(firefly_index, 1, seed);
        let third_roll = hash_to_unit(firefly_index, 2, seed);
        let fourth_roll = hash_to_unit(firefly_index, 3, seed);

        let blink_seconds = SHORTEST_BLINK_CYCLE_SECONDS + BLINK_CYCLE_SECONDS_RANGE * first_roll;
        let blink_position = ((time / blink_seconds as f64).rem_euclid(1.0) as f32 + second_roll).fract();
        let seconds_into_blink = blink_position * blink_seconds;
        let glow = if seconds_into_blink < FIREFLY_BRIGHTENING_SECONDS {
            seconds_into_blink / FIREFLY_BRIGHTENING_SECONDS
        } else if seconds_into_blink < FIREFLY_FULLY_LIT_UNTIL_SECONDS {
            1.0
        } else if seconds_into_blink < FIREFLY_DARK_AFTER_SECONDS {
            1.0 - (seconds_into_blink - FIREFLY_FULLY_LIT_UNTIL_SECONDS) / FIREFLY_FADING_SECONDS
        } else {
            continue;
        };
        let brightness = glow * (exact_count - firefly_index as f32).clamp(0.0, 1.0);

        let home_x = third_roll * canvas_width;
        let home_y =
            canvas_height * (HIGHEST_FIREFLY_HOME_SHARE_OF_HEIGHT + FIREFLY_HOME_SPAN_SHARE_OF_HEIGHT * fourth_roll);
        let wander_x = FIREFLY_WANDER_WIDTH_PIXELS
            * sine_wave(
                time,
                SLOWEST_SIDEWAYS_WANDER_CYCLES_PER_SECOND + SIDEWAYS_WANDER_RATE_RANGE * second_roll,
                first_roll,
            );
        let wobble_x = FIREFLY_WOBBLE_WIDTH_PIXELS
            * sine_wave(
                time,
                SLOWEST_SIDEWAYS_WOBBLE_CYCLES_PER_SECOND + SIDEWAYS_WOBBLE_RATE_RANGE * third_roll,
                fourth_roll,
            );
        let wander_y = FIREFLY_WANDER_HEIGHT_PIXELS
            * sine_wave(
                time,
                SLOWEST_VERTICAL_WANDER_CYCLES_PER_SECOND + VERTICAL_WANDER_RATE_RANGE * fourth_roll,
                third_roll,
            );
        let wobble_y = FIREFLY_WOBBLE_HEIGHT_PIXELS * sine_wave(time, VERTICAL_WOBBLE_CYCLES_PER_SECOND, second_roll);
        let rise_while_glowing = FIREFLY_RISE_PIXELS * smoothstep(0.0, FIREFLY_RISE_SECONDS, seconds_into_blink);
        let pixel_x = (home_x + wander_x + wobble_x).floor() as i32;
        let pixel_y = (home_y + wander_y + wobble_y - rise_while_glowing).floor() as i32;

        blend_onto_canvas(canvas, pixel_x, pixel_y, FIREFLY_COLOUR, blend_weight(brightness));
        let inner_glow_weight = blend_weight(FIREFLY_INNER_GLOW_STRENGTH * brightness);
        for (offset_x, offset_y) in FIREFLY_INNER_GLOW_OFFSETS {
            glow_onto_canvas(
                canvas,
                pixel_x + offset_x,
                pixel_y + offset_y,
                FIREFLY_INNER_GLOW_COLOUR,
                inner_glow_weight,
            );
        }
        let outer_glow_weight = blend_weight(FIREFLY_OUTER_GLOW_STRENGTH * brightness);
        for (offset_x, offset_y) in FIREFLY_OUTER_GLOW_OFFSETS {
            glow_onto_canvas(
                canvas,
                pixel_x + offset_x,
                pixel_y + offset_y,
                FIREFLY_OUTER_GLOW_COLOUR,
                outer_glow_weight,
            );
        }
    }
}

struct Flock {
    centre_x: f32,
    centre_y: f32,
    direction_down_the_frame: i32,
    bird_count: u32,
    flies_in_a_vee: bool,
    birds_are_small: bool,
    height_above_ground_pixels: f32,
    seed: u32,
}

fn flock_in_the_air(environment: &Environment, canvas_width: f32, canvas_height: f32) -> Option<Flock> {
    let good_flying_weather =
        smoothstep(DAYLIGHT_AT_WHICH_BIRDS_START_FLYING, DAYLIGHT_AT_WHICH_BIRDS_FLY_FREELY, environment.daylight())
            * (1.0 - murkiness_of_air(environment));
    if good_flying_weather < WORST_WEATHER_BIRDS_WILL_FLY_IN {
        return None;
    }
    let chance_seed = environment.seed as u32 ^ FLOCK_SEED_MIX;
    let whole_chances = (environment.animation_seconds / SECONDS_BETWEEN_FLOCK_CHANCES).floor();
    let seconds_into_chance = (environment.animation_seconds - whole_chances * SECONDS_BETWEEN_FLOCK_CHANCES) as f32;
    let chance_index = (whole_chances as i64 & CYCLE_HASH_MASK) as u32;
    if hash_to_unit(chance_index, 0, chance_seed) > CHANCE_OF_A_FLOCK {
        return None;
    }
    let crossing_seconds =
        SHORTEST_CROSSING_SECONDS + CROSSING_SECONDS_RANGE * hash_to_unit(chance_index, 1, chance_seed);
    let crossing_starts_at =
        hash_to_unit(chance_index, 2, chance_seed) * (SECONDS_BETWEEN_FLOCK_CHANCES as f32 - crossing_seconds);
    let crossing_progress = (seconds_into_chance - crossing_starts_at) / crossing_seconds;
    if !(0.0..=1.0).contains(&crossing_progress) {
        return None;
    }
    let direction_down_the_frame = if hash_to_unit(chance_index, 3, chance_seed) < 0.5 { 1 } else { -1 };
    let is_goose_season = matches!(environment.season, Season::Autumn | Season::Winter);
    let chance_of_a_vee = if is_goose_season { CHANCE_OF_A_VEE_IN_GOOSE_SEASON } else { CHANCE_OF_A_VEE_OTHERWISE };
    let flies_in_a_vee = hash_to_unit(chance_index, 4, chance_seed) < chance_of_a_vee;
    let extra_birds = (hash_to_unit(chance_index, 5, chance_seed) * EXTRA_BIRDS_RANGE) as u32;
    let bird_count = if flies_in_a_vee { FEWEST_GEESE + extra_birds } else { FEWEST_SONGBIRDS + extra_birds };
    let birds_are_small = !flies_in_a_vee && hash_to_unit(chance_index, 8, chance_seed) < CHANCE_SONGBIRDS_ARE_SMALL;
    let entry_x = canvas_width
        * (ENTRY_MARGIN_SHARE_OF_WIDTH + ENTRY_SPAN_SHARE_OF_WIDTH * hash_to_unit(chance_index, 6, chance_seed));
    let sideways_lean =
        (hash_to_unit(chance_index, 7, chance_seed) - 0.5) * LARGEST_SIDEWAYS_LEAN_SHARE_OF_WIDTH * canvas_width;
    let distance_along_path = interpolate(-FLOCK_MARGIN_PIXELS, canvas_height + FLOCK_MARGIN_PIXELS, crossing_progress);
    let centre_y = if direction_down_the_frame > 0 { distance_along_path } else { canvas_height - distance_along_path };
    let centre_x = entry_x
        + sideways_lean * (crossing_progress - 0.5)
        + FLOCK_WEAVE_PIXELS * (crossing_progress * FLOCK_WEAVE_RADIANS_PER_CROSSING).sin();
    let height_above_ground_pixels =
        if birds_are_small { SMALL_BIRD_HEIGHT_ABOVE_GROUND_PIXELS } else { LARGE_BIRD_HEIGHT_ABOVE_GROUND_PIXELS };
    Some(Flock {
        centre_x,
        centre_y,
        direction_down_the_frame,
        bird_count,
        flies_in_a_vee,
        birds_are_small,
        height_above_ground_pixels,
        seed: chance_seed ^ chance_index.wrapping_mul(FLOCK_SEED_MULTIPLIER),
    })
}

fn bird_position(flock: &Flock, environment: &Environment, bird_index: u32) -> (i32, i32) {
    let first_roll = hash_to_unit(bird_index, 20, flock.seed);
    let second_roll = hash_to_unit(bird_index, 21, flock.seed);
    let (offset_x, distance_behind_leader) = if flock.flies_in_a_vee {
        let rank_behind_leader = bird_index.div_ceil(2) as f32;
        let side_of_vee = if bird_index & 1 == 0 { 1.0 } else { -1.0 };
        (
            side_of_vee * rank_behind_leader * VEE_SPACING_ACROSS_PIXELS + (first_roll - 0.5) * VEE_RAGGEDNESS_PIXELS,
            rank_behind_leader * VEE_SPACING_BEHIND_PIXELS + (second_roll - 0.5) * VEE_RAGGEDNESS_PIXELS,
        )
    } else {
        ((first_roll - 0.5) * LOOSE_FLOCK_WIDTH_PIXELS, (second_roll - 0.5) * LOOSE_FLOCK_LENGTH_PIXELS)
    };
    let bob_x = BIRD_BOB_WIDTH_PIXELS
        * sine_wave(
            environment.animation_seconds,
            SLOWEST_SIDEWAYS_BOB_CYCLES_PER_SECOND + BOB_RATE_RANGE * second_roll,
            first_roll,
        );
    let bob_y = BIRD_BOB_HEIGHT_PIXELS
        * sine_wave(
            environment.animation_seconds,
            SLOWEST_VERTICAL_BOB_CYCLES_PER_SECOND + BOB_RATE_RANGE * first_roll,
            second_roll,
        );
    let bird_x = flock.centre_x + offset_x + bob_x;
    let bird_y = flock.centre_y - flock.direction_down_the_frame as f32 * distance_behind_leader + bob_y;
    (bird_x.floor() as i32, bird_y.floor() as i32)
}

pub(crate) fn draw_bird_shadows(environment: &Environment, canvas: &mut Canvas) {
    let sun_strength = environment.shadow_strength();
    if sun_strength < WEAKEST_SUN_THAT_CASTS_BIRD_SHADOWS {
        return;
    }
    let Some(flock) = flock_in_the_air(environment, canvas.width as f32, canvas.height as f32) else {
        return;
    };
    let (shadow_right_per_height, shadow_down_per_height) = environment.shadow_direction();
    let shadow_weight = blend_weight(BIRD_SHADOW_OPACITY * sun_strength);
    let height = flock.height_above_ground_pixels;
    let shadow_offset_x = (shadow_right_per_height * height) as i32;
    let shadow_offset_y =
        (height * GROUND_PIXELS_BELOW_BIRD_PER_PIXEL_OF_HEIGHT + shadow_down_per_height * height) as i32;
    for bird_index in 0..flock.bird_count {
        let (bird_x, bird_y) = bird_position(&flock, environment, bird_index);
        let shadow_x = bird_x + shadow_offset_x;
        let shadow_y = bird_y + shadow_offset_y;
        blend_onto_canvas(canvas, shadow_x, shadow_y, BIRD_SHADOW_COLOUR, shadow_weight);
        blend_onto_canvas(canvas, shadow_x - 1, shadow_y, BIRD_SHADOW_COLOUR, shadow_weight);
        if !flock.birds_are_small {
            blend_onto_canvas(canvas, shadow_x + 1, shadow_y, BIRD_SHADOW_COLOUR, shadow_weight);
            blend_onto_canvas(canvas, shadow_x - 2, shadow_y, BIRD_SHADOW_COLOUR, shadow_weight / 2);
            blend_onto_canvas(canvas, shadow_x + 2, shadow_y, BIRD_SHADOW_COLOUR, shadow_weight / 2);
        }
    }
}

pub(crate) fn draw_birds(environment: &Environment, canvas: &mut Canvas) {
    let Some(flock) = flock_in_the_air(environment, canvas.width as f32, canvas.height as f32) else {
        return;
    };
    let dark_body_colour = if flock.flies_in_a_vee { GOOSE_BODY_COLOUR } else { SONGBIRD_BODY_COLOUR };
    let dark_wing_colour = if flock.flies_in_a_vee { GOOSE_WING_COLOUR } else { SONGBIRD_WING_COLOUR };
    let lit_dark_body_colour = colour_in_scene_light(environment, dark_body_colour);
    let lit_dark_wing_colour = colour_in_scene_light(environment, dark_wing_colour);
    let lit_pale_colour = colour_in_scene_light(environment, PALE_BIRD_COLOUR);
    let slowest_wingbeats_per_second =
        if flock.flies_in_a_vee { SLOWEST_GOOSE_WINGBEATS_PER_SECOND } else { SLOWEST_SONGBIRD_WINGBEATS_PER_SECOND };
    for bird_index in 0..flock.bird_count {
        let (bird_x, bird_y) = bird_position(&flock, environment, bird_index);
        let is_far_off_canvas = bird_x < -BIRD_MARGIN_PIXELS
            || bird_y < -BIRD_MARGIN_PIXELS
            || bird_x > canvas.width + BIRD_MARGIN_PIXELS
            || bird_y > canvas.height + BIRD_MARGIN_PIXELS;
        if is_far_off_canvas {
            continue;
        }
        let flap_roll = hash_to_unit(bird_index, 22, flock.seed);
        let is_gliding = !flock.flies_in_a_vee
            && sine_wave(
                environment.animation_seconds,
                SLOWEST_GLIDE_CYCLES_PER_SECOND + GLIDE_RATE_RANGE * flap_roll,
                flap_roll,
            ) > SHARE_OF_GLIDE_CYCLE_SPENT_FLAPPING;
        let wingbeats_per_second = slowest_wingbeats_per_second + WINGBEAT_RATE_RANGE * flap_roll as f64;
        let quarter_beats = ((environment.animation_seconds * wingbeats_per_second
            + flap_roll as f64 * QUARTER_BEATS_PER_WINGBEAT)
            .rem_euclid(QUARTER_BEATS_PER_WINGBEAT)) as usize;
        let wing_pose = if is_gliding { WINGS_LEVEL } else { WING_POSE_THROUGH_A_BEAT[quarter_beats & 3] };

        let ground_under_bird = canvas.pixel_at(bird_x.clamp(0, canvas.width - 1), bird_y.clamp(0, canvas.height - 1));
        let ground_is_too_dark_for_a_dark_bird = pixel_luma(ground_under_bird) < DARKEST_GROUND_THAT_SHOWS_A_DARK_BIRD;
        let (body_colour, wing_colour) = if ground_is_too_dark_for_a_dark_bird {
            (lit_pale_colour, lit_pale_colour)
        } else {
            (lit_dark_body_colour, lit_dark_wing_colour)
        };

        if flock.birds_are_small {
            for (offset_x, offset_y) in SMALL_BIRD_WING_POSES[wing_pose] {
                blend_onto_canvas(canvas, bird_x + offset_x, bird_y + offset_y, body_colour, FULL_BLEND_WEIGHT);
            }
            blend_onto_canvas(canvas, bird_x, bird_y, body_colour, FULL_BLEND_WEIGHT);
            continue;
        }
        for (offset_x, offset_y) in LARGE_BIRD_WING_POSES[wing_pose] {
            blend_onto_canvas(canvas, bird_x + offset_x, bird_y + offset_y, wing_colour, FULL_BLEND_WEIGHT);
        }
        let head_y = bird_y + flock.direction_down_the_frame;
        let tail_y = bird_y - flock.direction_down_the_frame;
        blend_onto_canvas(canvas, bird_x, bird_y, body_colour, FULL_BLEND_WEIGHT);
        blend_onto_canvas(canvas, bird_x, head_y, body_colour, FULL_BLEND_WEIGHT);
        if flock.flies_in_a_vee {
            blend_onto_canvas(canvas, bird_x, tail_y, wing_colour, FULL_BLEND_WEIGHT);
        }
    }
}
