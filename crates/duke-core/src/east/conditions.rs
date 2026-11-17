use crate::east::shading_tools::smoothstep;
use crate::fractal_noise;
use crate::{blend_colours, fold_seed_to_32_bits, rgb, Colour, Environment, Season};

pub mod palette {
    use crate::{rgb, Colour};

    pub const BRICK: [Colour; 5] =
        [rgb(98, 44, 48), rgb(128, 54, 46), rgb(156, 70, 52), rgb(178, 88, 60), rgb(198, 110, 76)];
    pub const SHADED_BRICK: [Colour; 4] = [rgb(70, 34, 44), rgb(92, 42, 44), rgb(114, 50, 46), rgb(134, 60, 50)];
    pub const MARBLE: [Colour; 5] =
        [rgb(124, 127, 148), rgb(164, 166, 178), rgb(204, 203, 200), rgb(232, 229, 220), rgb(251, 248, 239)];
    pub const SLATE: [Colour; 5] =
        [rgb(44, 48, 64), rgb(58, 64, 80), rgb(74, 82, 96), rgb(94, 102, 114), rgb(118, 126, 134)];
    pub const DOME: [Colour; 6] = [
        rgb(58, 66, 78),
        rgb(76, 86, 92),
        rgb(96, 108, 106),
        rgb(120, 132, 122),
        rgb(148, 158, 142),
        rgb(184, 190, 168),
    ];
    pub const BRONZE: [Colour; 3] = [rgb(64, 44, 40), rgb(96, 66, 48), rgb(134, 98, 64)];
    pub const COPPER: [Colour; 4] = [rgb(62, 112, 112), rgb(88, 146, 136), rgb(120, 176, 158), rgb(160, 204, 184)];
    pub const FLAT_ROOF_DECK: [Colour; 4] =
        [rgb(92, 96, 110), rgb(120, 124, 132), rgb(142, 145, 148), rgb(162, 164, 163)];
    pub const ASPHALT: [Colour; 4] = [rgb(104, 104, 112), rgb(124, 124, 128), rgb(140, 140, 140), rgb(154, 153, 150)];
    pub const STREET_ASPHALT: [Colour; 3] = [rgb(70, 72, 84), rgb(86, 88, 98), rgb(100, 102, 110)];
    pub const CONCRETE: [Colour; 4] = [rgb(150, 146, 144), rgb(176, 172, 162), rgb(196, 191, 176), rgb(212, 208, 192)];
    pub const BRICK_PAVER: [Colour; 4] = [rgb(124, 62, 56), rgb(148, 78, 62), rgb(166, 94, 70), rgb(184, 112, 84)];
    pub const MULCH: [Colour; 3] = [rgb(96, 60, 46), rgb(122, 78, 52), rgb(146, 98, 62)];
    pub const GRAVEL: [Colour; 3] = [rgb(160, 144, 116), rgb(184, 168, 134), rgb(202, 188, 154)];
    pub const GLASS: [Colour; 3] = [rgb(40, 52, 74), rgb(62, 80, 104), rgb(110, 140, 168)];
    pub const SNOW: [Colour; 4] = [rgb(160, 176, 210), rgb(198, 210, 232), rgb(228, 236, 246), rgb(246, 249, 253)];
    pub const HEDGE: [Colour; 4] = [rgb(24, 58, 44), rgb(36, 80, 50), rgb(54, 104, 58), rgb(80, 130, 68)];
    pub const GRANITE: [Colour; 4] = [rgb(112, 112, 122), rgb(146, 146, 150), rgb(176, 175, 172), rgb(204, 202, 196)];
    pub const BIDDLE_STONE: [Colour; 4] =
        [rgb(150, 146, 150), rgb(184, 180, 174), rgb(210, 206, 196), rgb(228, 225, 214)];
    pub const TREE_TRUNK: [Colour; 3] = [rgb(58, 46, 46), rgb(86, 68, 58), rgb(118, 98, 80)];
    pub const LIT_WINDOW: Colour = rgb(255, 214, 132);
    pub const COOL_LIT_WINDOW: Colour = rgb(226, 232, 214);
}

const SPRING_LAWN: [Colour; 4] = [rgb(58, 122, 52), rgb(84, 150, 58), rgb(112, 172, 66), rgb(146, 194, 82)];
const SUMMER_LAWN: [Colour; 4] = [rgb(40, 104, 54), rgb(58, 128, 58), rgb(80, 150, 64), rgb(112, 172, 76)];
const AUTUMN_LAWN: [Colour; 4] = [rgb(62, 108, 52), rgb(84, 130, 58), rgb(110, 148, 64), rgb(140, 166, 78)];
const WINTER_LAWN: [Colour; 4] = [rgb(108, 112, 70), rgb(134, 132, 82), rgb(156, 150, 94), rgb(176, 168, 110)];
const WORN_LAWN_IN_WINTER: Colour = rgb(170, 156, 112);
const WORN_LAWN_IN_AUTUMN: Colour = rgb(158, 150, 92);
const WORN_LAWN_IN_GROWING_SEASON: Colour = rgb(150, 160, 86);

const SEASON_PROGRESS_STEPS: f32 = 40.0;
const SNOW_COVER_STEPS: f32 = 20.0;
const WETNESS_STEPS: f32 = 10.0;
const SHADOW_DIRECTION_STEPS: f32 = 12.0;
const SHADOW_STRENGTH_STEPS: f32 = 8.0;

const SNOW_NOISE_SEED: u32 = 0x5A0E;

#[derive(Clone, Debug, PartialEq)]
pub struct CacheKey {
    pub season: Season,
    pub season_progress_step: i32,
    pub snow_cover_step: i32,
    pub wetness_step: i32,
    pub shadow_x_step: i32,
    pub shadow_y_step: i32,
    pub shadow_strength_step: i32,
    pub seed: u64,
}

impl CacheKey {
    pub fn of(environment: &Environment) -> CacheKey {
        let (shadow_x, shadow_y) = environment.shadow_direction();
        let shadow_strength_step = (environment.shadow_strength() * SHADOW_STRENGTH_STEPS).round() as i32;
        let sun_casts_shadows = shadow_strength_step != 0;
        CacheKey {
            season: environment.season,
            season_progress_step: (environment.season_progress.clamp(0.0, 1.0) * SEASON_PROGRESS_STEPS).round() as i32,
            snow_cover_step: (environment.snow_cover.clamp(0.0, 1.0) * SNOW_COVER_STEPS).round() as i32,
            wetness_step: (environment.wetness.clamp(0.0, 1.0) * WETNESS_STEPS).round() as i32,
            shadow_x_step: if sun_casts_shadows { (shadow_x * SHADOW_DIRECTION_STEPS).round() as i32 } else { 0 },
            shadow_y_step: if sun_casts_shadows { (shadow_y * SHADOW_DIRECTION_STEPS).round() as i32 } else { 0 },
            shadow_strength_step,
            seed: environment.seed,
        }
    }
}

pub struct Conditions {
    pub season: Season,
    pub season_progress: f32,
    pub snow_cover: f32,
    pub wetness: f32,
    pub seed: u32,
    pub shadow_x_per_height: f32,
    pub shadow_y_per_height: f32,
    pub shadow_strength: f32,
    pub direction_to_light: [f32; 3],
    pub east_facing_wall_light: f32,
    pub lawn_ramp: [Colour; 4],
    pub worn_lawn_colour: Colour,
    pub leaf_litter: f32,
    pub flower_bloom: f32,
}

fn blended_ramp(from: [Colour; 4], to: [Colour; 4], amount: f32) -> [Colour; 4] {
    [
        blend_colours(from[0], to[0], amount),
        blend_colours(from[1], to[1], amount),
        blend_colours(from[2], to[2], amount),
        blend_colours(from[3], to[3], amount),
    ]
}

impl Conditions {
    pub fn new(key: &CacheKey) -> Conditions {
        let season_progress = key.season_progress_step as f32 / SEASON_PROGRESS_STEPS;
        let shadow_x_per_height = key.shadow_x_step as f32 / SHADOW_DIRECTION_STEPS;

        let noon_lean_to_the_west = 1.0 - (shadow_x_per_height.abs() / 0.45).min(1.0);
        let light_east = -0.6 * shadow_x_per_height.clamp(-1.0, 1.0) - 0.3 * noon_lean_to_the_west;
        let light_south = -0.26;
        let light_up = 0.8;
        let light_length = (light_east * light_east + light_south * light_south + light_up * light_up).sqrt();

        let lawn_ramp = match key.season {
            Season::Spring => {
                blended_ramp(WINTER_LAWN, SPRING_LAWN, smoothstep(0.0, 0.3, season_progress) * 0.85 + 0.15)
            }
            Season::Summer => blended_ramp(SPRING_LAWN, SUMMER_LAWN, smoothstep(0.0, 0.3, season_progress)),
            Season::Autumn => {
                let turning = blended_ramp(SUMMER_LAWN, AUTUMN_LAWN, smoothstep(0.0, 0.5, season_progress));
                blended_ramp(turning, WINTER_LAWN, smoothstep(0.75, 1.1, season_progress) * 0.6)
            }
            Season::Winter => {
                let fading = blended_ramp(AUTUMN_LAWN, WINTER_LAWN, 0.6);
                blended_ramp(fading, WINTER_LAWN, smoothstep(0.0, 0.25, season_progress))
            }
        };
        let worn_lawn_colour = match key.season {
            Season::Winter => WORN_LAWN_IN_WINTER,
            Season::Autumn => WORN_LAWN_IN_AUTUMN,
            Season::Spring | Season::Summer => WORN_LAWN_IN_GROWING_SEASON,
        };
        let leaf_litter = match key.season {
            Season::Autumn => smoothstep(0.3, 0.9, season_progress),
            Season::Winter => 0.55 * (1.0 - smoothstep(0.1, 0.9, season_progress)),
            Season::Spring | Season::Summer => 0.0,
        };
        let flower_bloom = match key.season {
            Season::Spring => smoothstep(0.05, 0.3, season_progress),
            Season::Summer => 1.0 - 0.4 * smoothstep(0.6, 1.0, season_progress),
            Season::Autumn => 0.6 * (1.0 - smoothstep(0.1, 0.5, season_progress)),
            Season::Winter => 0.0,
        };
        Conditions {
            season: key.season,
            season_progress,
            snow_cover: key.snow_cover_step as f32 / SNOW_COVER_STEPS,
            wetness: key.wetness_step as f32 / WETNESS_STEPS,
            seed: fold_seed_to_32_bits(key.seed),
            shadow_x_per_height,
            shadow_y_per_height: key.shadow_y_step as f32 / SHADOW_DIRECTION_STEPS,
            shadow_strength: key.shadow_strength_step as f32 / SHADOW_STRENGTH_STEPS,
            direction_to_light: [light_east / light_length, light_south / light_length, light_up / light_length],
            east_facing_wall_light: (0.5 - 0.55 * shadow_x_per_height).clamp(0.0, 1.0),
            lawn_ramp,
            worn_lawn_colour,
            leaf_litter,
            flower_bloom,
        }
    }

    #[inline]
    pub fn snow_covers_noise_level(&self, noise_level: f32, exposure: f32) -> bool {
        self.snow_cover > 0.0 && self.snow_cover * exposure * 1.35 > noise_level * 0.95 + 0.1
    }

    #[inline]
    pub fn snow_covers_point(&self, metres_east: f32, metres_south: f32, exposure: f32) -> bool {
        if self.snow_cover <= 0.0 {
            return false;
        }
        let noise_level =
            fractal_noise(metres_east * 0.22 + 31.0, metres_south * 0.22 - 17.0, 3, self.seed ^ SNOW_NOISE_SEED);
        self.snow_covers_noise_level(noise_level, exposure)
    }
}
