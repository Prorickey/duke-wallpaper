use crate::chapel::art::polygon_spans;
use crate::chapel::tex::Caster;
use crate::hash_fraction;
use crate::{tint_colour, Colour};

pub fn shadow_mask(
    casters: &[Caster],
    shift_x_per_height_pixel: f32,
    shift_y_per_height_pixel: f32,
    width: i32,
    height: i32,
) -> Vec<u8> {
    let (shift_x, shift_y) = (shift_x_per_height_pixel, shift_y_per_height_pixel);
    let mut mask = vec![0u8; (width * height) as usize];
    let mut darken = |column: i32, row: i32| {
        if column >= 0 && row >= 0 && column < width && row < height {
            mask[(row * width + column) as usize] = 255;
        }
    };
    for caster in casters {
        match *caster {
            Caster::Prism { ref footprint, height_pixels } => {
                let roof_shift = ((shift_x * height_pixels).round() as i32, (shift_y * height_pixels).round() as i32);
                let roof: Vec<(i32, i32)> =
                    footprint.iter().map(|&(column, row)| (column + roof_shift.0, row + roof_shift.1)).collect();
                let mut darken_span = |left: i32, right: i32, row: i32| {
                    for column in left..=right {
                        darken(column, row);
                    }
                };
                polygon_spans(&roof, width, height, &mut darken_span);
                polygon_spans(footprint, width, height, &mut darken_span);
                for corner in 0..footprint.len() {
                    let next_corner = (corner + 1) % footprint.len();
                    let wall = [footprint[corner], footprint[next_corner], roof[next_corner], roof[corner]];
                    polygon_spans(&wall, width, height, &mut darken_span);
                }
            }
            Caster::Canopy {
                trunk_column,
                trunk_row,
                radius_columns,
                radius_rows,
                bottom_height_pixels,
                top_height_pixels,
                density,
            } => {
                let crown_depth = top_height_pixels - bottom_height_pixels;
                let shadow_length = ((shift_x * crown_depth).abs().max((shift_y * crown_depth).abs())).max(0.0);
                let spacing = (radius_columns.min(radius_rows) * 0.6).max(1.0);
                let slices = (shadow_length / spacing).ceil() as i32 + 1;
                for slice in 0..=slices {
                    let up_the_crown = slice as f32 / slices as f32;
                    let slice_height = bottom_height_pixels + crown_depth * up_the_crown;
                    let bulge = 0.72 + 0.28 * (1.0 - (2.0 * up_the_crown - 1.0).abs());
                    let centre_x = trunk_column as f32 + shift_x * slice_height;
                    let centre_y = trunk_row as f32 + shift_y * slice_height;
                    let (slice_radius_x, slice_radius_y) = (radius_columns * bulge, radius_rows * bulge);
                    let top_row = (centre_y - slice_radius_y).floor() as i32;
                    let bottom_row = (centre_y + slice_radius_y).ceil() as i32;
                    for row in top_row..=bottom_row {
                        let across_y = (row as f32 - centre_y) / slice_radius_y.max(0.5);
                        if across_y.abs() > 1.0 {
                            continue;
                        }
                        let half_width = slice_radius_x * (1.0 - across_y * across_y).sqrt();
                        for column in (centre_x - half_width).round() as i32..=(centre_x + half_width).round() as i32 {
                            let across_x = (column as f32 - centre_x) / slice_radius_x.max(0.5);
                            let distance_squared = across_x * across_x + across_y * across_y;
                            let roll = hash_fraction(column, row, 0x5AD0);
                            let is_ragged_edge = distance_squared > 0.72 && roll < 0.4;
                            if !is_ragged_edge && (density >= 0.99 || roll < density) {
                                darken(column, row);
                            }
                        }
                    }
                }
                darken_pole(&mut darken, trunk_column, trunk_row, bottom_height_pixels, shift_x, shift_y);
            }
            Caster::Pole { foot_column, foot_row, height_pixels } => {
                darken_pole(&mut darken, foot_column, foot_row, height_pixels, shift_x, shift_y);
            }
        }
    }
    mask
}

fn darken_pole(
    darken: &mut impl FnMut(i32, i32),
    foot_column: i32,
    foot_row: i32,
    height_pixels: f32,
    shift_x: f32,
    shift_y: f32,
) {
    let steps = (shift_x.abs().max(shift_y.abs()) * height_pixels).ceil() as i32;
    for step in 0..=steps {
        let up_the_pole = if steps == 0 { 0.0 } else { step as f32 / steps as f32 };
        darken(
            (foot_column as f32 + shift_x * height_pixels * up_the_pole).round() as i32,
            (foot_row as f32 + shift_y * height_pixels * up_the_pole).round() as i32,
        );
    }
}

pub fn shaded(colour: Colour, strength: f32) -> Colour {
    let strength = strength.clamp(0.0, 1.0);
    tint_colour(colour, [1.0 - 0.50 * strength, 1.0 - 0.44 * strength, 1.0 - 0.27 * strength])
}
