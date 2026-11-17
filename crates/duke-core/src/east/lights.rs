use crate::east::animation::Memories;
use crate::east::bus::{buses_in_view, BusView, Timetable, BUS_HALF_LENGTH, FRONT_DOOR_FROM_CENTRE};
use crate::east::geometry::{CANVAS_HEIGHT, CANVAS_WIDTH};
use crate::east::ground::is_within_quad_lawns;
use crate::east::projection::*;
use crate::east::shading_tools::*;
use crate::east::static_layer::StaticLayer;
use crate::east::vehicles::{markham_avenue_cars, service_loop_car, Car};
use crate::east::walkers::how_busy_the_quad_is;
use crate::{
    add_colours, blend_colours, fold_seed_to_32_bits, hash_position, rgb, Canvas, Colour, Environment, Season, Weather,
};

const FLOODLIGHT: Colour = rgb(255, 224, 168);
const LAMP_POOL: Colour = rgb(255, 188, 104);
const LAMP_GLOBE: Colour = rgb(255, 236, 180);
const LAMP_HALO: Colour = rgb(255, 200, 120);
const HEADLAMP: Colour = rgb(255, 244, 210);
const HEADLAMP_BEAM: Colour = rgb(255, 236, 190);
const TAIL_LAMP: Colour = rgb(255, 40, 30);
const BUS_SALOON_LIGHT: Colour = rgb(255, 226, 150);
const ALWAYS_LIT_GLOW_GROUP: u16 = 1;
const LAMP_GLOBE_PIXELS_AND_BRIGHTNESS: [(i32, i32, f32); 7] =
    [(0, 0, 1.0), (-1, 0, 0.9), (1, 0, 0.8), (0, -1, 0.9), (0, 1, 0.7), (-1, 1, 0.5), (1, 1, 0.4)];
const LAMP_HALO_PIXELS: [(i32, i32); 8] = [(-2, 0), (2, 0), (0, -2), (-1, -1), (1, -1), (-2, 1), (2, 1), (0, 2)];

fn window_brightness(glow_group: u16, environment: &Environment, seed: u32) -> f32 {
    if glow_group == ALWAYS_LIT_GLOW_GROUP {
        return 1.0;
    }
    let hour = environment.hour.rem_euclid(24.0);
    let share_of_rooms_lit = if (5.5..17.0).contains(&hour) {
        0.3
    } else if hour >= 17.0 {
        0.68 - 0.2 * ((hour - 21.0) / 3.0).clamp(0.0, 1.0)
    } else {
        0.42 - 0.3 * (hour / 3.0).clamp(0.0, 1.0)
    };
    let group = glow_group as i32;
    let seconds_per_switch = 70.0 + (hash_position(group, 1, seed) % 150) as f64;
    let switches =
        environment.animation_seconds / seconds_per_switch + (hash_position(group, 2, seed) % 1000) as f64 / 1000.0;
    let switch = switches.floor() as i64 as i32;
    let into_switch = switches.fract() as f32;
    let is_lit = |switch: i32| (unit_hash(group, switch, seed ^ 0x11D) < share_of_rooms_lit) as i32 as f32;
    let fade = (into_switch * seconds_per_switch as f32 / 2.0).min(1.0);
    is_lit(switch - 1) + (is_lit(switch) - is_lit(switch - 1)) * fade
}

pub fn render(environment: &Environment, static_layer: &StaticLayer, canvas: &mut Canvas, memories: &mut Memories) {
    let lightning_dims_the_lamps = 1.0 - 0.75 * environment.lightning_flash.clamp(0.0, 1.0);
    let night = smoothstep(0.22, 0.72, environment.darkness()) * lightning_dims_the_lamps;
    if night <= 0.0 {
        return;
    }
    let seed = fold_seed_to_32_bits(environment.seed);

    for &(pixel, strength) in static_layer.floodlit_pixels.iter() {
        let pixel = pixel as usize;
        canvas.pixels[pixel] = add_colours(canvas.pixels[pixel], FLOODLIGHT, night * (strength as f32 / 255.0) * 0.62);
    }

    let mut current_group_and_brightness = (0u16, 0.0f32);
    for glowing in static_layer.glowing_pixels.iter() {
        if glowing.glow_group != current_group_and_brightness.0 {
            current_group_and_brightness =
                (glowing.glow_group, window_brightness(glowing.glow_group, environment, seed));
        }
        let brightness = current_group_and_brightness.1 * night;
        if brightness <= 0.01 {
            continue;
        }
        let pixel = glowing.pixel as usize;
        let colour = glowing.glow_colour;
        canvas.pixels[pixel] = blend_colours(canvas.pixels[pixel], colour, brightness * 0.95);
        let (x, y) = ((pixel % CANVAS_WIDTH) as i32, (pixel / CANVAS_WIDTH) as i32);
        let spill = brightness * 0.1;
        canvas.add_glow(x - 1, y, colour, spill);
        canvas.add_glow(x + 1, y, colour, spill);
        canvas.add_glow(x, y + 1, colour, spill * 1.4);
        canvas.add_glow(x, y - 1, colour, spill * 0.6);
    }

    for &(pixel, strength) in static_layer.lamp_pools.iter() {
        let pixel = pixel as usize;
        canvas.pixels[pixel] = add_colours(canvas.pixels[pixel], LAMP_POOL, strength as f32 / 255.0 * night);
    }
    for lamp in static_layer.lamps.iter().filter(|lamp| !lamp.is_arcade_lantern) {
        for (along, down, brightness) in LAMP_GLOBE_PIXELS_AND_BRIGHTNESS {
            let (x, y) = (lamp.globe_x + along, lamp.globe_y + down);
            if x < 0 || y < 0 || x as usize >= CANVAS_WIDTH || y as usize >= CANVAS_HEIGHT {
                continue;
            }
            let pixel = y as usize * CANVAS_WIDTH + x as usize;
            if lamp.metres_south + 0.3 >= static_layer.depths[pixel] {
                canvas.pixels[pixel] = blend_colours(canvas.pixels[pixel], LAMP_GLOBE, brightness * night);
            }
        }
        for (along, down) in LAMP_HALO_PIXELS {
            canvas.add_glow(lamp.globe_x + along, lamp.globe_y + down, LAMP_HALO, 0.3 * night);
        }
    }

    let mut headlights = |car: &Car, beam_length: i32| {
        let (x, y) = project_to_pixel(car.metres_east, car.metres_south, 0.7);
        if y < 0 {
            return;
        }
        let forward = if car.direction >= 0.0 { 1 } else { -1 };
        let nose = (2.2 * pixels_per_metre_at(car.metres_south)).round() as i32;
        canvas.add_glow(x + forward * nose, y, HEADLAMP, 0.9 * night);
        for reach in (2..beam_length).step_by(2) {
            canvas.add_glow(
                x + forward * (nose + reach),
                y + 1,
                HEADLAMP_BEAM,
                0.24 * night * (1.0 - reach as f32 / beam_length as f32),
            );
        }
        canvas.add_glow(x - forward * nose, y, TAIL_LAMP, 0.7 * night);
    };
    if let Some(car) = service_loop_car(environment) {
        headlights(&car, 12);
    }
    let busyness = how_busy_the_quad_is(environment);
    for car in markham_avenue_cars(environment, seed, busyness, &mut memories.who_turns_up).iter().flatten() {
        headlights(car, 10);
    }

    for bus in buses_in_view(&Timetable::now(environment, seed, busyness), &mut memories.bus_stop_crowds) {
        let pixels_per_metre = pixels_per_metre_at(bus.metres_south);
        let height = (3.1 * screen_rows_per_metre_of_height_at(bus.metres_south)).round() as i32;
        let (x, y) = project_to_pixel(bus.metres_east, bus.metres_south, 0.0);
        match bus.view {
            BusView::SideOn => {
                let half_length = (BUS_HALF_LENGTH * pixels_per_metre).round() as i32;
                let front_door_back_from_nose =
                    half_length - (FRONT_DOOR_FROM_CENTRE * pixels_per_metre).round() as i32;
                for along in -half_length + 2..half_length - front_door_back_from_nose - 2 {
                    let back_from_nose = half_length - along;
                    let is_window_pillar = (back_from_nose - front_door_back_from_nose - 3) % 5 == 4;
                    if is_window_pillar {
                        continue;
                    }
                    for row in 2..=height / 2 {
                        canvas.add_glow(x + along, y - height + row, BUS_SALOON_LIGHT, 0.5 * night);
                    }
                }
                canvas.add_glow(x + half_length, y - 3, HEADLAMP, night);
                for reach in (2..26).step_by(2) {
                    let beam_strength = 0.3 * night * (1.0 - reach as f32 / 26.0);
                    canvas.add_glow(x + half_length + reach, y - 1, HEADLAMP_BEAM, beam_strength);
                    canvas.add_glow(x + half_length + reach, y - 2, HEADLAMP_BEAM, beam_strength * 0.5);
                }
                canvas.add_glow(x - half_length, y - 3, TAIL_LAMP, if bus.braking { 0.9 } else { 0.5 } * night);
            }
            BusView::Turning => {
                let (heading_cos, heading_sin) = (bus.heading.cos(), bus.heading.sin());
                let lamp_at = |along: f32, up: f32| {
                    project_to_pixel(
                        bus.metres_east + along * heading_cos + 1.3 * heading_sin,
                        bus.metres_south - along * heading_sin + 1.3 * heading_cos,
                        up,
                    )
                };
                let (nose_x, nose_y) = lamp_at(BUS_HALF_LENGTH, 0.7);
                let (tail_x, tail_y) = lamp_at(-BUS_HALF_LENGTH, 0.7);
                canvas.add_glow(nose_x, nose_y, HEADLAMP, night);
                canvas.add_glow(tail_x, tail_y, TAIL_LAMP, 0.7 * night);
                for reach in (2..18).step_by(2) {
                    let beam_strength = 0.26 * night * (1.0 - reach as f32 / 18.0);
                    let (beam_x, beam_y) = lamp_at(BUS_HALF_LENGTH + reach as f32 * 0.6, 0.0);
                    canvas.add_glow(beam_x, beam_y - 1, HEADLAMP_BEAM, beam_strength);
                }
            }
            BusView::NoseOn => {
                let half_width = (1.3 * pixels_per_metre).round() as i32;
                canvas.add_glow(x - half_width + 1, y - 2, HEADLAMP, night);
                canvas.add_glow(x + half_width - 1, y - 2, HEADLAMP, night);
            }
            BusView::TailOn => {
                let half_width = (1.3 * pixels_per_metre).round() as i32;
                canvas.add_glow(x - half_width + 1, y - 2, TAIL_LAMP, 0.8 * night);
                canvas.add_glow(x + half_width - 1, y - 2, TAIL_LAMP, 0.8 * night);
            }
        }
    }

    let warm_night = environment.season == Season::Summer
        || (environment.season == Season::Spring && environment.season_progress > 0.6);
    if !warm_night || !matches!(environment.weather, Weather::Clear | Weather::Cloudy | Weather::Fog) {
        return;
    }
    for firefly in 0..70 {
        let seconds_per_flash = 2.6 + (firefly % 7) as f64 * 0.37;
        let flashes = environment.animation_seconds / seconds_per_flash + firefly as f64 * 0.618;
        let flash = flashes.floor() as i64 as i32;
        let into_flash = flashes.fract() as f32;
        if into_flash > 0.32 {
            continue;
        }
        let metres_east = (unit_hash(firefly, flash, seed ^ 0xF1) - 0.5) * 38.0;
        let metres_south = 54.0 + unit_hash(firefly, flash + 77, seed ^ 0xF1) * 256.0;
        if !is_within_quad_lawns(metres_east, metres_south, -1.0) {
            continue;
        }
        let (x, y) = project_to_pixel(metres_east + into_flash * 1.5, metres_south, 0.8 + into_flash * 1.2);
        let brightness = (1.0 - (into_flash / 0.32 - 0.5).abs() * 2.0).max(0.0) * night;
        canvas.add_glow(x, y, rgb(214, 255, 120), 0.9 * brightness);
        canvas.add_glow(x + 1, y, rgb(160, 220, 80), 0.25 * brightness);
        canvas.add_glow(x - 1, y, rgb(160, 220, 80), 0.25 * brightness);
    }
}
