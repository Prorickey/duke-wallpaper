use crate::chapel::art::Layer;
use crate::chapel::ground::DROP_OFF_CIRCLE;
use crate::chapel::palette::{HALF_LIT, SHADED};
use crate::chapel::proj::*;
use crate::chapel::tex::{Caster, DrawingContext, Lamp, ShadowAndLightSources};
use crate::hash_fraction;
use crate::{blend_colours, hash_position, Colour};

pub fn car_park_aisle_metres_east(metres_north: f32) -> f32 {
    -46.65 + (metres_north - 124.0) * 0.232
}

pub struct LampPost {
    pub metres_east: f32,
    pub metres_north: f32,
    pub is_car_park_mast: bool,
    pub carries_a_banner: bool,
}

pub const LAMP_POSTS: [LampPost; 14] = [
    LampPost { metres_east: -24.9, metres_north: -23.0, is_car_park_mast: false, carries_a_banner: true },
    LampPost { metres_east: 25.3, metres_north: -23.0, is_car_park_mast: false, carries_a_banner: true },
    LampPost { metres_east: -24.9, metres_north: -0.5, is_car_park_mast: false, carries_a_banner: true },
    LampPost { metres_east: 25.3, metres_north: -0.5, is_car_park_mast: false, carries_a_banner: true },
    LampPost { metres_east: -25.9, metres_north: 21.0, is_car_park_mast: false, carries_a_banner: false },
    LampPost { metres_east: 33.0, metres_north: 30.0, is_car_park_mast: false, carries_a_banner: false },
    LampPost { metres_east: -36.6, metres_north: 52.0, is_car_park_mast: false, carries_a_banner: false },
    LampPost { metres_east: -63.4, metres_north: 44.0, is_car_park_mast: false, carries_a_banner: false },
    LampPost { metres_east: -50.6, metres_north: 14.0, is_car_park_mast: false, carries_a_banner: false },
    LampPost { metres_east: -84.6, metres_north: -10.0, is_car_park_mast: false, carries_a_banner: false },
    LampPost { metres_east: -46.6, metres_north: 86.0, is_car_park_mast: false, carries_a_banner: false },
    LampPost { metres_east: -28.0, metres_north: 96.0, is_car_park_mast: false, carries_a_banner: false },
    LampPost { metres_east: 31.6, metres_north: 80.0, is_car_park_mast: false, carries_a_banner: false },
    LampPost { metres_east: -58.0, metres_north: 114.0, is_car_park_mast: true, carries_a_banner: false },
];

fn draw_lamp_post(layer: &mut Layer, drawing: &DrawingContext, sources: &mut ShadowAndLightSources, post: &LampPost) {
    const LANTERN_POST_METRES: f32 = 3.9;
    const CAR_PARK_MAST_METRES: f32 = 7.5;
    let (column, ground_row) = drawing.projection.ground_pixel((post.metres_east, post.metres_north));
    layer.pen_depth = depth_tag(post.metres_north);
    let iron = drawing.palette.iron;
    let post_metres = if post.is_car_park_mast { CAR_PARK_MAST_METRES } else { LANTERN_POST_METRES };
    let post_pixels = drawing.projection.height_in_pixels(post.metres_north, post_metres);
    layer.vertical_line(column, ground_row - post_pixels, ground_row, iron[0]);
    let head_row = ground_row - post_pixels;
    if post.is_car_park_mast {
        layer.horizontal_line(column - 1, column + 1, head_row, iron[0]);
        layer.put(column - 1, head_row + 1, 0xdfe3e6);
        layer.put(column + 1, head_row + 1, 0xdfe3e6);
    } else {
        layer.put(column, head_row - 2, iron[0]);
        layer.put(column - 1, head_row - 1, 0xe9e4cc);
        layer.put(column, head_row - 1, 0xf5f1dc);
        layer.put(column + 1, head_row - 1, 0xd2ccb4);
        layer.put(column - 1, head_row, 0xd2ccb4);
        layer.put(column + 1, head_row, 0xbdb7a0);
        if drawing.look.snow_cover > 0.3 {
            layer.put(column, head_row - 2, drawing.palette.snow[0]);
        }
    }
    sources.lamps.push(Lamp {
        column,
        row: if post.is_car_park_mast { head_row + 1 } else { head_row - 1 },
        ground_row,
        depth_tag: depth_tag(post.metres_north),
        head_depth_tag: depth_tag(post.metres_north),
        kind: if post.is_car_park_mast { 2 } else { 0 },
    });
    sources.casters.push(Caster::Pole { foot_column: column, foot_row: ground_row, height_pixels: post_pixels as f32 });
}

pub const CAR_COLOURS: [u32; 10] =
    [0xe8e8e6, 0xb9bec4, 0x2c2f36, 0x2f4f8f, 0xa8322e, 0x6d7278, 0xcfc4a8, 0x3a6ea8, 0x1f3a2e, 0x8a8f96];

fn draw_parked_car(
    layer: &mut Layer,
    drawing: &DrawingContext,
    sources: &mut ShadowAndLightSources,
    metres_east: f32,
    metres_north: f32,
    car_number: u32,
    nose_across_the_view: bool,
) {
    let (column, row) = drawing.projection.ground_pixel((metres_east, metres_north));
    layer.pen_depth = depth_tag(metres_north - 1.0);
    let body_colour = if drawing.look.snow_cover > 0.45 {
        drawing.palette.snow[1]
    } else {
        CAR_COLOURS[(hash_position(car_number as i32, 3, drawing.look.seed) % CAR_COLOURS.len() as u32) as usize]
    };
    let facing = if car_number.is_multiple_of(2) { 1.0 } else { -1.0 };
    let (heading_east, heading_north) = if nose_across_the_view { (facing, 0.0) } else { (0.0, facing) };
    crate::chapel::anim::car_pixels(
        drawing.projection,
        (column, row),
        heading_east,
        heading_north,
        body_colour,
        0.0,
        &mut |pixel_x, pixel_y, colour| layer.put(pixel_x, pixel_y, colour),
    );
    let (half_length_east, half_length_north) = if nose_across_the_view { (2.2, 0.9) } else { (0.9, 2.2) };
    sources.casters.push(Caster::Prism {
        footprint: drawing.projection.ground_polygon(&[
            (metres_east - half_length_east, metres_north - half_length_north),
            (metres_east + half_length_east, metres_north - half_length_north),
            (metres_east + half_length_east, metres_north + half_length_north),
            (metres_east - half_length_east, metres_north + half_length_north),
        ]),
        height_pixels: drawing.projection.height_in_pixels(metres_north, 1.4) as f32,
    });
}

fn draw_cafe_table(
    layer: &mut Layer,
    drawing: &DrawingContext,
    metres_east: f32,
    metres_north: f32,
    umbrella: Option<Colour>,
) {
    let (column, row) = drawing.projection.ground_pixel((metres_east, metres_north));
    layer.pen_depth = depth_tag(metres_north);
    let snowy = drawing.look.snow_cover > 0.4;
    let Some(umbrella_colour) = umbrella else {
        let table_top = if snowy { drawing.palette.snow[0] } else { 0x9a9ea3 };
        layer.horizontal_line(column - 1, column + 1, row - 1, table_top);
        layer.put(column, row - 2, table_top);
        layer.put(column, row, 0x4b4f55);
        layer.put(column - 2, row, 0x5a4a3a);
        layer.put(column + 2, row, 0x5a4a3a);
        return;
    };
    let canopy = if snowy { drawing.palette.snow[0] } else { umbrella_colour };
    layer.put(column, row, drawing.palette.iron[0]);
    layer.fill_rect(column - 2, row - 5, column + 2, row - 2, canopy);
    layer.horizontal_line(column - 2, column + 2, row - 2, blend_colours(canopy, 0x000000, 0.3));
    layer.put(column, row - 4, blend_colours(canopy, 0xffffff, 0.5));
}

fn draw_low_wall_running_north(
    layer: &mut Layer,
    drawing: &DrawingContext,
    metres_east: f32,
    south_metres: f32,
    north_metres: f32,
    height_metres: f32,
) {
    layer.pen_depth = depth_tag(south_metres);
    let south_end = (
        drawing.projection.pixel_x(metres_east, south_metres),
        drawing.projection.pixel_y(south_metres, height_metres),
    );
    let north_end = (
        drawing.projection.pixel_x(metres_east, north_metres),
        drawing.projection.pixel_y(north_metres, height_metres),
    );
    layer.line(south_end.0, south_end.1, north_end.0, north_end.1, drawing.palette.limestone[1]);
    let foot_row = drawing.projection.pixel_y(south_metres, 0.0);
    for rows_up in 0..drawing.projection.height_in_pixels(south_metres, height_metres) {
        let light_level = if rows_up == 0 { SHADED } else { HALF_LIT };
        layer.put(south_end.0, foot_row - rows_up, drawing.stone(light_level, south_end.0, foot_row - rows_up));
    }
}

pub const PLAZA_TABLES: [(f32, f32); 5] = [(-89.8, 20.0), (-89.8, 1.5), (-89.8, -3.5), (-89.8, -10.0), (-89.8, -14.0)];
pub const COURT_TABLES: [(f32, f32); 8] =
    [(70.0, 41.0), (72.5, 37.5), (70.0, 34.0), (73.0, 30.5), (71.0, 26.0), (74.0, 22.5), (70.5, 19.0), (74.5, 39.5)];
pub const BENCHES: [(f32, f32); 4] = [(-84.4, 87.0), (-84.4, 90.4), (-84.4, 93.8), (-82.6, 97.0)];

const PARKED_CARS: [(f32, f32, bool); 9] = [
    (-83.6, -25.2, false),
    (-80.3, -25.0, false),
    (-62.0, -25.0, false),
    (-58.6, -25.0, false),
    (53.0, -13.5, false),
    (56.2, -16.0, false),
    (59.4, -18.4, false),
    (62.4, -20.0, false),
    (75.2, -11.2, true),
];

#[derive(Clone, Copy)]
pub enum Prop {
    LampPost { post_number: usize },
    CarParkBay { bay: u32, metres_north: f32 },
    ParkedCar { car_number: u32, metres_east: f32, metres_north: f32, nose_across_the_view: bool },
    PlazaTable { metres_east: f32, metres_north: f32 },
    CourtTable { metres_east: f32, metres_north: f32 },
    Bench { metres_east: f32, metres_north: f32 },
    BollardsRoundTheDropOffCircle,
    NaveGardenWall,
    PlazaWall,
    Planter { metres_east: f32 },
}

pub fn props() -> Vec<(f32, Prop)> {
    const CAR_PARK_BAYS: u32 = 12;
    const PLANTERS_METRES_NORTH: f32 = -1.2;
    let mut props = Vec::new();
    for (post_number, post) in LAMP_POSTS.iter().enumerate() {
        props.push((post.metres_north, Prop::LampPost { post_number }));
    }
    for bay in 0..CAR_PARK_BAYS {
        let metres_north = 105.5 + bay as f32 * 2.9;
        props.push((metres_north - 0.2, Prop::CarParkBay { bay, metres_north }));
    }
    for (number, &(metres_east, metres_north, nose_across_the_view)) in PARKED_CARS.iter().enumerate() {
        props.push((
            metres_north,
            Prop::ParkedCar { car_number: 900 + number as u32, metres_east, metres_north, nose_across_the_view },
        ));
    }
    for (metres_east, metres_north) in PLAZA_TABLES {
        props.push((metres_north, Prop::PlazaTable { metres_east, metres_north }));
    }
    for (metres_east, metres_north) in COURT_TABLES {
        props.push((metres_north, Prop::CourtTable { metres_east, metres_north }));
    }
    for (metres_east, metres_north) in BENCHES {
        props.push((metres_north, Prop::Bench { metres_east, metres_north }));
    }
    props.push((DROP_OFF_CIRCLE.0 .1 - DROP_OFF_CIRCLE.1, Prop::BollardsRoundTheDropOffCircle));
    props.push((49.6, Prop::NaveGardenWall));
    props.push((4.0, Prop::PlazaWall));
    props.push((PLANTERS_METRES_NORTH, Prop::Planter { metres_east: -9.3 }));
    props.push((PLANTERS_METRES_NORTH, Prop::Planter { metres_east: 9.3 }));
    props
}

pub fn draw_prop(prop: Prop, layer: &mut Layer, drawing: &DrawingContext, sources: &mut ShadowAndLightSources) {
    let snowy = drawing.look.snow_cover > 0.4;
    match prop {
        Prop::LampPost { post_number } => draw_lamp_post(layer, drawing, sources, &LAMP_POSTS[post_number]),
        Prop::CarParkBay { bay, metres_north } => {
            let share_of_bays_taken = if drawing.look.snow_cover > 0.6 { 0.45 } else { 0.8 };
            if hash_fraction(bay as i32, 77, drawing.look.seed) < share_of_bays_taken {
                let metres_east = car_park_aisle_metres_east(metres_north) + 5.7;
                draw_parked_car(layer, drawing, sources, metres_east, metres_north, bay, true);
            }
        }
        Prop::ParkedCar { car_number, metres_east, metres_north, nose_across_the_view } => {
            draw_parked_car(layer, drawing, sources, metres_east, metres_north, car_number, nose_across_the_view)
        }
        Prop::PlazaTable { metres_east, metres_north } => {
            draw_cafe_table(layer, drawing, metres_east, metres_north, Some(0x2f62b0))
        }
        Prop::CourtTable { metres_east, metres_north } => {
            draw_cafe_table(layer, drawing, metres_east, metres_north, None)
        }
        Prop::Bench { metres_east, metres_north } => {
            let (column, row) = drawing.projection.ground_pixel((metres_east, metres_north));
            layer.pen_depth = depth_tag(metres_north);
            let seat_colour = if snowy { drawing.palette.snow[0] } else { 0xe6e2d6 };
            layer.horizontal_line(column - 2, column + 2, row - 1, seat_colour);
            layer.horizontal_line(column - 2, column + 2, row, 0x9c988c);
        }
        Prop::BollardsRoundTheDropOffCircle => {
            let (centre, outer_radius, _) = DROP_OFF_CIRCLE;
            for step in 0..9 {
                let angle = (205.0 + step as f32 * 14.0).to_radians();
                let metres_east = centre.0 + (outer_radius + 1.0) * angle.cos();
                let metres_north = centre.1 + (outer_radius + 1.0) * angle.sin();
                let (column, row) = drawing.projection.ground_pixel((metres_east, metres_north));
                layer.pen_depth = depth_tag(metres_north);
                layer.put(column, row, 0x55585c);
                layer.put(column, row - 1, if snowy { drawing.palette.snow[0] } else { 0xd9d6cc });
            }
        }
        Prop::NaveGardenWall => draw_low_wall_running_north(layer, drawing, -26.6, 12.4, 49.6, 1.1),
        Prop::PlazaWall => draw_low_wall_running_north(layer, drawing, -88.4, -21.0, 4.0, 0.8),
        Prop::Planter { metres_east } => {
            const PLANTER_METRES_NORTH: f32 = -1.2;
            const SHRUB_PIXELS: [(i32, i32, usize); 4] = [(-1, -2, 1), (0, -3, 3), (1, -2, 1), (0, -2, 2)];
            let (column, row) = drawing.projection.ground_pixel((metres_east, PLANTER_METRES_NORTH));
            layer.pen_depth = depth_tag(PLANTER_METRES_NORTH);
            let limestone = drawing.palette.limestone;
            layer.fill_rect(column - 1, row - 1, column + 1, row, limestone[2]);
            layer.horizontal_line(column - 1, column + 1, row - 1, limestone[0]);
            for (columns_across, rows_down, hedge_shade) in SHRUB_PIXELS {
                let snow_on_top = drawing.look.snow_cover > 0.5 && rows_down <= -3;
                let colour = if snow_on_top { drawing.palette.snow[0] } else { drawing.palette.hedge[hedge_shade] };
                layer.put(column + columns_across, row + rows_down, colour);
            }
        }
    }
}
