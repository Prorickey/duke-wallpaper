use crate::chapel::arch::*;
use crate::chapel::art::{Layer, FLOODLIT_TOWER_STONE, SOFTLY_FLOODLIT_STONE};
use crate::chapel::palette::{HALF_LIT, SHADED, SUNLIT};
use crate::chapel::proj::*;
use crate::chapel::tex::{
    DrawingContext, GlassPixel, Lamp, ShadowAndLightSources, BELFRY_LOUVRES, OPEN_DOORWAY, STAINED_GLASS,
};
use crate::{blend_colours, hash_position};

const NAVE_BUTTRESS_PIERS_METRES_NORTH: [f32; 5] = [23.1, 29.1, 35.2, 41.2, 47.2];
const TOWER_BATTLEMENTS_TOP_METRES: f32 = 54.6;
const TOWER_HEIGHT_METRES: f32 = 64.0;
const TOWER_BACK_WALL_METRES_NORTH: f32 = 11.6;
const TURRET_LANTERN_PIXELS: i32 = 5;
const TRANSEPT_HALF_SPAN_METRES: f32 = 18.0;
const NAVE_HALF_WIDTH_METRES: f32 = 6.6;
const AISLE_OUTER_WALL_METRES_FROM_AXIS: f32 = 10.0;
const HIGH_ROOF_EAVE_METRES: f32 = 24.5;
const HIGH_ROOF_RIDGE_METRES: f32 = 26.6;
const AISLE_ROOF_TOP_METRES: f32 = 14.6;
const AISLE_EAVE_METRES: f32 = 12.2;
const TOWER_SHAFT_HALF_WIDTH_PIXELS: i32 = 18;
const CORNER_TURRET_OFFSET_PIXELS: i32 = 17;
const LEAD_SEAM_SPACING_PIXELS: i32 = 3;

#[derive(Clone, Copy)]
pub enum Part {
    MemorialChapel,
    Chancel,
    Transepts,
    Nave,
    StairTurrets,
    Arcades,
    Tower,
    PierPair(f32),
}

pub fn parts() -> Vec<(f32, Part)> {
    let mut parts = vec![
        (67.3, Part::MemorialChapel),
        (66.0, Part::Chancel),
        (53.0, Part::Transepts),
        (52.5, Part::Nave),
        (14.0, Part::StairTurrets),
        (5.6, Part::Arcades),
        (0.0, Part::Tower),
    ];
    for metres_north in NAVE_BUTTRESS_PIERS_METRES_NORTH {
        parts.push((metres_north, Part::PierPair(metres_north)));
    }
    parts
}

pub fn draw_part(part: Part, layer: &mut Layer, drawing: &DrawingContext, sources: &mut ShadowAndLightSources) {
    match part {
        Part::MemorialChapel => draw_memorial_chapel(layer, drawing),
        Part::Chancel => draw_chancel(layer, drawing, sources),
        Part::Transepts => draw_transepts(layer, drawing, sources),
        Part::Nave => draw_nave(layer, drawing, sources),
        Part::StairTurrets => draw_stair_turrets(layer, drawing, sources),
        Part::Arcades => draw_arcades(layer, drawing, sources),
        Part::Tower => draw_tower(layer, drawing, sources),
        Part::PierPair(metres_north) => draw_pier_pair(layer, drawing, metres_north),
    }
}

fn corner_turret_columns(projection: Projection, rear_pair: bool) -> (i32, i32) {
    let axis_column = projection.pixel_x(0.0, 0.0);
    let offset = if rear_pair { CORNER_TURRET_OFFSET_PIXELS - 1 } else { CORNER_TURRET_OFFSET_PIXELS };
    (axis_column - offset, axis_column + offset)
}

pub fn strike_points(projection: Projection) -> Vec<(i32, i32)> {
    let (front_left, front_right) = corner_turret_columns(projection, false);
    let (rear_left, rear_right) = corner_turret_columns(projection, true);
    let front_tip_row = projection.pixel_y(0.0, TOWER_HEIGHT_METRES);
    let rear_tip_row = projection.pixel_y(TOWER_BACK_WALL_METRES_NORTH, TOWER_HEIGHT_METRES);
    vec![
        (front_left, front_tip_row),
        (front_right, front_tip_row),
        (rear_left, rear_tip_row),
        (rear_right, rear_tip_row),
    ]
}

fn mirrored_column(projection: Projection, mirrored: bool, column: i32) -> i32 {
    if mirrored {
        2 * projection.pixel_x(0.0, 0.0) - column
    } else {
        column
    }
}

fn draw_chancel(layer: &mut Layer, drawing: &DrawingContext, sources: &mut ShadowAndLightSources) {
    const SOUTH_END_METRES_NORTH: f32 = 66.0;
    const EAST_END_METRES_NORTH: f32 = 86.0;
    let limestone = drawing.palette.limestone;
    let drawing = &drawing.for_distant_stonework();
    let point = |east: f32, north: f32, up: f32| drawing.projection.screen_point(east, north, up);
    layer.pen_flags = 0;
    for side in [-1.0f32, 1.0] {
        let slope_light = light_level_of_roof_slope(drawing, side);
        let eave_start = point(side * NAVE_HALF_WIDTH_METRES, SOUTH_END_METRES_NORTH, HIGH_ROOF_EAVE_METRES);
        let eave_end = point(side * NAVE_HALF_WIDTH_METRES, EAST_END_METRES_NORTH, HIGH_ROOF_EAVE_METRES);
        let roof = [
            point(0.0, SOUTH_END_METRES_NORTH, HIGH_ROOF_RIDGE_METRES),
            eave_start,
            eave_end,
            point(0.0, EAST_END_METRES_NORTH, HIGH_ROOF_RIDGE_METRES),
        ];
        fill_lead_roof(layer, drawing, &roof, LEAD_SEAM_SPACING_PIXELS, slope_light, false);
        let parapet_colour = if slope_light == SHADED { limestone[2] } else { limestone[0] };
        layer.line(eave_start.0, eave_start.1, eave_end.0, eave_end.1, parapet_colour);
        for buttress_metres_north in [71.5f32, 77.0, 82.5] {
            let (column, row) = point(side * NAVE_HALF_WIDTH_METRES, buttress_metres_north, HIGH_ROOF_EAVE_METRES);
            draw_pinnacle(layer, drawing, column, row, 5, 1);
        }
    }
    let ridge_start = point(0.0, SOUTH_END_METRES_NORTH, HIGH_ROOF_RIDGE_METRES);
    let ridge_end = point(0.0, EAST_END_METRES_NORTH, HIGH_ROOF_RIDGE_METRES);
    layer.line(ridge_start.0, ridge_start.1, ridge_end.0, ridge_end.1, drawing.palette.lead[0]);
    let gable_apex = point(0.0, EAST_END_METRES_NORTH, HIGH_ROOF_RIDGE_METRES + 0.9);
    let gable_west_foot = point(-NAVE_HALF_WIDTH_METRES, EAST_END_METRES_NORTH, HIGH_ROOF_EAVE_METRES);
    let gable_east_foot = point(NAVE_HALF_WIDTH_METRES, EAST_END_METRES_NORTH, HIGH_ROOF_EAVE_METRES);
    layer.line(gable_west_foot.0, gable_west_foot.1, gable_apex.0, gable_apex.1, limestone[0]);
    layer.line(gable_apex.0, gable_apex.1, gable_east_foot.0, gable_east_foot.1, limestone[1]);
    for side in [-1.0f32, 1.0] {
        let (column, row) = point(side * NAVE_HALF_WIDTH_METRES, EAST_END_METRES_NORTH, HIGH_ROOF_EAVE_METRES);
        draw_pinnacle(layer, drawing, column, row, 9, 3);
    }
    layer.vertical_line(gable_apex.0, gable_apex.1 - 4, gable_apex.1, limestone[0]);
    layer.horizontal_line(gable_apex.0 - 1, gable_apex.0 + 1, gable_apex.1 - 3, limestone[0]);
    sources.add_block_caster(drawing.projection, -7.6, 7.6, 66.0, 86.0, 26.0);
    sources.add_block_caster(drawing.projection, -18.0, -7.7, 67.3, 82.0, 12.0);
}

fn draw_memorial_chapel(layer: &mut Layer, drawing: &DrawingContext) {
    const EAVE_METRES: f32 = 9.0;
    const RIDGE_METRES: f32 = 12.6;
    const RIDGE_METRES_EAST: f32 = -12.8;
    let limestone = drawing.palette.limestone;
    layer.pen_flags = 0;
    let drawing = &drawing.for_distant_stonework();
    let point = |east: f32, north: f32, up: f32| drawing.projection.screen_point(east, north, up);
    let ridge_south = point(RIDGE_METRES_EAST, 67.3, RIDGE_METRES);
    let ridge_north = point(RIDGE_METRES_EAST, 80.4, RIDGE_METRES);
    let apse_corners = [
        point(-17.0, 81.0, EAVE_METRES),
        point(-15.0, 83.4, EAVE_METRES),
        point(-12.0, 84.5, EAVE_METRES),
        point(-8.9, 84.0, EAVE_METRES),
    ];
    let west_slope =
        [point(-18.0, 67.3, EAVE_METRES), ridge_south, ridge_north, apse_corners[0], point(-18.0, 79.1, EAVE_METRES)];
    let east_slope =
        [ridge_south, point(-7.7, 67.3, EAVE_METRES), point(-7.7, 82.6, EAVE_METRES), apse_corners[3], ridge_north];
    let apse = [ridge_north, apse_corners[0], apse_corners[1], apse_corners[2], apse_corners[3]];
    fill_lead_roof(layer, drawing, &apse, LEAD_SEAM_SPACING_PIXELS, SHADED, true);
    fill_lead_roof(
        layer,
        drawing,
        &west_slope,
        LEAD_SEAM_SPACING_PIXELS,
        light_level_of_roof_slope(drawing, -1.0),
        false,
    );
    fill_lead_roof(
        layer,
        drawing,
        &east_slope,
        LEAD_SEAM_SPACING_PIXELS,
        light_level_of_roof_slope(drawing, 1.0),
        false,
    );
    layer.line(ridge_south.0, ridge_south.1, ridge_north.0, ridge_north.1, drawing.palette.lead[0]);
    for corner in apse_corners {
        layer.line(ridge_north.0, ridge_north.1, corner.0, corner.1, drawing.palette.lead[4]);
    }
    let eave_south = point(-18.0, 67.3, EAVE_METRES);
    let eave_north = point(-18.0, 79.1, EAVE_METRES);
    layer.line(eave_south.0, eave_south.1, eave_north.0, eave_north.1, limestone[1]);
    for buttress_metres_north in [70.4f32, 73.7, 76.9] {
        let (column, row) = point(-18.0, buttress_metres_north, EAVE_METRES);
        draw_pinnacle(layer, drawing, column, row, 4, 1);
    }
}

fn draw_transepts(layer: &mut Layer, drawing: &DrawingContext, sources: &mut ShadowAndLightSources) {
    const FRONT_WALL_METRES_NORTH: f32 = 53.0;
    const RIDGE_METRES_NORTH: f32 = 59.5;
    const BACK_WALL_METRES_NORTH: f32 = 66.0;
    const WINDOW_SILL_METRES: f32 = 8.6;
    const WINDOW_APEX_METRES: f32 = 21.4;
    const ROUNDEL_METRES: f32 = 22.9;
    const SIDE_DOOR_HEAD_METRES: f32 = 4.6;
    const SIDE_DOOR_FOOT_METRES: f32 = 0.9;
    const SIDE_DOOR_LAMP_METRES: f32 = 3.4;
    const FOOT_OF_THE_STEPS_METRES_NORTH: f32 = 51.2;
    let limestone = drawing.palette.limestone;
    let drawing = &drawing.for_distant_stonework();
    let point = |east: f32, north: f32, up: f32| drawing.projection.screen_point(east, north, up);
    let arm_end = TRANSEPT_HALF_SPAN_METRES + 0.5;
    layer.pen_flags = 0;
    let far_slope = [
        point(-arm_end, RIDGE_METRES_NORTH, HIGH_ROOF_RIDGE_METRES),
        point(arm_end, RIDGE_METRES_NORTH, HIGH_ROOF_RIDGE_METRES),
        point(arm_end, BACK_WALL_METRES_NORTH, HIGH_ROOF_EAVE_METRES),
        point(-arm_end, BACK_WALL_METRES_NORTH, HIGH_ROOF_EAVE_METRES),
    ];
    fill_lead_roof(layer, drawing, &far_slope, LEAD_SEAM_SPACING_PIXELS, SHADED, true);
    let near_slope = [
        point(-arm_end, FRONT_WALL_METRES_NORTH, HIGH_ROOF_EAVE_METRES),
        point(arm_end, FRONT_WALL_METRES_NORTH, HIGH_ROOF_EAVE_METRES),
        point(arm_end, RIDGE_METRES_NORTH, HIGH_ROOF_RIDGE_METRES),
        point(-arm_end, RIDGE_METRES_NORTH, HIGH_ROOF_RIDGE_METRES),
    ];
    fill_lead_roof(layer, drawing, &near_slope, LEAD_SEAM_SPACING_PIXELS, SUNLIT, true);
    for side in [-1.0f32, 1.0] {
        let front_valley_foot = point(side * NAVE_HALF_WIDTH_METRES, FRONT_WALL_METRES_NORTH, HIGH_ROOF_EAVE_METRES);
        let back_valley_foot = point(side * NAVE_HALF_WIDTH_METRES, BACK_WALL_METRES_NORTH, HIGH_ROOF_EAVE_METRES);
        let crossing_centre = point(0.0, RIDGE_METRES_NORTH, HIGH_ROOF_RIDGE_METRES);
        let nave_roof_end =
            [front_valley_foot, point(0.0, FRONT_WALL_METRES_NORTH, HIGH_ROOF_RIDGE_METRES), crossing_centre];
        fill_lead_roof(
            layer,
            drawing,
            &nave_roof_end,
            LEAD_SEAM_SPACING_PIXELS,
            light_level_of_roof_slope(drawing, side),
            false,
        );
        let chancel_roof_end =
            [back_valley_foot, point(0.0, BACK_WALL_METRES_NORTH, HIGH_ROOF_RIDGE_METRES), crossing_centre];
        fill_lead_roof(
            layer,
            drawing,
            &chancel_roof_end,
            LEAD_SEAM_SPACING_PIXELS,
            light_level_of_roof_slope(drawing, side),
            false,
        );
        layer.line(
            front_valley_foot.0,
            front_valley_foot.1,
            crossing_centre.0,
            crossing_centre.1,
            drawing.palette.lead[4],
        );
        layer.line(
            back_valley_foot.0,
            back_valley_foot.1,
            crossing_centre.0,
            crossing_centre.1,
            drawing.palette.lead[4],
        );
    }
    let ridge_west = point(-arm_end, RIDGE_METRES_NORTH, HIGH_ROOF_RIDGE_METRES);
    let ridge_east = point(arm_end, RIDGE_METRES_NORTH, HIGH_ROOF_RIDGE_METRES);
    layer.horizontal_line(ridge_west.0, ridge_east.0, ridge_west.1, drawing.palette.lead[0]);
    let crossing_ridge_front = point(0.0, FRONT_WALL_METRES_NORTH, HIGH_ROOF_RIDGE_METRES);
    let crossing_ridge_back = point(0.0, BACK_WALL_METRES_NORTH, HIGH_ROOF_RIDGE_METRES);
    layer.line(
        crossing_ridge_front.0,
        crossing_ridge_front.1,
        crossing_ridge_back.0,
        crossing_ridge_back.1,
        drawing.palette.lead[0],
    );
    for (from_east, to_east) in [(-arm_end, -NAVE_HALF_WIDTH_METRES), (NAVE_HALF_WIDTH_METRES, arm_end)] {
        let from = point(from_east, BACK_WALL_METRES_NORTH, HIGH_ROOF_EAVE_METRES);
        let to = point(to_east, BACK_WALL_METRES_NORTH, HIGH_ROOF_EAVE_METRES);
        layer.line(from.0, from.1, to.0, to.1, limestone[1]);
    }
    for side in [-1.0f32, 1.0] {
        let gable_apex = point(side * arm_end, RIDGE_METRES_NORTH, HIGH_ROOF_RIDGE_METRES + 0.9);
        let gable_front_foot = point(side * arm_end, FRONT_WALL_METRES_NORTH, HIGH_ROOF_EAVE_METRES);
        let gable_back_foot = point(side * arm_end, BACK_WALL_METRES_NORTH, HIGH_ROOF_EAVE_METRES);
        let is_lit = light_level_of_roof_slope(drawing, side) != SHADED;
        for thickness in 0..2 {
            let outward = if side < 0.0 { -thickness } else { thickness };
            let front_colour = if is_lit { limestone[0] } else { limestone[2] };
            let back_colour = if is_lit { limestone[1] } else { limestone[3] };
            layer.line(
                gable_front_foot.0 + outward,
                gable_front_foot.1,
                gable_apex.0 + outward,
                gable_apex.1,
                front_colour,
            );
            layer.line(
                gable_apex.0 + outward,
                gable_apex.1,
                gable_back_foot.0 + outward,
                gable_back_foot.1,
                back_colour,
            );
        }
        layer.vertical_line(gable_apex.0, gable_apex.1 - 4, gable_apex.1, limestone[0]);
        layer.horizontal_line(gable_apex.0 - 1, gable_apex.0 + 1, gable_apex.1 - 3, limestone[0]);
        draw_pinnacle(layer, drawing, gable_back_foot.0, gable_back_foot.1, 8, 3);
    }

    let wall_north = FRONT_WALL_METRES_NORTH;
    for mirrored in [false, true] {
        layer.pen_flags = SOFTLY_FLOODLIT_STONE;
        let mirror = |column: i32| mirrored_column(drawing.projection, mirrored, column);
        let foot_row = drawing.projection.pixel_y(wall_north, 0.0);
        let eave_row = drawing.projection.pixel_y(wall_north, HIGH_ROOF_EAVE_METRES);
        let aisle_column = drawing.projection.pixel_x(AISLE_OUTER_WALL_METRES_FROM_AXIS, wall_north);
        let arm_end_column = drawing.projection.pixel_x(TRANSEPT_HALF_SPAN_METRES, wall_north);
        let nave_wall_column = drawing.projection.pixel_x(NAVE_HALF_WIDTH_METRES, wall_north) + 2;
        layer.fill_rect_with(mirror(aisle_column), eave_row + 1, mirror(arm_end_column), foot_row, |column, row| {
            drawing.stone(HALF_LIT, column, row)
        });
        let aisle_roof_row = drawing.projection.pixel_y(wall_north, AISLE_ROOF_TOP_METRES);
        layer.fill_rect_with(
            mirror(nave_wall_column),
            eave_row + 1,
            mirror(aisle_column),
            aisle_roof_row,
            |column, row| drawing.stone(SHADED, column, row),
        );
        layer.horizontal_line(mirror(aisle_column), mirror(arm_end_column), foot_row, limestone[3]);
        layer.horizontal_line(mirror(aisle_column), mirror(arm_end_column), foot_row - 2, limestone[3]);
        layer.horizontal_line(mirror(nave_wall_column), mirror(arm_end_column + 2), eave_row, limestone[0]);
        layer.horizontal_line(mirror(nave_wall_column), mirror(arm_end_column + 2), eave_row + 1, limestone[2]);
        drawing.snow_along_ledge(layer, mirror(nave_wall_column), mirror(arm_end_column + 2), eave_row);
        let sill_row = drawing.projection.pixel_y(wall_north, WINDOW_SILL_METRES);
        layer.horizontal_line(mirror(aisle_column), mirror(arm_end_column), sill_row + 2, limestone[2]);
        let window_column = (aisle_column + arm_end_column) / 2 + 1;
        draw_gothic_window(
            layer,
            drawing,
            sources,
            &GothicWindow {
                centre_column: mirror(window_column),
                sill_row,
                apex_row: drawing.projection.pixel_y(wall_north, WINDOW_APEX_METRES),
                glass_width: 9,
                lights: 2,
                glass_kind: STAINED_GLASS,
                window_number: if mirrored { 11 } else { 10 },
                transom_rows_above_sill: 0,
            },
        );
        let roundel_row = drawing.projection.pixel_y(wall_north, ROUNDEL_METRES);
        for roundel_column in [window_column - 6, window_column + 6] {
            layer.put(mirror(roundel_column), roundel_row, 0x2a2f3a);
            layer.put(mirror(roundel_column), roundel_row - 1, limestone[1]);
        }
        for (first_column, last_column) in [(aisle_column, aisle_column + 2), (arm_end_column - 1, arm_end_column + 2)]
        {
            let (start, end) = (mirror(first_column), mirror(last_column));
            let buttress_top_row = drawing.projection.pixel_y(wall_north, 20.5);
            fill_stone_lit_from_the_side(
                layer,
                drawing,
                start.min(end),
                buttress_top_row,
                start.max(end),
                foot_row,
                SUNLIT,
            );
            for set_off_metres in [6.0f32, 13.0, 20.5] {
                let set_off_row = drawing.projection.pixel_y(wall_north, set_off_metres);
                layer.horizontal_line(start, end, set_off_row, limestone[1]);
                layer.horizontal_line(start, end, set_off_row + 1, limestone[3]);
            }
            layer.pen_flags = 0;
            draw_pinnacle(layer, drawing, mirror((first_column + last_column) / 2), eave_row - 1, 9, 3);
            layer.pen_flags = SOFTLY_FLOODLIT_STONE;
        }
        let door_head_row = drawing.projection.pixel_y(wall_north, SIDE_DOOR_HEAD_METRES);
        let door_foot_row = drawing.projection.pixel_y(wall_north, SIDE_DOOR_FOOT_METRES);
        for row in door_head_row..=door_foot_row {
            let half_width = (1 + (row - door_head_row)).min(3);
            for column in window_column - half_width..=window_column + half_width {
                let is_arch = (column - window_column).abs() == half_width || row == door_head_row;
                let colour = if is_arch {
                    limestone[1]
                } else if column == window_column {
                    drawing.palette.wood[3]
                } else if (column - window_column).abs() == 1 {
                    drawing.palette.wood[0]
                } else {
                    drawing.palette.wood[1]
                };
                layer.put(mirror(column), row, colour);
            }
        }
        layer.pen_flags = 0;
        let steps_foot_row = drawing.projection.pixel_y(FOOT_OF_THE_STEPS_METRES_NORTH, 0.0);
        for (step, row) in (door_foot_row + 1..=steps_foot_row).enumerate() {
            let colour = if step % 2 == 0 { limestone[0] } else { limestone[3] };
            layer.horizontal_line(mirror(window_column - 3), mirror(window_column + 3), row, colour);
        }
        let lamp_column = mirror(window_column - 5);
        let lamp_row = drawing.projection.pixel_y(wall_north, SIDE_DOOR_LAMP_METRES);
        layer.put(lamp_column, lamp_row - 1, drawing.palette.iron[0]);
        layer.put(lamp_column, lamp_row, 0xd8c890);
        sources.lamps.push(Lamp {
            column: lamp_column,
            row: lamp_row,
            ground_row: steps_foot_row,
            depth_tag: depth_tag(52.5),
            head_depth_tag: layer.pen_depth,
            kind: 1,
        });
    }
    layer.pen_flags = 0;
    sources.add_block_caster(
        drawing.projection,
        -TRANSEPT_HALF_SPAN_METRES - 1.2,
        TRANSEPT_HALF_SPAN_METRES + 1.2,
        53.0,
        66.0,
        25.0,
    );
}

fn draw_nave(layer: &mut Layer, drawing: &DrawingContext, sources: &mut ShadowAndLightSources) {
    const WEST_END_METRES_NORTH: f32 = TOWER_BACK_WALL_METRES_NORTH;
    const CROSSING_METRES_NORTH: f32 = 53.0;
    const BAY_METRES: f32 = 6.1;
    const FIRST_BAY_METRES_NORTH: f32 = 14.2;
    const SLIVER_STEP_METRES: f32 = 0.4;
    let limestone = drawing.palette.limestone;
    layer.pen_depth = depth_tag(WEST_END_METRES_NORTH);
    let drawing = &drawing.for_distant_stonework();
    let point = |east: f32, north: f32, up: f32| drawing.projection.screen_point(east, north, up);
    for side in [-1.0f32, 1.0] {
        let outward = side as i32;
        layer.pen_flags = 0;
        let slope_light = light_level_of_roof_slope(drawing, side);
        let high_roof = [
            point(0.0, WEST_END_METRES_NORTH, HIGH_ROOF_RIDGE_METRES),
            point(side * NAVE_HALF_WIDTH_METRES, WEST_END_METRES_NORTH, HIGH_ROOF_EAVE_METRES),
            point(side * NAVE_HALF_WIDTH_METRES, CROSSING_METRES_NORTH, HIGH_ROOF_EAVE_METRES),
            point(0.0, CROSSING_METRES_NORTH, HIGH_ROOF_RIDGE_METRES),
        ];
        fill_lead_roof(layer, drawing, &high_roof, LEAD_SEAM_SPACING_PIXELS, slope_light, false);
        let aisle_roof = [
            point(side * NAVE_HALF_WIDTH_METRES, WEST_END_METRES_NORTH, AISLE_ROOF_TOP_METRES),
            point(side * AISLE_OUTER_WALL_METRES_FROM_AXIS, WEST_END_METRES_NORTH, AISLE_EAVE_METRES),
            point(side * AISLE_OUTER_WALL_METRES_FROM_AXIS, CROSSING_METRES_NORTH, AISLE_EAVE_METRES),
            point(side * NAVE_HALF_WIDTH_METRES, CROSSING_METRES_NORTH, AISLE_ROOF_TOP_METRES),
        ];
        fill_lead_roof(layer, drawing, &aisle_roof, LEAD_SEAM_SPACING_PIXELS, slope_light, false);
        let wall_light = if slope_light == SHADED { SHADED } else { HALF_LIT };
        let coping_colour = limestone[if slope_light == SHADED { 2 } else { 0 }];
        let clerestory_and_aisle_walls = [
            (NAVE_HALF_WIDTH_METRES, AISLE_ROOF_TOP_METRES, HIGH_ROOF_EAVE_METRES),
            (AISLE_OUTER_WALL_METRES_FROM_AXIS, 0.0, AISLE_EAVE_METRES),
        ];
        for (wall_metres_from_axis, wall_foot_metres, wall_top_metres) in clerestory_and_aisle_walls {
            let mut metres_north = CROSSING_METRES_NORTH;
            while metres_north >= WEST_END_METRES_NORTH {
                let column = drawing.projection.pixel_x(side * wall_metres_from_axis, metres_north) + outward;
                let top_row = drawing.projection.pixel_y(metres_north, wall_top_metres);
                let foot_row = drawing.projection.pixel_y(metres_north, wall_foot_metres);
                let position_in_bay = ((metres_north - FIRST_BAY_METRES_NORTH) / BAY_METRES).rem_euclid(1.0);
                for row in top_row..=foot_row {
                    for thickness in 0..2 {
                        let is_window = position_in_bay > 0.25
                            && position_in_bay < 0.75
                            && row > top_row + 3
                            && row < top_row + (foot_row - top_row) * 2 / 3;
                        let colour = if row == top_row {
                            coping_colour
                        } else if is_window {
                            drawing.palette.glass[0]
                        } else {
                            drawing.stone(wall_light, column + thickness * outward, row)
                        };
                        layer.put(column + thickness * outward, row, colour);
                    }
                }
                metres_north -= SLIVER_STEP_METRES;
            }
        }
        for metres_north in NAVE_BUTTRESS_PIERS_METRES_NORTH {
            let (column, row) = point(side * NAVE_HALF_WIDTH_METRES, metres_north, HIGH_ROOF_EAVE_METRES);
            draw_pinnacle(layer, drawing, column + outward, row, 5, 1);
        }
        layer.pen_flags = SOFTLY_FLOODLIT_STONE;
        let aisle_west_wall = [
            point(side * NAVE_HALF_WIDTH_METRES, WEST_END_METRES_NORTH, 0.0),
            point(side * NAVE_HALF_WIDTH_METRES, WEST_END_METRES_NORTH, AISLE_ROOF_TOP_METRES),
            point(side * 11.0, WEST_END_METRES_NORTH, 12.0),
            point(side * 11.0, WEST_END_METRES_NORTH, 0.0),
        ];
        layer.fill_polygon_with(&aisle_west_wall, |column, row| Some(drawing.stone(HALF_LIT, column, row)));
        layer.line(
            aisle_west_wall[1].0,
            aisle_west_wall[1].1,
            aisle_west_wall[2].0,
            aisle_west_wall[2].1,
            limestone[1],
        );
        let (window_column, window_row) = point(side * 9.3, WEST_END_METRES_NORTH, 7.6);
        layer.put(window_column, window_row - 3, limestone[1]);
        layer.vertical_line(window_column - 1, window_row - 2, window_row + 3, limestone[2]);
        layer.vertical_line(window_column, window_row - 2, window_row + 3, drawing.palette.glass[1]);
        layer.vertical_line(window_column + 1, window_row - 2, window_row + 3, limestone[2]);
        layer.pen_flags = 0;
    }
    let ridge_west = point(0.0, WEST_END_METRES_NORTH, HIGH_ROOF_RIDGE_METRES);
    let ridge_east = point(0.0, CROSSING_METRES_NORTH, HIGH_ROOF_RIDGE_METRES);
    layer.line(ridge_west.0, ridge_west.1, ridge_east.0, ridge_east.1, drawing.palette.lead[0]);
    sources.add_block_caster(drawing.projection, -10.6, 10.6, 11.6, 53.0, 25.0);
}

fn draw_pier_pair(layer: &mut Layer, drawing: &DrawingContext, metres_north: f32) {
    const FLYING_WEATHERING_TOP_METRES: f32 = 17.4;
    const FLYING_WEATHERING_FOOT_METRES: f32 = 13.4;
    const PIER_TOP_METRES: f32 = 12.4;
    const SET_OFF_METRES: f32 = 5.6;
    const PIER_OUTER_FACE_METRES_FROM_AXIS: f32 = 11.8;
    let limestone = drawing.palette.limestone;
    let drawing = &drawing.for_distant_stonework();
    for mirrored in [false, true] {
        layer.pen_flags = SOFTLY_FLOODLIT_STONE;
        let mirror = |column: i32| mirrored_column(drawing.projection, mirrored, column);
        let clerestory_column = drawing.projection.pixel_x(NAVE_HALF_WIDTH_METRES, metres_north) + 2;
        let aisle_column = drawing.projection.pixel_x(AISLE_OUTER_WALL_METRES_FROM_AXIS, metres_north);
        let outer_face_column = drawing.projection.pixel_x(PIER_OUTER_FACE_METRES_FROM_AXIS, metres_north);
        let weathering_top =
            (mirror(clerestory_column), drawing.projection.pixel_y(metres_north, FLYING_WEATHERING_TOP_METRES));
        let weathering_foot =
            (mirror(aisle_column), drawing.projection.pixel_y(metres_north, FLYING_WEATHERING_FOOT_METRES));
        layer.line(weathering_top.0, weathering_top.1, weathering_foot.0, weathering_foot.1, limestone[1]);
        layer.line(weathering_top.0, weathering_top.1 + 1, weathering_foot.0, weathering_foot.1 + 1, limestone[3]);
        let foot_row = drawing.projection.pixel_y(metres_north, 0.0);
        let top_row = drawing.projection.pixel_y(metres_north, PIER_TOP_METRES);
        let (inner, outer) = (mirror(aisle_column), mirror(outer_face_column));
        fill_stone_lit_from_the_side(layer, drawing, inner.min(outer), top_row, inner.max(outer), foot_row, SUNLIT);
        let set_off_row = drawing.projection.pixel_y(metres_north, SET_OFF_METRES);
        layer.horizontal_line(inner, outer, set_off_row, limestone[1]);
        layer.horizontal_line(inner, outer, set_off_row + 1, limestone[3]);
        layer.horizontal_line(inner, outer, foot_row, limestone[3]);
        layer.horizontal_line(inner, outer, top_row, limestone[0]);
        layer.horizontal_line(inner, outer, top_row + 1, limestone[2]);
        let centre_column = mirror((aisle_column + outer_face_column) / 2);
        draw_gablet(layer, drawing, centre_column, top_row - 1, 2);
        layer.pen_flags = 0;
        draw_pinnacle(layer, drawing, centre_column, top_row - 3, 7, 1);
        drawing.snow_along_ledge(layer, inner.min(outer), inner.max(outer), top_row);
    }
}

fn draw_stair_turrets(layer: &mut Layer, drawing: &DrawingContext, sources: &mut ShadowAndLightSources) {
    const TURRET_METRES_NORTH: f32 = 14.0;
    const TURRET_METRES_FROM_AXIS: f32 = 12.0;
    const SHAFT_TOP_METRES: f32 = 11.6;
    const STRING_COURSE_METRES: f32 = 5.6;
    const CONE_TIP_METRES: f32 = 16.6;
    const CONE_HALF_WIDTH_PIXELS: f32 = 6.0;
    const SLIT_WINDOWS_UP_THE_STAIR: [(i32, f32); 4] = [(0, 3.2), (4, 5.0), (-4, 7.2), (0, 9.2)];
    let limestone = drawing.palette.limestone;
    let metres_north = TURRET_METRES_NORTH;
    for mirrored in [false, true] {
        layer.pen_flags = SOFTLY_FLOODLIT_STONE;
        let centre = mirrored_column(
            drawing.projection,
            mirrored,
            drawing.projection.pixel_x(TURRET_METRES_FROM_AXIS, metres_north),
        );
        let foot_row = drawing.projection.pixel_y(metres_north, 0.0);
        let top_row = drawing.projection.pixel_y(metres_north, SHAFT_TOP_METRES);
        let octagon_facets =
            [(centre - 5, centre - 3, -1.0f32), (centre - 2, centre + 2, 0.0), (centre + 3, centre + 5, 1.0)];
        for (first_column, last_column, facing) in octagon_facets {
            let light_level = if facing == 0.0 {
                SUNLIT
            } else if drawing.light_level_facing(facing) == SUNLIT {
                HALF_LIT
            } else {
                SHADED
            };
            layer.fill_rect_with(first_column, top_row, last_column, foot_row, |column, row| {
                drawing.stone(light_level, column, row)
            });
        }
        for row in (top_row + 2..foot_row).step_by(3) {
            layer.put(centre - 3, row, limestone[2]);
            layer.put(centre + 3, row, limestone[3]);
        }
        layer.horizontal_line(centre - 5, centre + 5, foot_row, limestone[3]);
        layer.horizontal_line(
            centre - 5,
            centre + 5,
            drawing.projection.pixel_y(metres_north, STRING_COURSE_METRES),
            limestone[2],
        );
        layer.horizontal_line(centre - 6, centre + 6, top_row, limestone[0]);
        layer.horizontal_line(centre - 5, centre + 5, top_row + 1, limestone[3]);
        for (columns_from_centre, window_metres_up) in SLIT_WINDOWS_UP_THE_STAIR {
            let window_column = centre + columns_from_centre;
            let window_row = drawing.projection.pixel_y(metres_north, window_metres_up);
            layer.put(window_column, window_row - 2, limestone[1]);
            layer.vertical_line(window_column, window_row - 1, window_row + 1, 0x232831);
        }
        layer.pen_flags = 0;
        let cone_tip_row = drawing.projection.pixel_y(metres_north, CONE_TIP_METRES);
        let cone_rows = (top_row - 1 - cone_tip_row).max(1);
        for rows_up in 0..=cone_rows {
            let row = top_row - 1 - rows_up;
            let half_width =
                (CONE_HALF_WIDTH_PIXELS * (1.0 - rows_up as f32 / (cone_rows as f32 + 0.5))).round() as i32;
            for column in centre - half_width..=centre + half_width {
                let across_the_cone = (column - centre) as f32 / half_width.max(1) as f32;
                let light_level = if across_the_cone < -0.36 {
                    drawing.light_level_facing(-1.0)
                } else if across_the_cone > 0.36 {
                    drawing.light_level_facing(1.0)
                } else {
                    HALF_LIT
                };
                let lead = match light_level {
                    SUNLIT => drawing.palette.lead[0],
                    HALF_LIT => drawing.palette.lead[1],
                    _ => drawing.palette.lead[3],
                };
                let colour = drawing.snow_colour(column, row, 1.9, -0.1, 0).unwrap_or(lead);
                layer.put(column, row, colour);
            }
        }
        layer.put(centre, cone_tip_row - 1, limestone[0]);
        layer.put(centre, cone_tip_row - 2, limestone[1]);
        let turret_metres_east = if mirrored { -12.0 } else { 12.0 };
        sources.add_block_caster(
            drawing.projection,
            turret_metres_east - 1.4,
            turret_metres_east + 1.4,
            14.0,
            17.4,
            13.5,
        );
    }
}

fn draw_arcades(layer: &mut Layer, drawing: &DrawingContext, sources: &mut ShadowAndLightSources) {
    const FRONT_METRES_NORTH: f32 = 5.6;
    const BACK_METRES_NORTH: f32 = 9.4;
    const PARAPET_METRES: f32 = 5.7;
    const ROOF_BACK_METRES: f32 = 6.3;
    const CLEAR_OF_THE_TOWER_BUTTRESS_PIXELS: i32 = 28;
    const WEST_AND_EAST_ARCADES: [(f32, f32, i32); 2] = [(-8.2, -26.9, 4), (8.2, 30.0, 5)];
    let limestone = drawing.palette.limestone;
    let axis_column = drawing.projection.pixel_x(0.0, 0.0);
    for (tower_end_metres_east, far_end_metres_east, bays) in WEST_AND_EAST_ARCADES {
        let inner_column = if tower_end_metres_east < 0.0 {
            axis_column - CLEAR_OF_THE_TOWER_BUTTRESS_PIXELS
        } else {
            axis_column + CLEAR_OF_THE_TOWER_BUTTRESS_PIXELS
        };
        let outer_column = drawing.projection.pixel_x(far_end_metres_east, FRONT_METRES_NORTH);
        let left = inner_column.min(outer_column);
        let right = inner_column.max(outer_column);
        let ground_row = drawing.projection.pixel_y(FRONT_METRES_NORTH, 0.0);
        let parapet_row = drawing.projection.pixel_y(FRONT_METRES_NORTH, PARAPET_METRES);
        let roof_back_row = drawing.projection.pixel_y(BACK_METRES_NORTH, ROOF_BACK_METRES);
        layer.pen_flags = 0;
        let roof = [(left, roof_back_row), (right, roof_back_row), (right, parapet_row - 1), (left, parapet_row - 1)];
        fill_lead_roof(layer, drawing, &roof, LEAD_SEAM_SPACING_PIXELS, HALF_LIT, true);
        layer.horizontal_line(left, right, roof_back_row, limestone[1]);
        layer.horizontal_line(left, right, roof_back_row + 1, drawing.palette.lead[0]);
        layer.pen_flags = SOFTLY_FLOODLIT_STONE;
        layer.fill_rect_with(left, parapet_row, right, ground_row, |column, row| drawing.stone(SUNLIT, column, row));
        layer.horizontal_line(left, right, parapet_row, limestone[0]);
        layer.horizontal_line(left, right, parapet_row + 1, limestone[2]);
        drawing.snow_along_ledge(layer, left, right, parapet_row);
        let bay_width = (right - left + 1) as f32 / bays as f32;
        for bay in 0..bays {
            let bay_left = left + (bay as f32 * bay_width).round() as i32;
            let bay_right = left + ((bay + 1) as f32 * bay_width).round() as i32 - 1;
            let arch_centre = (bay_left + bay_right) / 2;
            let half_width = ((bay_right - bay_left - 3) / 2).clamp(3, 5);
            let apex_row = parapet_row + 3;
            let springing_row = apex_row + 3;
            for row in apex_row..=ground_row {
                let half_width_here =
                    if row >= springing_row { half_width } else { 1 + (row - apex_row) * (half_width - 1) / 3 }
                        .min(half_width);
                layer.put(arch_centre - half_width_here - 1, row, limestone[1]);
                layer.put(arch_centre + half_width_here + 1, row, limestone[2]);
                for column in arch_centre - half_width_here..=arch_centre + half_width_here {
                    let colour = if row >= ground_row - 1 {
                        blend_colours(drawing.palette.flagstone[1], 0x30343c, 0.45)
                    } else if hash_position(column, row, 0xA2C).is_multiple_of(7) {
                        0x2e323b
                    } else {
                        0x23262e
                    };
                    layer.put(column, row, colour);
                }
            }
            layer.put(arch_centre, apex_row - 1, limestone[1]);
        }
        for pier in 0..=bays {
            let pier_column = (left + (pier as f32 * bay_width).round() as i32).clamp(left, right);
            layer.pen_flags = SOFTLY_FLOODLIT_STONE;
            layer.fill_rect_with(pier_column, parapet_row + 2, pier_column, ground_row, |column, row| {
                drawing.stone(SUNLIT, column, row)
            });
            layer.put(pier_column, parapet_row + 4, limestone[1]);
            layer.put(pier_column, ground_row, limestone[3]);
            layer.pen_flags = 0;
            draw_pinnacle(layer, drawing, pier_column, parapet_row - 1, 5, 1);
        }
        layer.pen_flags = 0;
        let west_metres = tower_end_metres_east.min(far_end_metres_east);
        let east_metres = tower_end_metres_east.max(far_end_metres_east);
        sources.add_block_caster(drawing.projection, west_metres, east_metres, 5.6, 9.4, 6.2);
    }
}

fn draw_turret_shaft(
    layer: &mut Layer,
    drawing: &DrawingContext,
    centre: i32,
    top_row: i32,
    bottom_row: i32,
    duke_stone_below_row: i32,
) {
    const COURSE_ROWS: i32 = 9;
    let limestone = drawing.palette.limestone;
    let sun_side = drawing.look.sun_side;
    for row in top_row..=bottom_row {
        let row_in_course = (bottom_row - row) % COURSE_ROWS;
        for columns_from_centre in -3i32..=3 {
            let facet = columns_from_centre.signum() * (columns_from_centre.abs() >= 2) as i32;
            let mut limestone_shade: usize = match facet {
                -1 => {
                    if sun_side > 0.2 {
                        3
                    } else {
                        0
                    }
                }
                1 => {
                    if sun_side > 0.2 {
                        0
                    } else {
                        3
                    }
                }
                _ => 1,
            };
            if sun_side.abs() < 0.2 && facet == 1 {
                limestone_shade = 2;
            }
            let is_angle_shaft = columns_from_centre.abs() == 3 || columns_from_centre.abs() == 1;
            let colour = if row_in_course == COURSE_ROWS - 1 {
                limestone[0]
            } else if row_in_course == 0 {
                limestone[3]
            } else if row > duke_stone_below_row && !is_angle_shaft {
                let light_level = if facet == 0 { SUNLIT } else { drawing.light_level_facing(facet as f32) };
                drawing.stone(light_level, centre + columns_from_centre, row)
            } else if columns_from_centre == 0 && row_in_course > 2 && row_in_course < 7 && row <= duke_stone_below_row
            {
                limestone[3]
            } else {
                limestone[limestone_shade]
            };
            layer.put(centre + columns_from_centre, row, colour);
        }
    }
}

fn draw_turret_crown(layer: &mut Layer, drawing: &DrawingContext, centre: i32, top_of_shaft_row: i32, finial_row: i32) {
    let limestone = drawing.palette.limestone;
    let lit_from_the_left = drawing.look.sun_side <= 0.2;
    let lantern_top_row = top_of_shaft_row - TURRET_LANTERN_PIXELS;
    for row in lantern_top_row + 1..=top_of_shaft_row {
        for columns_from_centre in -3i32..=3 {
            let is_opening = (columns_from_centre == -2 || columns_from_centre == 0 || columns_from_centre == 2)
                && row > lantern_top_row + 1
                && row < top_of_shaft_row;
            let limestone_shade = if columns_from_centre <= -2 {
                if lit_from_the_left {
                    0
                } else {
                    3
                }
            } else if columns_from_centre >= 2 {
                if lit_from_the_left {
                    3
                } else {
                    0
                }
            } else {
                1
            };
            layer.put(
                centre + columns_from_centre,
                row,
                if is_opening { 0x3a3f4a } else { limestone[limestone_shade] },
            );
        }
    }
    layer.horizontal_line(centre - 4, centre + 4, lantern_top_row, limestone[0]);
    drawing.snow_along_ledge(layer, centre - 4, centre + 4, lantern_top_row);
    draw_pinnacle(layer, drawing, centre - 4, lantern_top_row - 1, 6, 1);
    draw_pinnacle(layer, drawing, centre + 4, lantern_top_row - 1, 6, 1);
    let spire_height = (lantern_top_row - 1) - finial_row;
    draw_pinnacle(layer, drawing, centre, lantern_top_row - 1, spire_height, 5);
}

fn draw_tower_top(layer: &mut Layer, drawing: &DrawingContext) {
    const WALK_METRES: f32 = 52.6;
    let limestone = drawing.palette.limestone;
    let lead = drawing.palette.lead;
    let axis_column = drawing.projection.pixel_x(0.0, 0.0);
    let (rear_left_turret, rear_right_turret) = corner_turret_columns(drawing.projection, true);
    let rear_parapet_top_row = drawing.projection.pixel_y(TOWER_BACK_WALL_METRES_NORTH, TOWER_BATTLEMENTS_TOP_METRES);
    let rear_walk_row = drawing.projection.pixel_y(TOWER_BACK_WALL_METRES_NORTH, WALK_METRES);
    let front_parapet_top_row = drawing.projection.pixel_y(0.0, TOWER_BATTLEMENTS_TOP_METRES);
    let rear_finial_row = drawing.projection.pixel_y(TOWER_BACK_WALL_METRES_NORTH, TOWER_HEIGHT_METRES);
    for turret_column in [rear_left_turret, rear_right_turret] {
        draw_turret_shaft(layer, drawing, turret_column, rear_parapet_top_row, rear_walk_row + 6, i32::MIN);
        draw_turret_crown(layer, drawing, turret_column, rear_parapet_top_row, rear_finial_row);
    }
    layer.fill_rect(rear_left_turret + 4, rear_parapet_top_row, rear_right_turret - 4, rear_walk_row, limestone[2]);
    layer.horizontal_line(rear_left_turret + 4, rear_right_turret - 4, rear_walk_row, limestone[3]);
    for column in rear_left_turret + 4..=rear_right_turret - 4 {
        let is_merlon = (column - rear_left_turret) % 4 < 2;
        if is_merlon {
            layer.put(column, rear_parapet_top_row - 1, limestone[1]);
            layer.put(column, rear_parapet_top_row - 2, limestone[0]);
        }
    }
    layer.pen_flags = 0;
    let walk_left = axis_column - TOWER_SHAFT_HALF_WIDTH_PIXELS + 2;
    let walk_right = axis_column + TOWER_SHAFT_HALF_WIDTH_PIXELS - 2;
    layer.fill_rect_with(walk_left, rear_walk_row + 1, walk_right, front_parapet_top_row, |column, row| {
        if let Some(snow) = drawing.snow_colour(column, row, 1.9, -0.15, 0) {
            return snow;
        }
        if hash_position(column / 4, row / 2, 0xDEC).is_multiple_of(5) {
            0x77736b
        } else {
            0x8a867c
        }
    });
    layer.pen_flags = FLOODLIT_TOWER_STONE;
    for side in [-1, 1] {
        let is_lit = (side < 0) == (drawing.look.sun_side <= 0.2);
        for row in rear_parapet_top_row..=front_parapet_top_row {
            let is_merlon = (row - rear_parapet_top_row) % 4 < 2;
            let outer_colour = if is_merlon { limestone[if is_lit { 0 } else { 2 }] } else { limestone[3] };
            layer.put(axis_column + side * TOWER_SHAFT_HALF_WIDTH_PIXELS, row, outer_colour);
            layer.put(
                axis_column + side * (TOWER_SHAFT_HALF_WIDTH_PIXELS - 1),
                row,
                if is_merlon { limestone[1] } else { limestone[4] },
            );
            layer.put(axis_column + side * (TOWER_SHAFT_HALF_WIDTH_PIXELS - 2), row, 0x5f5c56);
        }
    }
    layer.pen_flags = 0;
    let roof_left = axis_column - TOWER_SHAFT_HALF_WIDTH_PIXELS + 5;
    let roof_right = axis_column + TOWER_SHAFT_HALF_WIDTH_PIXELS - 5;
    let roof_back_row = rear_walk_row + 3;
    let roof_front_row = front_parapet_top_row - 3;
    let roof_apex = (axis_column, (roof_back_row + roof_front_row) / 2 - 3);
    let back_left = (roof_left, roof_back_row);
    let back_right = (roof_right, roof_back_row);
    let front_left = (roof_left, roof_front_row);
    let front_right = (roof_right, roof_front_row);
    fill_lead_roof(layer, drawing, &[back_left, back_right, roof_apex], LEAD_SEAM_SPACING_PIXELS, SHADED, true);
    fill_lead_roof(
        layer,
        drawing,
        &[back_left, roof_apex, front_left],
        LEAD_SEAM_SPACING_PIXELS,
        light_level_of_roof_slope(drawing, -1.0),
        false,
    );
    fill_lead_roof(
        layer,
        drawing,
        &[back_right, roof_apex, front_right],
        LEAD_SEAM_SPACING_PIXELS,
        light_level_of_roof_slope(drawing, 1.0),
        false,
    );
    fill_lead_roof(layer, drawing, &[front_left, front_right, roof_apex], LEAD_SEAM_SPACING_PIXELS, SUNLIT, true);
    for (corner_column, corner_row) in [back_left, back_right, front_left, front_right] {
        layer.line(corner_column, corner_row, roof_apex.0, roof_apex.1, lead[0]);
    }
    layer.fill_rect(roof_left, roof_front_row, roof_right, roof_front_row, lead[4]);
    layer.put(roof_apex.0, roof_apex.1, limestone[0]);
    layer.put(roof_apex.0, roof_apex.1 - 1, limestone[1]);
    layer.pen_flags = FLOODLIT_TOWER_STONE;
    draw_pinnacle(layer, drawing, axis_column, rear_parapet_top_row - 1, 10, 1);
    for side in [-1, 1] {
        draw_pinnacle(
            layer,
            drawing,
            axis_column + side * TOWER_SHAFT_HALF_WIDTH_PIXELS,
            (rear_parapet_top_row + front_parapet_top_row) / 2,
            9,
            1,
        );
    }
}

fn draw_tower(layer: &mut Layer, drawing: &DrawingContext, sources: &mut ShadowAndLightSources) {
    const BALCONY_METRES: f32 = 37.6;
    const BATTLEMENTS_FOOT_METRES: f32 = 52.6;
    const BUTTRESS_SET_OFFS_METRES: [f32; 3] = [10.0, 21.0, 30.0];
    const STRING_COURSES_METRES: [f32; 3] = [11.6, 21.0, 30.0];
    const SLIT_WINDOWS: [(i32, f32, f32); 6] =
        [(8, 23.2, 26.8), (-8, 23.2, 26.8), (-11, 31.2, 34.2), (-5, 31.2, 34.2), (5, 31.2, 34.2), (11, 31.2, 34.2)];
    let limestone = drawing.palette.limestone;
    let axis = drawing.projection.pixel_x(0.0, 0.0);
    let ground_row = drawing.projection.pixel_y(0.0, 0.0);
    let row_at = |metres_up: f32| drawing.projection.pixel_y(0.0, metres_up);
    let sun_side = drawing.look.sun_side;
    let (left_turret, right_turret) = corner_turret_columns(drawing.projection, false);
    let balcony_row = row_at(BALCONY_METRES);
    let battlements_foot_row = row_at(BATTLEMENTS_FOOT_METRES);
    let battlements_top_row = row_at(TOWER_BATTLEMENTS_TOP_METRES);

    layer.pen_flags = FLOODLIT_TOWER_STONE;
    draw_tower_top(layer, drawing);

    let side_buttress_outer_column = |row: i32| {
        if row > row_at(10.0) {
            axis - 27
        } else if row > row_at(21.0) {
            axis - 26
        } else if row > row_at(30.0) {
            axis - 25
        } else {
            axis - 24
        }
    };
    let front_buttress_outer_column = axis - 19;
    let distant_drawing = &drawing.for_distant_stonework();
    for row in balcony_row..=ground_row {
        let side_buttress_outer = side_buttress_outer_column(row);
        let front_buttress_inner = if row > row_at(21.0) { axis - 15 } else { axis - 16 };
        for column in side_buttress_outer..=2 * axis - side_buttress_outer {
            let column_on_the_left_half = if column > axis { 2 * axis - column } else { column };
            let is_left_half = column <= axis;
            let light_level = if column_on_the_left_half < front_buttress_outer_column {
                HALF_LIT
            } else if column_on_the_left_half <= front_buttress_inner {
                let is_lee_edge = column_on_the_left_half == front_buttress_inner
                    && ((is_left_half && sun_side < 0.2) || (!is_left_half && sun_side > -0.2));
                if is_lee_edge {
                    HALF_LIT
                } else {
                    SUNLIT
                }
            } else if column_on_the_left_half == front_buttress_inner + 1 {
                let in_the_buttress_shadow = (is_left_half && sun_side <= 0.0) || (!is_left_half && sun_side >= 0.0);
                if in_the_buttress_shadow {
                    SHADED
                } else {
                    HALF_LIT
                }
            } else {
                HALF_LIT
            };
            layer.put(column, row, distant_drawing.stone(light_level, column, row));
        }
        let seam_colour = distant_drawing.stone(SHADED, front_buttress_outer_column - 1, row);
        layer.put(front_buttress_outer_column - 1, row, seam_colour);
        layer.put(2 * axis - front_buttress_outer_column + 1, row, seam_colour);
        if (ground_row - row) % 4 < 2 {
            for column in [
                front_buttress_outer_column,
                front_buttress_inner,
                2 * axis - front_buttress_outer_column,
                2 * axis - front_buttress_inner,
            ] {
                layer.put(column, row, limestone[if column < axis { 1 } else { 2 }]);
            }
        }
    }
    for set_off_metres in BUTTRESS_SET_OFFS_METRES {
        let row = row_at(set_off_metres);
        for side in [-1, 1] {
            let (outer, inner) = (axis + side * 27, axis + side * 15);
            layer.horizontal_line(outer, inner, row, limestone[1]);
            layer.horizontal_line(outer, inner, row + 1, limestone[3]);
            drawing.snow_along_ledge(layer, outer.min(inner), outer.max(inner), row);
            draw_gablet(layer, drawing, axis + side * 17, row, 2);
        }
    }
    layer.horizontal_line(axis - 27, axis + 27, ground_row, limestone[3]);
    layer.horizontal_line(axis - 27, axis + 27, ground_row - 1, limestone[2]);
    draw_quoins(layer, drawing, axis - 27, row_at(10.0) + 3, ground_row - 3, 1);
    draw_quoins(layer, drawing, axis + 27, row_at(10.0) + 3, ground_row - 3, -1);
    for string_course_metres in STRING_COURSES_METRES {
        layer.horizontal_line(axis - 14, axis + 14, row_at(string_course_metres), limestone[2]);
    }
    for row in balcony_row..row_at(21.0) {
        layer.put(axis, row, limestone[1]);
        layer.put(axis + 1, row, limestone[3]);
    }
    for side in [-1, 1] {
        let lancet = GothicWindow {
            centre_column: axis + side * 4,
            sill_row: row_at(12.6),
            apex_row: row_at(19.6),
            glass_width: 3,
            lights: 1,
            glass_kind: STAINED_GLASS,
            window_number: 1,
            transom_rows_above_sill: 0,
        };
        draw_gothic_window(layer, drawing, sources, &lancet);
    }
    for (columns_from_axis, sill_metres, head_metres) in SLIT_WINDOWS {
        let column = axis + columns_from_axis;
        let (head_row, sill_row) = (row_at(head_metres), row_at(sill_metres));
        layer.vertical_line(column - 1, head_row, sill_row, limestone[2]);
        layer.vertical_line(column + 1, head_row, sill_row, limestone[2]);
        layer.put(column, head_row - 1, limestone[1]);
        layer.vertical_line(column, head_row, sill_row, 0x20242c);
        layer.put(column, sill_row + 1, limestone[1]);
    }

    draw_portal(layer, drawing, sources);

    let corbel_row = row_at(35.4);
    layer.horizontal_line(axis - 15, axis + 15, corbel_row, limestone[1]);
    for column in axis - 15..=axis + 15 {
        let is_corbel = (column - axis) % 2 == 0;
        layer.put(column, corbel_row + 1, if is_corbel { limestone[3] } else { limestone[1] });
        layer.put(column, corbel_row + 2, if is_corbel { limestone[4] } else { limestone[2] });
    }

    let belfry_left = left_turret + 4;
    let belfry_right = right_turret - 4;
    layer.fill_rect_with(belfry_left, battlements_foot_row, belfry_right, balcony_row, |column, row| {
        if hash_position(column / 3, row / 2, 0xBE1).is_multiple_of(7) {
            limestone[2]
        } else {
            limestone[1]
        }
    });
    draw_turret_shaft(layer, drawing, left_turret, battlements_top_row, balcony_row + 3, row_at(44.0));
    draw_turret_shaft(layer, drawing, right_turret, battlements_top_row, balcony_row + 3, row_at(44.0));
    for turret in [left_turret, right_turret] {
        layer.horizontal_line(turret - 4, turret + 4, balcony_row + 4, limestone[2]);
        layer.horizontal_line(turret - 3, turret + 3, balcony_row + 5, limestone[3]);
    }
    let balcony_parapet_row = row_at(38.2);
    layer.horizontal_line(belfry_left, belfry_right, balcony_parapet_row - 1, limestone[0]);
    for row in balcony_parapet_row..corbel_row {
        for column in belfry_left..=belfry_right {
            let is_quatrefoil = (column - left_turret) % 3 == 1 && row == balcony_parapet_row + 1;
            let colour = if is_quatrefoil {
                0x363b46
            } else if (column - left_turret) % 3 == 0 {
                limestone[2]
            } else {
                limestone[1]
            };
            layer.put(column, row, colour);
        }
    }
    drawing.snow_along_ledge(layer, belfry_left, belfry_right, balcony_parapet_row - 1);
    let louvre_sill_row = balcony_parapet_row - 3;
    let louvre_apex_row = row_at(50.4);
    for side in [-1, 1] {
        let louvre_centre = axis + side * 7;
        layer.vertical_line(louvre_centre - 5, louvre_apex_row - 1, louvre_sill_row + 1, limestone[0]);
        layer.vertical_line(louvre_centre + 5, louvre_apex_row - 1, louvre_sill_row + 1, limestone[3]);
        let belfry_opening = GothicWindow {
            centre_column: louvre_centre,
            sill_row: louvre_sill_row,
            apex_row: louvre_apex_row,
            glass_width: 9,
            lights: 2,
            glass_kind: BELFRY_LOUVRES,
            window_number: 3,
            transom_rows_above_sill: (louvre_sill_row - louvre_apex_row) * 2 / 5,
        };
        draw_gothic_window(layer, drawing, sources, &belfry_opening);
        layer.put(louvre_centre, louvre_apex_row - 2, limestone[0]);
    }
    layer.vertical_line(axis, battlements_foot_row, balcony_row - 4, limestone[0]);
    layer.vertical_line(axis - 1, battlements_foot_row, balcony_row - 4, limestone[2]);
    layer.vertical_line(axis + 1, battlements_foot_row, balcony_row - 4, limestone[3]);
    for row in battlements_foot_row + 1..=battlements_foot_row + 3 {
        for column in belfry_left..=belfry_right {
            let is_tracery_line = row == battlements_foot_row + 3 || (column - left_turret) % 3 == 0;
            layer.put(column, row, if is_tracery_line { limestone[3] } else { limestone[1] });
        }
    }
    layer.horizontal_line(belfry_left, belfry_right, battlements_foot_row, limestone[4]);
    for column in belfry_left..=belfry_right {
        let position_in_merlon = (column - belfry_left) % 5;
        for row in battlements_top_row..battlements_foot_row {
            let colour = if position_in_merlon >= 3 {
                if row >= battlements_foot_row - 1 {
                    limestone[3]
                } else {
                    0x55524d
                }
            } else if row == battlements_top_row {
                limestone[0]
            } else if position_in_merlon == 2 {
                limestone[3]
            } else {
                limestone[1]
            };
            layer.put(column, row, colour);
        }
    }
    drawing.snow_along_ledge(layer, belfry_left, belfry_right, battlements_top_row);
    draw_turret_crown(layer, drawing, left_turret, battlements_top_row, row_at(TOWER_HEIGHT_METRES));
    draw_turret_crown(layer, drawing, right_turret, battlements_top_row, row_at(TOWER_HEIGHT_METRES));
    draw_pinnacle(layer, drawing, axis, battlements_top_row - 1, 11, 3);
    layer.pen_flags = 0;

    sources.add_block_caster(drawing.projection, -7.4, 7.4, 0.0, 11.6, 55.0);
    for (spire_metres_east, spire_metres_north) in [(-5.9f32, 0.4f32), (5.9, 0.4), (-5.9, 11.2), (5.9, 11.2)] {
        sources.add_block_caster(
            drawing.projection,
            spire_metres_east - 0.8,
            spire_metres_east + 0.8,
            spire_metres_north - 0.5,
            spire_metres_north + 0.5,
            62.0,
        );
    }
}

fn draw_portal(layer: &mut Layer, drawing: &DrawingContext, sources: &mut ShadowAndLightSources) {
    const PORTAL_HALF_WIDTH_PIXELS: i32 = 13;
    const ARCH_HALF_WIDTH_PIXELS: i32 = 8;
    const PARAPET_STATUES: [(i32, i32); 3] = [(-7, 5), (0, 7), (7, 5)];
    let limestone = drawing.palette.limestone;
    let axis = drawing.projection.pixel_x(0.0, 0.0);
    let ground_row = drawing.projection.pixel_y(0.0, 0.0);
    let row_at = |metres_up: f32| drawing.projection.pixel_y(0.0, metres_up);
    let left = axis - PORTAL_HALF_WIDTH_PIXELS;
    let right = axis + PORTAL_HALF_WIDTH_PIXELS;
    let platform_row = drawing.projection.pixel_y(0.0, 0.75);
    let coping_row = row_at(9.4);
    fill_with_limestone_blocks(layer, drawing, left, coping_row, right, platform_row, 1);
    for side in [-1, 1] {
        layer.vertical_line(
            axis + side * 13,
            coping_row,
            platform_row,
            if side < 0 { limestone[0] } else { limestone[3] },
        );
        layer.vertical_line(axis + side * 10, coping_row + 2, platform_row, limestone[2]);
        for statue in 0..2 {
            let statue_top_row = row_at(6.4) + statue * 5;
            layer.vertical_line(axis + side * 11, statue_top_row, statue_top_row + 3, limestone[3]);
            layer.vertical_line(axis + side * 12, statue_top_row, statue_top_row + 3, limestone[2]);
            layer.put(axis + side * 11, statue_top_row, limestone[0]);
        }
    }
    let arch_apex_row = row_at(8.2);
    let arch_springing_row = row_at(4.6);
    for row in arch_apex_row..=platform_row {
        let half_width = if row >= arch_springing_row {
            ARCH_HALF_WIDTH_PIXELS
        } else {
            (1 + (row - arch_apex_row) * 7 / (arch_springing_row - arch_apex_row).max(1)).min(ARCH_HALF_WIDTH_PIXELS)
        };
        for column in axis - half_width..=axis + half_width {
            let moulding_order = half_width - (column - axis).abs();
            let colour = match moulding_order {
                0 => limestone[0],
                1 => limestone[3],
                2 => limestone[2],
                _ => limestone[4],
            };
            layer.put(column, row, colour);
        }
    }
    let lintel_row = row_at(5.0);
    for row in arch_apex_row + 2..lintel_row {
        for column in axis - 6..=axis + 6 {
            if layer.colour_at(column, row) == limestone[4] {
                layer.put(column, row, if (column + row) % 2 == 0 { limestone[2] } else { limestone[3] });
            }
        }
    }
    draw_plank_door(layer, drawing, axis - 5, lintel_row, axis - 1, platform_row);
    draw_plank_door(layer, drawing, axis + 1, lintel_row, axis + 5, platform_row);
    layer.vertical_line(axis, lintel_row - 1, platform_row, limestone[1]);
    layer.put(axis, lintel_row - 2, limestone[0]);
    for column in [axis - 4, axis - 2, axis + 2, axis + 4] {
        layer.put(column, lintel_row + 2, drawing.palette.iron[0]);
    }
    for row in lintel_row + 1..platform_row {
        for column in [axis - 3, axis + 3] {
            sources.glass_pixels.push(GlassPixel {
                index: (row * layer.width + column) as u32,
                kind: OPEN_DOORWAY,
                window: 2,
            });
        }
    }
    layer.pen_flags = 0;
    layer.fill_rect(left + 1, coping_row - 2, right - 1, coping_row - 1, drawing.palette.lead[2]);
    layer.pen_flags = FLOODLIT_TOWER_STONE;
    layer.horizontal_line(left - 1, right + 1, coping_row, limestone[0]);
    layer.horizontal_line(left, right, coping_row + 1, limestone[3]);
    drawing.snow_along_ledge(layer, left, right, coping_row);
    for (columns_from_axis, statue_height) in PARAPET_STATUES {
        let column = axis + columns_from_axis;
        let base_row = coping_row - 1;
        let top_row = base_row - statue_height + 2;
        layer.fill_rect(column - 1, top_row, column + 1, base_row, limestone[1]);
        layer.vertical_line(column, top_row + 1, base_row, limestone[4]);
        layer.vertical_line(column - 1, top_row, base_row, limestone[0]);
        layer.vertical_line(column + 1, top_row, base_row, limestone[3]);
        draw_pinnacle(layer, drawing, column, top_row - 1, 3, 1);
    }
    draw_pinnacle(layer, drawing, left, coping_row - 1, 7, 1);
    draw_pinnacle(layer, drawing, right, coping_row - 1, 7, 1);
    for side in [-1, 1] {
        let lantern_column = axis + side * 17;
        let lantern_row = row_at(3.6);
        layer.put(lantern_column, lantern_row - 1, drawing.palette.iron[0]);
        layer.put(lantern_column, lantern_row, 0xe0d098);
        layer.put(lantern_column, lantern_row + 1, drawing.palette.iron[0]);
        sources.lamps.push(Lamp {
            column: lantern_column,
            row: lantern_row,
            ground_row: ground_row + 4,
            depth_tag: depth_tag(-0.5),
            head_depth_tag: layer.pen_depth,
            kind: 1,
        });
    }
}
