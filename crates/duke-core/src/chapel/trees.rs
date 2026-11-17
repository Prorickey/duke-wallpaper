use crate::chapel::art::{Layer, FOLIAGE};
use crate::chapel::palette::{four_step_ramp, hue_shifted};
use crate::chapel::proj::*;
use crate::chapel::tex::{Caster, DrawingContext, ShadowAndLightSources};
use crate::hash_fraction;
use crate::{blend_colours, hash_position, Colour, RandomNumberGenerator, Season};

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum TreeKind {
    Oak,
    Maple,
    Poplar,
    Magnolia,
    Pine,
    Cedar,
    Holly,
    CrepeMyrtle,
    Dogwood,
    Redbud,
    Cherry,
    Ginkgo,
    JapaneseMaple,
    EvergreenShrub,
}

#[derive(Clone, Copy, Debug)]
pub struct Tree {
    pub metres_east: f32,
    pub metres_north: f32,
    pub kind: TreeKind,
    pub height_metres: f32,
    pub crown_radius_metres: f32,
    pub seed: u32,
}

impl Tree {
    pub fn is_evergreen(&self) -> bool {
        matches!(
            self.kind,
            TreeKind::Magnolia | TreeKind::Pine | TreeKind::Cedar | TreeKind::Holly | TreeKind::EvergreenShrub
        )
    }

    fn crown_half_height_metres(&self) -> f32 {
        let crown_height_to_width = match self.kind {
            TreeKind::Oak => 1.25,
            TreeKind::Maple => 1.2,
            TreeKind::Poplar => 1.6,
            TreeKind::Magnolia => 1.3,
            TreeKind::Pine => 0.85,
            TreeKind::Cedar | TreeKind::Holly => 1.0,
            TreeKind::Ginkgo => 1.5,
            TreeKind::EvergreenShrub => 0.75,
            _ => 1.0,
        };
        (self.crown_radius_metres * crown_height_to_width).min(self.height_metres * 0.43)
    }
}

fn summer_leaf_colour(kind: TreeKind, seed: u32) -> Colour {
    let typical_colour = match kind {
        TreeKind::Oak => 0x3f782c,
        TreeKind::Maple => 0x47872e,
        TreeKind::Poplar => 0x4f8d33,
        TreeKind::Magnolia => 0x2f6431,
        TreeKind::Pine => 0x39694a,
        TreeKind::Cedar => 0x2c5f42,
        TreeKind::Holly => 0x29592f,
        TreeKind::CrepeMyrtle => 0x4d8a37,
        TreeKind::Dogwood => 0x539238,
        TreeKind::Redbud => 0x558e36,
        TreeKind::Cherry => 0x4d8834,
        TreeKind::Ginkgo => 0x5d9836,
        TreeKind::JapaneseMaple => 0x7a2f34,
        TreeKind::EvergreenShrub => 0x356c30,
    };
    let (hue, saturation, value) = crate::chapel::palette::hue_saturation_value(typical_colour);
    let hue_jitter = hash_fraction(seed as i32, 1, 0x7EE) - 0.5;
    let shade_jitter = hash_fraction(seed as i32, 2, 0x7EE) - 0.5;
    crate::chapel::palette::colour_from_hue_saturation_value(
        hue + hue_jitter * 16.0,
        saturation * (1.0 + shade_jitter * 0.12),
        value * (1.0 + shade_jitter * 0.16),
    )
}

fn autumn_leaf_colour(kind: TreeKind, seed: u32) -> Colour {
    let pick = |choices: &[u32]| choices[(hash_position(seed as i32, 7, 0xA7) as usize) % choices.len()];
    match kind {
        TreeKind::Oak => pick(&[0xb9872f, 0xa9632a, 0xc49b3b, 0x9a5a2c]),
        TreeKind::Maple => pick(&[0xd9412b, 0xe3682a, 0xc8302d, 0xe89c2e, 0xdd5326]),
        TreeKind::Poplar => pick(&[0xe2ba30, 0xd8a62c, 0xe9c63e]),
        TreeKind::CrepeMyrtle => pick(&[0xda5a2a, 0xc94030, 0xe18a30]),
        TreeKind::Dogwood => pick(&[0xa9323b, 0x8f2f45, 0xb8402f]),
        TreeKind::Redbud => pick(&[0xd9b93a, 0xcfa832]),
        TreeKind::Cherry => pick(&[0xe1832a, 0xdaa030, 0xd0602a]),
        TreeKind::Ginkgo => 0xf0cc30,
        TreeKind::JapaneseMaple => pick(&[0xd0302a, 0xb8262c]),
        _ => 0x6a8a3a,
    }
}

fn spring_blossom_colour(kind: TreeKind, seed: u32) -> Option<Colour> {
    let is_the_rarer_shade = hash_position(seed as i32, 9, 0xB1).is_multiple_of(3);
    match kind {
        TreeKind::Cherry => Some(if is_the_rarer_shade { 0xfbeef2 } else { 0xf6cfe0 }),
        TreeKind::Redbud => Some(if is_the_rarer_shade { 0xd066b0 } else { 0xc2529f }),
        TreeKind::Dogwood => Some(if is_the_rarer_shade { 0xf3b9cc } else { 0xf8f5ea }),
        _ => None,
    }
}

fn crepe_myrtle_flower_colour(seed: u32) -> Colour {
    [0xe85a9a, 0xf6f1f4, 0xb54ab0, 0xe23c50, 0xf08ab8][(hash_position(seed as i32, 11, 0xC2) % 5) as usize]
}

const JUST_UNDER_A_FULL_TURN: f32 = 628.0 / 100.0;

struct Clump {
    column: f32,
    row: f32,
    radius: f32,
}

struct Crown {
    centre_column: f32,
    centre_row: f32,
    radius_columns: f32,
    radius_rows: f32,
}

struct Foliage<'a> {
    ramp: &'a [Colour; 4],
    flowers: Option<(&'a [Colour; 4], f32)>,
    seed: u32,
    snow_gain: f32,
    is_glossy: bool,
}

fn paint_leaf_clump(
    layer: &mut Layer,
    drawing: &DrawingContext,
    clump: &Clump,
    crown: &Crown,
    foliage: &Foliage,
    share_of_leaves_missing: f32,
) {
    let sun_direction_across = drawing.look.sun_side * 0.55;
    let sun_direction_down = -0.75f32;
    let radius = clump.radius.max(1.2);
    let (first_column, last_column) = ((clump.column - radius).floor() as i32, (clump.column + radius).ceil() as i32);
    let (first_row, last_row) = ((clump.row - radius).floor() as i32, (clump.row + radius).ceil() as i32);
    for row in first_row..=last_row {
        for column in first_column..=last_column {
            let across = (column as f32 - clump.column) / radius;
            let down = (row as f32 - clump.row) / radius;
            let distance_squared = across * across + down * down;
            let roll = hash_fraction(column, row, foliage.seed);
            let is_outside = distance_squared > 1.0;
            let is_ragged_edge = distance_squared > 0.74 && roll < 0.34;
            let is_gap = share_of_leaves_missing > 0.0
                && hash_fraction(column, row, foliage.seed ^ 0xD0) < share_of_leaves_missing;
            if is_outside || is_ragged_edge || is_gap {
                continue;
            }
            let facing_away_from_the_sun = -(across * sun_direction_across + down * sun_direction_down);
            let brighter_towards_the_top = 0.24 * (crown.centre_row - row as f32) / crown.radius_rows.max(1.0);
            let light = 0.50 - 0.40 * facing_away_from_the_sun + brighter_towards_the_top + (roll - 0.5) * 0.24;
            let mut shade = if light > 0.76 {
                3
            } else if light > 0.50 {
                2
            } else if light > 0.27 {
                1
            } else {
                0
            };
            if distance_squared > 0.66 && facing_away_from_the_sun > 0.25 {
                shade = if facing_away_from_the_sun > 0.55 { 0 } else { shade.min(1) };
            }
            if foliage.is_glossy && shade == 3 && roll > 0.45 {
                shade = 2;
            }
            let mut colour = foliage.ramp[shade];
            if let Some((flower_ramp, flower_share)) = foliage.flowers {
                if hash_fraction(column, row, foliage.seed ^ 0xB10) < flower_share {
                    colour = flower_ramp[shade.max(1)];
                }
            }
            if shade >= 2 || down < -0.25 {
                if let Some(snow) =
                    drawing.snow_colour(column, row, foliage.snow_gain, -0.14 + 0.2 * (-down).max(0.0), 0)
                {
                    colour = if shade <= 1 { drawing.palette.snow[2] } else { snow };
                }
            }
            layer.put(column, row, colour);
        }
    }
}

fn grow_branch(
    limb_pixels: &mut Vec<(i32, i32, u8)>,
    start: (f32, f32),
    angle: f32,
    length: f32,
    forks_left: u32,
    seed: u32,
    level: u8,
) {
    let end = (start.0 + angle.cos() * length, start.1 + angle.sin() * length);
    let steps = length.abs().ceil().max(1.0) as i32;
    for step in 0..=steps {
        let along = step as f32 / steps as f32;
        limb_pixels.push((
            (start.0 + (end.0 - start.0) * along).round() as i32,
            (start.1 + (end.1 - start.1) * along).round() as i32,
            level,
        ));
    }
    if forks_left == 0 || length < 2.5 {
        return;
    }
    let left_spread = hash_fraction(seed as i32, forks_left as i32, 0xB2A) * 0.35;
    let right_spread = hash_fraction(seed as i32, forks_left as i32 + 9, 0xB2A) * 0.35;
    grow_branch(
        limb_pixels,
        end,
        angle - 0.38 - left_spread,
        length * 0.64,
        forks_left - 1,
        seed.wrapping_mul(31).wrapping_add(1),
        level + 1,
    );
    grow_branch(
        limb_pixels,
        end,
        angle + 0.34 + right_spread,
        length * 0.60,
        forks_left - 1,
        seed.wrapping_mul(37).wrapping_add(2),
        level + 1,
    );
}

fn bark_colours(tree: &Tree, drawing: &DrawingContext) -> [Colour; 3] {
    if tree.kind == TreeKind::CrepeMyrtle {
        [0xb7a995, 0x9a8c7a, 0x7a6d60]
    } else {
        drawing.palette.bark
    }
}

fn draw_bare_branches(
    layer: &mut Layer,
    drawing: &DrawingContext,
    tree: &Tree,
    trunk_column: i32,
    crown: &Crown,
    top_row: f32,
    twig_density: f32,
) {
    let is_big = crown.radius_columns > 13.0;
    let bark = bark_colours(tree, drawing);
    let mut limb_pixels: Vec<(i32, i32, u8)> = Vec::new();
    let limb_count = if is_big { 6 + (tree.seed % 2) as i32 } else { 4 };
    for limb in 0..limb_count {
        let share_of_the_fan = (limb as f32 + 0.5) / limb_count as f32;
        let angle = -std::f32::consts::PI * (0.04 + 0.92 * share_of_the_fan)
            + (hash_fraction(tree.seed as i32, limb, 0x5CE) - 0.5) * 0.3;
        let reach_to_the_crown_edge =
            ((angle.cos() * crown.radius_columns).powi(2) + (angle.sin() * crown.radius_rows * 1.2).powi(2)).sqrt();
        let forks = if is_big { 3 } else { 2 };
        grow_branch(
            &mut limb_pixels,
            (trunk_column as f32, top_row),
            angle,
            reach_to_the_crown_edge * 0.52,
            forks,
            tree.seed.wrapping_add(limb as u32 * 977),
            0,
        );
    }

    let frost = drawing.look.snow_cover.clamp(0.0, 1.0);
    let twig_colour = blend_colours(blend_colours(bark[0], 0x8d8378, 0.45), 0xb9c2d2, frost * 0.75);
    let twig_density = twig_density * (1.0 - 0.45 * frost);
    let (first_column, last_column) =
        ((crown.centre_column - crown.radius_columns) as i32, (crown.centre_column + crown.radius_columns) as i32);
    let (first_row, last_row) =
        ((crown.centre_row - crown.radius_rows) as i32, (crown.centre_row + crown.radius_rows) as i32);
    for row in first_row..=last_row {
        for column in first_column..=last_column {
            let across = (column as f32 - crown.centre_column) / crown.radius_columns;
            let down = (row as f32 - crown.centre_row) / crown.radius_rows;
            let distance_squared = across * across + down * down;
            if distance_squared < 1.0
                && hash_fraction(column, row, tree.seed ^ 0x7716) < 0.15 * twig_density * (1.15 - distance_squared)
            {
                layer.put(column, row, twig_colour);
            }
        }
    }

    let pale_limb = blend_colours(bark[0], 0xb0a696, 0.42);
    for &(column, row, level) in &limb_pixels {
        let colour = match level {
            0 => bark[1],
            1 => bark[0],
            _ => pale_limb,
        };
        layer.put(column, row, colour);
        if level == 0 && is_big {
            layer.put(column + 1, row, bark[1]);
        }
    }
    let snow_cover = drawing.look.snow_cover;
    if snow_cover > 0.08 {
        for &(column, row, level) in &limb_pixels {
            if level <= 2 && hash_fraction(column, row, 0x5709) < snow_cover * 1.1 {
                layer.put(column, row - 1, drawing.palette.snow[if level == 0 { 0 } else { 1 }]);
            }
        }
    }
}

pub fn draw_tree(
    layer: &mut Layer,
    drawing: &DrawingContext,
    sources: &mut ShadowAndLightSources,
    tree: &Tree,
    owner_id: u16,
) {
    let projection = drawing.projection;
    let look = drawing.look;
    let seed = tree.seed;
    let is_evergreen = tree.is_evergreen();
    let has_a_visible_trunk = tree.kind != TreeKind::EvergreenShrub && tree.kind != TreeKind::Holly;
    let (trunk_column, trunk_foot_row) = projection.ground_pixel((tree.metres_east, tree.metres_north));
    let pixels_per_metre_across = projection.pixels_per_metre_across(tree.metres_north);
    let pixels_per_metre_up = (projection.exact_pixel_y(tree.metres_north, 0.0)
        - projection.exact_pixel_y(tree.metres_north, tree.height_metres))
        / tree.height_metres.max(0.1);
    let height_pixels = tree.height_metres * pixels_per_metre_up;
    let crown_half_height_metres = tree.crown_half_height_metres();
    let trunk_showing_pixels = if has_a_visible_trunk { (height_pixels * 0.2).clamp(3.0, 9.0) } else { 0.0 };
    let crown_radius_rows = ((tree.crown_radius_metres * projection.pixels_per_metre_of_depth(tree.metres_north))
        .powi(2)
        + (crown_half_height_metres * pixels_per_metre_up).powi(2))
    .sqrt()
    .min((height_pixels - trunk_showing_pixels) * 0.5)
    .max(2.0);
    let crown = if projection.is_plan_view() {
        Crown {
            centre_column: trunk_column as f32,
            centre_row: trunk_foot_row as f32,
            radius_columns: tree.crown_radius_metres * pixels_per_metre_across,
            radius_rows: tree.crown_radius_metres * pixels_per_metre_across,
        }
    } else {
        Crown {
            centre_column: trunk_column as f32,
            centre_row: trunk_foot_row as f32 - height_pixels + crown_radius_rows,
            radius_columns: tree.crown_radius_metres * pixels_per_metre_across,
            radius_rows: crown_radius_rows,
        }
    };
    layer.pen_depth = depth_tag(tree.metres_north);
    layer.pen_owner = 0;
    layer.pen_flags = 0;

    let timing_roll = hash_fraction(seed as i32, 3, 0x0FF);
    let seasonal_lateness = match tree.kind {
        TreeKind::Maple | TreeKind::Dogwood | TreeKind::Cherry | TreeKind::JapaneseMaple => timing_roll * 0.6,
        TreeKind::Oak => 0.35 + timing_roll * 0.65,
        _ => 0.15 + timing_roll * 0.75,
    };
    let leaves_left = if is_evergreen { 1.0 } else { look.leaves_left(seasonal_lateness) };
    let autumn_turn = if is_evergreen { 0.0 } else { look.autumn_colour_turn(seasonal_lateness) };
    let mut leaf_colour = summer_leaf_colour(tree.kind, seed);
    if is_evergreen {
        leaf_colour = blend_colours(leaf_colour, hue_shifted(leaf_colour, 0.93, 4.0, 0.9), look.depth_of_winter());
        leaf_colour = blend_colours(leaf_colour, 0x6aa83e, look.spring_freshness() * look.bedding_in() * 0.12);
    } else {
        leaf_colour = blend_colours(leaf_colour, 0x93c84a, look.spring_freshness() * 0.62);
        leaf_colour = blend_colours(leaf_colour, autumn_leaf_colour(tree.kind, seed), autumn_turn);
        if matches!(look.season, Season::Autumn | Season::Winter) && leaves_left < 0.6 {
            leaf_colour = blend_colours(leaf_colour, 0x8f6a3a, (0.6 - leaves_left) * 0.7);
        }
    }
    let leaf_ramp = four_step_ramp(crate::chapel::palette::wetted(leaf_colour, look.wetness * 0.35));

    let mut flower_ramp = [0; 4];
    let mut flower_share = 0.0;
    let mut leaf_cover = leaves_left;
    let blossom = look.blossom();
    if let Some(blossom_colour) = spring_blossom_colour(tree.kind, seed) {
        if blossom > 0.02 {
            flower_ramp = [
                hue_shifted(blossom_colour, 0.62, 16.0, 1.3),
                hue_shifted(blossom_colour, 0.82, 8.0, 1.15),
                blossom_colour,
                hue_shifted(blossom_colour, 1.08, 4.0, 0.7),
            ];
            flower_share = (blossom / (blossom + leaves_left * 0.55 + 0.05)).min(0.95);
            leaf_cover = leaf_cover.max(blossom);
        }
    }
    let summer_bloom = look.summer_bloom();
    if tree.kind == TreeKind::CrepeMyrtle && summer_bloom > 0.05 {
        let bloom_colour = crepe_myrtle_flower_colour(seed);
        flower_ramp = [
            leaf_ramp[0],
            hue_shifted(bloom_colour, 0.8, 8.0, 1.1),
            bloom_colour,
            hue_shifted(bloom_colour, 1.1, 4.0, 0.75),
        ];
        flower_share = 0.5 * summer_bloom;
    }

    let trunk_width = ((tree.crown_radius_metres * 0.095 * pixels_per_metre_across).round() as i32).clamp(1, 4);
    let trunk_top_row =
        if tree.kind == TreeKind::Pine { crown.centre_row } else { crown.centre_row + crown.radius_rows * 0.30 };
    let bark = bark_colours(tree, drawing);
    if has_a_visible_trunk {
        let is_lit_from_the_left = look.sun_side <= 0.0;
        for row in trunk_top_row as i32..=trunk_foot_row {
            let flare = if trunk_foot_row - row < 2 && trunk_width >= 2 { 1 } else { 0 };
            for offset in -flare..trunk_width + flare {
                let is_left_edge = offset <= 0;
                let is_right_edge = offset >= trunk_width - 1;
                let colour = if trunk_width == 1 {
                    bark[1]
                } else if (is_left_edge && is_lit_from_the_left) || (is_right_edge && !is_lit_from_the_left) {
                    bark[0]
                } else if is_left_edge || is_right_edge {
                    bark[2]
                } else {
                    bark[1]
                };
                layer.put(trunk_column - trunk_width / 2 + offset, row, colour);
            }
        }
    }

    sources.casters.push(Caster::Canopy {
        trunk_column,
        trunk_row: trunk_foot_row,
        radius_columns: crown.radius_columns * 0.92,
        radius_rows: (tree.crown_radius_metres * projection.pixels_per_metre_of_depth(tree.metres_north) * 0.92)
            .max(1.0),
        bottom_height_pixels: ((tree.height_metres - 2.0 * crown_half_height_metres).max(0.5)) * pixels_per_metre_up,
        top_height_pixels: height_pixels,
        density: if leaf_cover > 0.7 { 1.0 } else { 0.22 + 0.78 * leaf_cover },
    });

    if !is_evergreen && leaf_cover < 0.93 {
        draw_bare_branches(layer, drawing, tree, trunk_column, &crown, trunk_top_row + 1.0, 1.0 - leaf_cover * 0.6);
    }
    if leaf_cover < 0.04 {
        return;
    }

    layer.pen_owner = owner_id;
    layer.pen_flags = FOLIAGE;
    let foliage = Foliage {
        ramp: &leaf_ramp,
        flowers: if flower_share > 0.0 { Some((&flower_ramp, flower_share)) } else { None },
        seed,
        snow_gain: if is_evergreen { 1.9 } else { 1.5 },
        is_glossy: tree.kind == TreeKind::Magnolia,
    };
    let sway_amplitude = (0.045 * height_pixels).clamp(0.8, 2.0);
    let sway_at_row = |row: f32| -> f32 {
        (drawing.sway * sway_amplitude * ((trunk_foot_row as f32 - row) / height_pixels.max(1.0)).clamp(0.0, 1.0))
            .round()
    };
    let mut rng = RandomNumberGenerator::new(seed as u64 ^ 0x7EE5);

    match tree.kind {
        TreeKind::Cedar | TreeKind::Holly => {
            let tier_count = ((height_pixels / 5.0) as i32).clamp(3, 14);
            let lowest_tier_row =
                trunk_foot_row as f32 - height_pixels * if tree.kind == TreeKind::Holly { 0.02 } else { 0.12 };
            let tip_row = trunk_foot_row as f32 - height_pixels;
            for tier in 0..tier_count {
                let share_of_the_way_up = tier as f32 / tier_count as f32;
                let tier_row = lowest_tier_row - share_of_the_way_up * (lowest_tier_row - tip_row);
                let tier_half_width = crown.radius_columns * (1.0 - share_of_the_way_up).powf(0.85) + 0.8;
                let clumps_in_tier = ((tier_half_width / 3.2) as i32).max(1);
                for clump_number in 0..clumps_in_tier {
                    let position_across = if clumps_in_tier == 1 {
                        0.0
                    } else {
                        (clump_number as f32 / (clumps_in_tier - 1) as f32) * 2.0 - 1.0
                    };
                    let clump = Clump {
                        column: trunk_column as f32
                            + position_across * (tier_half_width - 1.5)
                            + rng.float_between(-0.6, 0.6)
                            + sway_at_row(tier_row),
                        row: tier_row + rng.float_between(-0.8, 0.8),
                        radius: (tier_half_width * 0.55).clamp(1.8, 5.0),
                    };
                    paint_leaf_clump(layer, drawing, &clump, &crown, &foliage, 0.0);
                }
            }
            let tip_pixel_row = trunk_foot_row - height_pixels as i32;
            layer.put(trunk_column + sway_at_row(tip_pixel_row as f32) as i32, tip_pixel_row, leaf_ramp[2]);
        }
        TreeKind::Pine => {
            let clump_count = 6 + (seed % 4) as i32;
            let mut clumps: Vec<Clump> = (0..clump_count)
                .map(|_| {
                    let angle = rng.float_between(0.0, std::f32::consts::TAU);
                    let distance_out = rng.float_between(0.15, 0.85);
                    let row = crown.centre_row + angle.sin() * distance_out * crown.radius_rows * 0.9;
                    Clump {
                        column: crown.centre_column
                            + angle.cos() * distance_out * crown.radius_columns * 0.9
                            + sway_at_row(row),
                        row,
                        radius: (crown.radius_columns * rng.float_between(0.34, 0.5)).max(2.5),
                    }
                })
                .collect();
            clumps.sort_by(|upper, lower| upper.row.total_cmp(&lower.row));
            for clump in &clumps {
                paint_leaf_clump(layer, drawing, clump, &crown, &foliage, 0.10);
            }
        }
        _ => {
            let crown_area = crown.radius_columns * crown.radius_rows;
            let (clump_count_factor, clump_size_factor) = match tree.kind {
                TreeKind::Magnolia => (0.75, 1.25),
                TreeKind::Oak => (1.25, 0.92),
                TreeKind::EvergreenShrub => (0.8, 1.2),
                _ => (1.0, 1.0),
            };
            let clump_count = ((crown_area / 20.0 * clump_count_factor) as i32).clamp(4, 30);
            let typical_clump_radius = (crown_area / clump_count as f32).sqrt() * 1.12 * clump_size_factor;
            let spiral_phase = rng.float_between(0.0, JUST_UNDER_A_FULL_TURN);
            let clumps_still_in_leaf = (clump_count as f32 * leaf_cover.min(1.0)).ceil() as i32;
            if leaf_cover > 0.85 {
                let underfill_column = crown.centre_column + sway_at_row(crown.centre_row) * 0.5;
                let underfill_row = crown.centre_row + crown.radius_rows * 0.08;
                let (radius_columns, radius_rows) = (crown.radius_columns * 0.78, crown.radius_rows * 0.78);
                for row in (underfill_row - radius_rows) as i32..=(underfill_row + radius_rows) as i32 {
                    for column in
                        (underfill_column - radius_columns) as i32..=(underfill_column + radius_columns) as i32
                    {
                        let across = (column as f32 - underfill_column) / radius_columns;
                        let down = (row as f32 - underfill_row) / radius_rows;
                        if across * across + down * down < 1.0 {
                            layer.put(
                                column,
                                row,
                                if hash_position(column, row, seed).is_multiple_of(4) {
                                    leaf_ramp[1]
                                } else {
                                    leaf_ramp[0]
                                },
                            );
                        }
                    }
                }
            }
            let mut clumps: Vec<Clump> = (0..clump_count)
                .map(|clump_number| {
                    let distance_out = ((clump_number as f32 + 0.5) / clump_count as f32).sqrt() * 0.80;
                    let angle = clump_number as f32 * 2.399_963 + spiral_phase;
                    let row = crown.centre_row
                        + angle.sin() * distance_out * crown.radius_rows
                        + rng.float_between(-0.08, 0.08) * crown.radius_rows;
                    Clump {
                        column: crown.centre_column
                            + angle.cos() * distance_out * crown.radius_columns
                            + rng.float_between(-0.08, 0.08) * crown.radius_columns
                            + sway_at_row(row),
                        row,
                        radius: typical_clump_radius * rng.float_between(0.82, 1.22),
                    }
                })
                .collect();
            clumps.truncate(clumps_still_in_leaf.max(1) as usize);
            clumps.sort_by(|upper, lower| upper.row.total_cmp(&lower.row));
            let share_of_leaves_missing = if leaf_cover < 0.93 { (0.5 - leaf_cover * 0.4).max(0.0) } else { 0.0 };
            for clump in &clumps {
                paint_leaf_clump(layer, drawing, clump, &crown, &foliage, share_of_leaves_missing);
            }
            if tree.kind == TreeKind::Magnolia && summer_bloom > 0.1 {
                for _ in 0..(crown.radius_columns * 0.5 * summer_bloom) as i32 {
                    let angle = rng.float_between(0.0, JUST_UNDER_A_FULL_TURN);
                    let distance_out = rng.float_between(0.0, 0.8);
                    let column = (crown.centre_column + angle.cos() * distance_out * crown.radius_columns) as i32;
                    let row = (crown.centre_row + angle.sin() * distance_out * crown.radius_rows) as i32;
                    let owner_index =
                        (row.clamp(0, layer.height - 1) * layer.width + column.clamp(0, layer.width - 1)) as usize;
                    if layer.is_drawn(column, row) && layer.owner[owner_index] == owner_id {
                        layer.put(column, row, 0xf4f1e2);
                    }
                }
            }
        }
    }
    layer.pen_owner = 0;
    layer.pen_flags = 0;
}

pub const TREES_LIGHTNING_CAN_STRIKE: &[(f32, f32)] = &[
    (-27.0, -1.0),
    (25.5, -3.5),
    (-76.0, 44.0),
    (-80.5, 20.0),
    (-49.0, 50.0),
    (-40.0, 92.0),
    (26.0, 91.0),
    (58.0, 82.0),
    (-27.0, -18.5),
    (27.6, -20.5),
];

const WOODS_OUTLINE: &[(f32, f32)] = &[
    (-23.3, 156.7),
    (-10.1, 198.6),
    (104.6, 204.4),
    (102.5, 197.9),
    (84.7, 195.0),
    (73.1, 185.7),
    (70.3, 157.1),
    (82.5, 147.7),
    (90.7, 136.2),
    (90.7, 126.5),
    (112.0, 98.3),
    (111.1, 86.7),
    (87.6, 79.4),
    (60.9, 79.0),
    (42.9, 83.0),
    (25.0, 91.1),
    (1.8, 118.8),
    (-27.6, 114.5),
];

fn is_inside_outline(outline: &[(f32, f32)], metres_east: f32, metres_north: f32) -> bool {
    let mut is_inside = false;
    for corner in 0..outline.len() {
        let (start, end) = (outline[corner], outline[(corner + 1) % outline.len()]);
        let edge_crosses_this_latitude = (start.1 > metres_north) != (end.1 > metres_north);
        if edge_crosses_this_latitude
            && metres_east < start.0 + (metres_north - start.1) / (end.1 - start.1) * (end.0 - start.0)
        {
            is_inside = !is_inside;
        }
    }
    is_inside
}

pub fn plant(seed: u32) -> Vec<Tree> {
    use TreeKind::*;
    let mut trees: Vec<Tree> = Vec::new();
    let mut add =
        |metres_east: f32, metres_north: f32, kind: TreeKind, height_metres: f32, crown_radius_metres: f32| {
            let tree_number = trees.len() as u32 + 1;
            trees.push(Tree {
                metres_east,
                metres_north,
                kind,
                height_metres,
                crown_radius_metres,
                seed: seed.wrapping_mul(2654435761).wrapping_add(tree_number * 7919),
            });
        };

    add(-27.0, -1.0, Magnolia, 14.0, 5.6);
    add(25.5, -3.5, Magnolia, 16.0, 8.0);
    add(-27.0, -18.5, CrepeMyrtle, 7.5, 3.8);
    add(27.6, -20.5, CrepeMyrtle, 8.0, 4.4);

    for &(metres_east, metres_north, crown_radius_metres) in &[
        (-60.0f32, 35.0f32, 4.2f32),
        (-50.0, 35.0, 4.6),
        (-45.0, 42.5, 4.0),
        (-55.0, 25.0, 4.4),
        (-45.0, 22.5, 4.2),
        (-37.5, 30.0, 4.6),
        (-32.5, 20.0, 3.8),
        (-50.0, 17.5, 3.6),
        (-40.0, 47.5, 4.0),
        (-30.0, 40.0, 3.8),
        (-57.0, 24.0, 3.4),
        (-44.0, 14.0, 3.6),
        (-38.0, 16.0, 3.4),
        (-30.5, 14.5, 3.2),
        (-33.0, 26.0, 3.6),
    ] {
        let kind = if crown_radius_metres > 4.1 {
            Magnolia
        } else if (metres_east * 3.0) as i32 % 2 == 0 {
            Holly
        } else {
            Cedar
        };
        add(metres_east, metres_north, kind, 7.0 + crown_radius_metres * 1.5, crown_radius_metres);
    }
    add(-76.0, 44.0, Oak, 20.0, 8.0);
    add(-49.0, 50.0, Oak, 18.0, 6.4);
    add(-80.5, 20.0, Oak, 19.0, 7.0);
    add(-38.0, 25.0, Maple, 15.0, 5.4);
    add(-70.0, 60.0, Oak, 19.0, 6.6);
    add(-58.0, 62.0, Maple, 15.0, 5.4);
    add(-72.0, 36.0, Poplar, 21.0, 5.0);
    add(-56.0, 46.0, Maple, 14.0, 5.0);
    add(-80.0, 50.0, Oak, 18.0, 6.0);
    add(-79.0, 64.0, Maple, 14.0, 5.0);
    add(-52.0, 66.0, Dogwood, 6.0, 3.0);
    add(-44.0, 60.0, Redbud, 6.0, 3.0);
    add(-38.0, 72.0, Dogwood, 6.5, 3.2);
    add(-33.0, 60.0, JapaneseMaple, 4.5, 2.8);
    add(-47.0, 74.0, Cherry, 6.0, 3.0);
    add(-29.0, 59.0, Cherry, 5.5, 3.0);

    add(-64.4, 94.6, CrepeMyrtle, 5.5, 2.6);
    for &(metres_east, metres_north) in &[
        (-40.0f32, 92.0f32),
        (-36.0, 84.0),
        (-42.0, 82.0),
        (-85.0, 125.0),
        (-80.0, 118.0),
        (-84.0, 110.0),
        (-78.0, 106.0),
    ] {
        add(metres_east, metres_north, Magnolia, 14.0, 4.4);
    }
    add(-72.0, 118.0, Oak, 18.0, 6.2);
    add(-66.0, 126.0, Maple, 15.0, 5.4);

    for plaza_row in 0..8 {
        for plaza_column in 0..4 {
            add(-104.0 + plaza_column as f32 * 4.5, 19.0 - plaza_row as f32 * 5.0, Ginkgo, 8.0, 2.3);
        }
    }

    add(19.0, 102.0, Magnolia, 13.0, 4.0);
    add(4.4, 100.0, Cedar, 12.0, 3.0);
    add(-10.0, 104.0, Holly, 9.0, 3.0);
    add(26.0, 91.0, Oak, 18.0, 6.0);
    add(17.0, 78.5, EvergreenShrub, 3.0, 2.0);
    add(-22.6, 55.6, EvergreenShrub, 4.2, 3.2);

    add(36.0, 38.0, Holly, 6.0, 2.4);
    add(36.0, 22.0, JapaneseMaple, 4.5, 2.8);
    add(36.2, 44.5, EvergreenShrub, 2.4, 2.2);
    add(76.0, 42.0, Cherry, 7.0, 2.8);
    add(82.0, 38.5, Cherry, 7.0, 2.8);
    add(80.0, 32.5, Cherry, 6.5, 2.6);
    add(84.0, -19.5, Cherry, 6.0, 2.6);
    add(87.0, -25.0, Cherry, 6.0, 2.6);
    add(105.0, 62.5, Maple, 10.0, 3.6);
    add(101.5, 56.5, Maple, 9.0, 3.4);
    add(108.5, 56.5, Maple, 9.5, 3.4);
    for &(metres_east, metres_north) in
        &[(104.0f32, 25.0f32), (106.0, 18.0), (107.0, 11.0), (108.0, 4.0), (105.0, -3.0)]
    {
        add(metres_east, metres_north, Cedar, 16.0, 3.6);
    }

    for &(metres_east, metres_north) in &[
        (1.3f32, 136.0f32),
        (4.4, 125.0),
        (29.0, 139.0),
        (35.6, 138.0),
        (42.0, 123.0),
        (46.6, 117.5),
        (18.4, 103.5),
        (60.0, 140.0),
        (66.0, 133.0),
        (72.0, 138.0),
        (78.0, 128.0),
        (63.0, 101.0),
        (70.0, 97.0),
        (76.0, 100.0),
        (44.0, 88.0),
        (52.0, 85.0),
        (58.0, 82.0),
        (48.0, 80.5),
        (90.0, 105.0),
        (93.0, 98.0),
    ] {
        add(metres_east, metres_north, Pine, 22.0, 4.4);
    }

    const WOODLAND_GRID_SPACING_EAST: f32 = 9.6;
    const WOODLAND_GRID_SPACING_NORTH: f32 = 8.6;
    let mut rng = RandomNumberGenerator::new(seed as u64 ^ 0xF0E5);
    let mut grid_metres_north = 78.0;
    while grid_metres_north < 175.0 {
        let mut grid_metres_east = -150.0;
        while grid_metres_east < 150.0 {
            let metres_east = grid_metres_east + rng.float_between(-0.3, 0.3) * WOODLAND_GRID_SPACING_EAST;
            let metres_north = grid_metres_north + rng.float_between(-0.3, 0.3) * WOODLAND_GRID_SPACING_EAST;
            let species_roll = rng.next_fraction();
            let size_roll = rng.next_fraction();
            grid_metres_east += WOODLAND_GRID_SPACING_EAST;
            let car_park_aisle = crate::chapel::props::car_park_aisle_metres_east(metres_north);
            let is_in_the_car_park =
                metres_east > car_park_aisle - 12.0 && metres_east < car_park_aisle + 12.5 && metres_north > 97.0;
            let distance_from_the_drop_off_circle =
                ((metres_east + 64.4).powi(2) + (metres_north - 94.6).powi(2)).sqrt();
            let is_in_the_woods = is_inside_outline(WOODS_OUTLINE, metres_east, metres_north);
            let is_beside_the_car_park =
                metres_north > 108.0 && metres_east > -34.0 && metres_east < 2.0 && !is_in_the_car_park;
            let is_west_of_the_drive = metres_north > 100.0
                && metres_east > -93.0
                && metres_east < -62.0
                && !is_in_the_car_park
                && distance_from_the_drop_off_circle > 22.0;
            let is_east_of_the_east_circle = metres_east > 112.0 && metres_north > 20.0 && metres_north < 100.0;
            if !(is_in_the_woods || is_beside_the_car_park || is_west_of_the_drive || is_east_of_the_east_circle) {
                continue;
            }
            let (kind, height_metres, crown_radius_metres) = if species_roll < 0.42 {
                (Oak, 17.0 + size_roll * 7.0, 5.6 + size_roll * 1.8)
            } else if species_roll < 0.64 {
                (Poplar, 20.0 + size_roll * 7.0, 4.8 + size_roll * 1.4)
            } else if species_roll < 0.88 {
                (Maple, 13.0 + size_roll * 6.0, 4.8 + size_roll * 1.6)
            } else {
                (Pine, 22.0 + size_roll * 6.0, 4.2 + size_roll * 1.2)
            };
            add(metres_east, metres_north, kind, height_metres, crown_radius_metres);
        }
        grid_metres_north += WOODLAND_GRID_SPACING_NORTH;
    }
    trees
}
