use crate::chapel::art::Layer;
use crate::chapel::palette::{hue_shifted, smooth_step, wetted};
use crate::chapel::proj::*;
use crate::chapel::tex::{flagstone_colour, DrawingContext};
use crate::{blend_colours, hash_position, Colour, Season};
use crate::{fractal_noise, hash_fraction};

type GroundPoint = (f32, f32);

fn strip_along(projection: Projection, from: GroundPoint, to: GroundPoint, width_metres: f32) -> Vec<(i32, i32)> {
    let (east, north) = (to.0 - from.0, to.1 - from.1);
    let length = (east * east + north * north).sqrt().max(1e-3);
    let (side_east, side_north) = (-north / length * width_metres * 0.5, east / length * width_metres * 0.5);
    projection.ground_polygon(&[
        (from.0 + side_east, from.1 + side_north),
        (to.0 + side_east, to.1 + side_north),
        (to.0 - side_east, to.1 - side_north),
        (from.0 - side_east, from.1 - side_north),
    ])
}

#[derive(Clone, Copy, PartialEq)]
enum Paving {
    Flagstone,
    Concrete,
    Asphalt,
}

fn puddle_noise(column: i32, row: i32, seed: u32) -> f32 {
    fractal_noise(column as f32 * 0.09, row as f32 * 0.17, 2, seed ^ 0x9d1)
}

fn paved_colour(drawing: &DrawingContext, paving: Paving, column: i32, row: i32) -> Colour {
    let palette = drawing.palette;
    let mut colour = match paving {
        Paving::Flagstone => flagstone_colour(palette, column, row),
        Paving::Concrete => {
            if hash_position(column, row, 0xC0C0) % 100 < 8 {
                palette.concrete[1]
            } else {
                palette.concrete[0]
            }
        }
        Paving::Asphalt => {
            let roll = hash_position(column, row, 0xA5F) % 100;
            if roll < 10 {
                palette.asphalt[0]
            } else if roll < 18 {
                palette.asphalt[2]
            } else {
                palette.asphalt[1]
            }
        }
    };
    let wetness = drawing.look.wetness;
    if wetness > 0.25 && drawing.look.snow_cover < 0.5 {
        let noise = puddle_noise(column, row, drawing.look.seed);
        let puddle_level = 0.27 + (wetness - 0.25) * 0.14;
        if noise < puddle_level {
            let is_far_rim = puddle_noise(column, row - 1, drawing.look.seed) >= puddle_level;
            colour = if is_far_rim {
                0x9fb0c2
            } else if noise < puddle_level - 0.05 {
                crate::chapel::anim::PUDDLE_COLOUR
            } else {
                0x6c7d92
            };
        }
    }
    let (snow_gain, snow_bias) = match paving {
        Paving::Flagstone => (1.55, -0.30),
        Paving::Concrete => (1.6, -0.30),
        Paving::Asphalt => (1.35, -0.33),
    };
    if let Some(snow) = drawing.snow_colour(column, row, snow_gain, snow_bias, 0) {
        let slush_roll = hash_fraction(column, row, 0x7A3);
        colour = if slush_roll < 0.16 { drawing.palette.snow[2] } else { snow };
    } else if drawing.look.snow_cover > 0.05 {
        colour = blend_colours(colour, 0x59616c, 0.28 * drawing.look.snow_cover.min(1.0));
    }
    colour
}

#[derive(Clone, Copy)]
enum MowingStripes {
    RunningNorth,
    RunningEast,
}

fn lawn_colour(
    drawing: &DrawingContext,
    column: i32,
    row: i32,
    stripes: MowingStripes,
    stripe_width_metres: f32,
    leaf_litter_share: f32,
) -> Colour {
    let (metres_east, metres_north) =
        drawing.projection.world_point_under_pixel(column as f32 + 0.5, row as f32 + 0.5, 0.0);
    let distance_across_stripes = match stripes {
        MowingStripes::RunningNorth => metres_east,
        MowingStripes::RunningEast => metres_north,
    };
    let stripe = ((distance_across_stripes / stripe_width_metres).floor() as i32) & 1;
    let roll = hash_position(column, row, 0x6A55) % 100;
    let mut colour = if roll < 3 {
        drawing.palette.lawn[2]
    } else if roll < 5 {
        drawing.palette.lawn[3]
    } else {
        drawing.palette.lawn[stripe as usize]
    };
    let litter = drawing.look.leaf_litter() * leaf_litter_share;
    if litter > 0.02 {
        let patch = fractal_noise(column as f32 * 0.06, row as f32 * 0.1, 2, drawing.look.seed ^ 0x11);
        let chance_of_a_leaf = litter * (0.10 + 0.55 * ((patch - 0.35) * 2.2).clamp(0.0, 1.0));
        if hash_fraction(column, row, 0x1EAF) < chance_of_a_leaf {
            colour = drawing.palette.leaf_litter[(hash_position(column, row, 0x77) % 4) as usize];
        }
    }
    if let Some(snow) = drawing.snow_colour(column, row, 2.5, 0.04, 0) {
        colour = snow;
    } else if drawing.look.snow_cover > 0.02 && hash_fraction(column, row, 0x5E) < 0.25 {
        colour = blend_colours(colour, drawing.palette.snow[1], 0.45);
    }
    colour
}

fn woodland_floor_colour(drawing: &DrawingContext, column: i32, row: i32) -> Colour {
    let noise = crate::value_noise(column as f32 * 0.07, row as f32 * 0.12, drawing.look.seed ^ 0xF100);
    let roll = hash_fraction(column, row, 0xF1);
    let year = drawing.look.position_in_year();
    let share_of_green_understory = if year < 2.0 {
        0.80
    } else if year < 3.0 {
        0.80 - 0.5 * (year - 2.0)
    } else {
        0.30 + 0.5 * ((year - 3.6) / 0.4).clamp(0.0, 1.0)
    };
    let hedge = drawing.palette.hedge;
    let mut colour = if noise + (roll - 0.5) * 0.25 < share_of_green_understory {
        let ivy_patch = hash_fraction(column >> 2, row >> 1, drawing.look.seed ^ 0xF101);
        if ivy_patch < 0.42 {
            if roll < 0.75 {
                hedge[0]
            } else {
                hedge[1]
            }
        } else if roll < 0.12 {
            hedge[2]
        } else if roll < 0.7 {
            hedge[1]
        } else {
            hedge[0]
        }
    } else {
        let dullness_of_fallen_leaves = if year > 2.6 {
            ((year - 2.6) / 0.6).clamp(0.0, 1.0) * (1.0 - ((year - 3.7) / 0.3).clamp(0.0, 1.0))
        } else {
            0.0
        };
        let mulch = if roll < 0.3 {
            drawing.palette.mulch[1]
        } else if roll < 0.85 {
            drawing.palette.mulch[0]
        } else {
            drawing.palette.mulch[2]
        };
        blend_colours(mulch, 0x4f4a44, 0.45 * dullness_of_fallen_leaves)
    };
    let litter = drawing.look.leaf_litter();
    if litter > 0.05 && hash_fraction(column, row, 0xF2) < litter * 0.5 {
        colour = drawing.palette.leaf_litter[(hash_position(column, row, 0x78) % 4) as usize];
    }
    if let Some(snow) = drawing.snow_colour(column, row, 1.9, -0.12, 0) {
        let chance_of_twigs_showing = (0.10 - drawing.look.snow_cover * 0.09).max(0.012);
        colour = if hash_fraction(column >> 1, row, 0xF3) < chance_of_twigs_showing {
            drawing.palette.mulch[1]
        } else {
            snow
        };
    }
    colour
}

fn draw_walk(layer: &mut Layer, drawing: &DrawingContext, route: &[GroundPoint], width_metres: f32, paving: Paving) {
    for drawing_the_kerb in [true, false] {
        let width_here = if drawing_the_kerb { width_metres + 0.5 } else { width_metres };
        let colour_at = |column: i32, row: i32| {
            let colour = paved_colour(drawing, paving, column, row);
            if drawing_the_kerb {
                blend_colours(colour, 0x3c4034, 0.42)
            } else {
                colour
            }
        };
        for index in 0..route.len() {
            let has_next_corner = index + 1 < route.len();
            if has_next_corner {
                let outline = strip_along(drawing.projection, route[index], route[index + 1], width_here);
                layer.fill_polygon_with(&outline, |column, row| Some(colour_at(column, row)));
            }
            let (corner_column, corner_row) = drawing.projection.ground_pixel(route[index]);
            let radius_columns =
                (width_here * 0.5 * drawing.projection.pixels_per_metre_across(route[index].1)).round() as i32;
            let radius_rows =
                (width_here * 0.5 * drawing.projection.pixels_per_metre_of_depth(route[index].1)).round() as i32;
            if radius_columns >= 1 && radius_rows >= 1 {
                layer.fill_ellipse_with(corner_column, corner_row, radius_columns, radius_rows, |column, row| {
                    Some(colour_at(column, row))
                });
            }
            if !drawing_the_kerb && has_next_corner {
                let (next_column, next_row) = drawing.projection.ground_pixel(route[index + 1]);
                let steps = (next_column - corner_column).abs().max((next_row - corner_row).abs()).max(1);
                for step in 0..=steps {
                    let column = corner_column + (next_column - corner_column) * step / steps;
                    let row = corner_row + (next_row - corner_row) * step / steps;
                    layer.put(column, row, colour_at(column, row));
                }
            }
        }
    }
}

fn draw_hedge(layer: &mut Layer, drawing: &DrawingContext, outline: &[GroundPoint], height_metres: f32, seed: u32) {
    let base = drawing.projection.ground_polygon(outline);
    let height_pixels = drawing.projection.height_in_pixels(outline[0].1, height_metres).max(1);
    let hedge = drawing.palette.hedge;
    for rows_up in 0..=height_pixels {
        let raised: Vec<(i32, i32)> = base.iter().map(|&(column, row)| (column, row - rows_up)).collect();
        if rows_up < height_pixels {
            layer.fill_polygon_with(&raised, |column, row| {
                Some(if hash_position(column, row, seed).is_multiple_of(5) { hedge[1] } else { hedge[0] })
            });
            continue;
        }
        layer.fill_polygon_with(&raised, |column, row| {
            if let Some(snow) = drawing.snow_colour(column, row, 2.6, 0.0, 0) {
                return Some(snow);
            }
            let roll = hash_position(column, row, seed) % 100;
            Some(if roll < 14 {
                hedge[3]
            } else if roll < 34 {
                hedge[1]
            } else {
                hedge[2]
            })
        });
    }
}

fn draw_flower_bed(layer: &mut Layer, drawing: &DrawingContext, outline: &[GroundPoint], seed: u32) {
    const SPRING_FLOWERS: [Colour; 6] = [0xe23b4e, 0xf2c230, 0xf08fb4, 0xfafafa, 0x9a4fc0, 0xf57a2a];
    const SUMMER_FLOWERS: [Colour; 5] = [0xe8463c, 0xf5d13a, 0xf7f3ea, 0xd9479a, 0x6f7fe0];
    const AUTUMN_FLOWERS: [Colour; 4] = [0xd9822b, 0xe2b53a, 0x8f2f3a, 0xc8502a];
    const WINTER_FLOWERS: [Colour; 3] = [0x7a4fb0, 0xe6c84a, 0xf0f0f4];
    let look = drawing.look;
    let progress = look.season_progress;
    let (flower_colours, share_in_flower): (&[Colour], f32) = match look.season {
        Season::Spring => {
            let tulips_out = 0.8 * smooth_step(0.06, 0.14, progress) * (1.0 - smooth_step(0.7, 0.8, progress));
            (&SPRING_FLOWERS, (0.15 + 0.6 * look.blossom().max(tulips_out)) * look.bedding_in())
        }
        Season::Summer => (&SUMMER_FLOWERS, 0.62 * look.bedding_in()),
        Season::Autumn => {
            (&AUTUMN_FLOWERS, 0.6 * (1.0 - ((progress - 0.75) / 0.2).clamp(0.0, 1.0)) * look.bedding_in())
        }
        Season::Winter => (&WINTER_FLOWERS, 0.16 * look.bedding_in()),
    };
    let hedge = drawing.palette.hedge;
    layer.fill_polygon_with(&drawing.projection.ground_polygon(outline), |column, row| {
        if let Some(snow) = drawing.snow_colour(column, row, 2.3, -0.02, 0) {
            return Some(snow);
        }
        let roll = hash_fraction(column, row, seed);
        let drift_colour = (hash_position(column / 4, row / 2, seed ^ 9) as usize) % flower_colours.len();
        Some(if roll < share_in_flower {
            flower_colours[drift_colour]
        } else if roll < share_in_flower + 0.3 {
            hedge[1]
        } else if roll < 0.9 {
            drawing.palette.mulch[1]
        } else {
            hedge[2]
        })
    });
}

fn rectangle_with_cut_corners(west: f32, south: f32, east: f32, north: f32, corner_cut: f32) -> [GroundPoint; 8] {
    [
        (west + corner_cut, south),
        (east - corner_cut, south),
        (east, south + corner_cut),
        (east, north - corner_cut),
        (east - corner_cut, north),
        (west + corner_cut, north),
        (west, north - corner_cut),
        (west, south + corner_cut),
    ]
}

fn draw_ellipse_on_the_ground(
    layer: &mut Layer,
    projection: Projection,
    centre: GroundPoint,
    radius_metres: f32,
    mut colour_at: impl FnMut(i32, i32) -> Colour,
) {
    let (column, row) = projection.ground_pixel(centre);
    let radius_columns = (radius_metres * projection.pixels_per_metre_across(centre.1)).round() as i32;
    let radius_rows = (radius_metres * projection.pixels_per_metre_of_depth(centre.1)).round() as i32;
    layer
        .fill_ellipse_with(column, row, radius_columns, radius_rows.max(1), |column, row| Some(colour_at(column, row)));
}

fn bryan_center_plaza_colour(drawing: &DrawingContext, column: i32, row: i32) -> Colour {
    if let Some(snow) = drawing.snow_colour(column, row, 1.55, -0.3, 0) {
        return snow;
    }
    let (metres_east, metres_north) =
        drawing.projection.world_point_under_pixel(column as f32 + 0.5, row as f32 + 0.5, 0.0);
    let is_cross_band =
        (metres_north - 8.0).abs() < 1.8 || (metres_north + 10.5).abs() < 2.2 || (metres_north + 31.0).abs() < 1.8;
    let is_joint = (metres_east / 4.0).rem_euclid(1.0) < 0.07 || (metres_north / 4.0).rem_euclid(1.0) < 0.09;
    let is_buff_concrete = metres_east < -104.5 && !is_cross_band;
    let colour = if is_joint {
        0xa9a59a
    } else if is_buff_concrete {
        0xe0cfa8
    } else if is_cross_band {
        0xb9b8b2
    } else {
        0xcfccc2
    };
    wetted(colour, drawing.look.wetness * 0.7)
}

fn circle_island_colour(drawing: &DrawingContext, column: i32, row: i32, centre: GroundPoint) -> Colour {
    if let Some(snow) = drawing.snow_colour(column, row, 1.55, -0.3, 0) {
        return snow;
    }
    let (metres_east, metres_north) =
        drawing.projection.world_point_under_pixel(column as f32 + 0.5, row as f32 + 0.5, 0.0);
    let angle = (metres_north - centre.1).atan2(metres_east - centre.0);
    let is_radial_joint = (angle / std::f32::consts::TAU * 16.0).rem_euclid(1.0) < 0.07;
    wetted(if is_radial_joint { 0xb8b2a2 } else { 0xddd7c6 }, drawing.look.wetness * 0.7)
}

const NOTHING_LAID_YET: Colour = 0x00ff_00ff;
pub const DROP_OFF_CIRCLE: (GroundPoint, f32, f32) = ((-64.4, 94.6), 17.6, 9.7);
const EAST_TURNING_CIRCLE: (GroundPoint, f32, f32) = ((105.0, 58.8), 18.0, 11.0);

#[derive(Clone, Copy)]
enum Surface {
    Lawn { leaf_litter_share: f32 },
    CourtLawnPanel,
    Paved(Paving),
    DarkenedConcrete,
    WoodlandFloor,
    BryanCenterPlaza,
    PaleConcreteOfTheCircleIsland,
    BuffPavers { seed: u32 },
}

fn lay_surface(layer: &mut Layer, drawing: &DrawingContext, outline: &[GroundPoint], surface: Surface) {
    let screen_outline = drawing.projection.ground_polygon(outline);
    if matches!(surface, Surface::CourtLawnPanel) && drawing.look.snow_cover <= 0.3 {
        let trimmed_edge: Vec<(i32, i32)> = screen_outline.iter().map(|&(column, row)| (column, row + 1)).collect();
        layer.fill_polygon(&trimmed_edge, drawing.palette.lawn_edge);
    }
    layer.fill_polygon_with(&screen_outline, |column, row| {
        Some(match surface {
            Surface::Lawn { leaf_litter_share } => {
                lawn_colour(drawing, column, row, MowingStripes::RunningEast, 1000.0, leaf_litter_share)
            }
            Surface::CourtLawnPanel => lawn_colour(drawing, column, row, MowingStripes::RunningNorth, 1.3, 1.0),
            Surface::Paved(paving) => paved_colour(drawing, paving, column, row),
            Surface::DarkenedConcrete => {
                blend_colours(paved_colour(drawing, Paving::Concrete, column, row), 0x4a4a48, 0.3)
            }
            Surface::WoodlandFloor => woodland_floor_colour(drawing, column, row),
            Surface::BryanCenterPlaza => bryan_center_plaza_colour(drawing, column, row),
            Surface::PaleConcreteOfTheCircleIsland => circle_island_colour(drawing, column, row, DROP_OFF_CIRCLE.0),
            Surface::BuffPavers { seed } => drawing
                .snow_colour(column, row, 1.55, -0.3, 0)
                .unwrap_or(if hash_position(column / 2, row, seed).is_multiple_of(5) { 0xb9a688 } else { 0xc9b698 }),
        })
    });
}

const LAWN_NORTH_OF_PAGE_AUDITORIUM: &[GroundPoint] = &[
    (-83.4, 3.4),
    (-49.9, 3.4),
    (-58.6, 18.6),
    (-62.6, 27.7),
    (-64.2, 36.2),
    (-69.4, 30.6),
    (-78.2, 24.0),
    (-83.4, 24.0),
];
const LAWN_BEHIND_THE_CHANCEL_WEST: &[GroundPoint] =
    &[(-28.5, 81.0), (-17.5, 90.5), (0.2, 101.2), (-11.1, 106.6), (-28.2, 108.9), (-29.0, 98.0)];
const LAWN_BEHIND_THE_CHANCEL_EAST: &[GroundPoint] =
    &[(-15.5, 90.6), (5.0, 94.2), (19.5, 87.0), (24.4, 82.3), (12.3, 100.7), (8.3, 100.7), (1.2, 99.4)];
const LAWN_EAST_OF_THE_TRANSEPT: &[GroundPoint] =
    &[(24.5, 54.0), (37.0, 54.0), (37.0, 71.5), (33.5, 72.4), (24.5, 66.5)];
const PAGE_AUDITORIUM_PARKING_COURT_ASPHALT: &[GroundPoint] =
    &[(-88.0, -21.6), (-56.0, -21.6), (-56.0, -42.0), (-88.0, -42.0)];
const PAGE_AUDITORIUM_PARKING_COURT_CONCRETE: &[GroundPoint] =
    &[(-56.0, -21.6), (-45.2, -21.6), (-45.2, -42.0), (-56.0, -42.0)];
const LOT_BETWEEN_GRAY_BUILDING_AND_DIVINITY_ADDITION: &[GroundPoint] = &[
    (45.2, -25.6),
    (45.2, 2.6),
    (49.0, 3.2),
    (54.6, 1.3),
    (64.2, -11.0),
    (70.6, -11.0),
    (74.8, -13.6),
    (74.4, -18.3),
    (72.8, -23.3),
    (74.3, -31.7),
    (77.0, -42.0),
    (65.6, -42.0),
    (65.6, -25.6),
];
const APRON_BY_THE_EAST_TURNING_CIRCLE: &[GroundPoint] =
    &[(75.2, 52.0), (88.0, 49.0), (90.0, 62.0), (86.0, 69.0), (75.2, 67.0)];
const FORECOURT_BY_THE_DROP_OFF_CIRCLE: &[GroundPoint] =
    &[(-86.0, 84.0), (-78.0, 84.0), (-77.0, 101.0), (-86.0, 101.0)];
const BRYAN_CENTER_NORTH_ENTRANCE_COURT: &[GroundPoint] =
    &[(-103.4, 75.6), (-76.2, 75.6), (-78.0, 83.4), (-103.4, 83.4)];
const BRYAN_CENTER_PLAZA: &[GroundPoint] = &[(-150.0, 28.4), (-88.4, 28.4), (-88.4, -46.0), (-150.0, -46.0)];
const WALK_BETWEEN_THE_PLAZA_AND_PAGE_AUDITORIUM: &[GroundPoint] =
    &[(-88.4, 4.0), (-82.0, 4.0), (-82.0, -21.6), (-88.4, -21.6)];
const CHAPEL_COURT: &[GroundPoint] = &[(-26.9, 9.3), (30.0, 9.3), (27.4, -50.5), (-27.4, -50.5)];
const CHAPEL_COURT_WEST_STEP_OUT: &[GroundPoint] = &[(-33.0, -24.0), (-27.4, -24.0), (-27.4, -40.0), (-33.0, -40.0)];

const WEST_GARDEN_FLAGSTONE_WALK: &[GroundPoint] = &[(-26.9, 9.3), (-14.0, 9.3), (-14.0, 12.6), (-26.9, 12.6)];
const WEST_GARDEN_BED_BY_THE_NAVE: &[GroundPoint] = &[(-14.0, 12.0), (-10.4, 12.0), (-10.4, 52.5), (-14.0, 52.5)];
const WEST_GARDEN_PAVING: &[GroundPoint] =
    &[(-17.9, 9.3), (-14.0, 9.3), (-14.0, 49.0), (-10.8, 49.0), (-10.8, 53.0), (-17.9, 53.0)];
const WEST_GARDEN_LAWN: &[GroundPoint] = &[(-25.6, 12.6), (-17.9, 12.6), (-17.9, 50.0), (-25.6, 50.0)];
const WEST_GARDEN_DIAGONAL_WALKS_MEET_AT: GroundPoint = (-25.6, 30.5);
const WEST_GARDEN_DIAGONAL_WALKS_END_AT: [GroundPoint; 3] = [(-17.9, 47.6), (-17.9, 13.5), (-17.9, 30.5)];
const WEST_GARDEN_HEDGE: &[GroundPoint] = &[(-25.4, 48.6), (-18.4, 48.6), (-18.4, 49.9), (-25.4, 49.9)];
const EAST_GARDEN_BED_BY_THE_NAVE: &[GroundPoint] = &[(10.4, 12.0), (13.4, 12.0), (13.4, 52.5), (10.4, 52.5)];
const EAST_GARDEN_PAVING: &[GroundPoint] =
    &[(13.4, 9.3), (33.6, 9.3), (33.6, 53.6), (10.8, 53.6), (10.8, 49.0), (13.4, 49.0)];
const EAST_GARDEN_BED_BY_THE_DIVINITY_SCHOOL: &[GroundPoint] =
    &[(33.6, 13.5), (39.4, 13.5), (39.4, 51.6), (33.6, 51.6)];
const EAST_GARDEN_LOOP_HEDGE: [GroundPoint; 9] = [
    (17.6, 13.2),
    (24.4, 12.6),
    (30.4, 18.4),
    (30.6, 46.8),
    (29.0, 49.2),
    (27.0, 50.8),
    (23.4, 51.0),
    (20.4, 48.6),
    (17.9, 46.0),
];
const DIVINITY_COURT_CONCRETE: &[GroundPoint] = &[(50.75, 14.2), (64.0, 14.2), (64.0, 47.6), (50.75, 47.6)];
const DIVINITY_COURT_PAVERS: &[GroundPoint] =
    &[(64.0, 14.2), (85.2, 14.2), (85.6, 23.5), (81.5, 27.4), (84.7, 37.2), (86.0, 47.6), (64.0, 47.6)];
const BED_WEST_OF_THE_COURT: &[GroundPoint] = &[(-32.6, -31.4), (-25.3, -31.4), (-25.3, -0.3), (-32.6, -0.3)];
const BED_EAST_OF_THE_COURT: &[GroundPoint] = &[(25.7, -34.0), (31.9, -34.0), (31.9, 2.3), (25.7, 2.3)];
const HEDGE_UNDER_PAGE_AUDITORIUM: &[GroundPoint] = &[(-81.5, -22.6), (-46.0, -22.6), (-46.0, -21.4), (-81.5, -21.4)];
const TERRACE_BY_PERKINS_LIBRARY: &[GroundPoint] =
    &[(79.5, -16.6), (97.5, -16.6), (97.5, -21.2), (89.2, -21.2), (89.2, -29.6), (79.5, -29.6)];
const FLOWER_BED_WEST_OF_THE_PORTAL: &[GroundPoint] = &[(-26.6, 5.6), (-24.2, 5.6), (-24.2, 8.2), (-26.6, 8.2)];
const FLOWER_BED_EAST_OF_THE_PORTAL: &[GroundPoint] = &[(27.4, 5.8), (29.8, 5.8), (29.8, 8.4), (27.4, 8.4)];

pub fn draw_ground(layer: &mut Layer, drawing: &DrawingContext) {
    use crate::chapel::survey::{CAR_PARK_OUTLINE, SURVEYED_ROADS, SURVEYED_WALKS};
    let palette = drawing.palette;
    let projection = drawing.projection;
    layer.pixels.fill(NOTHING_LAID_YET);

    for lawn in [
        LAWN_NORTH_OF_PAGE_AUDITORIUM,
        LAWN_BEHIND_THE_CHANCEL_WEST,
        LAWN_BEHIND_THE_CHANCEL_EAST,
        LAWN_EAST_OF_THE_TRANSEPT,
    ] {
        lay_surface(layer, drawing, lawn, Surface::Lawn { leaf_litter_share: 1.6 });
    }

    let car_park = projection.ground_polygon(CAR_PARK_OUTLINE);
    let car_park_kerb: Vec<(i32, i32)> = car_park.iter().map(|&(column, row)| (column, row + 1)).collect();
    layer.fill_polygon(&car_park_kerb, palette.kerb[1]);
    layer.fill_polygon_with(&car_park, |column, row| Some(paved_colour(drawing, Paving::Concrete, column, row)));
    for road in SURVEYED_ROADS {
        let is_part_of_a_circle = road.iter().all(|point| {
            let to_drop_off_circle =
                ((point.0 - DROP_OFF_CIRCLE.0 .0).powi(2) + (point.1 - DROP_OFF_CIRCLE.0 .1).powi(2)).sqrt();
            let to_east_circle =
                ((point.0 - EAST_TURNING_CIRCLE.0 .0).powi(2) + (point.1 - EAST_TURNING_CIRCLE.0 .1).powi(2)).sqrt();
            to_drop_off_circle < 16.0 || to_east_circle < 17.5
        });
        if is_part_of_a_circle {
            continue;
        }
        let is_concrete_drive_to_a_circle = road.iter().any(|point| point.1 > 30.0);
        let paving = if is_concrete_drive_to_a_circle { Paving::Concrete } else { Paving::Asphalt };
        for leg in road.windows(2) {
            let outline = strip_along(projection, leg[0], leg[1], 6.4);
            layer.fill_polygon_with(&outline, |column, row| Some(paved_colour(drawing, paving, column, row)));
            draw_ellipse_on_the_ground(layer, projection, leg[1], 3.2, |column, row| {
                paved_colour(drawing, paving, column, row)
            });
        }
    }
    lay_surface(layer, drawing, PAGE_AUDITORIUM_PARKING_COURT_ASPHALT, Surface::Paved(Paving::Asphalt));
    lay_surface(layer, drawing, PAGE_AUDITORIUM_PARKING_COURT_CONCRETE, Surface::Paved(Paving::Concrete));
    lay_surface(layer, drawing, LOT_BETWEEN_GRAY_BUILDING_AND_DIVINITY_ADDITION, Surface::Paved(Paving::Asphalt));
    for (centre, outer_radius, island_radius) in [DROP_OFF_CIRCLE, EAST_TURNING_CIRCLE] {
        draw_ellipse_on_the_ground(layer, projection, centre, outer_radius + 0.6, |_, _| palette.kerb[1]);
        draw_ellipse_on_the_ground(layer, projection, centre, outer_radius, |column, row| {
            paved_colour(drawing, Paving::Concrete, column, row)
        });
        draw_ellipse_on_the_ground(layer, projection, centre, island_radius + 0.5, |_, _| palette.kerb[0]);
        let is_the_drop_off_circle = centre.0 < 0.0;
        if is_the_drop_off_circle {
            draw_ellipse_on_the_ground(layer, projection, centre, island_radius, |column, row| {
                circle_island_colour(drawing, column, row, centre)
            });
            draw_ellipse_on_the_ground(layer, projection, centre, 3.8, |column, row| {
                woodland_floor_colour(drawing, column, row)
            });
        } else {
            draw_ellipse_on_the_ground(layer, projection, centre, island_radius, |column, row| {
                woodland_floor_colour(drawing, column, row)
            });
        }
    }
    lay_surface(layer, drawing, APRON_BY_THE_EAST_TURNING_CIRCLE, Surface::Paved(Paving::Concrete));
    lay_surface(layer, drawing, FORECOURT_BY_THE_DROP_OFF_CIRCLE, Surface::PaleConcreteOfTheCircleIsland);
    lay_surface(layer, drawing, BRYAN_CENTER_NORTH_ENTRANCE_COURT, Surface::DarkenedConcrete);
    if drawing.look.snow_cover < 0.55 {
        for bay in 0..12 {
            let metres_north = 104.0 + bay as f32 * 2.9;
            let aisle_metres_east = crate::chapel::props::car_park_aisle_metres_east(metres_north);
            let from = projection.ground_pixel((aisle_metres_east + 3.3, metres_north));
            let to = projection.ground_pixel((aisle_metres_east + 8.1, metres_north));
            layer.line(from.0, from.1, to.0, to.1, palette.road_paint);
        }
        for bay in 0..9 {
            let metres_east = -85.5 + bay as f32 * 3.3;
            let from = projection.ground_pixel((metres_east, -22.2));
            let to = projection.ground_pixel((metres_east, -27.4));
            layer.line(from.0, from.1, to.0, to.1, palette.road_paint);
        }
    }

    lay_surface(layer, drawing, BRYAN_CENTER_PLAZA, Surface::BryanCenterPlaza);
    lay_surface(layer, drawing, WALK_BETWEEN_THE_PLAZA_AND_PAGE_AUDITORIUM, Surface::Paved(Paving::Concrete));
    for walk in SURVEYED_WALKS {
        draw_walk(layer, drawing, walk, 2.2, Paving::Concrete);
    }

    lay_surface(layer, drawing, CHAPEL_COURT, Surface::Paved(Paving::Flagstone));
    lay_surface(layer, drawing, CHAPEL_COURT_WEST_STEP_OUT, Surface::Paved(Paving::Flagstone));
    lay_surface(layer, drawing, &[(-22.5, 4.5), (-9.2, 4.5), (-9.2, -8.0), (-22.5, -8.0)], Surface::CourtLawnPanel);
    lay_surface(layer, drawing, &[(22.5, 4.5), (9.2, 4.5), (9.2, -8.0), (22.5, -8.0)], Surface::CourtLawnPanel);
    lay_surface(layer, drawing, &rectangle_with_cut_corners(-22.5, -33.0, -9.0, -12.5, 0.6), Surface::CourtLawnPanel);
    lay_surface(layer, drawing, &rectangle_with_cut_corners(-4.0, -33.0, 4.0, -12.5, 0.6), Surface::CourtLawnPanel);
    lay_surface(layer, drawing, &rectangle_with_cut_corners(9.0, -33.0, 22.5, -12.5, 0.6), Surface::CourtLawnPanel);
    let limestone = &palette.limestone;
    let portal_steps_top_row = projection.pixel_y(0.4, 0.75);
    let platform_lip_row = projection.pixel_y(-2.3, 0.75);
    let steps_foot_row = projection.pixel_y(-4.8, 0.0);
    for row in portal_steps_top_row..=steps_foot_row {
        let widening = if row > platform_lip_row { (row - platform_lip_row) / 2 } else { 0 };
        let left = projection.pixel_x(-7.4, 0.0) - widening;
        let right = projection.pixel_x(7.4, 0.0) + widening;
        let is_platform = row <= platform_lip_row;
        let step_colour = if is_platform {
            limestone[1]
        } else if (row - platform_lip_row) % 2 == 1 {
            limestone[3]
        } else {
            limestone[0]
        };
        for column in left..=right {
            let colour = if is_platform {
                let worn_flag = if hash_position(column / 5, row / 2, 0x57E9).is_multiple_of(6) {
                    limestone[2]
                } else {
                    step_colour
                };
                drawing.snow_colour(column, row, 1.6, -0.28, 0).unwrap_or(worn_flag)
            } else {
                step_colour
            };
            layer.put(column, row, colour);
        }
    }

    lay_surface(layer, drawing, WEST_GARDEN_FLAGSTONE_WALK, Surface::Paved(Paving::Flagstone));
    lay_surface(layer, drawing, WEST_GARDEN_BED_BY_THE_NAVE, Surface::WoodlandFloor);
    lay_surface(layer, drawing, WEST_GARDEN_PAVING, Surface::Paved(Paving::Flagstone));
    lay_surface(layer, drawing, WEST_GARDEN_LAWN, Surface::Lawn { leaf_litter_share: 1.3 });
    for walk_end in WEST_GARDEN_DIAGONAL_WALKS_END_AT {
        draw_walk(layer, drawing, &[WEST_GARDEN_DIAGONAL_WALKS_MEET_AT, walk_end], 1.5, Paving::Flagstone);
    }
    draw_hedge(layer, drawing, WEST_GARDEN_HEDGE, 1.2, 31);

    lay_surface(layer, drawing, EAST_GARDEN_BED_BY_THE_NAVE, Surface::WoodlandFloor);
    lay_surface(layer, drawing, EAST_GARDEN_PAVING, Surface::Paved(Paving::Flagstone));
    lay_surface(layer, drawing, EAST_GARDEN_BED_BY_THE_DIVINITY_SCHOOL, Surface::WoodlandFloor);
    let pale_lawn_stripe = hue_shifted(palette.lawn[0], 0.86, 8.0, 1.08);
    let dark_lawn_stripe = hue_shifted(palette.lawn[0], 0.66, 14.0, 1.1);
    draw_hedge(layer, drawing, &EAST_GARDEN_LOOP_HEDGE, 1.0, 21);
    let lawn_inside_the_hedge: Vec<GroundPoint> = EAST_GARDEN_LOOP_HEDGE
        .iter()
        .map(|point| (24.1 + (point.0 - 24.1) * 0.86, 31.5 + (point.1 - 31.5) * 0.94))
        .collect();
    layer.fill_polygon_with(&projection.ground_polygon(&lawn_inside_the_hedge), |column, row| {
        if let Some(snow) = drawing.snow_colour(column, row, 2.4, 0.03, 0) {
            return Some(snow);
        }
        let (metres_east, _) = projection.world_point_under_pixel(column as f32 + 0.5, row as f32 + 0.5, 0.0);
        Some(if ((metres_east / 1.4).floor() as i32) & 1 == 0 { pale_lawn_stripe } else { dark_lawn_stripe })
    });

    lay_surface(layer, drawing, DIVINITY_COURT_CONCRETE, Surface::Paved(Paving::Concrete));
    lay_surface(layer, drawing, DIVINITY_COURT_PAVERS, Surface::BuffPavers { seed: 0xD1B });
    lay_surface(layer, drawing, BED_WEST_OF_THE_COURT, Surface::WoodlandFloor);
    lay_surface(layer, drawing, BED_EAST_OF_THE_COURT, Surface::WoodlandFloor);
    draw_hedge(layer, drawing, HEDGE_UNDER_PAGE_AUDITORIUM, 0.9, 43);
    lay_surface(layer, drawing, TERRACE_BY_PERKINS_LIBRARY, Surface::BuffPavers { seed: 0xD1C });
    draw_flower_bed(layer, drawing, FLOWER_BED_WEST_OF_THE_PORTAL, 0xF13);
    draw_flower_bed(layer, drawing, FLOWER_BED_EAST_OF_THE_PORTAL, 0xF14);

    let snow_cover = drawing.look.snow_cover;
    if snow_cover > 0.10 {
        const TRODDEN_SLUSH: [Colour; 3] = [0x5b626d, 0x666e79, 0x737b86];
        const TYRE_TRACK_SAMPLES: i32 = 160;
        for route in crate::chapel::anim::WALKING_ROUTES {
            for leg in route.windows(2) {
                let trail = strip_along(projection, leg[0], leg[1], 2.2);
                layer.fill_polygon_with(&trail, |column, row| {
                    let roll = hash_fraction(column, row, 0x7AA);
                    if snow_cover < 0.55 {
                        if roll < 0.82 {
                            Some(TRODDEN_SLUSH[(hash_position(column / 3, row / 2, 0x7AB) % 3) as usize])
                        } else {
                            None
                        }
                    } else if roll < 0.035 {
                        Some(TRODDEN_SLUSH[0])
                    } else if hash_fraction(column >> 1, row, 0x7AE) < 0.30 {
                        Some(palette.snow[3])
                    } else {
                        Some(palette.snow[2])
                    }
                });
            }
        }
        for (centre, outer_radius, island_radius) in [DROP_OFF_CIRCLE, EAST_TURNING_CIRCLE] {
            let (centre_column, centre_row) = projection.ground_pixel(centre);
            for track_radius in [island_radius + 2.6, outer_radius - 2.2] {
                for sample in 0..TYRE_TRACK_SAMPLES {
                    let angle = sample as f32 / TYRE_TRACK_SAMPLES as f32 * std::f32::consts::TAU;
                    let column = centre_column
                        + (angle.cos() * track_radius * projection.pixels_per_metre_across(centre.1)) as i32;
                    let row = centre_row
                        + (angle.sin() * track_radius * projection.pixels_per_metre_of_depth(centre.1)) as i32;
                    if hash_fraction(column, row, 0x7AC) < 0.8 {
                        layer.put(column, row, if snow_cover < 0.55 { palette.asphalt[2] } else { palette.snow[3] });
                    }
                }
            }
        }
    }

    let leaf_litter = drawing.look.leaf_litter();
    if leaf_litter > 0.15 && drawing.look.snow_cover < 0.2 {
        for row in projection.pixel_y(9.0, 0.0)..layer.height {
            for column in projection.pixel_x(-27.0, -50.0)..projection.pixel_x(30.0, -50.0) {
                let drift = fractal_noise(column as f32 * 0.11, row as f32 * 0.2, 2, drawing.look.seed ^ 0xD1F);
                if drift < 0.2 + leaf_litter * 0.14 && hash_fraction(column, row, 0xD2) < leaf_litter * 0.45 {
                    let (metres_east, metres_north) =
                        projection.world_point_under_pixel(column as f32, row as f32, 0.0);
                    if metres_east.abs() < 29.0 && metres_north > -50.0 {
                        layer.put(column, row, palette.leaf_litter[(hash_position(column, row, 0xD3) % 4) as usize]);
                    }
                }
            }
        }
    }
    for row in 0..layer.height {
        for column in 0..layer.width {
            let index = (row * layer.width + column) as usize;
            if layer.pixels[index] == NOTHING_LAID_YET {
                layer.pixels[index] = woodland_floor_colour(drawing, column, row);
            }
        }
    }
}
