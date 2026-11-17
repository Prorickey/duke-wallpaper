use crate::chapel::art::{Layer, WINDOW_GLASS};
use crate::chapel::palette::{HALF_LIT, SHADED, SUNLIT};
use crate::chapel::tex::{
    lead_colour, DrawingContext, GlassPixel, ShadowAndLightSources, BELFRY_LOUVRES, STAINED_GLASS,
};
use crate::hash_fraction;
use crate::{hash_position, Colour};

pub struct GothicWindow {
    pub centre_column: i32,
    pub sill_row: i32,
    pub apex_row: i32,
    pub glass_width: i32,
    pub lights: i32,
    pub glass_kind: u8,
    pub window_number: u16,
    pub transom_rows_above_sill: i32,
}

pub fn fill_with_limestone_blocks(
    layer: &mut Layer,
    drawing: &DrawingContext,
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
    shade: usize,
) {
    let limestone = drawing.palette.limestone;
    layer.fill_rect_with(left, top, right, bottom, |column, row| {
        let block_roll = hash_position(column / 3, row / 2, 0x11E) % 10;
        if block_roll == 0 {
            limestone[(shade + 1).min(4)]
        } else if block_roll == 1 && shade > 0 {
            limestone[shade - 1]
        } else {
            limestone[shade]
        }
    });
}

pub fn fill_stone_lit_from_the_side(
    layer: &mut Layer,
    drawing: &DrawingContext,
    first_column: i32,
    top: i32,
    last_column: i32,
    bottom: i32,
    light_level: usize,
) {
    let left = first_column.min(last_column);
    let right = first_column.max(last_column);
    let sun_side = drawing.look.sun_side;
    let wide_enough_for_lit_edges = right - left >= 2;
    layer.fill_rect_with(left, top, right, bottom, |column, row| {
        let edge_towards_the_sun = (column == left && sun_side < -0.2) || (column == right && sun_side > 0.2);
        let edge_away_from_the_sun = (column == right && sun_side < -0.2) || (column == left && sun_side > 0.2);
        let level = if wide_enough_for_lit_edges && edge_towards_the_sun {
            light_level.saturating_sub(1)
        } else if wide_enough_for_lit_edges && edge_away_from_the_sun {
            (light_level + 1).min(3)
        } else {
            light_level
        };
        drawing.stone(level, column, row)
    });
}

pub fn draw_quoins(
    layer: &mut Layer,
    drawing: &DrawingContext,
    edge_column: i32,
    top: i32,
    bottom: i32,
    direction: i32,
) {
    let limestone = drawing.palette.limestone;
    for row in top.min(bottom)..=top.max(bottom) {
        let is_long_stone = (row.div_euclid(2)) & 1 == 0;
        let stone_width = if is_long_stone { 2 } else { 1 };
        let colour = if (row & 1) == 0 { limestone[1] } else { limestone[2] };
        for step in 0..stone_width {
            layer.put(edge_column + direction * step, row, colour);
        }
    }
}

pub fn draw_gablet(layer: &mut Layer, drawing: &DrawingContext, centre_column: i32, base_row: i32, half_width: i32) {
    let limestone = drawing.palette.limestone;
    let sun_from_the_left = drawing.look.sun_side <= 0.0;
    for rows_up in 0..=half_width {
        let row = base_row - rows_up;
        let half_width_here = half_width - rows_up;
        for column in centre_column - half_width_here..=centre_column + half_width_here {
            let is_left_edge = column == centre_column - half_width_here && half_width_here > 0;
            let is_right_edge = column == centre_column + half_width_here && half_width_here > 0;
            let colour = if is_left_edge {
                if sun_from_the_left {
                    limestone[0]
                } else {
                    limestone[2]
                }
            } else if is_right_edge {
                if sun_from_the_left {
                    limestone[3]
                } else {
                    limestone[0]
                }
            } else {
                limestone[1]
            };
            layer.put(column, row, colour);
        }
    }
    if half_width >= 2 {
        layer.put(centre_column, base_row - 1, limestone[3]);
    }
    drawing.snow_along_ledge(layer, centre_column - 1, centre_column + 1, base_row - half_width);
}

pub fn draw_pinnacle(
    layer: &mut Layer,
    drawing: &DrawingContext,
    centre_column: i32,
    base_row: i32,
    height_pixels: i32,
    foot_width: i32,
) {
    let limestone = drawing.palette.limestone;
    let sun_side = drawing.look.sun_side;
    let (left_edge_colour, right_edge_colour) = if sun_side < -0.2 {
        (limestone[0], limestone[3])
    } else if sun_side > 0.2 {
        (limestone[3], limestone[0])
    } else {
        (limestone[1], limestone[2])
    };
    let shaft_rows = (height_pixels as f32 * 0.28) as i32;
    for rows_up in 0..height_pixels {
        let row = base_row - rows_up;
        let width_here = if rows_up < shaft_rows {
            foot_width
        } else {
            let fraction_up_the_spire = (rows_up - shaft_rows) as f32 / (height_pixels - shaft_rows).max(1) as f32;
            let tapered_width = ((foot_width as f32) * (1.0 - fraction_up_the_spire).powf(0.9)).round() as i32;
            (tapered_width | 1).clamp(1, foot_width)
        };
        let half_width = width_here / 2;
        for column in centre_column - half_width..=centre_column + half_width {
            let colour = if half_width == 0 {
                limestone[1]
            } else if column == centre_column - half_width {
                left_edge_colour
            } else if column == centre_column + half_width {
                right_edge_colour
            } else {
                limestone[1]
            };
            layer.put(column, row, colour);
        }
        let is_crocket_row = rows_up > shaft_rows
            && width_here >= 1
            && width_here < foot_width
            && (rows_up - shaft_rows) % 3 == 1
            && height_pixels > 9;
        if is_crocket_row {
            layer.put(centre_column - half_width - 1, row, left_edge_colour);
            layer.put(centre_column + half_width + 1, row, right_edge_colour);
        }
        if rows_up == shaft_rows && foot_width >= 3 {
            layer.put(centre_column - half_width - 1, row, limestone[2]);
            layer.put(centre_column + half_width + 1, row, limestone[3]);
        }
    }
    layer.put(centre_column, base_row - height_pixels, limestone[0]);
    if height_pixels > 12 {
        layer.put(centre_column - 1, base_row - height_pixels + 2, left_edge_colour);
        layer.put(centre_column + 1, base_row - height_pixels + 2, right_edge_colour);
    }
    if drawing.look.snow_cover > 0.4 {
        layer.put(centre_column, base_row - height_pixels, drawing.palette.snow[0]);
    }
}

pub fn put_glass_pixel(
    layer: &mut Layer,
    sources: &mut ShadowAndLightSources,
    column: i32,
    row: i32,
    colour: Colour,
    glass_kind: u8,
    window_number: u16,
) {
    if !layer.contains(column, row) {
        return;
    }
    let previous_flags = layer.pen_flags;
    layer.pen_flags = WINDOW_GLASS;
    layer.put(column, row, colour);
    layer.pen_flags = previous_flags;
    sources.glass_pixels.push(GlassPixel {
        index: (row * layer.width + column) as u32,
        kind: glass_kind,
        window: window_number,
    });
}

pub fn daylight_glass_colour(drawing: &DrawingContext, column: i32, row: i32, glass_kind: u8) -> Colour {
    let glass = drawing.palette.glass;
    let roll = hash_position(column, row, 0x61A5) % 100;
    match glass_kind {
        STAINED_GLASS => match roll {
            0..=52 => glass[0],
            53..=72 => glass[1],
            73..=81 => glass[2],
            82..=91 => glass[3],
            _ => glass[4],
        },
        BELFRY_LOUVRES => {
            if row & 1 == 0 {
                0x23262d
            } else {
                0x3b414c
            }
        }
        _ => {
            if roll < 22 {
                0x51627a
            } else if roll < 60 {
                glass[1]
            } else {
                glass[0]
            }
        }
    }
}

pub fn draw_gothic_window(
    layer: &mut Layer,
    drawing: &DrawingContext,
    sources: &mut ShadowAndLightSources,
    window: &GothicWindow,
) {
    let limestone = drawing.palette.limestone;
    let centre = window.centre_column;
    let half_width = window.glass_width / 2;
    let arch_rows = ((window.glass_width as f32 * 0.95) as i32).max(2).min((window.sill_row - window.apex_row).max(2));
    let light_width =
        if window.lights > 1 { (window.glass_width - (window.lights - 1)) / window.lights } else { window.glass_width };
    layer.put(centre, window.apex_row - 1, limestone[1]);
    for row in window.apex_row..=window.sill_row {
        let rows_below_apex = row - window.apex_row;
        let half_width_here = if rows_below_apex >= arch_rows {
            half_width
        } else {
            ((half_width as f32) * ((rows_below_apex as f32 + 0.7) / arch_rows as f32).powf(0.6)).round() as i32
        }
        .min(half_width);
        layer.put(centre - half_width_here - 1, row, limestone[1]);
        layer.put(centre + half_width_here + 1, row, limestone[2]);
        if rows_below_apex < arch_rows && half_width_here < half_width {
            layer.put(centre - half_width_here - 2, row + 1, limestone[1]);
            layer.put(centre + half_width_here + 2, row + 1, limestone[2]);
        }
        for column in centre - half_width_here..=centre + half_width_here {
            let columns_from_left = column - (centre - half_width);
            let is_mullion = window.lights > 1
                && window.glass_width >= 5
                && (columns_from_left + 1) % (light_width + 1) == 0
                && columns_from_left < window.glass_width - 1;
            let is_tracery = window.glass_width >= 5
                && rows_below_apex < arch_rows
                && ((column - centre).abs() + rows_below_apex) % 2 == 0;
            let is_transom =
                window.transom_rows_above_sill > 0 && row == window.sill_row - window.transom_rows_above_sill;
            if is_mullion || is_tracery || is_transom {
                layer.put(column, row, if is_tracery { limestone[2] } else { limestone[1] });
            } else {
                let colour = daylight_glass_colour(drawing, column, row, window.glass_kind);
                put_glass_pixel(layer, sources, column, row, colour, window.glass_kind, window.window_number);
            }
        }
    }
    for column in centre - half_width - 1..=centre + half_width + 1 {
        layer.put(column, window.sill_row + 1, limestone[1]);
    }
    drawing.snow_along_ledge(layer, centre - half_width - 1, centre + half_width + 1, window.sill_row + 1);
}

pub fn draw_plank_door(layer: &mut Layer, drawing: &DrawingContext, left: i32, top: i32, right: i32, bottom: i32) {
    let wood = drawing.palette.wood;
    let middle_column = (left + right) / 2;
    for row in top..=bottom {
        for column in left..=right {
            let rows_down = row - top;
            let columns_from_side = (column - left).min(right - column);
            let is_cut_away_by_the_pointed_head = rows_down < 2 && columns_from_side < 2 - rows_down;
            if is_cut_away_by_the_pointed_head {
                continue;
            }
            let is_rail = rows_down == (bottom - top) / 2 || row == bottom;
            let colour = if is_rail {
                wood[2]
            } else if column == middle_column && (right - left) >= 4 {
                wood[3]
            } else if (column - left) % 2 == 0 {
                wood[1]
            } else {
                wood[0]
            };
            layer.put(column, row, colour);
        }
    }
}

pub fn fill_lead_roof(
    layer: &mut Layer,
    drawing: &DrawingContext,
    outline: &[(i32, i32)],
    seam_spacing: i32,
    light_level: usize,
    seams_run_down_the_screen: bool,
) {
    let steps_from_light = if light_level == SHADED { 1 } else { 0 };
    layer.fill_polygon_with(outline, |column, row| {
        let Some(snow) = drawing.snow_colour(column, row, 1.8, -0.10, steps_from_light) else {
            return Some(lead_colour(
                drawing.palette,
                column,
                row,
                seam_spacing,
                light_level,
                seams_run_down_the_screen,
            ));
        };
        let is_seam = if seams_run_down_the_screen {
            column.rem_euclid(seam_spacing) == 0
        } else {
            row.rem_euclid(seam_spacing) == 0
        };
        if is_seam && hash_fraction(column, row, 5) < 0.45 {
            Some(drawing.palette.snow[1])
        } else {
            Some(snow)
        }
    });
}

pub fn light_level_of_roof_slope(drawing: &DrawingContext, side: f32) -> usize {
    let sun_side = drawing.look.sun_side;
    if sun_side.abs() < 0.2 {
        return if side < 0.0 { SUNLIT } else { SHADED };
    }
    let away_from_sun = -side.signum() * sun_side;
    if away_from_sun < -0.2 {
        SUNLIT
    } else if away_from_sun > 0.2 {
        SHADED
    } else {
        HALF_LIT
    }
}
