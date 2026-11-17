use crate::chapel::art::Layer;
use crate::chapel::palette::{Look, Palette, HALF_LIT, SHADED, STONE_HUE_COUNT, SUNLIT};
use crate::chapel::proj::Projection;
use crate::{blend_colours, hash_position, Colour};
use crate::{fractal_noise, hash_fraction};

pub const STAINED_GLASS: u8 = 0;
pub const LIT_ROOM: u8 = 1;
pub const BELFRY_LOUVRES: u8 = 2;
pub const OPEN_DOORWAY: u8 = 3;

const STONE_HUE_WEIGHTS: [u32; STONE_HUE_COUNT] = [9, 17, 15, 17, 11, 11, 8, 5, 7];
const DISTANT_STONE_HUES: [usize; STONE_HUE_COUNT] = [1, 1, 2, 3, 4, 5, 6, 5, 4];

#[derive(Default)]
pub struct ShadowAndLightSources {
    pub casters: Vec<Caster>,
    pub glass_pixels: Vec<GlassPixel>,
    pub lamps: Vec<Lamp>,
}

impl ShadowAndLightSources {
    pub fn add_block_caster(
        &mut self,
        projection: Projection,
        west_metres: f32,
        east_metres: f32,
        south_metres: f32,
        north_metres: f32,
        height_metres: f32,
    ) {
        self.casters.push(Caster::Prism {
            footprint: projection.ground_polygon(&[
                (west_metres, south_metres),
                (east_metres, south_metres),
                (east_metres, north_metres),
                (west_metres, north_metres),
            ]),
            height_pixels: projection.height_in_pixels((south_metres + north_metres) * 0.5, height_metres) as f32,
        });
    }
}

pub enum Caster {
    Prism {
        footprint: Vec<(i32, i32)>,
        height_pixels: f32,
    },
    Canopy {
        trunk_column: i32,
        trunk_row: i32,
        radius_columns: f32,
        radius_rows: f32,
        bottom_height_pixels: f32,
        top_height_pixels: f32,
        density: f32,
    },
    Pole {
        foot_column: i32,
        foot_row: i32,
        height_pixels: f32,
    },
}

pub struct GlassPixel {
    pub index: u32,
    pub kind: u8,
    pub window: u16,
}

#[derive(Clone, Copy)]
pub struct Lamp {
    pub column: i32,
    pub row: i32,
    pub ground_row: i32,
    pub depth_tag: i16,
    pub head_depth_tag: i16,
    pub kind: u8,
}

#[derive(Clone, Copy)]
pub struct DrawingContext<'a> {
    pub projection: Projection,
    pub look: &'a Look,
    pub palette: &'a Palette,
    pub sway: f32,
    pub distant_stonework: bool,
}

impl<'a> DrawingContext<'a> {
    pub fn for_distant_stonework(&self) -> DrawingContext<'a> {
        DrawingContext { distant_stonework: true, ..*self }
    }

    pub fn stone(&self, light_level: usize, column: i32, row: i32) -> Colour {
        let seed = self.look.seed;
        let mut stone_start = column;
        let mut columns_back = 0;
        while columns_back < 3 && hash_position(stone_start, row, seed ^ 0x57) % 100 >= 40 {
            stone_start -= 1;
            columns_back += 1;
        }
        let roll = hash_position(stone_start, row, seed ^ 0xA5A5) % 100;
        let mut hue = 3;
        let mut weight_so_far = 0;
        for (candidate, weight) in STONE_HUE_WEIGHTS.iter().enumerate() {
            weight_so_far += weight;
            if roll < weight_so_far {
                hue = candidate;
                break;
            }
        }
        if self.distant_stonework {
            hue = DISTANT_STONE_HUES[hue];
        }
        self.palette.stone[light_level][hue]
    }

    pub fn light_level_facing(&self, side: f32) -> usize {
        let sun_side = self.look.sun_side;
        if sun_side.abs() < 0.2 {
            return if side < 0.0 { SUNLIT } else { HALF_LIT };
        }
        let away_from_sun = -side.signum() * sun_side;
        if away_from_sun < -0.25 {
            SUNLIT
        } else if away_from_sun > 0.25 {
            SHADED
        } else {
            HALF_LIT
        }
    }

    pub fn snow_colour(&self, column: i32, row: i32, gain: f32, bias: f32, steps_from_light: usize) -> Option<Colour> {
        let snow_cover = self.look.snow_cover;
        if snow_cover <= 0.01 {
            return None;
        }
        let cover = snow_cover * gain + bias;
        if cover <= 0.0 {
            return None;
        }
        let fine_flecks = hash_fraction((column + (row & 1)) >> 1, row, 77);
        let broad_noise = fractal_noise(column as f32 * 0.085, row as f32 * 0.13, 3, self.look.seed ^ 0x51a3);
        let broad_drifts = ((broad_noise - 0.22) / 0.56).clamp(0.0, 1.0);
        let broad_weight = (0.48 + cover * 0.4).clamp(0.48, 0.85);
        let noise = broad_drifts * broad_weight + fine_flecks * (1.0 - broad_weight);
        if noise >= cover {
            return None;
        }
        let depth_into_cover = cover - noise;
        let in_deep_drift = fractal_noise(column as f32 * 0.035, row as f32 * 0.06, 2, self.look.seed ^ 0xD21F) > 0.60;
        let mut shade = steps_from_light;
        if depth_into_cover < 0.06 || in_deep_drift || hash_fraction(column, row, 912) < 0.06 {
            shade += 1;
        }
        Some(self.palette.snow[shade.min(3)])
    }

    pub fn snow_along_ledge(&self, layer: &mut Layer, from_column: i32, to_column: i32, row: i32) {
        let snow_cover = self.look.snow_cover;
        if snow_cover <= 0.03 {
            return;
        }
        for column in from_column.min(to_column)..=from_column.max(to_column) {
            let roll = hash_fraction(column, row, 4401);
            if roll < snow_cover * 1.9 + 0.1 {
                layer.repaint(column, row, self.palette.snow[if roll < snow_cover * 0.25 { 1 } else { 0 }]);
                if snow_cover > 0.7 && hash_fraction(column >> 1, row, 4402) < (snow_cover - 0.6) * 1.2 {
                    layer.repaint(column, row - 1, self.palette.snow[0]);
                }
            }
        }
    }
}

pub fn slate_colour(
    palette: &Palette,
    light_level: usize,
    column: i32,
    row: i32,
    courses_run_down_the_screen: bool,
) -> Colour {
    let (along_course, across_courses) = if courses_run_down_the_screen { (row, column) } else { (column, row) };
    let course = across_courses.div_euclid(2);
    let tile = (along_course + (course & 1) * 2).div_euclid(3);
    let roll = hash_position(tile, course, 0x51A7) % 100;
    let weathering = fractal_noise(column as f32 * 0.045, row as f32 * 0.07, 2, 0x51A8);
    let roll = (roll as f32 + (weathering - 0.5) * 36.0).clamp(0.0, 99.0) as u32;
    let hue = match roll {
        0..=11 => 2,
        12..=46 => 0,
        47..=70 => 4,
        71..=94 => 1,
        _ => 3,
    };
    let colour = palette.slate[light_level.min(SHADED)][hue];
    let is_shadow_under_course = across_courses.rem_euclid(2) == 1;
    if !is_shadow_under_course {
        return colour;
    }
    blend_colours(colour, palette.slate[SHADED][4], if light_level >= SHADED { 0.30 } else { 0.22 })
}

pub fn lead_colour(
    palette: &Palette,
    column: i32,
    row: i32,
    seam_spacing: i32,
    light_level: usize,
    seams_run_down_the_screen: bool,
) -> Colour {
    let (is_seam, stain) = if seams_run_down_the_screen {
        (column.rem_euclid(seam_spacing) == 0, hash_fraction(column / 2, row / 4, 0x1EAD))
    } else {
        (row.rem_euclid(seam_spacing) == 0, hash_fraction(column / 4, row / 2, 0x1EAD))
    };
    let base = match light_level {
        SUNLIT => 1,
        HALF_LIT => 2,
        _ => 3,
    };
    if is_seam || stain < 0.10 {
        palette.lead[(base + 1).min(4)]
    } else if stain > 0.92 {
        palette.lead[base - 1]
    } else {
        palette.lead[base]
    }
}

pub fn flagstone_colour(palette: &Palette, column: i32, row: i32) -> Colour {
    let row_in_band_pair = row.rem_euclid(3);
    let band = row.div_euclid(3) * 2 + if row_in_band_pair < 2 { 0 } else { 1 };
    let is_last_row_of_wide_band = row_in_band_pair == 1;
    let is_break = |at_column: i32| hash_position(at_column, band, 0xF1A6) % 100 < 30;
    let slab_starts_at = |at_column: i32| is_break(at_column) && !is_break(at_column - 1) && !is_break(at_column - 2);
    let mut slab_start = column;
    let mut columns_back = 0;
    while columns_back < 8 && !slab_starts_at(slab_start) {
        slab_start -= 1;
        columns_back += 1;
    }
    let hue = match hash_position(slab_start, band, 0xBEEF) % 100 {
        0..=27 => 0,
        28..=44 => 1,
        45..=58 => 2,
        59..=69 => 3,
        70..=81 => 4,
        82..=90 => 5,
        _ => 6,
    };
    let colour = palette.flagstone[hue];
    if column == slab_start {
        blend_colours(colour, palette.flagstone_joint, 0.38)
    } else if is_last_row_of_wide_band {
        blend_colours(colour, palette.flagstone_joint, 0.16)
    } else {
        colour
    }
}
