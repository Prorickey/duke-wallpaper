use crate::east::campus_layout::{QUAD_HALLS, TRINITY_PARK_STREETS_EAST, WEST_MARKHAM_AVENUE_SOUTH};
use crate::east::conditions::{palette, Conditions};
use crate::east::geometry::{CANVAS_HEIGHT, CANVAS_WIDTH};
use crate::east::projection::*;
use crate::east::shading_tools::*;
use crate::east::static_layer::StaticPainter;
use crate::east::trinity_park_houses::TRINITY_PARK_HOUSE_FOOTPRINTS;
use crate::value_noise;
use crate::{blend_colours, hash_position, rgb, scale_brightness, tint_colour, Colour, Season};
use std::f32::consts::{PI, TAU};

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum TreeKind {
    Oak,
    Maple,
    GoldenMaple,
    Magnolia,
    Pine,
    Holly,
    CrepeMyrtle,
    Cherry,
    Dogwood,
    Redbud,
    Boxwood,
    Rose,
}

use TreeKind::*;

#[derive(Copy, Clone, Debug)]
pub struct Tree {
    pub metres_east: f32,
    pub metres_south: f32,
    pub crown_radius: f32,
    pub height: f32,
    pub kind: TreeKind,
    pub seed: u32,
}

pub struct ScreenEllipse {
    pub centre_x: f32,
    pub centre_y: f32,
    pub half_width: f32,
    pub half_height: f32,
}

impl Tree {
    pub fn is_evergreen(&self) -> bool {
        matches!(self.kind, Magnolia | Pine | Holly | Boxwood)
    }

    fn is_shrub(&self) -> bool {
        matches!(self.kind, Boxwood | Rose)
    }

    fn crown_half_height_and_centre_height(&self) -> (f32, f32) {
        match self.kind {
            Magnolia => (self.height * 0.48, self.height * 0.52),
            Holly => (self.height * 0.46, self.height * 0.52),
            Pine => (self.crown_radius * 0.75, self.height - self.crown_radius * 0.7),
            Boxwood | Rose => (self.height * 0.5, self.height * 0.5),
            _ => (self.crown_radius * 0.8, self.height - self.crown_radius * 0.8),
        }
    }

    pub fn crown_on_screen(&self) -> ScreenEllipse {
        let (crown_half_height, crown_centre_height) = self.crown_half_height_and_centre_height();
        let pixels_per_metre = pixels_per_metre_at(self.metres_south);
        let (centre_x, centre_y) = project_to_screen(self.metres_east, self.metres_south, crown_centre_height);
        let seen_from_above = self.crown_radius * ground_foreshortening_at(self.metres_south);
        let seen_from_the_side = crown_half_height * height_foreshortening_at(self.metres_south);
        let half_height = pixels_per_metre * (seen_from_above.powi(2) + seen_from_the_side.powi(2)).sqrt();
        ScreenEllipse { centre_x, centre_y, half_width: self.crown_radius * pixels_per_metre, half_height }
    }
}

struct Planting {
    metres_east: f32,
    metres_south: f32,
    crown_radius: f32,
    height: f32,
    kind: TreeKind,
}

const fn planting(metres_east: f32, metres_south: f32, crown_radius: f32, height: f32, kind: TreeKind) -> Planting {
    Planting { metres_east, metres_south, crown_radius, height, kind }
}

const BALDWIN_FORECOURT_MAGNOLIAS: [Planting; 2] =
    [planting(-15.4, 29.0, 7.6, 13.5, Magnolia), planting(12.8, 32.0, 7.0, 12.5, Magnolia)];

const NORTH_LAWN_SHADE_TREES: [Planting; 10] = [
    planting(11.3, 61.5, 3.4, 8.0, Maple),
    planting(-16.5, 70.6, 3.8, 9.0, GoldenMaple),
    planting(14.8, 74.4, 4.0, 9.5, Maple),
    planting(-15.6, 87.0, 4.4, 10.0, Maple),
    planting(15.0, 89.6, 4.4, 10.0, GoldenMaple),
    planting(-17.0, 104.0, 4.0, 9.5, Oak),
    planting(15.8, 108.6, 3.8, 9.0, Maple),
    planting(-16.5, 121.5, 3.6, 8.5, GoldenMaple),
    planting(14.0, 122.3, 3.8, 9.0, Oak),
    planting(-17.2, 58.0, 2.4, 5.5, Dogwood),
];

const UNION_CIRCLE_EVERGREENS: [Planting; 4] = [
    planting(29.6, 144.6, 4.0, 9.0, Magnolia),
    planting(29.0, 187.0, 4.2, 9.5, Magnolia),
    planting(-36.5, 186.5, 3.4, 8.0, Holly),
    planting(-37.5, 143.5, 3.4, 8.0, Holly),
];

const SOUTH_LAWN_SHADE_TREES: [Planting; 9] = [
    planting(-16.6, 210.0, 3.8, 9.0, Maple),
    planting(-16.2, 226.0, 4.2, 10.0, Oak),
    planting(-16.8, 247.0, 4.0, 9.5, GoldenMaple),
    planting(-17.0, 276.0, 4.4, 10.5, Maple),
    planting(-16.4, 295.0, 4.2, 10.0, Oak),
    planting(14.6, 213.0, 3.6, 8.5, GoldenMaple),
    planting(15.2, 238.0, 4.0, 9.5, Maple),
    planting(13.4, 268.0, 4.2, 10.0, Maple),
    planting(14.8, 286.0, 4.0, 9.5, GoldenMaple),
];

const HALL_FRONT_ORNAMENTALS: [TreeKind; 4] = [CrepeMyrtle, Dogwood, Cherry, Redbud];

const TREES_AROUND_BALDWIN: [Planting; 6] = [
    planting(-22.5, 41.0, 2.6, 5.5, Cherry),
    planting(22.0, 40.0, 2.4, 5.2, Dogwood),
    planting(-24.5, 6.0, 3.0, 7.5, Holly),
    planting(24.8, 7.5, 3.2, 8.0, Holly),
    planting(29.4, -22.0, 4.6, 10.0, Magnolia),
    planting(-22.0, -6.0, 2.0, 5.0, Holly),
];

const HALL_COURTYARD_TREES: [Planting; 8] = [
    planting(-59.0, 35.0, 7.4, 13.0, Magnolia),
    planting(58.0, 31.0, 3.4, 7.0, Dogwood),
    planting(60.5, 45.0, 4.2, 9.5, Maple),
    planting(-60.0, 102.0, 6.0, 13.0, Oak),
    planting(58.0, 96.0, 6.2, 12.0, Magnolia),
    planting(60.0, 113.0, 5.6, 11.0, Magnolia),
    planting(-58.5, 225.0, 5.0, 11.0, Maple),
    planting(-60.0, 293.0, 5.6, 12.5, Oak),
];

const TREES_BETWEEN_THE_BUILDINGS: [Planting; 8] = [
    planting(-64.0, 136.0, 4.4, 10.0, Maple),
    planting(-66.0, 191.5, 4.6, 10.5, GoldenMaple),
    planting(-56.0, 259.5, 4.6, 10.5, Maple),
    planting(58.0, 139.0, 4.2, 9.5, GoldenMaple),
    planting(60.0, 194.5, 4.4, 10.0, Oak),
    planting(54.0, 261.0, 4.4, 10.0, GoldenMaple),
    planting(-53.0, 69.0, 4.6, 10.5, Maple),
    planting(54.0, 71.5, 4.4, 10.0, GoldenMaple),
];

const HARDWOODS_NORTH_WEST_OF_BALDWIN: [Planting; 12] = [
    planting(-31.6, -37.0, 9.0, 20.0, Oak),
    planting(-51.6, -30.0, 8.5, 19.0, Oak),
    planting(-62.0, -10.0, 8.0, 18.0, Oak),
    planting(-41.0, -19.0, 6.5, 15.0, GoldenMaple),
    planting(-71.0, 3.0, 6.0, 14.0, Maple),
    planting(-36.0, -4.0, 4.0, 9.0, Maple),
    planting(-9.0, -27.5, 5.0, 12.0, Maple),
    planting(11.0, -26.5, 4.6, 11.0, GoldenMaple),
    planting(-84.0, 22.0, 7.5, 17.0, Oak),
    planting(-128.0, 26.0, 8.0, 18.0, Oak),
    planting(-106.0, 34.0, 6.5, 15.0, Maple),
    planting(-150.0, 10.0, 7.5, 17.0, GoldenMaple),
];

const TREES_WEST_OF_THE_HALLS: [Planting; 13] = [
    planting(-77.0, 44.0, 7.6, 17.0, Oak),
    planting(-76.0, 62.0, 6.4, 15.0, GoldenMaple),
    planting(-80.0, 118.0, 7.0, 16.0, Oak),
    planting(-82.0, 146.0, 6.6, 15.0, Maple),
    planting(-84.0, 180.0, 7.4, 17.0, Oak),
    planting(-80.0, 208.0, 6.8, 16.0, Oak),
    planting(-79.0, 240.0, 6.4, 15.0, GoldenMaple),
    planting(-81.0, 268.0, 7.2, 17.0, Oak),
    planting(-80.0, 300.0, 6.8, 16.0, Maple),
    planting(-82.0, 326.0, 7.4, 17.0, Oak),
    planting(-100.0, 190.0, 7.0, 16.0, Oak),
    planting(-112.0, 212.0, 6.4, 15.0, Maple),
    planting(-120.0, 92.0, 5.6, 13.0, Pine),
];

const LOBLOLLY_PINES_NORTH_EAST_OF_BALDWIN: [Planting; 16] = [
    planting(44.0, -24.0, 3.8, 21.0, Pine),
    planting(55.0, -13.0, 4.2, 23.0, Pine),
    planting(63.0, -27.0, 3.6, 20.0, Pine),
    planting(70.0, -7.0, 4.0, 22.0, Pine),
    planting(79.0, -20.0, 4.4, 24.0, Pine),
    planting(87.0, -2.0, 3.8, 21.0, Pine),
    planting(93.0, -27.0, 4.0, 22.0, Pine),
    planting(99.0, -12.0, 4.2, 23.0, Pine),
    planting(106.0, 5.0, 4.0, 22.0, Pine),
    planting(60.0, -1.0, 3.6, 19.0, Pine),
    planting(76.0, 3.0, 3.4, 18.0, Pine),
    planting(97.0, 14.0, 4.0, 21.0, Pine),
    planting(104.0, 52.0, 4.2, 22.0, Pine),
    planting(112.0, 64.0, 3.8, 20.0, Pine),
    planting(100.0, 132.0, 4.0, 21.0, Pine),
    planting(124.0, 6.0, 4.0, 22.0, Pine),
];

const TREES_EAST_OF_THE_HALLS: [Planting; 13] = [
    planting(38.0, -9.0, 6.0, 14.0, Oak),
    planting(24.0, -33.0, 5.5, 13.0, Oak),
    planting(128.0, 50.0, 7.0, 16.0, Oak),
    planting(98.0, 152.0, 7.0, 16.0, Oak),
    planting(104.0, 176.0, 6.4, 15.0, Maple),
    planting(82.0, 192.0, 5.6, 13.0, GoldenMaple),
    planting(122.0, 198.0, 7.0, 16.0, Oak),
    planting(100.0, 238.0, 7.4, 17.0, Oak),
    planting(81.0, 246.0, 5.0, 12.0, Maple),
    planting(84.0, 292.0, 6.6, 15.0, Oak),
    planting(102.0, 304.0, 7.0, 16.0, GoldenMaple),
    planting(86.0, 326.0, 6.4, 15.0, Oak),
    planting(112.0, 336.0, 7.2, 17.0, Maple),
];

const TREES_AROUND_THE_ENTRANCE_LOOP: [Planting; 18] = [
    planting(-41.5, 331.0, 4.2, 9.5, Magnolia),
    planting(-25.5, 354.0, 4.0, 9.5, Maple),
    planting(-26.5, 376.0, 3.4, 8.0, GoldenMaple),
    planting(-43.0, 352.0, 5.0, 11.5, Oak),
    planting(-42.5, 368.0, 5.2, 12.0, Maple),
    planting(41.5, 347.5, 4.4, 10.0, Maple),
    planting(42.5, 361.0, 5.0, 11.5, Oak),
    planting(41.5, 376.0, 5.0, 11.5, GoldenMaple),
    planting(-88.0, 352.0, 7.0, 16.0, Oak),
    planting(80.0, 352.0, 6.6, 15.0, GoldenMaple),
    planting(-74.0, 371.0, 6.0, 14.0, Oak),
    planting(-43.5, 386.0, 5.0, 11.5, GoldenMaple),
    planting(43.0, 393.0, 5.2, 12.0, Oak),
    planting(-96.0, 386.0, 6.6, 15.0, Maple),
    planting(88.0, 381.0, 6.4, 14.5, Oak),
    planting(-122.0, 372.0, 7.0, 16.0, Oak),
    planting(118.0, 366.0, 7.0, 16.0, Oak),
    planting(108.0, 396.0, 6.6, 15.0, GoldenMaple),
];

pub fn all_trees() -> Vec<Tree> {
    let mut trees: Vec<Tree> = Vec::new();
    let mut plant = |metres_east: f32, metres_south: f32, crown_radius: f32, height: f32, kind: TreeKind| {
        let seed = hash_position((metres_east * 10.0) as i32, (metres_south * 10.0) as i32, 0xEA57);
        trees.push(Tree { metres_east, metres_south, crown_radius, height, kind, seed });
    };
    let lawn_trees = BALDWIN_FORECOURT_MAGNOLIAS
        .iter()
        .chain(&NORTH_LAWN_SHADE_TREES)
        .chain(&UNION_CIRCLE_EVERGREENS)
        .chain(&SOUTH_LAWN_SHADE_TREES);
    for tree in lawn_trees {
        plant(tree.metres_east, tree.metres_south, tree.crown_radius, tree.height, tree.kind);
    }

    for (hall_index, hall) in QUAD_HALLS.iter().enumerate() {
        let side = hall.side_of_quad;
        let entrance_south = hall.entrance_south();
        let strip_positions = [hall.north_end + 6.0, entrance_south - 9.0, entrance_south + 9.5, hall.south_end - 6.5];
        for (position_index, metres_south) in strip_positions.iter().enumerate() {
            let kind =
                HALL_FRONT_ORNAMENTALS[(hall_index * 3 + position_index * 5 + if side > 0.0 { 1 } else { 0 }) % 4];
            let crown_radius = 2.0 + 0.1 * ((hall_index + position_index * 3) % 5) as f32;
            plant(side * 30.9, *metres_south, crown_radius, crown_radius * 2.3, kind);
        }
        plant(side * 31.6, entrance_south - 3.0, 0.9, 1.3, Boxwood);
        plant(side * 31.6, entrance_south + 3.0, 0.9, 1.3, Boxwood);
    }

    let campus_trees = TREES_AROUND_BALDWIN
        .iter()
        .chain(&HALL_COURTYARD_TREES)
        .chain(&TREES_BETWEEN_THE_BUILDINGS)
        .chain(&HARDWOODS_NORTH_WEST_OF_BALDWIN)
        .chain(&TREES_WEST_OF_THE_HALLS)
        .chain(&LOBLOLLY_PINES_NORTH_EAST_OF_BALDWIN)
        .chain(&TREES_EAST_OF_THE_HALLS)
        .chain(&TREES_AROUND_THE_ENTRANCE_LOOP);
    for tree in campus_trees {
        plant(tree.metres_east, tree.metres_south, tree.crown_radius, tree.height, tree.kind);
    }

    for index in 0..17 {
        let metres_east = -150.0 + index as f32 * 18.5 + (unit_hash(index, 1, 0x3A2) - 0.5) * 7.0;
        let in_a_service_loop_entrance = (-14.0..10.0).contains(&metres_east) || (52.0..68.0).contains(&metres_east);
        if in_a_service_loop_entrance {
            continue;
        }
        let metres_south = -60.0 + (unit_hash(index, 2, 0x3A2) - 0.5) * 9.0;
        let kind = match hash_position(index, 3, 0x3A2) % 5 {
            0 | 1 => Oak,
            2 => Pine,
            3 => Maple,
            _ => GoldenMaple,
        };
        let crown_radius = if kind == Pine { 4.0 } else { 6.0 + unit_hash(index, 4, 0x3A2) * 2.5 };
        plant(metres_east, metres_south, crown_radius, if kind == Pine { 21.0 } else { crown_radius * 2.3 }, kind);
    }

    for side in [-1.0f32, 1.0] {
        plant(side * 11.0, 18.3, 1.0, 1.4, Boxwood);
        plant(side * 13.2, 18.4, 1.1, 1.5, Boxwood);
        plant(side * 15.3, 18.3, 1.0, 1.4, Boxwood);
        for along_bed in 0..5 {
            for row in 0..2 {
                let metres_east = side * (10.2 + along_bed as f32 * 2.4 + row as f32 * 0.9);
                let metres_south = 39.6 + row as f32 * 2.6 + (along_bed % 2) as f32 * 0.5;
                plant(metres_east, metres_south, 1.05, 1.1, Rose);
            }
        }
    }

    for (street_index, &street_east) in TRINITY_PARK_STREETS_EAST.iter().enumerate() {
        for side in [-1.0f32, 1.0] {
            let mut metres_south =
                WEST_MARKHAM_AVENUE_SOUTH - 16.0 - unit_hash(street_index as i32, side as i32, 0x57) * 8.0;
            let mut tree_number = 0;
            while metres_south > -250.0 {
                let tree_hash = hash_position(street_index as i32 * 31 + tree_number, side as i32, 0x58);
                let kind = match tree_hash % 6 {
                    0..=2 => Oak,
                    4 => GoldenMaple,
                    _ => Maple,
                };
                let crown_radius = 5.2 + (tree_hash >> 5) as f32 % 3.0;
                plant(street_east + side * 9.0, metres_south, crown_radius, crown_radius * 2.5, kind);
                metres_south -= 15.0 + (tree_hash >> 9) as f32 % 9.0;
                tree_number += 1;
            }
        }
    }

    for index in 0..150 {
        let metres_east = -165.0 + unit_hash(index, 11, 0x7A9) * 330.0;
        let metres_south = -98.0 - unit_hash(index, 12, 0x7A9) * 150.0;
        let near_a_street =
            TRINITY_PARK_STREETS_EAST.iter().any(|street_east| (metres_east - street_east).abs() < 12.5);
        let margin = 2.5;
        let on_a_house = TRINITY_PARK_HOUSE_FOOTPRINTS.iter().any(|house| {
            metres_east > house.west - margin
                && metres_east < house.east + margin
                && metres_south > house.north - margin
                && metres_south < house.south + margin
        });
        if near_a_street || on_a_house {
            continue;
        }
        let tree_hash = hash_position(index, 13, 0x7A9);
        let kind = match tree_hash % 10 {
            0..=4 => Oak,
            5 | 6 => Maple,
            7 => GoldenMaple,
            8 => Pine,
            _ => Magnolia,
        };
        let (crown_radius, height) = match kind {
            Pine => (4.0, 22.0),
            Magnolia => (5.0, 10.0),
            _ => {
                let crown_radius = 5.5 + (tree_hash >> 6) as f32 % 4.0;
                (crown_radius, crown_radius * 2.4)
            }
        };
        plant(metres_east, metres_south, crown_radius, height, kind);
    }

    trees.sort_by(|first, second| first.metres_south.total_cmp(&second.metres_south));
    trees
}

pub struct Foliage {
    pub leaf_cover: f32,
    pub leaf_colours: [Colour; 4],
    pub blossom_share: f32,
    pub blossom_colours: [Colour; 3],
}

type LeafRamp = [Colour; 4];

const OAK_SUMMER: LeafRamp = [rgb(24, 66, 50), rgb(38, 92, 52), rgb(60, 118, 56), rgb(92, 146, 66)];
const MAPLE_SUMMER: LeafRamp = [rgb(28, 74, 50), rgb(44, 102, 54), rgb(68, 130, 58), rgb(104, 158, 70)];
const GOLDEN_MAPLE_SUMMER: LeafRamp = [rgb(34, 80, 48), rgb(52, 108, 52), rgb(80, 136, 58), rgb(118, 164, 72)];
const MAGNOLIA_LEAVES: LeafRamp = [rgb(16, 46, 42), rgb(24, 66, 46), rgb(38, 90, 54), rgb(72, 124, 76)];
const PINE_NEEDLES: LeafRamp = [rgb(20, 52, 46), rgb(30, 72, 50), rgb(48, 94, 56), rgb(76, 118, 68)];
const HOLLY_LEAVES: LeafRamp = [rgb(18, 52, 40), rgb(28, 74, 46), rgb(46, 98, 52), rgb(74, 124, 62)];
const ROSE_LEAVES: LeafRamp = [rgb(30, 70, 44), rgb(46, 96, 50), rgb(66, 120, 56), rgb(96, 146, 70)];
const ORNAMENTAL_SUMMER: LeafRamp = [rgb(32, 78, 46), rgb(50, 106, 50), rgb(76, 134, 56), rgb(112, 162, 70)];
const FRESH_SPRING_LEAVES: LeafRamp = [rgb(60, 112, 48), rgb(92, 148, 54), rgb(128, 178, 62), rgb(170, 206, 88)];
const WINTER_EVERGREEN: LeafRamp = [rgb(22, 48, 46), rgb(32, 66, 50), rgb(48, 86, 56), rgb(74, 110, 70)];
const DORMANT_ROSE: LeafRamp = [rgb(70, 62, 50), rgb(92, 82, 58), rgb(112, 100, 66), rgb(130, 118, 78)];

const RED_MAPLE_AUTUMN: LeafRamp = [rgb(112, 30, 40), rgb(160, 44, 38), rgb(204, 72, 40), rgb(236, 118, 54)];
const ORANGE_MAPLE_AUTUMN: LeafRamp = [rgb(134, 58, 30), rgb(186, 86, 32), rgb(222, 122, 40), rgb(244, 164, 64)];
const GOLDEN_MAPLE_AUTUMN: LeafRamp = [rgb(140, 100, 34), rgb(186, 138, 36), rgb(220, 174, 50), rgb(244, 208, 88)];
const OAK_AUTUMN: LeafRamp = [rgb(88, 48, 36), rgb(124, 68, 38), rgb(156, 92, 44), rgb(186, 122, 58)];
const DOGWOOD_AUTUMN: LeafRamp = [rgb(98, 30, 44), rgb(136, 40, 46), rgb(170, 58, 50), rgb(200, 88, 62)];
const CHERRY_AUTUMN: LeafRamp = [rgb(140, 78, 30), rgb(188, 112, 34), rgb(222, 148, 46), rgb(242, 186, 78)];
const CREPE_MYRTLE_AUTUMN: LeafRamp = [rgb(128, 50, 30), rgb(176, 74, 32), rgb(212, 104, 40), rgb(238, 146, 60)];
const REDBUD_AUTUMN: LeafRamp = [rgb(136, 110, 36), rgb(178, 150, 44), rgb(210, 184, 62), rgb(232, 212, 100)];

const WHITE_BLOSSOM: [Colour; 3] = [rgb(206, 208, 196), rgb(234, 234, 222), rgb(252, 250, 244)];
const MAGNOLIA_BLOSSOM: [Colour; 3] = [rgb(214, 212, 190), rgb(240, 238, 220), rgb(252, 250, 240)];
const CHERRY_BLOSSOM: [Colour; 3] = [rgb(214, 136, 164), rgb(242, 180, 200), rgb(255, 222, 232)];
const REDBUD_BLOSSOM: [Colour; 3] = [rgb(150, 60, 124), rgb(196, 92, 156), rgb(226, 140, 188)];
const PINK_DOGWOOD_BLOSSOM: [Colour; 3] = [rgb(222, 150, 170), rgb(244, 190, 204), rgb(255, 226, 232)];
const PINK_CREPE_MYRTLE_BLOSSOM: [Colour; 3] = [rgb(196, 62, 112), rgb(228, 100, 142), rgb(248, 156, 180)];
const LILAC_CREPE_MYRTLE_BLOSSOM: [Colour; 3] = [rgb(150, 96, 170), rgb(186, 132, 200), rgb(216, 176, 224)];

fn mix_ramps(from: LeafRamp, to: LeafRamp, amount: f32) -> LeafRamp {
    [
        blend_colours(from[0], to[0], amount),
        blend_colours(from[1], to[1], amount),
        blend_colours(from[2], to[2], amount),
        blend_colours(from[3], to[3], amount),
    ]
}

fn rise_and_fall(progress: f32, rise_start: f32, rise_end: f32, fall_start: f32, fall_end: f32) -> f32 {
    smoothstep(rise_start, rise_end, progress) * (1.0 - smoothstep(fall_start, fall_end, progress))
}

pub fn foliage_in_season(tree: &Tree, conditions: &Conditions) -> Foliage {
    let warmth_variation = (tree.seed >> 8) as f32 / 16_777_216.0;
    let lightness_variation = (tree.seed & 0xFF) as f32 / 255.0;
    let progress = conditions.season_progress;
    let summer = match tree.kind {
        Oak => OAK_SUMMER,
        Maple => MAPLE_SUMMER,
        GoldenMaple => GOLDEN_MAPLE_SUMMER,
        Magnolia => MAGNOLIA_LEAVES,
        Pine => PINE_NEEDLES,
        Holly | Boxwood => HOLLY_LEAVES,
        Rose => ROSE_LEAVES,
        _ => ORNAMENTAL_SUMMER,
    };
    let autumn = match tree.kind {
        Maple if lightness_variation < 0.55 => RED_MAPLE_AUTUMN,
        Maple => ORANGE_MAPLE_AUTUMN,
        GoldenMaple => GOLDEN_MAPLE_AUTUMN,
        Oak => OAK_AUTUMN,
        Dogwood => DOGWOOD_AUTUMN,
        Cherry => CHERRY_AUTUMN,
        CrepeMyrtle => CREPE_MYRTLE_AUTUMN,
        Redbud => REDBUD_AUTUMN,
        _ => summer,
    };
    let give_this_tree_its_own_tint = |ramp: LeafRamp| -> LeafRamp {
        let warmth = warmth_variation - 0.5;
        let lightness = 0.94 + 0.12 * lightness_variation;
        let tint =
            [lightness * (1.0 + 0.14 * warmth), lightness * (1.0 + 0.03 * warmth), lightness * (1.0 - 0.2 * warmth)];
        [tint_colour(ramp[0], tint), tint_colour(ramp[1], tint), tint_colour(ramp[2], tint), tint_colour(ramp[3], tint)]
    };
    let summer = give_this_tree_its_own_tint(summer);
    let autumn =
        if matches!(tree.kind, Maple | GoldenMaple | Oak) { give_this_tree_its_own_tint(autumn) } else { autumn };
    let mut foliage = Foliage { leaf_cover: 1.0, leaf_colours: summer, blossom_share: 0.0, blossom_colours: [0; 3] };

    if tree.is_evergreen() {
        if conditions.season == Season::Winter {
            foliage.leaf_colours = mix_ramps(summer, WINTER_EVERGREEN, 0.6);
        }
        if tree.kind == Magnolia {
            foliage.blossom_share = match conditions.season {
                Season::Spring => 0.07 * smoothstep(0.55, 0.9, progress),
                Season::Summer => 0.07 * (1.0 - smoothstep(0.3, 0.7, progress)),
                _ => 0.0,
            };
            foliage.blossom_colours = MAGNOLIA_BLOSSOM;
        }
        return foliage;
    }

    match conditions.season {
        Season::Spring => {
            let leaf_out_start = 0.02 + 0.14 * warmth_variation;
            foliage.leaf_cover = 0.12 + 0.88 * smoothstep(leaf_out_start, leaf_out_start + 0.5, progress);
            foliage.leaf_colours = mix_ramps(FRESH_SPRING_LEAVES, summer, smoothstep(0.45, 1.0, progress) * 0.7);
            match tree.kind {
                Cherry => {
                    foliage.blossom_share = rise_and_fall(progress, 0.0, 0.14, 0.34, 0.5);
                    foliage.blossom_colours = CHERRY_BLOSSOM;
                }
                Redbud => {
                    foliage.blossom_share = rise_and_fall(progress, -0.05, 0.1, 0.3, 0.45);
                    foliage.blossom_colours = REDBUD_BLOSSOM;
                }
                Dogwood => {
                    foliage.blossom_share = rise_and_fall(progress, 0.08, 0.22, 0.45, 0.62);
                    foliage.blossom_colours =
                        if lightness_variation < 0.3 { PINK_DOGWOOD_BLOSSOM } else { WHITE_BLOSSOM };
                }
                Rose => {
                    foliage.leaf_cover = 1.0;
                    foliage.blossom_share = 0.5 * smoothstep(0.5, 0.8, progress);
                    foliage.blossom_colours = WHITE_BLOSSOM;
                }
                CrepeMyrtle => foliage.leaf_cover = 0.05 + 0.95 * smoothstep(0.25, 0.75, progress),
                _ => {}
            }
            if foliage.blossom_share > 0.1 {
                foliage.leaf_cover = foliage.leaf_cover.max(foliage.blossom_share);
            }
        }
        Season::Summer => {
            foliage.leaf_colours = mix_ramps(FRESH_SPRING_LEAVES, summer, 0.7 + 0.3 * smoothstep(0.0, 0.3, progress));
            match tree.kind {
                CrepeMyrtle => {
                    foliage.blossom_share = 0.75 * rise_and_fall(progress, 0.05, 0.3, 0.8, 1.05);
                    foliage.blossom_colours = match tree.seed % 3 {
                        0 => PINK_CREPE_MYRTLE_BLOSSOM,
                        1 => WHITE_BLOSSOM,
                        _ => LILAC_CREPE_MYRTLE_BLOSSOM,
                    };
                }
                Rose => {
                    foliage.blossom_share = 0.55;
                    foliage.blossom_colours = WHITE_BLOSSOM;
                }
                _ => {}
            }
        }
        Season::Autumn => {
            let turning_start = match tree.kind {
                Dogwood => 0.06,
                Cherry => 0.1,
                Maple => 0.16,
                CrepeMyrtle => 0.2,
                Redbud => 0.2,
                GoldenMaple => 0.26,
                Oak => 0.38,
                _ => 0.3,
            } + (warmth_variation - 0.5) * 0.22;
            foliage.leaf_colours = mix_ramps(summer, autumn, smoothstep(turning_start, turning_start + 0.26, progress));
            foliage.leaf_cover = 1.0 - smoothstep(turning_start + 0.34, turning_start + 0.64, progress);
            if tree.kind == Oak {
                foliage.leaf_cover = foliage.leaf_cover.max(0.14);
            }
            if tree.kind == Rose {
                foliage.leaf_cover = 1.0;
                foliage.leaf_colours = summer;
                foliage.blossom_share = 0.3 * (1.0 - smoothstep(0.2, 0.6, progress));
                foliage.blossom_colours = WHITE_BLOSSOM;
            }
        }
        Season::Winter => {
            foliage.leaf_colours = autumn;
            foliage.leaf_cover = if tree.kind == Oak { 0.1 * (1.0 - smoothstep(0.2, 0.7, progress)) } else { 0.0 };
            if tree.kind == Rose {
                foliage.leaf_cover = 0.5;
                foliage.leaf_colours = mix_ramps(summer, DORMANT_ROSE, 0.75);
            }
        }
    }
    foliage
}

pub fn stamp_tree_shadows(trees: &[Tree], conditions: &Conditions, shadow_mask: &mut [u8]) {
    for tree in trees {
        let leaf_cover = foliage_in_season(tree, conditions).leaf_cover;
        let (crown_half_height, crown_centre_height) = tree.crown_half_height_and_centre_height();
        let pixels_per_metre = pixels_per_metre_at(tree.metres_south);
        let rows_per_metre = screen_rows_per_metre_of_height_at(tree.metres_south);
        let (foot_x, foot_y) = project_to_screen(tree.metres_east, tree.metres_south, 0.0);
        let crown_height_in_rows = crown_centre_height * rows_per_metre;
        let centre_x = foot_x + conditions.shadow_x_per_height * crown_height_in_rows;
        let centre_y = foot_y + conditions.shadow_y_per_height * crown_height_in_rows;
        let half_width = tree.crown_radius * pixels_per_metre * 0.95;
        let half_height = (tree.crown_radius * pixels_per_metre * ground_foreshortening_at(tree.metres_south) * 0.95)
            .max(1.0)
            + crown_half_height * rows_per_metre * conditions.shadow_y_per_height.abs() * 0.5;
        let density = if tree.is_evergreen() { 1.0 } else { 0.22 + 0.78 * leaf_cover };
        let (first_column, last_column) =
            ((centre_x - half_width).floor() as i32, (centre_x + half_width).ceil() as i32);
        let (first_row, last_row) = ((centre_y - half_height).floor() as i32, (centre_y + half_height).ceil() as i32);
        for y in first_row.max(0)..=last_row.min(CANVAS_HEIGHT as i32 - 1) {
            for x in first_column.max(0)..=last_column.min(CANVAS_WIDTH as i32 - 1) {
                let (across, down) =
                    ((x as f32 + 0.5 - centre_x) / half_width, (y as f32 + 0.5 - centre_y) / half_height);
                let distance_squared = across * across + down * down;
                if distance_squared > 1.0 {
                    continue;
                }
                let ragged_edge = 0.72 + value_noise(x as f32 * 0.5, y as f32 * 0.5, tree.seed) * 0.28;
                if distance_squared > ragged_edge {
                    continue;
                }
                if density < 1.0 && unit_hash(x, y, tree.seed ^ 5) > density {
                    continue;
                }
                shadow_mask[y as usize * CANVAS_WIDTH + x as usize] = 255;
            }
        }
        let trunk_height_in_rows = (crown_centre_height - crown_half_height).max(0.0) * rows_per_metre;
        if trunk_height_in_rows > 3.0 {
            for step in 0..trunk_height_in_rows as i32 {
                let x = (foot_x + conditions.shadow_x_per_height * step as f32) as i32;
                let y = (foot_y + conditions.shadow_y_per_height * step as f32) as i32;
                if x >= 0 && y >= 0 && (x as usize) < CANVAS_WIDTH && (y as usize) < CANVAS_HEIGHT {
                    shadow_mask[y as usize * CANVAS_WIDTH + x as usize] = 255;
                }
            }
        }
    }
}

#[derive(Copy, Clone, Default)]
pub struct CrownPixel {
    coverage: u8,
    normal_east: u8,
    normal_down: u8,
    shading_noise: u8,
    thinning_noise: u8,
}

pub struct CrownMap {
    left: i32,
    top: i32,
    width: usize,
    height: usize,
    pixels: Vec<CrownPixel>,
}

struct LeafClump {
    centre_x: f32,
    centre_y: f32,
    radius: f32,
    height_bias: f32,
}

pub fn crown_map(tree: &Tree) -> CrownMap {
    let crown = tree.crown_on_screen();
    let clump_count =
        if tree.is_shrub() { 2 } else { (5.0 + crown.half_width * crown.half_height / 26.0).clamp(5.0, 26.0) as i32 };
    let is_pine = tree.kind == Pine;
    let mut clumps = Vec::with_capacity(clump_count as usize + 1);
    clumps.push(LeafClump {
        centre_x: crown.centre_x,
        centre_y: crown.centre_y + crown.half_height * 0.05,
        radius: crown.half_width * 0.66,
        height_bias: 0.2,
    });
    for clump in 0..clump_count {
        let angle = unit_hash(clump, 1, tree.seed) * TAU;
        let distance_out = unit_hash(clump, 2, tree.seed).sqrt() * if is_pine { 0.82 } else { 0.66 };
        let radius = crown.half_width * (if is_pine { 0.22 } else { 0.3 } + 0.22 * unit_hash(clump, 3, tree.seed));
        let height_bias =
            (1.0 - distance_out) * 0.55 + unit_hash(clump, 4, tree.seed) * 0.45 - angle.sin() * distance_out * 0.35;
        clumps.push(LeafClump {
            centre_x: crown.centre_x + angle.cos() * distance_out * crown.half_width,
            centre_y: crown.centre_y + angle.sin() * distance_out * crown.half_height,
            radius: radius.max(1.2),
            height_bias,
        });
    }
    let squash = (crown.half_height / crown.half_width).clamp(0.5, 1.6);
    let (left, right) = (
        (crown.centre_x - crown.half_width - 2.0).floor() as i32,
        (crown.centre_x + crown.half_width + 2.0).ceil() as i32,
    );
    let (top, bottom) = (
        (crown.centre_y - crown.half_height - 2.0).floor() as i32,
        (crown.centre_y + crown.half_height + 2.0).ceil() as i32,
    );
    let width = (right - left + 1) as usize;
    let height = (bottom - top + 1) as usize;
    let mut pixels = vec![CrownPixel::default(); width * height];
    for y in top.max(-2)..=bottom.min(CANVAS_HEIGHT as i32 + 1) {
        for x in left.max(-2)..=right.min(CANVAS_WIDTH as i32 + 1) {
            let (pixel_x, pixel_y) = (x as f32 + 0.5, y as f32 + 0.5);
            let mut front_clump: Option<(f32, f32, f32, f32)> = None;
            for clump in clumps.iter() {
                let across = (pixel_x - clump.centre_x) / clump.radius;
                let down = (pixel_y - clump.centre_y) / (clump.radius * squash);
                let coverage = 1.0 - across * across - down * down;
                if coverage <= 0.0 {
                    continue;
                }
                let nearness = clump.height_bias + coverage * 0.6;
                if front_clump.is_none_or(|(best_nearness, ..)| nearness > best_nearness) {
                    front_clump = Some((nearness, coverage, across, down));
                }
            }
            let Some((_, coverage, across, down)) = front_clump else { continue };
            pixels[(y - top) as usize * width + (x - left) as usize] = CrownPixel {
                coverage: 1 + (coverage.clamp(0.0, 1.0) * 254.0) as u8,
                normal_east: ((across * 0.5 + 0.5).clamp(0.0, 1.0) * 255.0) as u8,
                normal_down: ((down * 0.5 + 0.5).clamp(0.0, 1.0) * 255.0) as u8,
                shading_noise: (value_noise(pixel_x * 0.42, pixel_y * 0.42, tree.seed ^ 0x13) * 255.0) as u8,
                thinning_noise: (value_noise(pixel_x * 0.55, pixel_y * 0.55, tree.seed ^ 0x31) * 255.0) as u8,
            };
        }
    }
    CrownMap { left, top, width, height, pixels }
}

const CONTACT_SHADOW: Colour = rgb(20, 30, 40);
const PINE_BARK: Colour = rgb(132, 92, 72);
const TWIG: Colour = rgb(122, 108, 104);
const WAXY_GLINT: Colour = rgb(120, 164, 120);

pub fn draw_tree(
    painter: &mut StaticPainter,
    tree: &Tree,
    crown_map: &CrownMap,
    sway_id: u16,
    conditions: &Conditions,
) {
    let foliage = foliage_in_season(tree, conditions);
    let crown = tree.crown_on_screen();
    let (foot_x, foot_y) = project_to_screen(tree.metres_east, tree.metres_south, 0.0);
    let pixels_per_metre = pixels_per_metre_at(tree.metres_south);
    let depth = tree.metres_south + 0.05;
    let is_shrub = tree.is_shrub();
    painter.current_tree_id = 0;

    if !is_shrub {
        let shadow_half_width = crown.half_width * 0.55;
        let shadow_half_height = (crown.half_width * 0.55 * ground_foreshortening_at(tree.metres_south) * 0.8).max(1.0);
        for y in (foot_y - shadow_half_height) as i32..=(foot_y + shadow_half_height) as i32 {
            for x in (foot_x - shadow_half_width) as i32..=(foot_x + shadow_half_width) as i32 {
                let (across, down) =
                    ((x as f32 + 0.5 - foot_x) / shadow_half_width, (y as f32 + 0.5 - foot_y) / shadow_half_height);
                if across * across + down * down < 1.0 && (x + y) & 1 == 0 {
                    painter.blend(x, y, tree.metres_south - 0.3, CONTACT_SHADOW, 0.3);
                }
            }
        }
    }

    let is_bare = foliage.leaf_cover < 0.97 && !tree.is_evergreen() && !is_shrub;
    let trunk_width = ((tree.crown_radius * pixels_per_metre) / 7.0).round().clamp(1.0, 4.0) as i32;
    if !is_shrub {
        let (crown_half_height, crown_centre_height) = tree.crown_half_height_and_centre_height();
        let crown_bottom_y = project_to_screen(
            tree.metres_east,
            tree.metres_south,
            (crown_centre_height - crown_half_height * 0.5).max(0.3),
        )
        .1;
        let trunk_top_y = if is_bare { crown.centre_y + crown.half_height * 0.35 } else { crown_bottom_y };
        let (first_row, last_row) = (trunk_top_y as i32, foot_y as i32);
        let trunk_left = foot_x as i32 - trunk_width / 2;
        for y in first_row..=last_row {
            let lean = if tree.kind == Pine || is_bare {
                0
            } else {
                ((last_row - y) as f32 * 0.04 * ((tree.seed % 5) as f32 - 2.0)) as i32
            };
            for strip in 0..trunk_width {
                let bark = if strip == 0 && trunk_width > 1 {
                    palette::TREE_TRUNK[2]
                } else if strip == trunk_width - 1 {
                    palette::TREE_TRUNK[0]
                } else {
                    palette::TREE_TRUNK[1]
                };
                painter.put(trunk_left + strip + lean, y, depth, bark);
            }
        }
        if tree.kind == Pine {
            for y in first_row..=last_row {
                if hash_position(y, tree.seed as i32, 3).is_multiple_of(3) {
                    painter.put(trunk_left, y, depth, PINE_BARK);
                }
            }
        }
    }

    if is_bare {
        let snow_on_limbs = conditions.snow_cover > 0.15;
        let twig_haze = 1.0 - foliage.leaf_cover;
        for y in (crown.centre_y - crown.half_height) as i32..=(crown.centre_y + crown.half_height) as i32 {
            for x in (crown.centre_x - crown.half_width) as i32..=(crown.centre_x + crown.half_width) as i32 {
                let (across, down) = (
                    (x as f32 + 0.5 - crown.centre_x) / crown.half_width,
                    (y as f32 + 0.5 - crown.centre_y) / crown.half_height,
                );
                let distance_squared = across * across + down * down;
                if distance_squared >= 1.0 {
                    continue;
                }
                let twig_spacing = if distance_squared > 0.7 { 9 } else { 5 };
                if hash_position(x, y, tree.seed ^ 0x717).is_multiple_of(twig_spacing) && (x + y) & 1 == 0 {
                    painter.blend(x, y, depth, TWIG, 0.5 * twig_haze);
                }
            }
        }
        let fork = (foot_x, crown.centre_y + crown.half_height * 0.35);
        let limb_count = (5.0 + crown.half_width / 5.0).min(9.0) as i32;
        for limb in 0..limb_count {
            let fraction_round_the_crown = (limb as f32 + 0.5) / limb_count as f32;
            let angle = PI * (1.12 - fraction_round_the_crown * 1.24) + (unit_hash(limb, 7, tree.seed) - 0.5) * 0.3;
            let reach = 0.78 + 0.2 * unit_hash(limb, 8, tree.seed);
            let limb_end = (
                crown.centre_x + angle.cos() * crown.half_width * reach,
                crown.centre_y - angle.sin() * crown.half_height * reach * 0.95 + crown.half_height * 0.1,
            );
            let elbow = (
                fork.0 + (limb_end.0 - fork.0) * 0.5 + (unit_hash(limb, 9, tree.seed) - 0.5) * crown.half_width * 0.2,
                fork.1 + (limb_end.1 - fork.1) * 0.55,
            );
            draw_limb(painter, fork, elbow, depth, palette::TREE_TRUNK[1], trunk_width >= 3, snow_on_limbs);
            draw_limb(painter, elbow, limb_end, depth, palette::TREE_TRUNK[1], false, snow_on_limbs);
            for twig in 0..3 {
                let twig_angle =
                    angle + (twig as f32 - 1.0) * 0.5 + (unit_hash(limb * 3 + twig, 10, tree.seed) - 0.5) * 0.3;
                let twig_reach = 0.3 + 0.15 * unit_hash(limb * 3 + twig, 11, tree.seed);
                let twig_start = if twig == 1 {
                    limb_end
                } else {
                    (elbow.0 + (limb_end.0 - elbow.0) * 0.4, elbow.1 + (limb_end.1 - elbow.1) * 0.4)
                };
                let reached = (
                    twig_start.0 + twig_angle.cos() * crown.half_width * twig_reach,
                    twig_start.1 - twig_angle.sin() * crown.half_height * twig_reach,
                );
                let (across, down) =
                    ((reached.0 - crown.centre_x) / crown.half_width, (reached.1 - crown.centre_y) / crown.half_height);
                let pull_inside_crown = (across * across + down * down).sqrt().max(1.0);
                let twig_end = (
                    crown.centre_x + (reached.0 - crown.centre_x) / pull_inside_crown,
                    crown.centre_y + (reached.1 - crown.centre_y) / pull_inside_crown,
                );
                draw_limb(painter, twig_start, twig_end, depth, TWIG, false, snow_on_limbs);
            }
        }
    }

    if foliage.leaf_cover <= 0.02 {
        return;
    }

    let shrink = 0.72 + 0.28 * foliage.leaf_cover;
    let inverse_shrink_squared = 1.0 / (shrink * shrink);
    let (map_width, map_height) = (crown_map.width, crown_map.height);
    let (left, top) = (crown_map.left, crown_map.top);
    let (right, bottom) = (left + map_width as i32 - 1, top + map_height as i32 - 1);
    let light_from_east =
        if conditions.direction_to_light[0].abs() < 0.08 { -0.3 } else { conditions.direction_to_light[0] * 1.3 };
    let leaves_kept_below = foliage.leaf_cover * 1.25 - 0.1;
    let mut tone_at: Vec<Option<usize>> = vec![None; map_width * map_height];
    for (map_index, pixel) in crown_map.pixels.iter().enumerate() {
        if pixel.coverage == 0 {
            continue;
        }
        let coverage = 1.0 - (1.0 - (pixel.coverage - 1) as f32 / 254.0) * inverse_shrink_squared;
        if coverage <= 0.0 {
            continue;
        }
        let x = left + (map_index % map_width) as i32;
        let y = top + (map_index / map_width) as i32;
        let edge_hash = hash_position(x, y, tree.seed ^ 0x77);
        if coverage < 0.14 && edge_hash.is_multiple_of(3) {
            continue;
        }
        let thinned_out =
            pixel.thinning_noise as f32 / 255.0 > leaves_kept_below || (foliage.leaf_cover < 0.5 && edge_hash % 5 < 2);
        if foliage.leaf_cover < 1.0 && thinned_out {
            continue;
        }
        let normal_east = (pixel.normal_east as f32 / 255.0 - 0.5) * 2.0;
        let normal_down = (pixel.normal_down as f32 / 255.0 - 0.5) * 2.0;
        let across_crown = (x as f32 + 0.5 - crown.centre_x) / crown.half_width;
        let down_crown = (y as f32 + 0.5 - crown.centre_y) / crown.half_height;
        let mut light = 0.52
            + (normal_east * light_from_east - normal_down * 0.62) * 0.42
            + coverage * 0.12
            + (across_crown * light_from_east - down_crown * 0.55) * 0.3;
        light += (pixel.shading_noise as f32 / 255.0 - 0.5) * 0.3;
        light += (ordered_dither_threshold(x, y) - 0.5) * 0.1;
        let mut tone: i32 = if light < 0.3 {
            0
        } else if light < 0.55 {
            1
        } else if light < 0.8 {
            2
        } else {
            3
        };
        let leaf_cluster_hash = hash_position(x >> 1, y, tree.seed ^ 0x99);
        if leaf_cluster_hash.is_multiple_of(9) {
            tone = (tone - 1).max(0);
        } else if leaf_cluster_hash.is_multiple_of(11) {
            tone = (tone + 1).min(3);
        }
        tone_at[map_index] = Some(tone as usize);
    }

    painter.current_tree_id = if is_shrub { 0 } else { sway_id };
    let outline = scale_brightness(foliage.leaf_colours[0], 0.72);
    for y in top.max(0)..=bottom.min(CANVAS_HEIGHT as i32 - 1) {
        for x in left.max(0)..=right.min(CANVAS_WIDTH as i32 - 1) {
            let map_index = (y - top) as usize * map_width + (x - left) as usize;
            let Some(tone) = tone_at[map_index] else { continue };
            let nothing_below = y == bottom || tone_at[map_index + map_width].is_none();
            let nothing_to_the_right = x == right || tone_at[map_index + 1].is_none();
            let mut colour = foliage.leaf_colours[tone];
            if (nothing_below || (nothing_to_the_right && light_from_east < 0.0)) && foliage.leaf_cover > 0.6 {
                colour = outline;
            }
            if tree.kind == Magnolia && tone >= 2 && hash_position(x, y, tree.seed ^ 0x55).is_multiple_of(13) {
                colour = WAXY_GLINT;
            }
            let pixel = &crown_map.pixels[map_index];
            if foliage.blossom_share > 0.0 {
                let shading_noise = pixel.shading_noise as f32 / 255.0;
                let blossom_hash = if tree.kind == Rose {
                    hash_position(x, y, tree.seed ^ 0xB1)
                } else {
                    hash_position(x >> 1, y >> 1, tree.seed ^ 0xB1)
                };
                let blossom_threshold = match tree.kind {
                    Rose => foliage.blossom_share * 0.45,
                    Magnolia => foliage.blossom_share,
                    _ => foliage.blossom_share * (0.55 + shading_noise * 0.9),
                };
                if (blossom_hash >> 8) as f32 / 16_777_216.0 < blossom_threshold {
                    colour = if nothing_below {
                        foliage.blossom_colours[0]
                    } else {
                        foliage.blossom_colours[tone.saturating_sub(1).min(2)]
                    };
                }
            }
            if conditions.snow_cover > 0.05 {
                let thinning_noise = pixel.thinning_noise as f32 / 255.0;
                let down_crown = (y as f32 + 0.5 - crown.centre_y) / crown.half_height;
                let snow_load = conditions.snow_cover * if tree.is_evergreen() { 1.25 } else { 0.9 } - 0.12;
                if tone >= 2 && thinning_noise * 0.8 + down_crown * 0.25 + 0.25 < snow_load {
                    colour = palette::SNOW[if tone == 3 { 3 } else { 2 }];
                } else if tone == 1 && thinning_noise + 0.55 < snow_load {
                    colour = palette::SNOW[1];
                }
            }
            painter.put(x, y, depth, colour);
        }
    }
    painter.current_tree_id = 0;
}

fn draw_limb(
    painter: &mut StaticPainter,
    from: (f32, f32),
    to: (f32, f32),
    depth: f32,
    colour: Colour,
    is_thick: bool,
    snow_on_top: bool,
) {
    let (from_x, from_y, to_x, to_y) = (from.0 as i32, from.1 as i32, to.0 as i32, to.1 as i32);
    painter.line(from_x, from_y, to_x, to_y, depth, colour);
    if is_thick {
        painter.line(from_x + 1, from_y, to_x + 1, to_y, depth, palette::TREE_TRUNK[0]);
    }
    let runs_sideways = (to_x - from_x).abs() > (to_y - from_y).abs();
    if snow_on_top && runs_sideways {
        painter.line(from_x, from_y - 1, to_x, to_y - 1, depth, palette::SNOW[3]);
    }
}
