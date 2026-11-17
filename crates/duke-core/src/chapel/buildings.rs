use crate::chapel::arch::*;
use crate::chapel::art::Layer;
use crate::chapel::palette::{wetted, HALF_LIT, SHADED, SUNLIT};
use crate::chapel::proj::*;
use crate::chapel::tex::{lead_colour, slate_colour, Caster, DrawingContext, ShadowAndLightSources, LIT_ROOM};
use crate::hash_position;

const GUTTER_COLOUR: u32 = 0x3b3a38;

#[derive(Clone, Copy, PartialEq)]
enum Roofing {
    Slate,
    Lead,
}

#[derive(Clone, Copy, PartialEq)]
enum FlatRoof {
    GreyMembrane,
    WhiteMembrane,
    BlueGreyStandingSeamMetal,
}

#[derive(Clone, Copy, PartialEq)]
enum Walling {
    BuffConcreteWithRibbonWindows,
    DukeStoneWithWindowRows,
}

#[derive(Clone, Copy)]
struct Footprint {
    west_metres: f32,
    east_metres: f32,
    south_metres: f32,
    north_metres: f32,
}

struct SouthWall {
    west_metres: f32,
    east_metres: f32,
    metres_north: f32,
    eave_metres: f32,
    storeys: i32,
    window_seed: u32,
}

struct FlatBlock<'a> {
    outline: &'a [(f32, f32)],
    roof_metres: f32,
    roof: FlatRoof,
    walling: Walling,
    storeys: i32,
    window_seed: u32,
}

struct ScreenRectangle {
    left: i32,
    right: i32,
    top: i32,
    bottom: i32,
}

#[derive(Clone, Copy)]
pub enum Building {
    BryanCenter,
    DivinityNorthRange,
    DivinityWestRange,
    DivinityAddition,
    GrayNorth,
    FlowersNorth,
    GrayWing,
    PageAuditorium,
    FlowersSouthWing,
    GraySouth,
}

pub const BUILDINGS: [(f32, Building); 10] = [
    (150.0, Building::BryanCenter),
    (69.8, Building::DivinityNorthRange),
    (51.6, Building::DivinityWestRange),
    (38.3, Building::DivinityAddition),
    (13.4, Building::GrayNorth),
    (12.2, Building::FlowersNorth),
    (3.2, Building::GrayWing),
    (0.4, Building::PageAuditorium),
    (-20.8, Building::FlowersSouthWing),
    (-25.8, Building::GraySouth),
];

pub fn draw_building(
    building: Building,
    layer: &mut Layer,
    drawing: &DrawingContext,
    sources: &mut ShadowAndLightSources,
) {
    match building {
        Building::BryanCenter => draw_bryan_center(layer, drawing, sources),
        Building::DivinityNorthRange => draw_divinity_school_north_range(layer, drawing, sources),
        Building::DivinityWestRange => draw_divinity_school_west_range(layer, drawing, sources),
        Building::DivinityAddition => draw_divinity_school_addition(layer, drawing, sources),
        Building::GrayNorth => draw_gray_building_north_block(layer, drawing, sources),
        Building::FlowersNorth => draw_flowers_building_north_block(layer, drawing, sources),
        Building::GrayWing => draw_gray_building_wing(layer, drawing, sources),
        Building::PageAuditorium => draw_page_auditorium(layer, drawing, sources),
        Building::FlowersSouthWing => draw_flowers_building_south_wing(layer, drawing, sources),
        Building::GraySouth => draw_gray_building_south_block_and_perkins_library(layer, drawing, sources),
    }
}

fn fill_roof_plane(
    layer: &mut Layer,
    drawing: &DrawingContext,
    outline: &[(i32, i32)],
    roofing: Roofing,
    light_level: usize,
    ridge_runs_across_the_screen: bool,
) {
    let steps_from_light = if light_level == SHADED { 1 } else { 0 };
    let snow_gain = if roofing == Roofing::Lead { 1.8 } else { 2.0 };
    layer.fill_polygon_with(outline, |column, row| {
        if let Some(snow) = drawing.snow_colour(column, row, snow_gain, -0.12, steps_from_light) {
            return Some(snow);
        }
        Some(match roofing {
            Roofing::Slate => slate_colour(drawing.palette, light_level, column, row, !ridge_runs_across_the_screen),
            Roofing::Lead => lead_colour(drawing.palette, column, row, 3, light_level, ridge_runs_across_the_screen),
        })
    });
}

fn draw_slate_roof_with_ridge_running_north(
    layer: &mut Layer,
    drawing: &DrawingContext,
    sources: &mut ShadowAndLightSources,
    footprint: Footprint,
    eave_metres: f32,
    ridge_metres: f32,
) {
    let Footprint { west_metres, east_metres, south_metres, north_metres } = footprint;
    let limestone = drawing.palette.limestone;
    let point = |east: f32, north: f32, up: f32| drawing.projection.screen_point(east, north, up);
    layer.pen_depth = depth_tag(south_metres);
    let ridge_metres_east = (west_metres + east_metres) * 0.5;
    let west_slope = [
        point(west_metres, south_metres, eave_metres),
        point(ridge_metres_east, south_metres, ridge_metres),
        point(ridge_metres_east, north_metres, ridge_metres),
        point(west_metres, north_metres, eave_metres),
    ];
    let east_slope = [
        point(ridge_metres_east, south_metres, ridge_metres),
        point(east_metres, south_metres, eave_metres),
        point(east_metres, north_metres, eave_metres),
        point(ridge_metres_east, north_metres, ridge_metres),
    ];
    let west_light = light_level_of_roof_slope(drawing, -1.0);
    let east_light = light_level_of_roof_slope(drawing, 1.0);
    fill_roof_plane(layer, drawing, &west_slope, Roofing::Slate, west_light, false);
    fill_roof_plane(layer, drawing, &east_slope, Roofing::Slate, east_light, false);
    layer.line(west_slope[1].0, west_slope[1].1, west_slope[2].0, west_slope[2].1, drawing.palette.slate[0][3]);
    let eave_colour = |light_level: usize| if light_level == SHADED { GUTTER_COLOUR } else { limestone[2] };
    layer.line(west_slope[0].0, west_slope[0].1, west_slope[3].0, west_slope[3].1, eave_colour(west_light));
    layer.line(east_slope[1].0, east_slope[1].1, east_slope[2].0, east_slope[2].1, eave_colour(east_light));
    layer.line(west_slope[3].0, west_slope[3].1, west_slope[2].0, west_slope[2].1, limestone[1]);
    layer.line(east_slope[2].0, east_slope[2].1, east_slope[3].0, east_slope[3].1, limestone[2]);
    let south_gable = [
        point(west_metres, south_metres, 0.0),
        point(west_metres, south_metres, eave_metres),
        point(ridge_metres_east, south_metres, ridge_metres),
        point(east_metres, south_metres, eave_metres),
        point(east_metres, south_metres, 0.0),
    ];
    layer.fill_polygon_with(&south_gable, |column, row| Some(drawing.stone(HALF_LIT, column, row)));
    layer.line(south_gable[1].0, south_gable[1].1, south_gable[2].0, south_gable[2].1, limestone[0]);
    layer.line(south_gable[3].0, south_gable[3].1, south_gable[2].0, south_gable[2].1, limestone[1]);
    layer.put(south_gable[2].0, south_gable[2].1 - 1, limestone[0]);
    let (window_column, window_row) = point(ridge_metres_east, south_metres, eave_metres + 0.4);
    layer.vertical_line(window_column, window_row - 3, window_row, drawing.palette.glass[1]);
    layer.put(window_column, window_row - 4, limestone[1]);
    layer.vertical_line(window_column - 1, window_row - 3, window_row, limestone[2]);
    layer.vertical_line(window_column + 1, window_row - 3, window_row, limestone[2]);
    sources.add_block_caster(
        drawing.projection,
        west_metres,
        east_metres,
        south_metres,
        north_metres,
        (eave_metres + ridge_metres) * 0.5 + 0.8,
    );
}

fn draw_slate_roof_with_ridge_running_east(
    layer: &mut Layer,
    drawing: &DrawingContext,
    sources: &mut ShadowAndLightSources,
    footprint: Footprint,
    eave_metres: f32,
    ridge_metres: f32,
) {
    let Footprint { west_metres, east_metres, south_metres, north_metres } = footprint;
    let limestone = drawing.palette.limestone;
    let point = |east: f32, north: f32, up: f32| drawing.projection.screen_point(east, north, up);
    layer.pen_depth = depth_tag(south_metres);
    let ridge_metres_north = (south_metres + north_metres) * 0.5;
    let front_slope = [
        point(west_metres, ridge_metres_north, ridge_metres),
        point(east_metres, ridge_metres_north, ridge_metres),
        point(east_metres, south_metres, eave_metres),
        point(west_metres, south_metres, eave_metres),
    ];
    let back_slope = [
        point(west_metres, north_metres, eave_metres),
        point(east_metres, north_metres, eave_metres),
        point(east_metres, ridge_metres_north, ridge_metres),
        point(west_metres, ridge_metres_north, ridge_metres),
    ];
    fill_roof_plane(layer, drawing, &back_slope, Roofing::Slate, SHADED, true);
    fill_roof_plane(layer, drawing, &front_slope, Roofing::Slate, HALF_LIT, true);
    layer.horizontal_line(front_slope[0].0, front_slope[1].0, front_slope[0].1, drawing.palette.slate[0][3]);
    layer.horizontal_line(back_slope[0].0, back_slope[1].0, back_slope[0].1, limestone[2]);
    let gable_ends =
        [(front_slope[3], front_slope[0], back_slope[0], 1usize), (front_slope[2], front_slope[1], back_slope[1], 2)];
    for (front_foot, apex, back_foot, shade) in gable_ends {
        layer.line(front_foot.0, front_foot.1, apex.0, apex.1, limestone[shade]);
        layer.line(apex.0, apex.1, back_foot.0, back_foot.1, limestone[shade]);
        layer.put(apex.0, apex.1 - 1, limestone[0]);
    }
    sources.add_block_caster(
        drawing.projection,
        west_metres,
        east_metres,
        south_metres,
        north_metres,
        (eave_metres + ridge_metres) * 0.5 + 0.8,
    );
}

fn draw_south_wall(
    layer: &mut Layer,
    drawing: &DrawingContext,
    sources: &mut ShadowAndLightSources,
    wall: SouthWall,
) -> ScreenRectangle {
    let limestone = drawing.palette.limestone;
    layer.pen_depth = depth_tag(wall.metres_north);
    let left = drawing.projection.pixel_x(wall.west_metres, wall.metres_north);
    let right = drawing.projection.pixel_x(wall.east_metres, wall.metres_north);
    let top = drawing.projection.pixel_y(wall.metres_north, wall.eave_metres);
    let bottom = drawing.projection.pixel_y(wall.metres_north, 0.0);
    let rectangle = ScreenRectangle { left, right, top, bottom };
    layer.fill_rect_with(left, top, right, bottom, |column, row| drawing.stone(HALF_LIT, column, row));
    layer.horizontal_line(left, right, bottom, limestone[3]);
    layer.horizontal_line(left, right, top, limestone[1]);
    layer.horizontal_line(left, right, top + 1, 0x34332f);
    drawing.snow_along_ledge(layer, left, right, top);
    if wall.storeys < 1 || right - left < 5 {
        return rectangle;
    }
    let storey_rows = (bottom - top - 2) as f32 / wall.storeys as f32;
    let window_rows = ((storey_rows * 0.55) as i32).clamp(2, 4);
    for storey in 0..wall.storeys {
        let window_top = top + 3 + (storey as f32 * storey_rows).round() as i32;
        if storey > 0 {
            layer.horizontal_line(left, right, window_top - 2, limestone[2]);
        }
        let mut window_left = left + 2;
        let mut window_index = 0;
        while window_left + 3 < right {
            let window_number = (hash_position(window_index, storey, wall.window_seed ^ 0x9) % 60000) as u16 + 100;
            let window_width = if hash_position(window_index, 7, wall.window_seed).is_multiple_of(4) { 3 } else { 2 };
            layer.horizontal_line(window_left - 1, window_left + window_width, window_top - 1, limestone[1]);
            layer.horizontal_line(window_left - 1, window_left + window_width, window_top + window_rows, limestone[2]);
            for rows_down in 0..window_rows {
                let row = window_top + rows_down;
                layer.put(window_left - 1, row, limestone[1]);
                layer.put(window_left + window_width, row, limestone[2]);
                for columns_across in 0..window_width {
                    let column = window_left + columns_across;
                    let colour = daylight_glass_colour(drawing, column, row, LIT_ROOM);
                    put_glass_pixel(layer, sources, column, row, colour, LIT_ROOM, window_number);
                }
            }
            window_left += window_width
                + if hash_position(window_index, storey, wall.window_seed).is_multiple_of(3) { 5 } else { 3 };
            window_index += 1;
        }
    }
    rectangle
}

fn draw_side_wall_under_eave(
    layer: &mut Layer,
    drawing: &DrawingContext,
    wall_metres_east: f32,
    south_metres: f32,
    north_metres: f32,
    eave_metres: f32,
    facing: i32,
) {
    const BAY_METRES: f32 = 3.6;
    const SLIVER_STEP_METRES: f32 = 0.35;
    layer.pen_depth = depth_tag(south_metres);
    let light_level = if drawing.light_level_facing(facing as f32) == SHADED { SHADED } else { HALF_LIT };
    let mut metres_north = north_metres;
    while metres_north >= south_metres {
        let column = drawing.projection.pixel_x(wall_metres_east, metres_north) + facing;
        let top_row = drawing.projection.pixel_y(metres_north, eave_metres);
        let foot_row = drawing.projection.pixel_y(metres_north, 0.0);
        let position_in_bay = (metres_north / BAY_METRES).rem_euclid(1.0);
        for row in top_row..=foot_row {
            let fraction_down = (row - top_row) as f32 / (foot_row - top_row).max(1) as f32;
            let in_a_window_row =
                (fraction_down > 0.12 && fraction_down < 0.3) || (fraction_down > 0.45 && fraction_down < 0.62);
            let is_window = position_in_bay > 0.3 && position_in_bay < 0.7 && in_a_window_row;
            let colour = if is_window { drawing.palette.glass[1] } else { drawing.stone(light_level, column, row) };
            layer.put(column, row, colour);
        }
        metres_north -= SLIVER_STEP_METRES;
    }
}

fn draw_chimney(
    layer: &mut Layer,
    drawing: &DrawingContext,
    metres_east: f32,
    metres_north: f32,
    foot_metres: f32,
    top_metres: f32,
) {
    layer.pen_depth = depth_tag(metres_north);
    let column = drawing.projection.pixel_x(metres_east, metres_north);
    let top_row = drawing.projection.pixel_y(metres_north, top_metres);
    let foot_row = drawing.projection.pixel_y(metres_north, foot_metres);
    fill_stone_lit_from_the_side(layer, drawing, column - 1, top_row, column + 1, foot_row, SUNLIT);
    layer.vertical_line(column + 1, top_row, foot_row, drawing.stone(SHADED, column + 1, top_row));
    layer.horizontal_line(column - 2, column + 2, top_row, drawing.palette.limestone[0]);
    layer.horizontal_line(column - 2, column + 2, top_row + 1, drawing.palette.limestone[3]);
    layer.put(column - 1, top_row - 1, 0x9a6248);
    layer.put(column + 1, top_row - 1, 0x7c4c38);
    drawing.snow_along_ledge(layer, column - 2, column + 2, top_row);
}

fn draw_dormer_facing_south(
    layer: &mut Layer,
    drawing: &DrawingContext,
    sources: &mut ShadowAndLightSources,
    metres_east: f32,
    metres_north: f32,
    metres_up: f32,
    window_seed: u32,
) {
    let limestone = drawing.palette.limestone;
    layer.pen_depth = depth_tag(metres_north);
    let (centre, sill_row) = drawing.projection.screen_point(metres_east, metres_north, metres_up);
    for rows_up in 0..7 {
        let row = sill_row - 4 - rows_up;
        for columns_from_centre in -3..=3i32 {
            let column = centre + columns_from_centre;
            let colour = if columns_from_centre == 0 {
                drawing.palette.slate[0][3]
            } else if columns_from_centre.abs() == 3 {
                GUTTER_COLOUR
            } else {
                let light_level = if columns_from_centre < 0 {
                    light_level_of_roof_slope(drawing, -1.0)
                } else {
                    light_level_of_roof_slope(drawing, 1.0)
                };
                let steps_from_light = if light_level == SHADED { 1 } else { 0 };
                drawing.snow_colour(column, row, 2.0, -0.12, steps_from_light).unwrap_or(slate_colour(
                    drawing.palette,
                    light_level,
                    column,
                    row,
                    true,
                ))
            };
            layer.put(column, row, colour);
        }
    }
    layer.fill_rect_with(centre - 3, sill_row - 3, centre + 3, sill_row, |column, row| {
        drawing.stone(SUNLIT, column, row)
    });
    draw_gablet(layer, drawing, centre, sill_row - 4, 3);
    let window_number = (hash_position(centre, sill_row, window_seed) % 60000) as u16 + 100;
    for rows_up in 0..3 {
        let row = sill_row - 1 - rows_up;
        for side in [-1, 1] {
            let colour = daylight_glass_colour(drawing, centre + side, sill_row - rows_up, LIT_ROOM);
            put_glass_pixel(layer, sources, centre + side, row, colour, LIT_ROOM, window_number);
        }
        layer.put(centre, row, limestone[1]);
        layer.put(centre - 2, row, limestone[1]);
        layer.put(centre + 2, row, limestone[2]);
    }
    layer.horizontal_line(centre - 2, centre + 2, sill_row, limestone[2]);
}

fn draw_dormers_on_a_side_slope(
    layer: &mut Layer,
    drawing: &DrawingContext,
    eave_metres_east: f32,
    facing: i32,
    eave_metres: f32,
    dormers_metres_north: &[f32],
) {
    let limestone = drawing.palette.limestone;
    for &metres_north in dormers_metres_north {
        layer.pen_depth = depth_tag(metres_north);
        let (eave_column, ridge_row) =
            drawing.projection.screen_point(eave_metres_east, metres_north, eave_metres + 1.4);
        for columns_in in 1..=7i32 {
            let column = eave_column - facing * columns_in;
            layer.put(column, ridge_row - 4, GUTTER_COLOUR);
            for rows_from_ridge in 1..=3 {
                let back_row = ridge_row - rows_from_ridge;
                let front_row = ridge_row + rows_from_ridge;
                let back_colour = drawing.snow_colour(column, back_row, 2.0, -0.12, 1).unwrap_or(slate_colour(
                    drawing.palette,
                    SHADED,
                    column,
                    back_row,
                    false,
                ));
                layer.put(column, back_row, back_colour);
                let front_colour = drawing.snow_colour(column, front_row, 2.0, -0.12, 0).unwrap_or(slate_colour(
                    drawing.palette,
                    SUNLIT,
                    column,
                    front_row,
                    false,
                ));
                layer.put(column, front_row, front_colour);
            }
            layer.put(column, ridge_row, drawing.palette.slate[0][3]);
            layer.put(column, ridge_row + 4, GUTTER_COLOUR);
        }
        for rows_from_ridge in -3..=3 {
            let row = ridge_row + rows_from_ridge;
            let near_the_ridge = rows_from_ridge.abs() <= 1;
            layer.put(eave_column, row, if near_the_ridge { limestone[0] } else { limestone[1] });
            let wall_colour =
                if near_the_ridge { limestone[2] } else { drawing.stone(HALF_LIT, eave_column + facing, row) };
            layer.put(eave_column + facing, row, wall_colour);
        }
        layer.put(eave_column + facing, ridge_row + 1, drawing.palette.glass[1]);
        layer.put(eave_column + facing, ridge_row + 2, drawing.palette.glass[1]);
    }
}

fn draw_battlemented_tower(
    layer: &mut Layer,
    drawing: &DrawingContext,
    sources: &mut ShadowAndLightSources,
    footprint: Footprint,
    height_metres: f32,
) -> ScreenRectangle {
    let Footprint { west_metres, east_metres, south_metres, north_metres } = footprint;
    let limestone = drawing.palette.limestone;
    let point = |east: f32, north: f32| drawing.projection.screen_point(east, north, height_metres);
    let draw_battlements = |layer: &mut Layer, left: i32, right: i32, row: i32| {
        for column in left..=right {
            layer.put(column, row, limestone[1]);
            layer.put(column, row + 1, limestone[3]);
            if (column - left) % 4 < 2 {
                layer.put(column, row - 1, limestone[0]);
            }
        }
        drawing.snow_along_ledge(layer, left, right, row - 1);
    };
    layer.pen_depth = depth_tag(south_metres);
    let roof = [
        point(west_metres, north_metres),
        point(east_metres, north_metres),
        point(east_metres, south_metres),
        point(west_metres, south_metres),
    ];
    fill_roof_plane(layer, drawing, &roof, Roofing::Lead, SHADED, true);
    draw_battlements(layer, roof[0].0, roof[1].0, roof[0].1);
    for row in roof[0].1..=roof[3].1 {
        let is_merlon = (row - roof[0].1) % 3 != 2;
        layer.put(roof[0].0, row, if is_merlon { limestone[0] } else { limestone[3] });
        layer.put(roof[1].0, row, if is_merlon { limestone[2] } else { limestone[4] });
    }
    let left = drawing.projection.pixel_x(west_metres, south_metres);
    let right = drawing.projection.pixel_x(east_metres, south_metres);
    let top = drawing.projection.pixel_y(south_metres, height_metres);
    let bottom = drawing.projection.pixel_y(south_metres, 0.0);
    layer.fill_rect_with(left, top, right, bottom, |column, row| drawing.stone(SUNLIT, column, row));
    layer.horizontal_line(left, right, bottom, limestone[3]);
    draw_battlements(layer, left, right, top);
    draw_quoins(layer, drawing, left, top + 3, bottom - 1, 1);
    draw_quoins(layer, drawing, right, top + 3, bottom - 1, -1);
    sources.add_block_caster(drawing.projection, west_metres, east_metres, south_metres, north_metres, height_metres);
    ScreenRectangle { left, right, top, bottom }
}

fn flat_roof_colour(drawing: &DrawingContext, roof: FlatRoof, column: i32, row: i32) -> u32 {
    if let Some(snow) = drawing.snow_colour(column, row, 1.9, -0.1, 0) {
        return snow;
    }
    let darkened_by_rain = |colour: u32| wetted(colour, drawing.look.wetness * 0.6);
    match roof {
        FlatRoof::GreyMembrane => {
            let stain = hash_position(column / 5, row / 2, 0xB0F) % 9;
            darkened_by_rain(if row.rem_euclid(4) == 0 {
                0x94979a
            } else if stain == 0 {
                0x9c9fa1
            } else if stain == 1 {
                0xb1b3b2
            } else {
                0xa7a9aa
            })
        }
        FlatRoof::WhiteMembrane => darkened_by_rain(if hash_position(column / 6, row / 3, 0xB10).is_multiple_of(7) {
            0xd6d7d2
        } else {
            0xe4e5e0
        }),
        FlatRoof::BlueGreyStandingSeamMetal => lead_colour(drawing.palette, column, row, 4, HALF_LIT, true),
    }
}

fn draw_flat_roofed_block(
    layer: &mut Layer,
    drawing: &DrawingContext,
    sources: &mut ShadowAndLightSources,
    block: FlatBlock,
) {
    let outline = block.outline;
    let corners = outline.len();
    let nearest_metres_north = outline.iter().map(|corner| corner.1).fold(f32::INFINITY, f32::min);
    let farthest_metres_north = outline.iter().map(|corner| corner.1).fold(f32::NEG_INFINITY, f32::max);
    layer.pen_depth = depth_tag(nearest_metres_north);
    let mut twice_area = 0.0;
    for index in 0..corners {
        let (from, to) = (outline[index], outline[(index + 1) % corners]);
        twice_area += from.0 * to.1 - to.0 * from.1;
    }
    let winding = if twice_area > 0.0 { 1.0 } else { -1.0 };
    let mut walls_facing_south: Vec<((f32, f32), (f32, f32))> = Vec::new();
    for index in 0..corners {
        let (from, to) = (outline[index], outline[(index + 1) % corners]);
        if (to.0 - from.0) * winding > 0.3 {
            walls_facing_south.push(if from.0 < to.0 { (from, to) } else { (to, from) });
        }
    }
    walls_facing_south.sort_by(|first, second| second.0 .1.max(second.1 .1).total_cmp(&first.0 .1.max(first.1 .1)));
    let limestone = drawing.palette.limestone;
    for (west_end, east_end) in walls_facing_south {
        let left = drawing.projection.pixel_x(west_end.0, west_end.1);
        let right = drawing.projection.pixel_x(east_end.0, east_end.1);
        for column in left..=right {
            let fraction_along = if right > left { (column - left) as f32 / (right - left) as f32 } else { 0.0 };
            let metres_north = west_end.1 + (east_end.1 - west_end.1) * fraction_along;
            let top = drawing.projection.pixel_y(metres_north, block.roof_metres);
            let foot = drawing.projection.pixel_y(metres_north, 0.0);
            for row in top..=foot {
                let fraction_up = (foot - row) as f32 / (foot - top).max(1) as f32;
                let position_in_storey = (fraction_up * block.storeys as f32).fract();
                let storey = (fraction_up * block.storeys as f32) as i32;
                let is_window_band =
                    block.storeys > 0 && position_in_storey > 0.36 && position_in_storey < 0.74 && row < foot;
                let colour = match block.walling {
                    Walling::BuffConcreteWithRibbonWindows => {
                        if is_window_band && column % 5 != 0 {
                            let window_number =
                                (hash_position(column / 10, storey, block.window_seed) % 60000) as u16 + 100;
                            let glass = daylight_glass_colour(drawing, column, row, LIT_ROOM);
                            put_glass_pixel(layer, sources, column, row, glass, LIT_ROOM, window_number);
                            continue;
                        }
                        if row == top {
                            0xd9d2c0
                        } else if hash_position(column / 7, row, block.window_seed).is_multiple_of(6) {
                            0xb5ab97
                        } else {
                            0xc4baa6
                        }
                    }
                    Walling::DukeStoneWithWindowRows => {
                        if is_window_band && column % 4 < 2 {
                            let window_number =
                                (hash_position(column / 4, storey, block.window_seed) % 60000) as u16 + 100;
                            let glass = daylight_glass_colour(drawing, column, row, LIT_ROOM);
                            put_glass_pixel(layer, sources, column, row, glass, LIT_ROOM, window_number);
                            continue;
                        }
                        if row == top {
                            limestone[1]
                        } else {
                            drawing.stone(HALF_LIT, column, row)
                        }
                    }
                };
                layer.put(column, row, colour);
            }
        }
    }
    let roof_corners: Vec<(i32, i32)> =
        outline.iter().map(|corner| drawing.projection.screen_point(corner.0, corner.1, block.roof_metres)).collect();
    layer.fill_polygon_with(&roof_corners, |column, row| Some(flat_roof_colour(drawing, block.roof, column, row)));
    let parapet_colour = if block.walling == Walling::BuffConcreteWithRibbonWindows { 0xcfc8b6 } else { limestone[1] };
    for index in 0..corners {
        let (from, to) = (roof_corners[index], roof_corners[(index + 1) % corners]);
        layer.line(from.0, from.1, to.0, to.1, parapet_colour);
    }
    sources.casters.push(Caster::Prism {
        footprint: drawing.projection.ground_polygon(outline),
        height_pixels: drawing
            .projection
            .height_in_pixels((nearest_metres_north + farthest_metres_north) * 0.5, block.roof_metres)
            as f32,
    });
}

fn draw_box_on_a_flat_roof(
    layer: &mut Layer,
    drawing: &DrawingContext,
    footprint: Footprint,
    roof_metres: f32,
    box_height_metres: f32,
    top_colour: u32,
    side_colour: u32,
) {
    let Footprint { west_metres, east_metres, south_metres, north_metres } = footprint;
    let top_metres = roof_metres + box_height_metres;
    let left = drawing.projection.pixel_x(west_metres, south_metres);
    let right = drawing.projection.pixel_x(east_metres, south_metres);
    let top_row = drawing.projection.pixel_y(south_metres, top_metres);
    let foot_row = drawing.projection.pixel_y(south_metres, roof_metres);
    layer.fill_rect(left, top_row, right, foot_row, side_colour);
    let top_corners = [
        drawing.projection.screen_point(west_metres, north_metres, top_metres),
        drawing.projection.screen_point(east_metres, north_metres, top_metres),
        drawing.projection.screen_point(east_metres, south_metres, top_metres),
        drawing.projection.screen_point(west_metres, south_metres, top_metres),
    ];
    layer.fill_polygon_with(&top_corners, |column, row| {
        Some(drawing.snow_colour(column, row, 1.9, -0.1, 0).unwrap_or(top_colour))
    });
}

fn draw_bryan_center(layer: &mut Layer, drawing: &DrawingContext, sources: &mut ShadowAndLightSources) {
    const ROOF_METRES: f32 = 12.0;
    const PLANT_SCREEN_HEIGHT_METRES: f32 = 2.2;
    const AIR_HANDLERS_METRES: [(f32, f32); 5] =
        [(-104.0, 41.0), (-97.5, 41.5), (-103.0, 50.0), (-95.5, 52.0), (-102.0, 58.0)];
    const SKYLIGHTS_AND_VENTS_METRES: [(f32, f32); 9] = [
        (-98.0, 92.0),
        (-112.0, 96.0),
        (-100.0, 112.0),
        (-116.0, 70.0),
        (-90.0, 70.0),
        (-82.0, 68.0),
        (-108.0, 132.0),
        (-120.0, 136.0),
        (-110.0, 30.0),
    ];
    const PLANT_GREY: u32 = 0x8e9194;
    const PLANT_SHADOW: u32 = 0x6f7377;
    const PLANT_WHITE: u32 = 0xe9e9e4;
    let block = FlatBlock {
        outline: crate::chapel::survey::BRYAN_CENTER_OUTLINE,
        roof_metres: ROOF_METRES,
        roof: FlatRoof::GreyMembrane,
        walling: Walling::BuffConcreteWithRibbonWindows,
        storeys: 3,
        window_seed: 0xBC1,
    };
    draw_flat_roofed_block(layer, drawing, sources, block);
    let point = |east: f32, north: f32| drawing.projection.screen_point(east, north, ROOF_METRES);
    let white_roofed_north_hall = [
        point(-148.6, 149.3),
        point(-127.3, 149.2),
        point(-127.3, 144.9),
        point(-94.3, 144.6),
        point(-94.3, 127.2),
        point(-150.4, 129.1),
    ];
    layer.fill_polygon_with(&white_roofed_north_hall, |column, row| {
        Some(flat_roof_colour(drawing, FlatRoof::WhiteMembrane, column, row))
    });
    let solar_array = [point(-121.0, 123.0), point(-95.0, 123.0), point(-95.0, 105.5), point(-121.0, 105.5)];
    layer.fill_polygon_with(&solar_array, |column, row| {
        if let Some(snow) = drawing.snow_colour(column, row, 1.6, -0.2, 0) {
            return Some(snow);
        }
        let (metres_east, metres_north) =
            drawing.projection.world_point_under_pixel(column as f32, row as f32, ROOF_METRES);
        let position_in_panel_row = ((metres_east + metres_north) / 2.6).rem_euclid(1.0);
        Some(if position_in_panel_row < 0.62 {
            if (column + row) % 3 == 0 {
                0x2c3a52
            } else {
                0x1f2a3d
            }
        } else {
            0xb9bcbd
        })
    });
    layer.pen_depth = depth_tag(37.0);
    let plant_screen = Footprint { west_metres: -106.0, east_metres: -88.5, south_metres: 38.0, north_metres: 63.5 };
    draw_box_on_a_flat_roof(
        layer,
        drawing,
        plant_screen,
        ROOF_METRES,
        PLANT_SCREEN_HEIGHT_METRES,
        PLANT_GREY,
        PLANT_SHADOW,
    );
    for (west_metres, south_metres) in AIR_HANDLERS_METRES {
        let air_handler =
            Footprint { west_metres, east_metres: west_metres + 4.4, south_metres, north_metres: south_metres + 3.4 };
        draw_box_on_a_flat_roof(
            layer,
            drawing,
            air_handler,
            ROOF_METRES + PLANT_SCREEN_HEIGHT_METRES,
            1.2,
            PLANT_WHITE,
            0xb9bbb8,
        );
    }
    for (index, (west_metres, south_metres)) in SKYLIGHTS_AND_VENTS_METRES.into_iter().enumerate() {
        let skylight = Footprint {
            west_metres,
            east_metres: west_metres + 2.4 + (index % 2) as f32,
            south_metres,
            north_metres: south_metres + 1.8,
        };
        let top_colour = if index % 3 == 0 { PLANT_WHITE } else { PLANT_GREY };
        draw_box_on_a_flat_roof(layer, drawing, skylight, ROOF_METRES, 0.8, top_colour, PLANT_SHADOW);
    }
}

fn draw_divinity_school_north_range(layer: &mut Layer, drawing: &DrawingContext, sources: &mut ShadowAndLightSources) {
    const EAVE_METRES: f32 = 11.0;
    const RIDGE_METRES: f32 = 16.5;
    const SOUTH_WALL_METRES_NORTH: f32 = 51.7;
    let gabled_bay = Footprint { west_metres: 58.6, east_metres: 66.1, south_metres: 63.0, north_metres: 69.8 };
    draw_slate_roof_with_ridge_running_north(layer, drawing, sources, gabled_bay, 11.0, 15.0);
    let range =
        Footprint { west_metres: 40.0, east_metres: 75.0, south_metres: SOUTH_WALL_METRES_NORTH, north_metres: 66.9 };
    draw_slate_roof_with_ridge_running_east(layer, drawing, sources, range, EAVE_METRES, RIDGE_METRES);
    let south_wall = SouthWall {
        west_metres: 40.0,
        east_metres: 75.0,
        metres_north: SOUTH_WALL_METRES_NORTH,
        eave_metres: EAVE_METRES,
        storeys: 3,
        window_seed: 0xE1,
    };
    draw_south_wall(layer, drawing, sources, south_wall);
    for dormer_metres_east in [46.0f32, 53.5, 61.0, 68.5] {
        draw_dormer_facing_south(
            layer,
            drawing,
            sources,
            dormer_metres_east,
            SOUTH_WALL_METRES_NORTH + 0.8,
            EAVE_METRES + 0.9,
            0xE2,
        );
    }
    draw_chimney(layer, drawing, 43.0, 59.3, RIDGE_METRES - 0.4, RIDGE_METRES + 3.2);
    draw_chimney(layer, drawing, 72.0, 59.3, RIDGE_METRES - 0.4, RIDGE_METRES + 3.2);
    let link_along_the_court = FlatBlock {
        outline: &[(50.75, 47.6), (74.5, 47.6), (74.5, 51.7), (50.75, 51.7)],
        roof_metres: 4.4,
        roof: FlatRoof::BlueGreyStandingSeamMetal,
        walling: Walling::DukeStoneWithWindowRows,
        storeys: 1,
        window_seed: 0xE3,
    };
    draw_flat_roofed_block(layer, drawing, sources, link_along_the_court);
}

fn draw_divinity_school_west_range(layer: &mut Layer, drawing: &DrawingContext, sources: &mut ShadowAndLightSources) {
    const ROOF_METRES: f32 = 10.0;
    const ROOF_VENTS_METRES: [(f32, f32); 3] = [(42.5, 23.0), (46.5, 34.0), (43.0, 44.0)];
    let range = FlatBlock {
        outline: &[(39.5, 13.5), (50.75, 13.5), (50.75, 51.7), (39.5, 51.7)],
        roof_metres: ROOF_METRES,
        roof: FlatRoof::WhiteMembrane,
        walling: Walling::DukeStoneWithWindowRows,
        storeys: 3,
        window_seed: 0xE5,
    };
    draw_flat_roofed_block(layer, drawing, sources, range);
    draw_side_wall_under_eave(layer, drawing, 39.5, 13.5, 51.7, ROOF_METRES, -1);
    for (west_metres, south_metres) in ROOF_VENTS_METRES {
        let vent =
            Footprint { west_metres, east_metres: west_metres + 2.2, south_metres, north_metres: south_metres + 1.8 };
        draw_box_on_a_flat_roof(layer, drawing, vent, ROOF_METRES, 0.8, 0xb8bab8, 0x7d8083);
    }
}

fn draw_divinity_school_addition(layer: &mut Layer, drawing: &DrawingContext, sources: &mut ShadowAndLightSources) {
    const ROOF_METRES: f32 = 13.0;
    const MAPPED_OUTLINE_LIES_SOUTH_BY_METRES: f32 = 3.2;
    const CHAPEL_ROOF_APEX_METRES: (f32, f32) = (92.2, 29.8);
    const CHAPEL_ROOF_RISE_METRES: f32 = 4.5;
    const CHAPEL_ROOF_RIM_METRES: [(f32, f32); 7] =
        [(84.7, 37.2), (91.2, 38.3), (101.4, 29.8), (101.9, 25.7), (98.5, 22.2), (86.0, 23.2), (82.0, 27.4)];
    const ROOF_PLANT_METRES: [(f32, f32); 3] = [(66.0, 3.2), (74.0, 5.7), (88.0, 9.2)];
    let outline: Vec<(f32, f32)> = crate::chapel::survey::LANGFORD_BUILDING_OUTLINE
        .iter()
        .map(|corner| (corner.0, corner.1 + MAPPED_OUTLINE_LIES_SOUTH_BY_METRES))
        .collect();
    let block = FlatBlock {
        outline: &outline,
        roof_metres: ROOF_METRES,
        roof: FlatRoof::BlueGreyStandingSeamMetal,
        walling: Walling::DukeStoneWithWindowRows,
        storeys: 3,
        window_seed: 0xE7,
    };
    draw_flat_roofed_block(layer, drawing, sources, block);
    let (apex_metres_east, apex_metres_north) = CHAPEL_ROOF_APEX_METRES;
    let apex =
        drawing.projection.screen_point(apex_metres_east, apex_metres_north, ROOF_METRES + CHAPEL_ROOF_RISE_METRES);
    let rim_corners = CHAPEL_ROOF_RIM_METRES.len();
    for index in 0..rim_corners {
        let (from, to) = (CHAPEL_ROOF_RIM_METRES[index], CHAPEL_ROOF_RIM_METRES[(index + 1) % rim_corners]);
        let facing_east = (from.0 + to.0) * 0.5 - apex_metres_east;
        let facing_north = (from.1 + to.1) * 0.5 - apex_metres_north;
        let light_level = if facing_north > 2.0 {
            SHADED
        } else if facing_east.abs() > facing_north.abs() {
            light_level_of_roof_slope(drawing, facing_east.signum())
        } else {
            SUNLIT
        };
        let from_corner = drawing.projection.screen_point(from.0, from.1, ROOF_METRES);
        let to_corner = drawing.projection.screen_point(to.0, to.1, ROOF_METRES);
        fill_lead_roof(
            layer,
            drawing,
            &[from_corner, to_corner, apex],
            4,
            light_level,
            facing_east.abs() < facing_north.abs(),
        );
        layer.line(from_corner.0, from_corner.1, apex.0, apex.1, drawing.palette.lead[0]);
    }
    layer.put(apex.0, apex.1 - 1, drawing.palette.limestone[0]);
    for (west_metres, south_metres) in ROOF_PLANT_METRES {
        let plant =
            Footprint { west_metres, east_metres: west_metres + 2.6, south_metres, north_metres: south_metres + 2.0 };
        draw_box_on_a_flat_roof(layer, drawing, plant, ROOF_METRES, 0.9, 0xaeb6c0, 0x6f7a88);
    }
}

fn draw_page_auditorium(layer: &mut Layer, drawing: &DrawingContext, sources: &mut ShadowAndLightSources) {
    const STAGE_HOUSE_WEST_METRES: f32 = -81.9;
    const HALL_WEST_METRES: f32 = -69.0;
    const HALL_EAST_METRES: f32 = -39.0;
    const SOUTH_METRES: f32 = -20.7;
    const NORTH_METRES: f32 = 0.5;
    const STAGE_HOUSE_ROOF_METRES: f32 = 13.0;
    const HALL_EAVE_METRES: f32 = 12.5;
    const HALL_RIDGE_METRES: f32 = 18.5;
    const HIP_LENGTH_METRES: f32 = 6.0;
    const BAY_PIXELS: i32 = 15;
    const BAY_WINDOW_WIDTH_PIXELS: i32 = 5;
    const FIRST_BAY_WINDOW_NUMBER: u16 = 60;
    let point = |east: f32, north: f32, up: f32| drawing.projection.screen_point(east, north, up);
    let stage_house = FlatBlock {
        outline: &[
            (STAGE_HOUSE_WEST_METRES, SOUTH_METRES),
            (HALL_WEST_METRES, SOUTH_METRES),
            (HALL_WEST_METRES, NORTH_METRES),
            (STAGE_HOUSE_WEST_METRES, NORTH_METRES),
        ],
        roof_metres: STAGE_HOUSE_ROOF_METRES,
        roof: FlatRoof::GreyMembrane,
        walling: Walling::DukeStoneWithWindowRows,
        storeys: 0,
        window_seed: 0xA0,
    };
    draw_flat_roofed_block(layer, drawing, sources, stage_house);
    let fly_tower_vent = Footprint { west_metres: -77.0, east_metres: -73.0, south_metres: -12.5, north_metres: -7.5 };
    draw_box_on_a_flat_roof(layer, drawing, fly_tower_vent, STAGE_HOUSE_ROOF_METRES, 1.4, 0x55585c, 0x3d4044);

    let ridge_metres_north = (SOUTH_METRES + NORTH_METRES) * 0.5;
    layer.pen_depth = depth_tag(SOUTH_METRES);
    let hip_point = point(HALL_WEST_METRES + HIP_LENGTH_METRES, ridge_metres_north, HALL_RIDGE_METRES);
    let ridge_east_end = point(HALL_EAST_METRES, ridge_metres_north, HALL_RIDGE_METRES);
    let south_west_corner = point(HALL_WEST_METRES, SOUTH_METRES, HALL_EAVE_METRES);
    let north_west_corner = point(HALL_WEST_METRES, NORTH_METRES, HALL_EAVE_METRES);
    let north_slope =
        [north_west_corner, point(HALL_EAST_METRES, NORTH_METRES, HALL_EAVE_METRES), ridge_east_end, hip_point];
    fill_roof_plane(layer, drawing, &north_slope, Roofing::Slate, SHADED, true);
    let west_hip = [south_west_corner, north_west_corner, hip_point];
    fill_roof_plane(layer, drawing, &west_hip, Roofing::Slate, light_level_of_roof_slope(drawing, -1.0), false);
    let south_slope =
        [south_west_corner, point(HALL_EAST_METRES, SOUTH_METRES, HALL_EAVE_METRES), ridge_east_end, hip_point];
    fill_roof_plane(layer, drawing, &south_slope, Roofing::Slate, HALF_LIT, true);
    let ridge_colour = drawing.palette.slate[0][3];
    layer.line(hip_point.0, hip_point.1, ridge_east_end.0, ridge_east_end.1, ridge_colour);
    for corner in [south_west_corner, north_west_corner] {
        layer.line(corner.0, corner.1, hip_point.0, hip_point.1, ridge_colour);
    }
    sources.add_block_caster(drawing.projection, HALL_WEST_METRES, HALL_EAST_METRES, SOUTH_METRES, NORTH_METRES, 15.5);
    let hall_south_wall = SouthWall {
        west_metres: HALL_WEST_METRES,
        east_metres: HALL_EAST_METRES,
        metres_north: SOUTH_METRES,
        eave_metres: HALL_EAVE_METRES,
        storeys: 0,
        window_seed: 0xA1,
    };
    let wall = draw_south_wall(layer, drawing, sources, hall_south_wall);
    let mut window_column = wall.left + BAY_PIXELS / 2;
    let mut bay_index = 0;
    while window_column < wall.right - 2 {
        let bay_window = GothicWindow {
            centre_column: window_column,
            sill_row: wall.top + (wall.bottom - wall.top) * 7 / 10,
            apex_row: wall.top + 3,
            glass_width: BAY_WINDOW_WIDTH_PIXELS,
            lights: 2,
            glass_kind: LIT_ROOM,
            window_number: FIRST_BAY_WINDOW_NUMBER + bay_index,
            transom_rows_above_sill: 0,
        };
        draw_gothic_window(layer, drawing, sources, &bay_window);
        let buttress_column = window_column + BAY_PIXELS / 2;
        if buttress_column < wall.right {
            layer.fill_rect_with(buttress_column, wall.top + 2, buttress_column + 1, wall.bottom, |column, row| {
                drawing.stone(SUNLIT, column, row)
            });
            layer.horizontal_line(buttress_column, buttress_column + 1, wall.top + 2, drawing.palette.limestone[1]);
            draw_pinnacle(layer, drawing, buttress_column, wall.top, 4, 1);
        }
        window_column += BAY_PIXELS;
        bay_index += 1;
    }

    let foyer_range =
        Footprint { west_metres: HALL_EAST_METRES, east_metres: -32.7, south_metres: SOUTH_METRES, north_metres: -0.1 };
    draw_slate_roof_with_ridge_running_north(layer, drawing, sources, foyer_range, 11.5, 16.0);
    draw_side_wall_under_eave(layer, drawing, -32.7, SOUTH_METRES, -0.1, 11.5, 1);
    draw_dormers_on_a_side_slope(layer, drawing, -32.7, 1, 11.5, &[-15.0, -5.5]);
    draw_chimney(layer, drawing, -41.0, -1.5, 15.0, 20.5);
    draw_chimney(layer, drawing, -40.5, -18.5, 15.0, 20.5);
}

fn draw_flowers_building_south_wing(layer: &mut Layer, drawing: &DrawingContext, sources: &mut ShadowAndLightSources) {
    const EAVE_METRES: f32 = 11.0;
    const RIDGE_METRES: f32 = 16.2;
    let wing = Footprint { west_metres: -44.7, east_metres: -32.7, south_metres: -31.6, north_metres: -20.7 };
    draw_slate_roof_with_ridge_running_north(layer, drawing, sources, wing, EAVE_METRES, RIDGE_METRES);
    draw_side_wall_under_eave(layer, drawing, wing.east_metres, wing.south_metres, wing.north_metres, EAVE_METRES, 1);
    draw_side_wall_under_eave(layer, drawing, wing.west_metres, wing.south_metres, wing.north_metres, EAVE_METRES, -1);
    draw_dormers_on_a_side_slope(layer, drawing, wing.east_metres, 1, EAVE_METRES, &[-29.5, -26.0, -22.5]);
    let cross_range = Footprint { west_metres: -46.2, east_metres: -29.2, south_metres: -39.9, north_metres: -31.6 };
    draw_slate_roof_with_ridge_running_east(layer, drawing, sources, cross_range, EAVE_METRES, RIDGE_METRES);
    draw_chimney(layer, drawing, -38.7, -26.0, RIDGE_METRES - 0.5, RIDGE_METRES + 3.4);
}

fn draw_flowers_building_north_block(layer: &mut Layer, drawing: &DrawingContext, sources: &mut ShadowAndLightSources) {
    const EAVE_METRES: f32 = 11.0;
    const RIDGE_METRES: f32 = 16.6;
    const CROSS_GABLE_WEST_METRES: f32 = -41.5;
    const CROSS_GABLE_EAST_METRES: f32 = -35.5;
    const CROSS_GABLE_RIDGE_METRES_EAST: f32 = -38.5;
    let block = Footprint { west_metres: -47.0, east_metres: -26.9, south_metres: -0.1, north_metres: 12.3 };
    let (south, north) = (block.south_metres, block.north_metres);
    let point = |east: f32, north: f32, up: f32| drawing.projection.screen_point(east, north, up);
    draw_slate_roof_with_ridge_running_east(layer, drawing, sources, block, EAVE_METRES, RIDGE_METRES);
    let cross_gable_ridge_south = point(CROSS_GABLE_RIDGE_METRES_EAST, south, RIDGE_METRES);
    let cross_gable_ridge_north = point(CROSS_GABLE_RIDGE_METRES_EAST, north, RIDGE_METRES);
    let west_slope = [
        point(CROSS_GABLE_WEST_METRES, south, EAVE_METRES),
        cross_gable_ridge_south,
        cross_gable_ridge_north,
        point(CROSS_GABLE_WEST_METRES, north, EAVE_METRES),
    ];
    fill_roof_plane(layer, drawing, &west_slope, Roofing::Slate, light_level_of_roof_slope(drawing, -1.0), false);
    let east_slope = [
        cross_gable_ridge_south,
        point(CROSS_GABLE_EAST_METRES, south, EAVE_METRES),
        point(CROSS_GABLE_EAST_METRES, north, EAVE_METRES),
        cross_gable_ridge_north,
    ];
    fill_roof_plane(layer, drawing, &east_slope, Roofing::Slate, light_level_of_roof_slope(drawing, 1.0), false);
    layer.line(
        cross_gable_ridge_south.0,
        cross_gable_ridge_south.1,
        cross_gable_ridge_north.0,
        cross_gable_ridge_north.1,
        drawing.palette.slate[0][3],
    );
    let east_end_wall = SouthWall {
        west_metres: -32.7,
        east_metres: -26.9,
        metres_north: south,
        eave_metres: EAVE_METRES,
        storeys: 3,
        window_seed: 0xB7,
    };
    draw_south_wall(layer, drawing, sources, east_end_wall);
    draw_dormer_facing_south(layer, drawing, sources, -29.8, south + 0.8, EAVE_METRES + 0.9, 0xB8);
    draw_chimney(layer, drawing, -44.5, 6.1, RIDGE_METRES - 0.4, RIDGE_METRES + 3.2);
    draw_chimney(layer, drawing, -29.0, 6.1, RIDGE_METRES - 0.4, RIDGE_METRES + 3.2);
}

fn draw_gray_building_north_block(layer: &mut Layer, drawing: &DrawingContext, sources: &mut ShadowAndLightSources) {
    const EAVE_METRES: f32 = 11.0;
    const RIDGE_METRES: f32 = 16.6;
    let block = Footprint { west_metres: 30.0, east_metres: 48.9, south_metres: 2.5, north_metres: 13.5 };
    draw_slate_roof_with_ridge_running_east(layer, drawing, sources, block, EAVE_METRES, RIDGE_METRES);
    for (west_metres, east_metres, window_seed) in [(30.0, 32.0, 0xC1), (45.0, 48.9, 0xC3)] {
        let wall = SouthWall {
            west_metres,
            east_metres,
            metres_north: block.south_metres,
            eave_metres: EAVE_METRES,
            storeys: 3,
            window_seed,
        };
        draw_south_wall(layer, drawing, sources, wall);
    }
    draw_chimney(layer, drawing, 46.5, 8.0, RIDGE_METRES - 0.4, RIDGE_METRES + 3.2);
    draw_chimney(layer, drawing, 33.0, 8.0, RIDGE_METRES - 0.4, RIDGE_METRES + 3.2);
}

fn draw_gray_building_wing(layer: &mut Layer, drawing: &DrawingContext, sources: &mut ShadowAndLightSources) {
    const EAVE_METRES: f32 = 11.0;
    const RIDGE_METRES: f32 = 16.4;
    let wing = Footprint { west_metres: 32.0, east_metres: 45.0, south_metres: -34.7, north_metres: 3.3 };
    draw_slate_roof_with_ridge_running_north(layer, drawing, sources, wing, EAVE_METRES, RIDGE_METRES);
    draw_side_wall_under_eave(layer, drawing, wing.west_metres, wing.south_metres, wing.north_metres, EAVE_METRES, -1);
    draw_side_wall_under_eave(layer, drawing, wing.east_metres, wing.south_metres, wing.north_metres, EAVE_METRES, 1);
    draw_dormers_on_a_side_slope(layer, drawing, wing.west_metres, -1, EAVE_METRES, &[-28.0, -21.0, -4.0]);
    draw_dormers_on_a_side_slope(layer, drawing, wing.east_metres, 1, EAVE_METRES, &[-22.0, -14.0, -6.0]);
    draw_chimney(layer, drawing, 38.5, -6.0, RIDGE_METRES - 0.5, RIDGE_METRES + 3.4);
    draw_chimney(layer, drawing, 38.5, -24.0, RIDGE_METRES - 0.5, RIDGE_METRES + 3.4);
    let entrance_tower = Footprint { west_metres: 28.6, east_metres: 33.3, south_metres: -13.5, north_metres: -9.9 };
    let tower_face = draw_battlemented_tower(layer, drawing, sources, entrance_tower, 14.6);
    let door_column = (tower_face.left + tower_face.right) / 2;
    let window = GothicWindow {
        centre_column: door_column,
        sill_row: tower_face.top + 13,
        apex_row: tower_face.top + 4,
        glass_width: 5,
        lights: 2,
        glass_kind: LIT_ROOM,
        window_number: 80,
        transom_rows_above_sill: 0,
    };
    draw_gothic_window(layer, drawing, sources, &window);
    layer.fill_rect(
        door_column - 3,
        tower_face.bottom - 6,
        door_column + 3,
        tower_face.bottom,
        drawing.palette.limestone[1],
    );
    draw_plank_door(layer, drawing, door_column - 2, tower_face.bottom - 5, door_column + 2, tower_face.bottom);
}

fn draw_gray_building_south_block_and_perkins_library(
    layer: &mut Layer,
    drawing: &DrawingContext,
    sources: &mut ShadowAndLightSources,
) {
    const EAVE_METRES: f32 = 11.0;
    const RIDGE_METRES: f32 = 16.6;
    let block = Footprint { west_metres: 45.0, east_metres: 65.5, south_metres: -35.2, north_metres: -25.7 };
    draw_slate_roof_with_ridge_running_east(layer, drawing, sources, block, EAVE_METRES, RIDGE_METRES);
    draw_chimney(layer, drawing, 52.0, -30.4, RIDGE_METRES - 0.4, RIDGE_METRES + 3.2);
    draw_chimney(layer, drawing, 61.0, -30.4, RIDGE_METRES - 0.4, RIDGE_METRES + 3.2);
    let perkins_library_corner = FlatBlock {
        outline: &[(97.9, -21.1), (136.7, -21.0), (136.7, -52.0), (89.0, -52.0), (89.0, -29.7), (97.9, -29.7)],
        roof_metres: 16.0,
        roof: FlatRoof::GreyMembrane,
        walling: Walling::DukeStoneWithWindowRows,
        storeys: 4,
        window_seed: 0xC9,
    };
    draw_flat_roofed_block(layer, drawing, sources, perkins_library_corner);
}
