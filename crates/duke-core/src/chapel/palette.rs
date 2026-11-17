use crate::{blend_colours, colour_channels, rgb, Colour, Season};

pub const SUNLIT: usize = 0;
pub const HALF_LIT: usize = 1;
pub const SHADED: usize = 2;
pub const DEEP_SHADE: usize = 3;

pub const STONE_HUE_COUNT: usize = 9;
const DARK_SLATE_BLUE: Colour = 0x434956;
const BLUE_GREY: Colour = 0x626c79;
const GREY: Colour = 0x7d7d7c;
const TAUPE: Colour = 0x8f8573;
const BROWN: Colour = 0x7b6650;
const OCHRE: Colour = 0xb39b6e;
const RUST: Colour = 0xa86f44;
const BUFF: Colour = 0xc6b48f;
const NEAR_BLACK: Colour = 0x3a3532;
const DUKE_STONE_HUES: [Colour; STONE_HUE_COUNT] =
    [DARK_SLATE_BLUE, BLUE_GREY, GREY, TAUPE, BROWN, OCHRE, RUST, BUFF, NEAR_BLACK];

const LIMESTONE_RAMP: [Colour; 5] = [0xf0e8d3, 0xdcd1b8, 0xbdb198, 0x958b79, 0x6c665a];
const LEAD_RAMP: [Colour; 5] = [0xc8d6e6, 0xa9bcd2, 0x8b9fba, 0x6f819d, 0x53627c];
const SLATE_HUES: [Colour; 5] = [0x8d959c, 0x7f8a95, 0x9a9c95, 0x959388, 0x76828e];
const FLAGSTONE_HUES: [Colour; 7] = [0xaab2b3, 0xc0bfb2, 0xcdbf9f, 0xc4ada0, 0xb3bba8, 0xd6d1be, 0x9fa8ae];
const LAWN_THROUGH_THE_YEAR: [(f32, Colour); 10] = [
    (0.0, 0x93ad4c),
    (0.25, 0x86cb3e),
    (0.8, 0x7cc43a),
    (1.3, 0x70ba36),
    (2.0, 0x74b838),
    (2.6, 0x93bf3a),
    (3.0, 0xa2bb44),
    (3.3, 0xa3a957),
    (3.8, 0x9da853),
    (4.0, 0x93ad4c),
];
const HEDGE_THROUGH_THE_YEAR: [(f32, Colour); 5] =
    [(0.0, 0x39702f), (0.5, 0x3a7a30), (1.5, 0x2f6a2d), (3.0, 0x2e6230), (4.0, 0x39702f)];
const COOL_HUE_DEGREES: f32 = 235.0;
const WARM_HUE_DEGREES: f32 = 55.0;
const WET_SHEEN: Colour = 0x2a3442;

pub fn hue_saturation_value(colour: Colour) -> (f32, f32, f32) {
    let (red, green, blue) = colour_channels(colour);
    let (red, green, blue) = (red as f32 / 255.0, green as f32 / 255.0, blue as f32 / 255.0);
    let brightest = red.max(green).max(blue);
    let darkest = red.min(green).min(blue);
    let spread = brightest - darkest;
    let hue = if spread < 1e-5 {
        0.0
    } else if brightest == red {
        60.0 * (((green - blue) / spread).rem_euclid(6.0))
    } else if brightest == green {
        60.0 * ((blue - red) / spread + 2.0)
    } else {
        60.0 * ((red - green) / spread + 4.0)
    };
    let saturation = if brightest < 1e-5 { 0.0 } else { spread / brightest };
    (hue, saturation, brightest)
}

pub fn colour_from_hue_saturation_value(hue: f32, saturation: f32, value: f32) -> Colour {
    let hue = hue.rem_euclid(360.0);
    let saturation = saturation.clamp(0.0, 1.0);
    let value = value.clamp(0.0, 1.0);
    let chroma = value * saturation;
    let second_largest = chroma * (1.0 - ((hue / 60.0).rem_euclid(2.0) - 1.0).abs());
    let lift = value - chroma;
    let (red, green, blue) = match (hue / 60.0) as i32 {
        0 => (chroma, second_largest, 0.0),
        1 => (second_largest, chroma, 0.0),
        2 => (0.0, chroma, second_largest),
        3 => (0.0, second_largest, chroma),
        4 => (second_largest, 0.0, chroma),
        _ => (chroma, 0.0, second_largest),
    };
    rgb(((red + lift) * 255.0) as u8, ((green + lift) * 255.0) as u8, ((blue + lift) * 255.0) as u8)
}

pub fn hue_shifted(colour: Colour, value_factor: f32, hue_shift_degrees: f32, saturation_factor: f32) -> Colour {
    let (hue, saturation, value) = hue_saturation_value(colour);
    let target_hue = if value_factor < 1.0 { COOL_HUE_DEGREES } else { WARM_HUE_DEGREES };
    let mut degrees_to_target = (target_hue - hue).rem_euclid(360.0);
    if degrees_to_target > 180.0 {
        degrees_to_target -= 360.0;
    }
    let shifted_hue = hue + degrees_to_target.clamp(-hue_shift_degrees, hue_shift_degrees);
    colour_from_hue_saturation_value(shifted_hue, saturation * saturation_factor, value * value_factor)
}

pub fn four_step_ramp(base: Colour) -> [Colour; 4] {
    [
        hue_shifted(base, 0.50, 26.0, 1.08),
        hue_shifted(base, 0.74, 12.0, 1.04),
        base,
        hue_shifted(base, 1.20, 12.0, 0.90),
    ]
}

pub fn wetted(colour: Colour, wetness: f32) -> Colour {
    if wetness <= 0.0 {
        return colour;
    }
    let (hue, saturation, value) = hue_saturation_value(colour);
    let soaked = colour_from_hue_saturation_value(hue, (saturation * 1.25 + 0.03).min(1.0), value * 0.70);
    blend_colours(colour, blend_colours(soaked, WET_SHEEN, 0.10), wetness)
}

pub fn smooth_step(start: f32, end: f32, value: f32) -> f32 {
    let fraction = ((value - start) / (end - start)).clamp(0.0, 1.0);
    fraction * fraction * (3.0 - 2.0 * fraction)
}

pub fn bayer_threshold(column: i32, row: i32) -> f32 {
    const BAYER_MATRIX: [u8; 16] = [0, 8, 2, 10, 12, 4, 14, 6, 3, 11, 1, 9, 15, 7, 13, 5];
    (BAYER_MATRIX[((row & 3) * 4 + (column & 3)) as usize] as f32 + 0.5) / 16.0
}

fn colour_at_time_of_year(keys: &[(f32, Colour)], position_in_year: f32) -> Colour {
    if position_in_year <= keys[0].0 {
        return keys[0].1;
    }
    for pair in keys.windows(2) {
        let ((start, start_colour), (end, end_colour)) = (pair[0], pair[1]);
        if position_in_year <= end {
            return blend_colours(start_colour, end_colour, (position_in_year - start) / (end - start));
        }
    }
    keys[keys.len() - 1].1
}

#[derive(Clone, Debug, PartialEq)]
pub struct Look {
    pub season: Season,
    pub season_progress: f32,
    pub snow_cover: f32,
    pub wetness: f32,
    pub sun_side: f32,
    pub seed: u32,
}

impl Look {
    pub fn position_in_year(&self) -> f32 {
        let season_start = match self.season {
            Season::Spring => 0.0,
            Season::Summer => 1.0,
            Season::Autumn => 2.0,
            Season::Winter => 3.0,
        };
        season_start + self.season_progress.clamp(0.0, 1.0)
    }

    pub fn autumn_colour_turn(&self, lateness: f32) -> f32 {
        let progress = self.season_progress;
        match self.season {
            Season::Summer => smooth_step(0.0, 1.0, (progress - 1.0 - (lateness * 0.50 - 0.08)) / 0.26),
            Season::Autumn => smooth_step(0.0, 1.0, (progress - (lateness * 0.50 - 0.08)) / 0.26),
            Season::Winter => 1.0,
            Season::Spring => 0.0,
        }
    }

    pub fn leaves_left(&self, lateness: f32) -> f32 {
        let progress = self.season_progress;
        match self.season {
            Season::Spring => {
                smooth_step(0.02, 0.55, progress - lateness * 0.12) * 0.9 + 0.1 * smooth_step(0.5, 1.0, progress)
            }
            Season::Summer => 1.0,
            Season::Autumn => 1.0 - smooth_step(0.0, 1.0, (progress - (0.50 + lateness * 0.30)) / 0.26),
            Season::Winter => {
                let left_at_the_end_of_autumn = 1.0 - smooth_step(0.0, 1.0, (1.0 - (0.50 + lateness * 0.30)) / 0.26);
                left_at_the_end_of_autumn * (1.0 - smooth_step(0.0, 0.12, progress))
            }
        }
    }

    pub fn blossom(&self) -> f32 {
        match self.season {
            Season::Spring => {
                smooth_step(0.0, 0.14, self.season_progress) * (1.0 - smooth_step(0.36, 0.62, self.season_progress))
            }
            _ => 0.0,
        }
    }

    pub fn spring_freshness(&self) -> f32 {
        match self.season {
            Season::Spring => 1.0 - smooth_step(0.45, 1.0, self.season_progress),
            _ => 0.0,
        }
    }

    pub fn leaf_litter(&self) -> f32 {
        let progress = self.season_progress;
        match self.season {
            Season::Autumn => smooth_step(0.35, 0.95, progress),
            Season::Winter => {
                (0.55 + 0.45 * (1.0 - smooth_step(0.0, 0.12, progress))) * (1.0 - smooth_step(0.0, 0.6, progress))
            }
            _ => 0.0,
        }
    }

    pub fn depth_of_winter(&self) -> f32 {
        match self.season {
            Season::Winter => {
                smooth_step(0.0, 0.12, self.season_progress) * (1.0 - smooth_step(0.88, 1.0, self.season_progress))
            }
            _ => 0.0,
        }
    }

    pub fn bedding_in(&self) -> f32 {
        smooth_step(0.0, 0.1, self.season_progress) * (1.0 - smooth_step(0.9, 1.0, self.season_progress))
    }

    pub fn summer_bloom(&self) -> f32 {
        let progress = self.season_progress;
        match self.season {
            Season::Summer => {
                let carried_over_from_spring = 0.3 * (1.0 - smooth_step(0.0, 0.3, progress));
                smooth_step(0.05, 0.3, progress).max(carried_over_from_spring)
                    * (1.0 - smooth_step(0.85, 1.0, progress) * 0.5)
            }
            Season::Spring => smooth_step(0.85, 1.0, progress) * 0.3,
            Season::Autumn => (1.0 - smooth_step(0.0, 0.2, progress)) * 0.5,
            Season::Winter => 0.0,
        }
    }
}

pub struct Palette {
    pub stone: [[Colour; STONE_HUE_COUNT]; 4],
    pub limestone: [Colour; 5],
    pub lead: [Colour; 5],
    pub slate: [[Colour; 5]; 3],
    pub flagstone: [Colour; 7],
    pub flagstone_joint: Colour,
    pub wood: [Colour; 4],
    pub glass: [Colour; 5],
    pub asphalt: [Colour; 3],
    pub road_paint: Colour,
    pub kerb: [Colour; 2],
    pub concrete: [Colour; 2],
    pub mulch: [Colour; 3],
    pub lawn: [Colour; 4],
    pub lawn_edge: Colour,
    pub hedge: [Colour; 4],
    pub snow: [Colour; 4],
    pub bark: [Colour; 3],
    pub iron: [Colour; 2],
    pub leaf_litter: [Colour; 4],
}

impl Palette {
    pub fn new(look: &Look) -> Palette {
        let wetness = look.wetness;
        let soaked = |colour: Colour| wetted(colour, wetness);
        let damp = |colour: Colour| wetted(colour, wetness * 0.7);
        let mut stone = [[0; STONE_HUE_COUNT]; 4];
        for (hue, &dry_colour) in DUKE_STONE_HUES.iter().enumerate() {
            let colour = soaked(dry_colour);
            stone[SUNLIT][hue] = hue_shifted(colour, 1.13, 6.0, 0.95);
            stone[HALF_LIT][hue] = colour;
            stone[SHADED][hue] = hue_shifted(colour, 0.76, 10.0, 0.95);
            stone[DEEP_SHADE][hue] = hue_shifted(colour, 0.55, 18.0, 0.95);
        }
        let mut slate = [[0; 5]; 3];
        for (hue, &dry_colour) in SLATE_HUES.iter().enumerate() {
            let colour = soaked(dry_colour);
            slate[SUNLIT][hue] = hue_shifted(colour, 1.19, 6.0, 0.95);
            slate[HALF_LIT][hue] = colour;
            slate[SHADED][hue] = hue_shifted(colour, 0.62, 14.0, 1.1);
        }
        let position_in_year = look.position_in_year();
        let lawn_colour = wetted(colour_at_time_of_year(&LAWN_THROUGH_THE_YEAR, position_in_year), wetness * 0.5);
        let hedge_colour = colour_at_time_of_year(&HEDGE_THROUGH_THE_YEAR, position_in_year);
        Palette {
            stone,
            limestone: LIMESTONE_RAMP.map(damp),
            lead: LEAD_RAMP.map(soaked),
            slate,
            flagstone: FLAGSTONE_HUES.map(soaked),
            flagstone_joint: soaked(0x7d7f79),
            wood: [0xc58238, 0xa9652b, 0x874c20, 0x5c3418],
            glass: [0x1f2836, 0x2c3a50, 0x4a3340, 0x2f4d63, 0x5a5338],
            asphalt: [soaked(0x63666c), soaked(0x595c62), soaked(0x4d5057)],
            road_paint: soaked(0xc9c9bd),
            kerb: [damp(0xb3b0a3), damp(0x8d8a80)],
            concrete: [damp(0xb9b5a8), damp(0xa39f93)],
            mulch: [soaked(0x6a4f38), soaked(0x5a4230), soaked(0x7a5c40)],
            lawn: [
                lawn_colour,
                hue_shifted(lawn_colour, 0.90, 5.0, 1.03),
                hue_shifted(lawn_colour, 0.78, 10.0, 1.05),
                hue_shifted(lawn_colour, 1.12, 6.0, 0.95),
            ],
            lawn_edge: hue_shifted(lawn_colour, 0.66, 14.0, 1.05),
            hedge: four_step_ramp(wetted(hedge_colour, wetness * 0.4)),
            snow: [0xf5f8fc, 0xe1e9f3, 0xc6d3e6, 0xa7b7d3],
            bark: [soaked(0x6f5d4b), soaked(0x58483b), soaked(0x3f342c)],
            iron: [0x2b2d33, 0x464a52],
            leaf_litter: [0xb5722e, 0x9a5a2a, 0xc79a3a, 0x7d5a36],
        }
    }
}
