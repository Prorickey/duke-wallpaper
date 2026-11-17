use crate::east::campus_layout::*;
use crate::east::conditions::{palette, Conditions};
use crate::east::geometry::SurfaceHit;
use crate::east::shading_tools::*;
use crate::{blend_colours, hash_position, rgb, Colour, Season};
use crate::{fractal_noise, value_noise};

pub mod surface {
    pub const OUTER_LAWN: u8 = 0;
    pub const QUAD_LAWN: u8 = 1;
    pub const QUAD_DRIVE: u8 = 2;
    pub const GRANITE_KERB: u8 = 3;
    pub const CONCRETE_FORECOURT: u8 = 4;
    pub const CONCRETE_WALK: u8 = 5;
    pub const BRICK_PAVING: u8 = 6;
    pub const MULCH_BED: u8 = 7;
    pub const FLOWER_BED: u8 = 8;
    pub const SERVICE_ROAD: u8 = 9;
    pub const PARKING_BAYS: u8 = 10;
    pub const GRAVEL_PATH: u8 = 11;
    pub const ARCADE_FLOOR: u8 = 12;
    pub const STREET: u8 = 13;
    pub const TENNIS_COURT: u8 = 14;
    pub const NOT_GROUND: u8 = 254;
}

use surface::*;

const LAWN_END_SQUARENESS: f32 = 2.6;
const KERB_WIDTH: f32 = 0.4;
const LOOP_PAVEMENT_WIDTH: f32 = 2.2;
const WALK_HALF_WIDTH: f32 = 1.3;

pub fn is_on_entrance_loop(metres_east: f32, metres_south: f32) -> bool {
    let from_axis = metres_east.abs();
    if metres_south < ENTRANCE_LOOP_NORTH_KERB || from_axis > ENTRANCE_LOOP_OUTER_KERB_FROM_AXIS {
        return false;
    }
    let outer_radius = ENTRANCE_LOOP_OUTER_CORNER_RADIUS;
    if from_axis > ENTRANCE_LOOP_OUTER_KERB_FROM_AXIS - outer_radius
        && metres_south < ENTRANCE_LOOP_NORTH_KERB + outer_radius
    {
        let east_of_corner_centre = from_axis - (ENTRANCE_LOOP_OUTER_KERB_FROM_AXIS - outer_radius);
        let south_of_corner_centre = metres_south - (ENTRANCE_LOOP_NORTH_KERB + outer_radius);
        if east_of_corner_centre * east_of_corner_centre + south_of_corner_centre * south_of_corner_centre
            > outer_radius * outer_radius
        {
            return false;
        }
    }
    let inner_radius = ENTRANCE_LOOP_INNER_CORNER_RADIUS;
    if metres_south > ENTRANCE_LOOP_SOUTH_KERB && from_axis < ENTRANCE_LOOP_INNER_KERB_FROM_AXIS {
        if from_axis > ENTRANCE_LOOP_INNER_KERB_FROM_AXIS - inner_radius
            && metres_south < ENTRANCE_LOOP_SOUTH_KERB + inner_radius
        {
            let east_of_corner_centre = from_axis - (ENTRANCE_LOOP_INNER_KERB_FROM_AXIS - inner_radius);
            let south_of_corner_centre = metres_south - (ENTRANCE_LOOP_SOUTH_KERB + inner_radius);
            return east_of_corner_centre * east_of_corner_centre + south_of_corner_centre * south_of_corner_centre
                > inner_radius * inner_radius;
        }
        return false;
    }
    true
}

pub fn is_within_quad_lawns(metres_east: f32, metres_south: f32, margin: f32) -> bool {
    let from_axis = metres_east.abs();
    let half_width = LAWN_HALF_WIDTH + margin;
    if from_axis > UNION_CIRCLE_OUTER_RADIUS + margin {
        return false;
    }
    let south_of_circle = metres_south - UNION_CIRCLE_SOUTH;
    let squared_distance_from_circle = metres_east * metres_east + south_of_circle * south_of_circle;
    if squared_distance_from_circle <= (UNION_CIRCLE_LAWN_RADIUS + margin) * (UNION_CIRCLE_LAWN_RADIUS + margin) {
        return true;
    }
    if from_axis > half_width {
        return false;
    }
    let ring_drive_outer_radius = UNION_CIRCLE_OUTER_RADIUS - margin;
    if squared_distance_from_circle < ring_drive_outer_radius * ring_drive_outer_radius {
        return false;
    }
    if metres_south < UNION_CIRCLE_SOUTH {
        if metres_south >= NORTH_LAWN_CAP_SOUTH {
            return true;
        }
        let cap_depth = LAWN_HALF_WIDTH + margin * 1.15;
        (from_axis / half_width).powf(LAWN_END_SQUARENESS)
            + ((NORTH_LAWN_CAP_SOUTH - metres_south) / cap_depth).powf(LAWN_END_SQUARENESS)
            <= 1.0
    } else {
        if metres_south <= SOUTH_LAWN_CAP_SOUTH {
            return true;
        }
        (from_axis / half_width).powf(LAWN_END_SQUARENESS)
            + ((metres_south - SOUTH_LAWN_CAP_SOUTH) / half_width).powf(LAWN_END_SQUARENESS)
            <= 1.0
    }
}

fn distance_to_segment(metres_east: f32, metres_south: f32, start: (f32, f32), end: (f32, f32)) -> f32 {
    let (along_east, along_south) = (end.0 - start.0, end.1 - start.1);
    let fraction = (((metres_east - start.0) * along_east + (metres_south - start.1) * along_south)
        / (along_east * along_east + along_south * along_south))
        .clamp(0.0, 1.0);
    let (offset_east, offset_south) =
        (start.0 + along_east * fraction - metres_east, start.1 + along_south * fraction - metres_south);
    (offset_east * offset_east + offset_south * offset_south).sqrt()
}

fn distance_to_nearest(metres_south: f32, lines_south: &[f32; 4]) -> f32 {
    let mut nearest = f32::MAX;
    for line_south in lines_south {
        nearest = nearest.min((metres_south - line_south).abs());
    }
    nearest
}

pub fn surface_at(metres_east: f32, metres_south: f32) -> u8 {
    let from_axis = metres_east.abs();
    let side = side_of_quad(metres_east);

    if metres_south > 40.0 && metres_south < 325.0 && from_axis < 46.0 {
        if is_within_quad_lawns(metres_east, metres_south, 0.0) {
            let on_walk_across_south_lawn = (metres_south - 259.6).abs() < 1.0;
            return if on_walk_across_south_lawn { CONCRETE_WALK } else { QUAD_LAWN };
        }
        if is_within_quad_lawns(metres_east, metres_south, KERB_WIDTH) {
            return GRANITE_KERB;
        }
        if is_within_quad_lawns(metres_east, metres_south, QUAD_DRIVE_WIDTH) {
            return QUAD_DRIVE;
        }
    }

    let between_quad_and_loop = metres_south > SOUTH_LAWN_CAP_SOUTH + 20.0 && metres_south < ENTRANCE_LOOP_NORTH_KERB;
    if between_quad_and_loop && (from_axis - QUAD_STEM_FROM_AXIS).abs() < 2.3 {
        return QUAD_DRIVE;
    }

    if metres_south >= ENTRANCE_LOOP_NORTH_KERB - 2.4 {
        if is_on_entrance_loop(metres_east, metres_south) {
            return STREET;
        }
        let on_island = from_axis < ENTRANCE_LOOP_INNER_KERB_FROM_AXIS && metres_south > ENTRANCE_LOOP_SOUTH_KERB;
        if on_island {
            if metres_south < ENTRANCE_LOOP_SOUTH_KERB + KERB_WIDTH {
                return GRANITE_KERB;
            }
            if from_axis < BUS_PLAZA_HALF_WIDTH && metres_south < BUS_PLAZA_SOUTH_EDGE {
                return BRICK_PAVING;
            }
            let fraction_down_the_lawn = ((metres_south - BUS_PLAZA_SOUTH_EDGE) / 60.0).max(0.0);
            let brick_walk_east = if metres_east < 0.0 {
                -11.3 - 15.7 * fraction_down_the_lawn
            } else {
                9.6 + 18.0 * fraction_down_the_lawn
            };
            if metres_south >= BUS_PLAZA_SOUTH_EDGE - 0.5 && (metres_east - brick_walk_east).abs() < 1.0 {
                return BRICK_PAVING;
            }
            return OUTER_LAWN;
        }
        let outer_edge = ENTRANCE_LOOP_OUTER_KERB_FROM_AXIS + LOOP_PAVEMENT_WIDTH;
        let shrunk_east = metres_east * (ENTRANCE_LOOP_OUTER_KERB_FROM_AXIS / outer_edge);
        let shrunk_south =
            ENTRANCE_LOOP_NORTH_KERB + (metres_south - ENTRANCE_LOOP_NORTH_KERB + LOOP_PAVEMENT_WIDTH) * 0.94;
        if from_axis < outer_edge && is_on_entrance_loop(shrunk_east, shrunk_south) {
            return CONCRETE_WALK;
        }
    }

    let on_walk_along_loop = (metres_south - 333.3).abs() < 1.1 && from_axis < 62.0;
    if on_walk_along_loop {
        return CONCRETE_WALK;
    }
    let footpath_from_axis = 23.6 + (metres_south - 303.0) * 0.106;
    if metres_south > 301.0 && metres_south < 333.3 && (from_axis - footpath_from_axis).abs() < 0.95 {
        return CONCRETE_WALK;
    }

    if metres_south > 19.5 && metres_south < 46.0 {
        let flare = ((metres_south - 41.0) / 3.5).clamp(0.0, 1.0);
        if from_axis < 7.6 + 2.2 * flare * flare {
            return CONCRETE_FORECOURT;
        }
    }

    let under_baldwin_arcade = from_axis > 16.3 && from_axis < 33.0 && metres_south > 11.6 && metres_south < 15.8;
    if under_baldwin_arcade {
        return ARCADE_FLOOR;
    }
    for floor in ARCADE_FLOORS.iter() {
        if metres_east > floor.west
            && metres_east < floor.east
            && metres_south > floor.north
            && metres_south < floor.south
        {
            return ARCADE_FLOOR;
        }
    }

    let beside_the_quad = metres_south > 6.0 && metres_south < 326.0 && from_axis < 60.0;
    if beside_the_quad {
        let beside_union_circle = (metres_south - UNION_CIRCLE_SOUTH).abs() < 26.0;
        let to_entrance = distance_to_nearest(metres_south, &hall_entrances_south(side));
        let to_cross_walk = distance_to_nearest(metres_south, &cross_walks_south(side));
        if metres_south > 40.0 && is_within_quad_lawns(metres_east, metres_south, QUAD_DRIVE_WIDTH + 0.35) {
            return GRANITE_KERB;
        }
        if metres_south > 40.0
            && is_within_quad_lawns(metres_east, metres_south, QUAD_DRIVE_WIDTH + 3.0)
            && (metres_south > 47.0 || from_axis > 9.8)
        {
            return BRICK_PAVING;
        }
        if beside_union_circle {
            let facade_from_axis =
                if side < 0.0 { LILLY_LIBRARY.facade_from_axis } else { EAST_CAMPUS_UNION.facade_from_axis };
            if from_axis < facade_from_axis - 2.4 && (metres_south - UNION_CIRCLE_SOUTH).abs() < 13.0 {
                return CONCRETE_FORECOURT;
            }
        } else {
            let on_hall_terrace = from_axis > 27.0 && from_axis < 33.0 && to_entrance < 5.0;
            if on_hall_terrace {
                if to_entrance < WALK_HALF_WIDTH {
                    return CONCRETE_WALK;
                }
                if from_axis > 30.6 && to_entrance > 2.1 && to_entrance < 4.7 {
                    return FLOWER_BED;
                }
                return BRICK_PAVING;
            }
            let in_planting_strip = from_axis > 29.4 && from_axis < 33.0;
            if in_planting_strip {
                return if to_cross_walk < WALK_HALF_WIDTH { CONCRETE_WALK } else { MULCH_BED };
            }
        }
        if from_axis > 26.5 && from_axis < 56.0 && to_cross_walk < WALK_HALF_WIDTH {
            return CONCRETE_WALK;
        }
        if from_axis > 26.4 && from_axis < 28.8 && metres_south > 15.0 && metres_south < 50.0 {
            return CONCRETE_WALK;
        }
    }

    if metres_south > 15.0 && metres_south < 44.0 {
        let forecourt_end = (side * 7.5, 30.5);
        let bend = (side * 17.0, 24.0);
        let side_walk_end = (side * 27.6, 22.5);
        let to_curved_walk = distance_to_segment(metres_east, metres_south, forecourt_end, bend)
            .min(distance_to_segment(metres_east, metres_south, bend, side_walk_end));
        if to_curved_walk < 1.15 {
            return CONCRETE_WALK;
        }
        if metres_south > 38.2 && from_axis > 8.6 && from_axis < 21.5 {
            return FLOWER_BED;
        }
        let (east_of_magnolia, south_of_magnolia, mulch_radius) = if metres_east < 0.0 {
            (metres_east + 15.4, metres_south - 29.0, 7.2)
        } else {
            (metres_east - 12.8, metres_south - 32.0, 6.8)
        };
        if east_of_magnolia * east_of_magnolia + south_of_magnolia * south_of_magnolia < mulch_radius * mulch_radius
            && from_axis > 7.6
        {
            return MULCH_BED;
        }
    }

    let on_baldwin_west_terrace =
        metres_east > -27.4 && metres_east < -16.3 && metres_south > -13.5 && metres_south < 1.6;
    if on_baldwin_west_terrace {
        return CONCRETE_FORECOURT;
    }
    let in_baldwin_side_bed = from_axis > 16.3 && from_axis < 18.4 && metres_south > -18.5 && metres_south < 11.5;
    let in_baldwin_front_bed = from_axis > 9.5 && from_axis < 16.3 && metres_south > 17.1 && metres_south < 19.2;
    if in_baldwin_side_bed || in_baldwin_front_bed {
        return MULCH_BED;
    }

    for (index, car_park) in CAR_PARKS.iter().enumerate() {
        if metres_east > car_park.west
            && metres_east < car_park.east
            && metres_south > car_park.north
            && metres_south < car_park.south
        {
            return if index == 0 { PARKING_BAYS } else { SERVICE_ROAD };
        }
    }
    for court in TENNIS_COURTS.iter() {
        if metres_east > court.west - 2.5
            && metres_east < court.east + 2.5
            && metres_south > court.north - 3.5
            && metres_south < court.south + 3.5
        {
            return TENNIS_COURT;
        }
    }
    let on_road_behind_east_halls =
        metres_east > 69.4 && metres_east < 75.0 && metres_south > 144.0 && metres_south < 345.0;
    if on_road_behind_east_halls {
        return SERVICE_ROAD;
    }
    let on_walk_beside_that_road = metres_east > 75.6 && metres_east < 77.2 && metres_south > 150.0;
    let on_walk_behind_bassett = metres_east > 68.6 && metres_east < 70.2 && metres_south > 5.0 && metres_south < 26.0;
    let on_walk_behind_west_halls =
        metres_east < -78.0 && metres_east > -79.8 && metres_south > 180.0 && metres_south < 238.0;
    if on_walk_beside_that_road || on_walk_behind_bassett || on_walk_behind_west_halls {
        return CONCRETE_WALK;
    }

    if metres_south < -18.5 {
        if metres_south > -72.0 {
            let south_of_service_road = metres_south - BALDWIN_SERVICE_ROAD_SOUTH;
            if metres_east > -9.5 && metres_east < 63.5 && south_of_service_road.abs() < 3.5 {
                return SERVICE_ROAD;
            }
            let on_a_leg_to_the_avenue = (metres_east - BALDWIN_SERVICE_ROAD_WEST_LEG_EAST).abs() < 3.5
                || (metres_east - BALDWIN_SERVICE_ROAD_EAST_LEG_EAST).abs() < 3.5;
            if metres_south < BALDWIN_SERVICE_ROAD_SOUTH && on_a_leg_to_the_avenue {
                return SERVICE_ROAD;
            }
            if (metres_south + 29.6).abs() < 0.9 && metres_east > -8.0 && metres_east < 63.0 {
                return CONCRETE_WALK;
            }
            if from_axis < 1.5 && metres_south > -30.0 {
                return CONCRETE_WALK;
            }
        }
        let from_avenue_centre = (metres_south - WEST_MARKHAM_AVENUE_SOUTH).abs();
        if from_avenue_centre < WEST_MARKHAM_AVENUE_HALF_WIDTH {
            return STREET;
        }
        if from_avenue_centre > 6.6 && from_avenue_centre < 8.2 {
            return CONCRETE_WALK;
        }
        if metres_south < WEST_MARKHAM_AVENUE_SOUTH - WEST_MARKHAM_AVENUE_HALF_WIDTH {
            for street_east in TRINITY_PARK_STREETS_EAST.iter() {
                let from_street_centre = (metres_east - street_east).abs();
                if from_street_centre < 4.2 {
                    return STREET;
                }
                if from_street_centre > 5.4 && from_street_centre < 7.0 {
                    return CONCRETE_WALK;
                }
            }
        }
    }

    let walk_to_biddle_south = -16.5 + 0.04 * (metres_east + 18.0);
    if metres_east < -18.4
        && metres_east > -80.0
        && metres_south < 0.0
        && (metres_south - walk_to_biddle_south).abs() < 1.0
    {
        return CONCRETE_WALK;
    }
    if metres_east > 18.4 && metres_south < 0.0 && metres_south > -60.0 {
        for leg in GRAVEL_PERIMETER_PATH.windows(2) {
            if distance_to_segment(metres_east, metres_south, leg[0], leg[1]) < 1.0 {
                return GRAVEL_PATH;
            }
        }
    }
    OUTER_LAWN
}

pub fn desire_line_strength(metres_east: f32, metres_south: f32) -> f32 {
    let mut nearest = 9.0f32;
    for (start, end) in DESIRE_LINES_ACROSS_THE_LAWNS.iter() {
        nearest = nearest.min(distance_to_segment(metres_east, metres_south, *start, *end));
    }
    (1.0 - nearest / 0.6).max(0.0)
}

pub struct GroundPixel {
    pub colour: Colour,
    pub is_puddle: bool,
}

#[derive(Copy, Clone, Default)]
pub struct GroundNoise {
    pub broad_tone: u8,
    pub wear: u8,
    pub snow: u8,
    pub hollows: u8,
    pub texture: u8,
    pub detail: u8,
}

#[inline]
fn unit_level(level: u8) -> f32 {
    level as f32 / 255.0
}

const PARKING_BAY_LINE: Colour = rgb(206, 206, 200);
const ZEBRA_STRIPE: Colour = rgb(214, 214, 206);
const COURT_SURROUND: Colour = rgb(62, 122, 84);
const COURT_SURROUND_SPECK: Colour = rgb(56, 112, 78);
const COURT_PLAYING_AREA: Colour = rgb(62, 106, 150);
const COURT_LINE: Colour = rgb(226, 230, 226);
const SPRING_FLOWERS: [Colour; 3] = [rgb(222, 58, 66), rgb(246, 212, 70), rgb(140, 92, 190)];
const SUMMER_FLOWERS: [Colour; 3] = [rgb(244, 132, 60), rgb(238, 100, 150), rgb(250, 246, 236)];
const AUTUMN_FLOWERS: [Colour; 3] = [rgb(206, 114, 40), rgb(160, 44, 60), rgb(230, 186, 66)];
const BEDDING_FOLIAGE: Colour = rgb(52, 104, 54);
const BROWNED_LEAF: Colour = rgb(120, 70, 30);
const YELLOWED_LEAF: Colour = rgb(232, 196, 90);
const WINTER_LEAF_MOULD: Colour = rgb(112, 84, 58);
const PUDDLE_LIP: Colour = rgb(34, 40, 58);
const PUDDLE_SKY: Colour = rgb(128, 142, 166);
const MUDDY_HOLLOW: Colour = rgb(98, 96, 70);
const SLUSH_GREY: Colour = rgb(104, 110, 128);
const UNKNOWN_SURFACE: Colour = rgb(255, 0, 255);

fn brick_paver_colour(screen_x: i32, screen_y: i32, pixel_hash: u32) -> Colour {
    let basket_weave = ((screen_x >> 1) + (screen_y >> 1)) & 1;
    if pixel_hash.is_multiple_of(11) {
        return palette::BRICK_PAVER[0];
    }
    if pixel_hash.is_multiple_of(17) {
        return palette::BRICK_PAVER[3];
    }
    if basket_weave == 0 {
        palette::BRICK_PAVER[1]
    } else {
        palette::BRICK_PAVER[2]
    }
}

#[allow(clippy::too_many_arguments)]
pub fn shade_ground(
    hit: &SurfaceHit,
    screen_x: i32,
    screen_y: i32,
    conditions: &Conditions,
    leaf_fall_density: u8,
    fallen_leaf_colour: Colour,
    surface_above: u8,
    noise: &GroundNoise,
) -> GroundPixel {
    let metres_east = hit.metres_east;
    let metres_south = hit.metres_south;
    let surface = hit.ground_surface;
    let seed = conditions.seed;
    let lawn = &conditions.lawn_ramp;
    let pixel_hash = hash_position(screen_x, screen_y, seed ^ 0x61);
    let pixel_chance = (pixel_hash >> 8) as f32 / 16_777_216.0;
    let dither = ordered_dither_threshold(screen_x, screen_y);
    let texture = unit_level(noise.texture);
    let is_turf = surface == QUAD_LAWN || surface == OUTER_LAWN;
    let is_paved = matches!(
        surface,
        QUAD_DRIVE
            | CONCRETE_FORECOURT
            | CONCRETE_WALK
            | BRICK_PAVING
            | SERVICE_ROAD
            | PARKING_BAYS
            | ARCADE_FLOOR
            | GRANITE_KERB
            | STREET
            | TENNIS_COURT
    );

    let mut colour = match surface {
        QUAD_LAWN | OUTER_LAWN => {
            let broad_tone = unit_level(noise.broad_tone);
            let mut turf;
            if surface == QUAD_LAWN {
                let mower_stripe = ((metres_east + 400.0) / 2.6).floor() as i32 & 1;
                turf = if mower_stripe == 0 { lawn[1] } else { blend_colours(lawn[1], lawn[2], 0.5) };
                if ((broad_tone - 0.56) / 0.1).clamp(0.0, 1.0) > dither {
                    turf = blend_colours(turf, lawn[2], 0.55);
                }
                if ((0.38 - broad_tone) / 0.1).clamp(0.0, 1.0) > dither {
                    turf = blend_colours(turf, lawn[0], 0.4);
                }
            } else {
                let tone = 0.5 + (broad_tone - 0.5) * 1.5 + (texture - 0.5) * 0.35;
                turf = banded_ramp_colour(&lawn[0..3], tone, screen_x, screen_y, 0.1);
            }
            if pixel_hash.is_multiple_of(71) {
                turf = lawn[3];
            } else if pixel_hash % 59 == 1 {
                turf = lawn[0];
            }
            let wear_noise = unit_level(noise.wear);
            let mut wear = (wear_noise - 0.7) * 3.0;
            if surface == QUAD_LAWN {
                let south_of_statue = metres_south - BENJAMIN_DUKE_STATUE_SOUTH;
                let distance_from_statue =
                    ((metres_east * metres_east) + south_of_statue * south_of_statue * 6.0).sqrt();
                wear += (1.0 - distance_from_statue / 4.5).max(0.0) * 0.9;
                wear += desire_line_strength(metres_east, metres_south) * 0.45 * (0.5 + wear_noise);
                wear += (1.0 - (metres_south - 51.5) / 3.0).clamp(0.0, 1.0)
                    * (1.0 - metres_east.abs() / 8.0).max(0.0)
                    * 0.5;
            }
            if wear > dither * 0.9 + 0.12 {
                turf = blend_colours(turf, conditions.worn_lawn_colour, 0.55);
            }
            let meets_paving_above = surface_above != QUAD_LAWN
                && surface_above != OUTER_LAWN
                && surface_above != MULCH_BED
                && surface_above != FLOWER_BED;
            if meets_paving_above {
                turf = blend_colours(turf, lawn[0], 0.6);
            }
            turf
        }
        QUAD_DRIVE => {
            let mut asphalt =
                banded_ramp_colour(&palette::ASPHALT[1..4], 0.15 + texture * 0.8, screen_x, screen_y, 0.08);
            if pixel_hash.is_multiple_of(37) {
                asphalt = palette::ASPHALT[1];
            } else if pixel_hash.is_multiple_of(31) {
                asphalt = palette::ASPHALT[3];
            }
            let distance_from_crack = (unit_level(noise.detail) - 0.5).abs();
            if distance_from_crack < 0.012 && texture > 0.45 {
                asphalt = palette::ASPHALT[0];
            }
            let side = side_of_quad(metres_east);
            let on_brick_crossing = metres_east.abs() > 22.0
                && distance_to_nearest(metres_south, &hall_entrances_south(side)) < WALK_HALF_WIDTH;
            if on_brick_crossing {
                asphalt = brick_paver_colour(screen_x, screen_y, pixel_hash);
            }
            asphalt
        }
        SERVICE_ROAD | PARKING_BAYS => {
            let asphalt = banded_ramp_colour(&palette::STREET_ASPHALT, 0.2 + texture * 0.7, screen_x, screen_y, 0.08);
            let on_bay_line = surface == PARKING_BAYS && (metres_east / 2.7).rem_euclid(1.0) < 0.12;
            if on_bay_line {
                PARKING_BAY_LINE
            } else {
                asphalt
            }
        }
        STREET => {
            let asphalt = banded_ramp_colour(&palette::STREET_ASPHALT, 0.1 + texture * 0.6, screen_x, screen_y, 0.08);
            let on_loop_north_leg = (ENTRANCE_LOOP_NORTH_KERB..=ENTRANCE_LOOP_SOUTH_KERB).contains(&metres_south);
            let on_zebra_crossing = on_loop_north_leg && (metres_east.abs() - ZEBRA_CROSSING_FROM_AXIS).abs() < 2.0;
            let on_stripe = ((metres_east * 1.3).floor() as i32) & 1 == 0;
            if on_zebra_crossing && on_stripe {
                ZEBRA_STRIPE
            } else {
                asphalt
            }
        }
        TENNIS_COURT => {
            let mut court_colour = if pixel_hash.is_multiple_of(13) { COURT_SURROUND_SPECK } else { COURT_SURROUND };
            for court in TENNIS_COURTS.iter() {
                let on_court = metres_east >= court.west
                    && metres_east <= court.east
                    && metres_south >= court.north
                    && metres_south <= court.south;
                if !on_court {
                    continue;
                }
                let from_side_line = (metres_east - court.west).min(court.east - metres_east);
                let from_base_line = (metres_south - court.north).min(court.south - metres_south);
                let from_net = (metres_south - 0.5 * (court.north + court.south)).abs();
                let on_line = from_side_line < 0.4 || from_base_line < 1.3 || from_net < 1.0;
                court_colour = if on_line { COURT_LINE } else { COURT_PLAYING_AREA };
            }
            court_colour
        }
        GRANITE_KERB => {
            if pixel_hash.is_multiple_of(5) {
                palette::GRANITE[2]
            } else {
                palette::GRANITE[3]
            }
        }
        CONCRETE_FORECOURT | ARCADE_FLOOR => {
            let mut concrete =
                banded_ramp_colour(&palette::CONCRETE[1..4], 0.5 + (texture - 0.5) * 1.2, screen_x, screen_y, 0.07);
            let across_joint = ((metres_east + 60.0) / 3.75).rem_euclid(1.0);
            let along_joint = (metres_south / 9.6).rem_euclid(1.0);
            if surface == CONCRETE_FORECOURT && (across_joint < 0.09 || along_joint < 0.045) {
                concrete = palette::CONCRETE[0];
            }
            if surface == ARCADE_FLOOR {
                concrete = blend_colours(concrete, palette::CONCRETE[0], 0.45);
            }
            concrete
        }
        CONCRETE_WALK => {
            if pixel_hash.is_multiple_of(37) {
                palette::CONCRETE[0]
            } else {
                banded_ramp_colour(&palette::CONCRETE[0..3], 0.68 + (texture - 0.5) * 0.9, screen_x, screen_y, 0.07)
            }
        }
        BRICK_PAVING => brick_paver_colour(screen_x, screen_y, pixel_hash),
        MULCH_BED | FLOWER_BED => {
            let mut bed = dithered_ramp_colour(&palette::MULCH, 0.25 + texture * 0.7, screen_x, screen_y);
            if pixel_hash.is_multiple_of(9) {
                bed = palette::MULCH[0];
            }
            let at_front_of_bed = metres_east.abs() >= 25.0 || metres_south > 42.9;
            if surface == FLOWER_BED && conditions.flower_bloom > 0.0 && at_front_of_bed {
                let plant_hash = hash_position(
                    (metres_east / 0.8).floor() as i32,
                    (metres_south / 0.8).floor() as i32,
                    seed ^ 0xF10,
                );
                let flowers = match conditions.season {
                    Season::Spring => SPRING_FLOWERS,
                    Season::Summer => SUMMER_FLOWERS,
                    Season::Autumn | Season::Winter => AUTUMN_FLOWERS,
                };
                if !plant_hash.is_multiple_of(4) {
                    let in_flower = (pixel_hash >> 3) as f32 / 536_870_912.0 < conditions.flower_bloom * 0.75;
                    bed = if in_flower { flowers[((plant_hash >> 5) % 3) as usize] } else { BEDDING_FOLIAGE };
                }
            }
            bed
        }
        GRAVEL_PATH => {
            if pixel_hash.is_multiple_of(4) {
                palette::GRAVEL[2]
            } else if pixel_hash.is_multiple_of(7) {
                palette::GRAVEL[0]
            } else {
                palette::GRAVEL[1]
            }
        }
        _ => UNKNOWN_SURFACE,
    };

    if conditions.leaf_litter > 0.0 && leaf_fall_density > 0 {
        let density = leaf_fall_density as f32 / 255.0 * conditions.leaf_litter * if is_paved { 0.22 } else { 1.0 };
        let clumping = if surface == QUAD_DRIVE { 0.5 } else { unit_level(noise.detail) };
        if pixel_chance < density * (1.3 * clumping - 0.2).max(0.0) {
            let leaf = match (pixel_hash >> 4) % 3 {
                0 => fallen_leaf_colour,
                1 => blend_colours(fallen_leaf_colour, BROWNED_LEAF, 0.45),
                _ => blend_colours(fallen_leaf_colour, YELLOWED_LEAF, 0.3),
            };
            colour =
                if conditions.season == Season::Winter { blend_colours(leaf, WINTER_LEAF_MOULD, 0.75) } else { leaf };
        }
    }

    let mut is_puddle = false;
    if conditions.wetness > 0.0 {
        colour = rain_darkened(colour, conditions.wetness * if is_paved { 1.0 } else { 0.55 });
        let hollow = unit_level(noise.hollows);
        if is_paved && surface != GRANITE_KERB {
            let puddle_limit = 0.27 + 0.13 * conditions.wetness;
            if conditions.wetness > 0.25 && hollow < puddle_limit {
                is_puddle = true;
                let at_lip = hollow > puddle_limit - 0.022;
                colour = if at_lip {
                    blend_colours(colour, PUDDLE_LIP, 0.45)
                } else {
                    blend_colours(colour, PUDDLE_SKY, 0.6)
                };
            }
        } else if is_turf && conditions.wetness > 0.7 && hollow < 0.26 {
            colour = blend_colours(colour, MUDDY_HOLLOW, 0.55);
        }
    }

    if conditions.snow_cover > 0.0 {
        let snow_noise = unit_level(noise.snow);
        let clearing = if is_paved { snow_clearing(surface, metres_east, metres_south) } else { 0.0 };
        let is_covered = if is_paved {
            let would_settle =
                conditions.snow_covers_noise_level(snow_noise, 1.15 * (0.5 + 0.5 * conditions.snow_cover));
            let kept_clear = clearing * smoothstep(0.0, 0.25, conditions.snow_cover);
            would_settle && kept_clear <= dither * 0.9 + 0.05
        } else {
            let exposure = if is_turf { 1.45 } else { 1.15 };
            conditions.snow_covers_noise_level(snow_noise, exposure)
        };
        if is_covered {
            let broad_tone = unit_level(noise.broad_tone);
            let mut snow =
                banded_ramp_colour(&palette::SNOW[1..4], 0.55 + (broad_tone - 0.5) * 1.2, screen_x, screen_y, 0.08);
            if !is_paved && conditions.snow_cover < 0.55 && pixel_hash.is_multiple_of(11) {
                snow = blend_colours(lawn[1], palette::SNOW[1], 0.35);
            }
            if pixel_hash.is_multiple_of(53) {
                snow = palette::SNOW[3];
            }
            let banked_beside_cleared_path = is_paved && clearing > 0.02 && conditions.snow_cover > 0.45;
            if banked_beside_cleared_path {
                snow = palette::SNOW[3];
            }
            colour = snow;
            is_puddle = false;
            if !is_paved && conditions.snow_cover > 0.2 {
                let footprint_trail =
                    desire_line_strength(metres_east + (metres_south * 0.9).sin() * 0.3, metres_south);
                let is_footprint = ((metres_east * 1.3 + metres_south * 2.1).floor() as i32) & 1 == 0;
                if footprint_trail > 0.4 && is_footprint {
                    colour = palette::SNOW[0];
                } else if footprint_trail > 0.0 && pixel_hash.is_multiple_of(4) {
                    colour = palette::SNOW[1];
                }
            }
        } else if is_paved {
            colour = blend_colours(
                rain_darkened(colour, 0.5 * conditions.snow_cover),
                SLUSH_GREY,
                0.22 * conditions.snow_cover,
            );
            if conditions.snow_cover > 0.35 && clearing < 0.97 && pixel_hash.is_multiple_of(3) {
                colour = blend_colours(colour, palette::SNOW[1], 0.6);
            } else if conditions.snow_cover > 0.35 && pixel_hash.is_multiple_of(19) {
                colour = blend_colours(colour, palette::SNOW[1], 0.4);
            }
        } else if pixel_hash.is_multiple_of(3) {
            colour = blend_colours(colour, palette::SNOW[2], 0.6);
        }
    }
    GroundPixel { colour, is_puddle }
}

pub fn snow_clearing(surface: u8, metres_east: f32, metres_south: f32) -> f32 {
    let from_axis = metres_east.abs();
    let side = side_of_quad(metres_east);
    let cleared_cross_walk = || {
        let to_entrance = distance_to_nearest(metres_south, &hall_entrances_south(side));
        let to_cross_walk = distance_to_nearest(metres_south, &cross_walks_south(side));
        1.0 - smoothstep(0.9, 1.4, to_entrance.min(to_cross_walk))
    };
    let shovelled_path_down_the_middle = 1.0 - smoothstep(2.3, 2.9, from_axis);
    match surface {
        CONCRETE_FORECOURT => {
            if metres_south > 19.0 && metres_south < 50.0 && from_axis < 12.0 {
                shovelled_path_down_the_middle
            } else {
                0.7
            }
        }
        QUAD_DRIVE => {
            if metres_south > SOUTH_LAWN_CAP_SOUTH + 21.0 {
                return 1.0;
            }
            let in_lane = is_within_quad_lawns(metres_east, metres_south, 4.9)
                && !is_within_quad_lawns(metres_east, metres_south, 1.6);
            let near_lane = is_within_quad_lawns(metres_east, metres_south, 5.5)
                && !is_within_quad_lawns(metres_east, metres_south, 1.0);
            let lane: f32 = if in_lane {
                1.0
            } else if near_lane {
                0.5
            } else {
                0.0
            };
            let forecourt_spur = if metres_south < 49.0 { shovelled_path_down_the_middle } else { 0.0 };
            let crossing = if from_axis > 22.0 { cleared_cross_walk() } else { 0.0 };
            lane.max(forecourt_spur).max(crossing)
        }
        CONCRETE_WALK | ARCADE_FLOOR => 1.0,
        BRICK_PAVING => {
            if metres_south > ENTRANCE_LOOP_SOUTH_KERB {
                return 1.0;
            }
            let in_middle_of_band = is_within_quad_lawns(metres_east, metres_south, QUAD_DRIVE_WIDTH + 2.3)
                && !is_within_quad_lawns(metres_east, metres_south, QUAD_DRIVE_WIDTH + 0.9);
            let band: f32 = if in_middle_of_band { 1.0 } else { 0.0 };
            band.max(cleared_cross_walk())
        }
        GRANITE_KERB if from_axis > 22.0 => cleared_cross_walk(),
        SERVICE_ROAD => {
            if metres_south >= -20.0 {
                return 1.0;
            }
            let from_road_centre = (metres_south - BALDWIN_SERVICE_ROAD_SOUTH).abs();
            let from_lane_centre = if from_road_centre < 3.5 {
                from_road_centre
            } else {
                (metres_east - BALDWIN_SERVICE_ROAD_WEST_LEG_EAST)
                    .abs()
                    .min((metres_east - BALDWIN_SERVICE_ROAD_EAST_LEG_EAST).abs())
            };
            1.0 - smoothstep(0.35, 0.7, (from_lane_centre - 1.3).abs())
        }
        STREET => {
            if metres_south >= WEST_MARKHAM_AVENUE_SOUTH - WEST_MARKHAM_AVENUE_HALF_WIDTH {
                return 1.0;
            }
            let mut from_street_centre = f32::MAX;
            for street_east in TRINITY_PARK_STREETS_EAST.iter() {
                from_street_centre = from_street_centre.min((metres_east - street_east).abs());
            }
            1.0 - smoothstep(0.4, 0.8, (from_street_centre - 1.2).abs())
        }
        PARKING_BAYS => 0.35,
        _ => 0.0,
    }
}

pub fn ground_noise_at(surface: u8, metres_east: f32, metres_south: f32, seed: u32) -> GroundNoise {
    let as_level = |value: f32| (value.clamp(0.0, 1.0) * 255.0) as u8;
    let depth_squash = 0.42;
    let is_turf = surface == QUAD_LAWN || surface == OUTER_LAWN;
    let hollows = if is_turf {
        fractal_noise(metres_east * 0.12 + 20.0, metres_south * 0.2 * depth_squash + 8.0, 3, seed ^ 98)
    } else {
        fractal_noise(metres_east * 0.24 + 50.0, metres_south * 0.4 * depth_squash - 8.0, 3, seed ^ 99)
    };
    let texture = match surface {
        OUTER_LAWN | QUAD_LAWN => value_noise(metres_east * 0.25, metres_south * 0.25 * depth_squash, seed ^ 11),
        QUAD_DRIVE => fractal_noise(metres_east * 0.25, metres_south * 0.25 * depth_squash, 3, seed ^ 3),
        SERVICE_ROAD | PARKING_BAYS | STREET => {
            fractal_noise(metres_east * 0.3, metres_south * 0.3 * depth_squash, 2, seed ^ 5)
        }
        CONCRETE_FORECOURT | ARCADE_FLOOR => {
            fractal_noise(metres_east * 0.2, metres_south * 0.2 * depth_squash, 2, seed ^ 13)
        }
        CONCRETE_WALK => fractal_noise(metres_east * 0.3 + 5.0, metres_south * 0.3 * depth_squash, 2, seed ^ 17),
        _ => value_noise(metres_east * 0.9, metres_south * 0.9 * depth_squash, seed ^ 19),
    };
    let detail = if surface == QUAD_DRIVE {
        value_noise(metres_east * 0.33 + 9.0, metres_south * 0.33 * depth_squash, seed ^ 41)
    } else {
        value_noise(metres_east * 0.6 + 3.0, metres_south * 0.6 * depth_squash, seed ^ 77)
    };
    GroundNoise {
        broad_tone: as_level(fractal_noise(metres_east * 0.06 + 4.0, metres_south * 0.06 * depth_squash, 3, seed ^ 7)),
        wear: as_level(fractal_noise(metres_east * 0.17 - 9.0, metres_south * 0.17 * depth_squash + 3.0, 3, seed ^ 23)),
        snow: as_level(fractal_noise(
            metres_east * 0.22 + 31.0,
            metres_south * 0.22 * depth_squash - 17.0,
            3,
            seed ^ 0x5A0E,
        )),
        hollows: as_level(hollows),
        texture: as_level(texture),
        detail: as_level(detail),
    }
}
