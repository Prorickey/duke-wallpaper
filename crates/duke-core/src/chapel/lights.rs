use crate::chapel::anim::World;
use crate::chapel::art::{Layer, FLOODLIT_TOWER_STONE, NOTHING_DRAWN, SOFTLY_FLOODLIT_STONE, WINDOW_GLASS};
use crate::chapel::palette::{bayer_threshold, smooth_step, Look};
use crate::chapel::proj::*;
use crate::chapel::tex::{GlassPixel, ShadowAndLightSources, BELFRY_LOUVRES, LIT_ROOM, OPEN_DOORWAY, STAINED_GLASS};
use crate::hash_fraction;
use crate::{colour_channels, hash_position, rgb, Canvas, Colour, Environment, CANVAS_HEIGHT, CANVAS_WIDTH};

#[derive(Default)]
pub struct Lights {
    added_light_at_full_night: Vec<(u32, Colour)>,
    lit_glass: Vec<LitGlassPixel>,
    lamp_heads: Vec<(i32, i32, u8)>,
}

struct LitGlassPixel {
    index: u32,
    colour: Colour,
    window: u16,
    kind: u8,
}

fn add_light(light: &mut [[f32; 3]], column: i32, row: i32, colour: [f32; 3], strength: f32) {
    if column < 0 || row < 0 || column >= CANVAS_WIDTH || row >= CANVAS_HEIGHT {
        return;
    }
    let pixel = &mut light[(row * CANVAS_WIDTH + column) as usize];
    for channel in 0..3 {
        pixel[channel] += colour[channel] * strength;
    }
}

impl Lights {
    pub fn build(objects: &Layer, sources: &ShadowAndLightSources, look: &Look, projection: Projection) -> Lights {
        const FLOODLIT_HEIGHT_METRES: f32 = 55.0;
        let pixel_count = (CANVAS_WIDTH * CANVAS_HEIGHT) as usize;
        let mut light = vec![[0.0f32; 3]; pixel_count];
        let tower_foot_row = projection.pixel_y(0.0, 0.0);
        let tower_pixels = (tower_foot_row - projection.pixel_y(0.0, FLOODLIT_HEIGHT_METRES)) as f32;
        for (index, added_light) in light.iter_mut().enumerate() {
            let flags = objects.flags[index];
            if flags & (FLOODLIT_TOWER_STONE | SOFTLY_FLOODLIT_STONE) == 0 {
                continue;
            }
            let is_tower = flags & FLOODLIT_TOWER_STONE != 0;
            let (red, green, blue) = colour_channels(objects.pixels[index]);
            let row = (index as i32) / CANVAS_WIDTH;
            let column = (index as i32) % CANVAS_WIDTH;
            let fraction_up_the_tower = ((tower_foot_row as f32 - row as f32) / tower_pixels).clamp(0.0, 1.0);
            let strength = if is_tower {
                let belfry_lights = if fraction_up_the_tower > 0.66 { 0.26 } else { 0.0 };
                let uplight_scallop = 0.08 * ((column as f32 * 0.21).sin() * (1.0 - fraction_up_the_tower));
                0.98 - 0.42 * fraction_up_the_tower + belfry_lights + uplight_scallop
            } else {
                0.30 + 0.10 * hash_fraction(column / 9, row / 14, 0xF100D)
            };
            let luma = 0.3 * red as f32 + 0.59 * green as f32 + 0.11 * blue as f32;
            let strength = strength * (1.0 - 0.42 * ((luma - 140.0) / 80.0).clamp(0.0, 1.0));
            let warmth = if is_tower { [1.0, 0.86, 0.60] } else { [0.95, 0.80, 0.58] };
            *added_light = [
                red as f32 * warmth[0] * strength,
                green as f32 * warmth[1] * strength,
                blue as f32 * warmth[2] * strength,
            ];
        }

        let mut lit_glass = Vec::with_capacity(sources.glass_pixels.len());
        for &GlassPixel { index, kind, window } in &sources.glass_pixels {
            let pixel = index as usize;
            let still_visible = if kind == OPEN_DOORWAY {
                objects.depth[pixel] != NOTHING_DRAWN
            } else {
                objects.flags[pixel] & WINDOW_GLASS != 0
            };
            if !still_visible {
                continue;
            }
            let (column, row) = ((index as i32) % CANVAS_WIDTH, (index as i32) / CANVAS_WIDTH);
            let room_roll = hash_position(column, row, look.seed ^ 0x61) % 100;
            let colour: Colour = match kind {
                STAINED_GLASS => {
                    let quarry_roll = hash_position(column, row >> 1, look.seed ^ 0x62) % 100;
                    let in_a_medallion_band = (row % 7) < 2 && quarry_roll < 70;
                    if in_a_medallion_band {
                        if quarry_roll < 40 {
                            0xf8cf5a
                        } else {
                            0xfbf0d2
                        }
                    } else {
                        match quarry_roll {
                            0..=37 => 0x3566dc,
                            38..=63 => 0xd8402f,
                            64..=79 => 0xf2b93c,
                            80..=88 => 0x3fa866,
                            89..=94 => 0x8a55d6,
                            _ => 0xf6ecd0,
                        }
                    }
                }
                BELFRY_LOUVRES => {
                    if row & 1 == 0 {
                        0x4a3418
                    } else {
                        0xb47a30
                    }
                }
                OPEN_DOORWAY => 0xffb860,
                _ => match room_roll {
                    0..=44 => 0xffd98a,
                    45..=74 => 0xffcc74,
                    75..=92 => 0xffe7b0,
                    _ => 0xf2f0e0,
                },
            };
            lit_glass.push(LitGlassPixel { index, colour, window, kind });
            if kind == LIT_ROOM {
                continue;
            }
            const SPILL_ONTO_THE_STONE: [(i32, i32, f32); 7] =
                [(-1, 0, 0.16), (1, 0, 0.16), (0, -1, 0.12), (0, 1, 0.16), (-2, 0, 0.06), (2, 0, 0.06), (0, 2, 0.08)];
            let (red, green, blue) = colour_channels(colour);
            let glow = [red as f32, green as f32, blue as f32];
            for (columns_across, rows_down, strength) in SPILL_ONTO_THE_STONE {
                let (spill_column, spill_row) = (column + columns_across, row + rows_down);
                let on_canvas =
                    spill_column >= 0 && spill_row >= 0 && spill_column < CANVAS_WIDTH && spill_row < CANVAS_HEIGHT;
                if on_canvas && objects.flags[(spill_row * CANVAS_WIDTH + spill_column) as usize] & WINDOW_GLASS == 0 {
                    add_light(&mut light, spill_column, spill_row, glow, strength);
                }
            }
        }

        let mut add_pool_of_light = |centre_column: i32,
                                     centre_row: i32,
                                     radius_columns: f32,
                                     radius_rows: f32,
                                     lamp_depth_tag: i16,
                                     colour: [f32; 3],
                                     gain: f32| {
            for row in (centre_row as f32 - radius_rows) as i32..=(centre_row as f32 + radius_rows) as i32 {
                for column in
                    (centre_column as f32 - radius_columns) as i32..=(centre_column as f32 + radius_columns) as i32
                {
                    if column < 0 || row < 0 || column >= CANVAS_WIDTH || row >= CANVAS_HEIGHT {
                        continue;
                    }
                    let across = (column - centre_column) as f32 / radius_columns;
                    let down = (row - centre_row) as f32 / radius_rows;
                    let distance_squared = across * across + down * down;
                    if distance_squared >= 1.0 {
                        continue;
                    }
                    let object_depth_tag = objects.depth[(row * CANVAS_WIDTH + column) as usize];
                    let object_stands_in_front_of_the_lamp =
                        object_depth_tag != NOTHING_DRAWN && object_depth_tag < lamp_depth_tag - 12;
                    if object_stands_in_front_of_the_lamp {
                        continue;
                    }
                    let falloff = (1.0 - distance_squared) * (1.0 - distance_squared);
                    let banded = ((falloff * 4.0 + bayer_threshold(column, row) - 0.5).floor() / 4.0).clamp(0.0, 1.0);
                    let surface_gain = if object_depth_tag == NOTHING_DRAWN { 1.0 } else { 0.6 };
                    add_light(&mut light, column, row, colour, banded * gain * surface_gain);
                }
            }
        };
        const WARM_LAMP_LIGHT: [f32; 3] = [255.0, 200.0, 120.0];
        const CAR_PARK_MAST_LIGHT: [f32; 3] = [235.0, 225.0, 190.0];
        let mut lamp_heads = Vec::new();
        for lamp in &sources.lamps {
            let pixels_per_metre = projection.pixels_per_metre_across(lamp.depth_tag as f32 / 4.0);
            let (pool_radius_metres, gain, colour) = match lamp.kind {
                0 => (4.4, 0.42, WARM_LAMP_LIGHT),
                1 => (3.0, 0.40, WARM_LAMP_LIGHT),
                _ => (7.0, 0.34, CAR_PARK_MAST_LIGHT),
            };
            let radius_columns = (pool_radius_metres * pixels_per_metre).max(3.0);
            add_pool_of_light(
                lamp.column,
                lamp.ground_row,
                radius_columns,
                (radius_columns * 0.42).max(2.0),
                lamp.depth_tag,
                colour,
                gain,
            );
            let head_is_on_canvas =
                lamp.column >= 0 && lamp.row >= 0 && lamp.column < CANVAS_WIDTH && lamp.row < CANVAS_HEIGHT;
            if head_is_on_canvas
                && objects.depth[(lamp.row * CANVAS_WIDTH + lamp.column) as usize] == lamp.head_depth_tag
            {
                lamp_heads.push((lamp.column, lamp.row, lamp.kind));
            }
        }
        let (floodlights_column, floodlights_row) = projection.ground_pixel((0.0, -3.0));
        add_pool_of_light(floodlights_column - 14, floodlights_row, 13.0, 5.0, depth_tag(-3.0), WARM_LAMP_LIGHT, 0.30);
        add_pool_of_light(floodlights_column + 14, floodlights_row, 13.0, 5.0, depth_tag(-3.0), WARM_LAMP_LIGHT, 0.30);
        add_pool_of_light(floodlights_column, floodlights_row + 1, 15.0, 6.0, depth_tag(-3.0), WARM_LAMP_LIGHT, 0.34);

        let mut added_light_at_full_night = Vec::new();
        for (index, pixel) in light.iter().enumerate() {
            if pixel[0] + pixel[1] + pixel[2] > 6.0 {
                let colour = rgb(pixel[0].min(255.0) as u8, pixel[1].min(255.0) as u8, pixel[2].min(255.0) as u8);
                added_light_at_full_night.push((index as u32, colour));
            }
        }
        Lights { added_light_at_full_night, lit_glass, lamp_heads }
    }
}

pub fn draw(environment: &Environment, canvas: &mut Canvas, lights: &Lights, world: &World) {
    const LAMP_HEAD_HALO: [(i32, i32, f32); 10] = [
        (-2, 0, 0.30),
        (2, 0, 0.30),
        (0, -1, 0.30),
        (0, 2, 0.24),
        (-1, -1, 0.2),
        (1, -1, 0.2),
        (-1, 1, 0.2),
        (1, 1, 0.2),
        (-3, 0, 0.12),
        (3, 0, 0.12),
    ];
    let projection = world.projection;
    let darkness = smooth_step(0.28, 0.72, environment.darkness());
    if darkness <= 0.004 {
        return;
    }
    for &(index, colour) in &lights.added_light_at_full_night {
        let pixel = index as usize;
        canvas.pixels[pixel] = crate::add_colours(canvas.pixels[pixel], colour, darkness);
    }
    let hour = environment.hour.rem_euclid(24.0);
    let share_of_rooms_lit = if hour >= 22.0 {
        0.58 - (hour - 22.0) * 0.16
    } else if hour < 2.0 {
        0.26 - hour * 0.06
    } else if hour < 5.0 {
        0.14
    } else if hour < 7.0 {
        0.14 + (hour - 5.0) * 0.22
    } else {
        0.58
    };
    for glass in &lights.lit_glass {
        let is_lit = glass.kind != LIT_ROOM || hash_fraction(glass.window as i32, 3, 0x11E) < share_of_rooms_lit;
        if !is_lit {
            continue;
        }
        let pixel = glass.index as usize;
        let strength = match glass.kind {
            STAINED_GLASS => 0.92,
            BELFRY_LOUVRES => 0.55,
            OPEN_DOORWAY => 0.35,
            _ => 0.95,
        };
        canvas.pixels[pixel] = crate::blend_colours(canvas.pixels[pixel], glass.colour, darkness * strength);
        if glass.kind == LIT_ROOM {
            let (column, row) = ((glass.index as i32) % CANVAS_WIDTH, (glass.index as i32) / CANVAS_WIDTH);
            canvas.add_glow(column, row + 1, glass.colour, 0.10 * darkness);
            canvas.add_glow(column - 1, row, glass.colour, 0.07 * darkness);
            canvas.add_glow(column + 1, row, glass.colour, 0.07 * darkness);
        }
    }
    for &(column, row, kind) in &lights.lamp_heads {
        let is_car_park_mast = kind == 2;
        let colour = if is_car_park_mast { 0xf4f0dc } else { 0xffe2a0 };
        canvas.blend_pixel(column, row, colour, darkness);
        if is_car_park_mast {
            canvas.blend_pixel(column - 2, row, colour, darkness);
            canvas.blend_pixel(column + 2, row, colour, darkness);
        } else {
            canvas.blend_pixel(column - 1, row, colour, darkness * 0.8);
            canvas.blend_pixel(column + 1, row, colour, darkness * 0.8);
            canvas.blend_pixel(column, row + 1, colour, darkness * 0.8);
        }
        for (columns_across, rows_down, strength) in LAMP_HEAD_HALO {
            canvas.add_glow(column + columns_across, row + rows_down, colour, strength * darkness);
        }
    }
    let seconds = crate::chapel::anim::seconds_on_wrapped_clock(environment);
    let Some(car) = crate::chapel::crowd::car_at_the_circle(environment, world, seconds) else {
        return;
    };
    let (car_column, car_row) = projection.ground_pixel((car.metres_east, car.metres_north));
    let heading_columns = car.heading_east * projection.pixels_per_metre_across(car.metres_north);
    let heading_rows = -car.heading_north * projection.pixels_per_metre_of_depth(car.metres_north);
    let beam_reach = (5.0 * projection.pixels_per_metre_across(car.metres_north)) as i32;
    let heading_length = (heading_columns * heading_columns + heading_rows * heading_rows).sqrt().max(0.1);
    let (along_columns, along_rows) = (heading_columns / heading_length, heading_rows / heading_length);
    let headlamp_column = car_column as f32 + along_columns * beam_reach as f32 * 0.5;
    let headlamp_row = car_row as f32 - 1.0 + along_rows * 3.0;
    for step in 0..beam_reach {
        let beam_half_width = 1 + step / 5;
        let strength = 0.42 * (1.0 - step as f32 / beam_reach as f32) * darkness * car.presence;
        for across in -beam_half_width..=beam_half_width {
            let column = (headlamp_column + along_columns * step as f32 - along_rows * across as f32).round() as i32;
            let row =
                (headlamp_row + along_rows * step as f32 * 0.6 + along_columns * across as f32 * 0.5).round() as i32;
            canvas.add_glow(column, row, 0xfff0c8, strength);
        }
    }
    canvas.blend_pixel(headlamp_column as i32, headlamp_row as i32, 0xfffbe6, darkness * car.presence);
}
