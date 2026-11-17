use crate::east::campus_layout::*;
use crate::east::conditions::{palette, Conditions};
use crate::east::projection::*;
use crate::east::shading_tools::rain_darkened;
use crate::east::static_layer::StaticPainter;
use crate::{blend_colours, hash_position, rgb, scale_brightness, Colour};
use std::f32::consts::PI;

pub const LAMP_POST_HEIGHT: f32 = 3.7;

const LAMP_POSTS_DOWN_THE_QUAD_DRIVE_SOUTH: [f32; 9] = [55.0, 76.0, 98.0, 120.0, 212.0, 234.0, 256.0, 278.0, 298.0];
const LAMP_POSTS_ROUND_UNION_CIRCLE_ANGLES: [f32; 4] = [0.42, 1.05, PI - 1.05, PI - 0.42];

pub fn lamp_post_positions() -> Vec<(f32, f32)> {
    let mut positions = Vec::new();
    for side in [-1.0f32, 1.0] {
        positions.push((side * 9.6, 44.9));
        positions.push((side * 22.5, 47.2));
        for metres_south in LAMP_POSTS_DOWN_THE_QUAD_DRIVE_SOUTH {
            positions.push((side * 27.4, metres_south));
        }
        for angle in LAMP_POSTS_ROUND_UNION_CIRCLE_ANGLES {
            positions.push((side * 36.2 * angle.sin(), UNION_CIRCLE_SOUTH - 36.2 * angle.cos()));
        }
        positions.push((side * 29.4, 19.0));
        positions.push((side * 29.4, 31.0));
        positions.push((side * 11.5, 319.5));
        positions.push((side * 15.4, 348.0));
        positions.push((side * 26.4, 360.0));
        positions.push((side * 26.4, 384.0));
        positions.push((side * 38.8, 334.0));
        positions.push((side * 38.8, 358.0));
        positions.push((side * 37.5, cross_walks_south(side)[0] + 1.9));
        positions.push((side * 37.5, cross_walks_south(side)[3] + 1.9));
    }
    positions.push((12.0, -29.0));
    positions.push((38.0, -29.0));
    positions.push((-20.0, -17.8));
    for metres_east in [-70.0f32, 30.0, 130.0] {
        positions.push((metres_east, WEST_MARKHAM_AVENUE_SOUTH + 7.4));
    }
    positions
}

const LAMP_POST_IRON: Colour = rgb(30, 32, 42);
const GLOBE_LIT_SIDE: Colour = rgb(244, 242, 228);
const GLOBE_HIGHLIGHT: Colour = rgb(255, 255, 248);
const GLOBE_SHADED_SIDE: Colour = rgb(200, 204, 206);
const GLOBE_LOWER_LIT_SIDE: Colour = rgb(214, 214, 206);
const GLOBE_LOWER_MIDDLE: Colour = rgb(232, 230, 216);
const GLOBE_LOWER_SHADED_SIDE: Colour = rgb(170, 176, 188);

pub fn draw_lamp_post(painter: &mut StaticPainter, metres_east: f32, metres_south: f32, conditions: &Conditions) {
    let (foot_x, foot_y) = project_to_pixel(metres_east, metres_south, 0.0);
    let (_, globe_y) = project_to_pixel(metres_east, metres_south, LAMP_POST_HEIGHT);
    for y in globe_y + 2..=foot_y {
        painter.put(foot_x, y, metres_south, LAMP_POST_IRON);
    }
    painter.put(foot_x - 1, foot_y, metres_south, LAMP_POST_IRON);
    painter.put(foot_x + 1, foot_y, metres_south, LAMP_POST_IRON);
    let globe_top = if conditions.snow_cover > 0.3 { palette::SNOW[3] } else { GLOBE_LIT_SIDE };
    painter.put(foot_x, globe_y - 1, metres_south, globe_top);
    painter.put(foot_x - 1, globe_y, metres_south, GLOBE_LIT_SIDE);
    painter.put(foot_x, globe_y, metres_south, GLOBE_HIGHLIGHT);
    painter.put(foot_x + 1, globe_y, metres_south, GLOBE_SHADED_SIDE);
    painter.put(foot_x - 1, globe_y + 1, metres_south, GLOBE_LOWER_LIT_SIDE);
    painter.put(foot_x, globe_y + 1, metres_south, GLOBE_LOWER_MIDDLE);
    painter.put(foot_x + 1, globe_y + 1, metres_south, GLOBE_LOWER_SHADED_SIDE);
}

const BENCHES_BY_THE_FORECOURT: [(f32, f32); 3] = [(-24.6, 35.2), (-24.6, 38.6), (24.4, 35.6)];
const BENCHES_DOWN_THE_QUAD_DRIVE_SOUTH: [f32; 4] = [58.5, 104.0, 219.0, 284.0];

pub fn bench_positions() -> Vec<(f32, f32)> {
    let mut positions = BENCHES_BY_THE_FORECOURT.to_vec();
    for side in [-1.0f32, 1.0] {
        for metres_south in BENCHES_DOWN_THE_QUAD_DRIVE_SOUTH {
            positions.push((side * 28.4, metres_south + side));
        }
        positions.push((side * 37.6, UNION_CIRCLE_SOUTH - 8.0));
        positions.push((side * 37.6, UNION_CIRCLE_SOUTH + 8.0));
    }
    for metres_east in BUS_PLAZA_BENCHES_EAST {
        positions.push((metres_east, BUS_PLAZA_BENCHES_SOUTH));
    }
    positions
}

const TEAK: [Colour; 3] = [rgb(104, 72, 52), rgb(150, 110, 72), rgb(186, 146, 96)];
const BENCH_LEG: Colour = rgb(50, 40, 40);
const GROUND_SHADOW: Colour = rgb(20, 24, 36);

pub fn draw_bench(painter: &mut StaticPainter, metres_east: f32, metres_south: f32, conditions: &Conditions) {
    let (foot_x, foot_y) = project_to_pixel(metres_east, metres_south, 0.0);
    let half_length = 3;
    for along in -half_length..=half_length {
        painter.put(foot_x + along, foot_y - 3, metres_south, TEAK[0]);
        let seat_slat = if conditions.snow_cover > 0.25 {
            palette::SNOW[2]
        } else if along & 1 == 0 {
            TEAK[2]
        } else {
            TEAK[1]
        };
        painter.put(foot_x + along, foot_y - 2, metres_south, seat_slat);
        painter.put(
            foot_x + along,
            foot_y - 1,
            metres_south,
            if conditions.snow_cover > 0.6 { palette::SNOW[1] } else { TEAK[1] },
        );
    }
    painter.put(foot_x - half_length, foot_y, metres_south, BENCH_LEG);
    painter.put(foot_x + half_length, foot_y, metres_south, BENCH_LEG);
    for along in -half_length + 1..half_length {
        painter.blend(foot_x + along, foot_y, metres_south - 0.2, GROUND_SHADOW, 0.35);
    }
}

pub struct BikeRack {
    pub metres_east: f32,
    pub metres_south: f32,
    pub bike_count: i32,
}

const fn bike_rack(metres_east: f32, metres_south: f32, bike_count: i32) -> BikeRack {
    BikeRack { metres_east, metres_south, bike_count }
}

pub const BIKE_RACKS: [BikeRack; 11] = [
    bike_rack(30.6, 69.2, 5),
    bike_rack(-30.8, 66.4, 4),
    bike_rack(21.5, 9.6, 3),
    bike_rack(-21.0, 9.4, 3),
    bike_rack(30.6, 136.6, 4),
    bike_rack(-30.6, 193.8, 4),
    bike_rack(30.6, 196.6, 3),
    bike_rack(-30.8, 262.0, 5),
    bike_rack(30.6, 258.6, 4),
    bike_rack(-38.0, 329.9, 5),
    bike_rack(44.4, 345.0, 5),
];

const BIKE_FRAME_COLOURS: [Colour; 6] =
    [rgb(196, 60, 52), rgb(52, 110, 190), rgb(236, 196, 60), rgb(70, 160, 110), rgb(230, 230, 230), rgb(40, 40, 50)];
const BIKE_TYRE: Colour = rgb(34, 34, 44);
const BIKE_SADDLE: Colour = rgb(30, 30, 36);

pub fn draw_bike_rack(painter: &mut StaticPainter, rack: &BikeRack) {
    let depth = rack.metres_south;
    let (foot_x, foot_y) = project_to_pixel(rack.metres_east, rack.metres_south, 0.0);
    for bike in 0..rack.bike_count {
        let bike_x = foot_x + bike * 3 - rack.bike_count;
        let frame = BIKE_FRAME_COLOURS[(hash_position(bike, foot_x, 99) % BIKE_FRAME_COLOURS.len() as u32) as usize];
        painter.put(bike_x, foot_y, depth, BIKE_TYRE);
        painter.put(bike_x, foot_y - 3, depth, BIKE_TYRE);
        painter.put(bike_x, foot_y - 1, depth, frame);
        painter.put(bike_x, foot_y - 2, depth, frame);
        painter.put(bike_x + 1, foot_y - 2, depth, BIKE_SADDLE);
    }
    for along in -rack.bike_count - 1..=rack.bike_count * 2 {
        painter.blend(foot_x + along, foot_y + 1, depth - 0.1, GROUND_SHADOW, 0.3);
    }
}

pub const STATUE_DEPTH: f32 = 69.6;
const BRONZE_PATINA: [Colour; 4] = [rgb(38, 44, 44), rgb(64, 72, 62), rgb(96, 104, 80), rgb(140, 146, 108)];
const BENJAMIN_DUKE_FIGURE: [&[u8]; 10] =
    [b"..23.", b"..12.", b".232.", b"12321", b"12211", b".221.", b".221.", b".121.", b".1.1.", b".1.1."];
const FIGURE_HEAD_ROW: usize = 0;
const FIGURE_SHOULDER_ROW: usize = 2;

pub fn draw_benjamin_duke_statue(painter: &mut StaticPainter, conditions: &Conditions) {
    let (plinth_top_x, plinth_top_y) = project_to_pixel(0.0, BENJAMIN_DUKE_STATUE_SOUTH, 1.5);
    for (row, pattern) in BENJAMIN_DUKE_FIGURE.iter().enumerate() {
        for (column, tone) in pattern.iter().enumerate() {
            if *tone == b'.' {
                continue;
            }
            let mut bronze = BRONZE_PATINA[(*tone - b'0') as usize];
            if conditions.snow_cover > 0.3 && (row == FIGURE_HEAD_ROW || row == FIGURE_SHOULDER_ROW) {
                bronze = palette::SNOW[3];
            }
            painter.put(
                plinth_top_x - 2 + column as i32,
                plinth_top_y - 10 + row as i32,
                STATUE_DEPTH,
                rain_darkened(bronze, conditions.wetness * 0.5),
            );
        }
    }
    for walking_stick_y in plinth_top_y - 5..=plinth_top_y - 2 {
        painter.put(plinth_top_x + 3, walking_stick_y, STATUE_DEPTH, BRONZE_PATINA[1]);
    }
}

#[derive(Copy, Clone)]
pub struct ParkedCar {
    pub metres_east: f32,
    pub metres_south: f32,
    pub car_hash: u32,
    pub seen_end_on: bool,
}

pub fn parked_cars(seed: u32) -> Vec<ParkedCar> {
    let mut cars = Vec::new();
    for (lot_index, lot) in CAR_PARKS.iter().enumerate() {
        let is_baldwin_parking_bays = lot_index == 0;
        if is_baldwin_parking_bays {
            let bay_count = ((lot.east - lot.west) / 2.7) as i32;
            for bay in 0..bay_count {
                let car_hash = hash_position(bay, 4, seed ^ 0xCA2);
                if car_hash % 5 >= 2 {
                    cars.push(ParkedCar {
                        metres_east: lot.west + 1.35 + bay as f32 * 2.7,
                        metres_south: 0.5 * (lot.north + lot.south) + 0.6,
                        car_hash,
                        seen_end_on: true,
                    });
                }
            }
            continue;
        }
        let mut rows_east = vec![lot.west + 2.6, lot.east - 2.6];
        if lot.east - lot.west > 34.0 {
            let mut double_row_east = lot.west + 17.0;
            while double_row_east < lot.east - 14.0 {
                rows_east.push(double_row_east);
                rows_east.push(double_row_east + 5.0);
                double_row_east += 17.5;
            }
        }
        let spaces_per_row = ((lot.south - lot.north - 3.0) / 2.75) as i32;
        for (row_index, &row_east) in rows_east.iter().enumerate() {
            for space in 0..spaces_per_row {
                let car_hash = hash_position(space, row_index as i32 * 7 + lot_index as i32, seed ^ 0xCA7);
                if car_hash % 10 < 6 {
                    cars.push(ParkedCar {
                        metres_east: row_east,
                        metres_south: lot.north + 2.0 + space as f32 * 2.75,
                        car_hash,
                        seen_end_on: false,
                    });
                }
            }
        }
    }
    cars
}

const CAR_PAINTS: [Colour; 8] = [
    rgb(170, 44, 48),
    rgb(222, 222, 224),
    rgb(52, 72, 124),
    rgb(44, 46, 56),
    rgb(150, 156, 164),
    rgb(196, 186, 150),
    rgb(60, 110, 96),
    rgb(232, 232, 236),
];
const CAR_GLASS: Colour = rgb(54, 70, 92);
const CAR_TAIL_LAMP: Colour = rgb(200, 60, 50);
const CAR_HEAD_LAMP: Colour = rgb(236, 230, 200);
const CAR_WHEEL: Colour = rgb(22, 22, 28);
const UNDER_CAR_SHADOW: Colour = rgb(10, 12, 24);

pub fn draw_parked_car(painter: &mut StaticPainter, car: &ParkedCar, conditions: &Conditions) {
    let paint = CAR_PAINTS[((car.car_hash >> 3) % CAR_PAINTS.len() as u32) as usize];
    let lit_paint = blend_colours(paint, 0xFFFFFF, 0.22);
    let shaded_paint = scale_brightness(paint, 0.68);
    let roof = if conditions.snow_cover > 0.3 { palette::SNOW[3] } else { lit_paint };
    let depth = car.metres_south;
    let (foot_x, foot_y) = project_to_pixel(car.metres_east, depth, 0.0);
    if car.seen_end_on {
        let rows = [roof, CAR_GLASS, paint, paint, shaded_paint];
        let lamp = if car.car_hash & 64 == 0 { CAR_TAIL_LAMP } else { CAR_HEAD_LAMP };
        for (row, row_colour) in rows.iter().enumerate() {
            for across in -2..=2 {
                let is_lamp = row == 3 && (across == -2 || across == 2);
                painter.put(foot_x + across, foot_y - 5 + row as i32, depth, if is_lamp { lamp } else { *row_colour });
            }
        }
        for across in -2..=2 {
            painter.blend(foot_x + across, foot_y, depth - 0.2, UNDER_CAR_SHADOW, 0.45);
        }
        return;
    }
    let half_length = ((4.3 * pixels_per_metre_at(depth)) * 0.5).round() as i32;
    let facing = if car.car_hash & 128 == 0 { 1 } else { -1 };
    for along in -half_length..=half_length {
        let under_cabin = along * facing > -half_length / 2 && along * facing < half_length - 1;
        if under_cabin {
            painter.put(foot_x + along, foot_y - 4, depth, roof);
            painter.put(foot_x + along, foot_y - 3, depth, CAR_GLASS);
        }
        painter.put(foot_x + along, foot_y - 2, depth, paint);
        let at_wheel = along.abs() == half_length - 2;
        painter.put(foot_x + along, foot_y - 1, depth, if at_wheel { CAR_WHEEL } else { shaded_paint });
    }
    for along in -half_length..=half_length {
        painter.blend(foot_x + along, foot_y, depth - 0.2, UNDER_CAR_SHADOW, 0.4);
    }
}

pub const BALDWIN_HANDRAIL_DEPTH: f32 = 25.4;
const HANDRAIL_IRON: Colour = rgb(36, 36, 46);

pub fn draw_baldwin_step_handrails(painter: &mut StaticPainter) {
    for side in [-1.0f32, 1.0] {
        let (top_x, top_y) = project_to_pixel(side * 6.6, 23.1, 2.2);
        let (bottom_x, bottom_y) = project_to_pixel(side * 6.6, 25.3, 0.9);
        painter.line(top_x, top_y, bottom_x, bottom_y, BALDWIN_HANDRAIL_DEPTH, HANDRAIL_IRON);
        painter.put(bottom_x, bottom_y + 1, BALDWIN_HANDRAIL_DEPTH, HANDRAIL_IRON);
        painter.put(bottom_x, bottom_y + 2, BALDWIN_HANDRAIL_DEPTH, HANDRAIL_IRON);
    }
}

const BALDWIN_ARCADE_LANTERNS_EAST: [f32; 3] = [-30.2, -24.6, -19.2];
const ARCADES_BETWEEN_HALLS_CENTRES: [(f32, f32); 8] = [
    (-42.0, 69.0),
    (42.0, 71.5),
    (-54.4, 136.0),
    (-54.4, 191.5),
    (47.6, 139.0),
    (47.6, 194.3),
    (-42.0, 259.3),
    (42.0, 261.2),
];

pub fn arcade_lantern_positions() -> Vec<(f32, f32)> {
    let mut positions = Vec::new();
    for metres_east in BALDWIN_ARCADE_LANTERNS_EAST {
        positions.push((metres_east, 13.4));
        positions.push((-metres_east, 14.0));
    }
    for (metres_east, metres_south) in ARCADES_BETWEEN_HALLS_CENTRES {
        positions.push((metres_east, metres_south - 2.2));
        positions.push((metres_east, metres_south + 2.2));
    }
    positions.push((-(LILLY_LIBRARY.facade_from_axis + 1.8), UNION_CIRCLE_SOUTH));
    positions.push((EAST_CAMPUS_UNION.facade_from_axis + 1.8, UNION_CIRCLE_SOUTH + 1.6));
    positions
}

pub const SNOWMAN_DEPTH: f32 = 84.2;
const SNOWMAN: [&[u8]; 9] =
    [b"..bb..", b".bbbb.", b"..wso.", b"..wco.", b".wwso.", b"twwkot", b".wwsso", b"owwkso", b".osso."];
const SNOWMAN_OUTLINE: Colour = rgb(112, 124, 160);
const SNOWMAN_COAL: Colour = rgb(40, 40, 50);
const SNOWMAN_CARROT: Colour = rgb(236, 130, 40);
const SNOWMAN_HAT: Colour = rgb(36, 60, 150);

pub fn draw_snowman(painter: &mut StaticPainter) {
    let (foot_x, foot_y) = project_to_pixel(7.5, SNOWMAN_DEPTH, 0.0);
    for (row, pattern) in SNOWMAN.iter().enumerate() {
        for (column, code) in pattern.iter().enumerate() {
            let colour = match code {
                b'w' => palette::SNOW[3],
                b's' => palette::SNOW[1],
                b'o' => SNOWMAN_OUTLINE,
                b'k' => SNOWMAN_COAL,
                b'c' => SNOWMAN_CARROT,
                b'b' => SNOWMAN_HAT,
                b't' => palette::TREE_TRUNK[1],
                _ => continue,
            };
            painter.put(foot_x - 2 + column as i32, foot_y - 8 + row as i32, SNOWMAN_DEPTH, colour);
        }
    }
    for along in -1..=5 {
        painter.blend(foot_x + along, foot_y + 1, SNOWMAN_DEPTH - 0.2, palette::SNOW[0], 0.7);
    }
}
