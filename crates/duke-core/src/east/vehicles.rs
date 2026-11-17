use crate::east::bus::{BusState, BusView, BUS_HALF_LENGTH, FRONT_DOOR_FROM_CENTRE, REAR_DOOR_FROM_CENTRE};
use crate::east::campus_layout::WEST_MARKHAM_AVENUE_SOUTH;
use crate::east::cycle_memory::{cycle_and_seconds_into_it, CycleMemory};
use crate::east::figures::{FramePainter, HAIR_COLOURS, SHIRTS, SKIN_TONES};
use crate::east::projection::*;
use crate::east::walkers::{remember_whether_it_turns_up, Plan, MARKHAM_CAR_SLOTS_START};
use crate::{blend_colours, hash_position, rgb, Colour, Environment};

pub const SERVICE_LOOP_SOUTH: f32 = -34.0;
const SERVICE_LOOP_CYCLE_SECONDS: f32 = 47.0;
const MARKHAM_AVENUE_LANE_FROM_CENTRE: f32 = 2.4;
const MARKHAM_CAR_COLOURS: [Colour; 4] = [rgb(200, 204, 210), rgb(150, 40, 44), rgb(50, 66, 110), rgb(44, 46, 54)];
const SERVICE_CAR_COLOUR: Colour = rgb(196, 198, 204);

const BUS_WHITE: Colour = rgb(236, 238, 240);
const BUS_PANEL: Colour = rgb(206, 210, 216);
const BUS_DUKE_BLUE: Colour = rgb(30, 52, 140);
const BUS_DARK_GLASS: Colour = rgb(40, 52, 72);
const BUS_DESTINATION_SIGN: Colour = rgb(250, 200, 60);
const BUS_HEADLAMP: Colour = rgb(255, 246, 200);
const BUS_BRAKE_LIGHT: Colour = rgb(255, 60, 40);
const BUS_TAIL_LIGHT: Colour = rgb(150, 30, 30);
const BUS_AMBER: Colour = rgb(255, 170, 40);
const BUS_TYRE: Colour = rgb(20, 20, 26);
const BUS_SKIRT: Colour = rgb(80, 84, 96);
const BUS_BUMPER: Colour = rgb(60, 64, 76);
const BUS_HEIGHT: f32 = 3.1;
const BUS_HALF_WIDTH: f32 = 1.3;

#[derive(Copy, Clone)]
pub struct Car {
    pub metres_east: f32,
    pub metres_south: f32,
    pub direction: f32,
    pub body: Colour,
}

pub fn service_loop_car(environment: &Environment) -> Option<Car> {
    let (lap, seconds_into_lap) =
        cycle_and_seconds_into_it(environment.animation_seconds + 12.0, SERVICE_LOOP_CYCLE_SECONDS as f64);
    let share_of_lap = seconds_into_lap / SERVICE_LOOP_CYCLE_SECONDS;
    if share_of_lap > 0.25 {
        return None;
    }
    let progress = share_of_lap / 0.25;
    let heading_west = lap & 1 == 0;
    let metres_east = if heading_west { 58.0 - 62.0 * progress } else { -4.0 + 62.0 * progress };
    Some(Car {
        metres_east,
        metres_south: SERVICE_LOOP_SOUTH,
        direction: if heading_west { -1.0 } else { 1.0 },
        body: SERVICE_CAR_COLOUR,
    })
}

pub fn markham_avenue_cars(
    environment: &Environment,
    seed: u32,
    busyness: f32,
    memory: &mut CycleMemory<Plan>,
) -> [Option<Car>; 4] {
    let mut cars = [None; 4];
    for (lane_slot, car) in cars.iter_mut().enumerate() {
        let cycle_seconds = 19.0 + lane_slot as f64 * 7.3;
        let (trip, seconds_into_trip) =
            cycle_and_seconds_into_it(environment.animation_seconds + lane_slot as f64 * 11.0, cycle_seconds);
        let turns_up = remember_whether_it_turns_up(
            memory,
            MARKHAM_CAR_SLOTS_START + lane_slot,
            trip,
            seconds_into_trip,
            environment,
            seed,
            || busyness > 0.12 || lane_slot == 0,
        );
        if !turns_up {
            continue;
        }
        let share_of_trip = seconds_into_trip / cycle_seconds as f32;
        let heading_east = lane_slot & 1 == 0;
        let metres_east = if heading_east { -170.0 + 340.0 * share_of_trip } else { 170.0 - 340.0 * share_of_trip };
        let lane_offset = if heading_east { MARKHAM_AVENUE_LANE_FROM_CENTRE } else { -MARKHAM_AVENUE_LANE_FROM_CENTRE };
        *car = Some(Car {
            metres_east,
            metres_south: WEST_MARKHAM_AVENUE_SOUTH + lane_offset,
            direction: if heading_east { 1.0 } else { -1.0 },
            body: MARKHAM_CAR_COLOURS[((trip + lane_slot as i32).rem_euclid(4)) as usize],
        });
    }
    cars
}

pub fn draw_car_side_on(painter: &mut FramePainter, car: &Car) {
    let depth = car.metres_south;
    let (centre_x, foot_y) = project_to_pixel(car.metres_east, depth, 0.0);
    let half_length = (2.2 * pixels_per_metre_at(depth)).round() as i32;
    let forward = if car.direction >= 0.0 { 1 } else { -1 };
    for along in -half_length..=half_length {
        let is_under_the_cabin = along * forward > -half_length / 2 - 1 && along * forward < half_length - 2;
        if is_under_the_cabin {
            painter.put(centre_x + along, foot_y - 4, depth, blend_colours(car.body, 0xFFFFFF, 0.2));
            painter.put(centre_x + along, foot_y - 3, depth, rgb(54, 70, 92));
        }
        painter.put(centre_x + along, foot_y - 2, depth, car.body);
        let is_a_wheel = along.abs() == half_length - 2;
        painter.put(
            centre_x + along,
            foot_y - 1,
            depth,
            if is_a_wheel { rgb(20, 20, 26) } else { blend_colours(car.body, 0, 0.3) },
        );
    }
}

pub fn draw_bus(painter: &mut FramePainter, bus: &BusState, night: f32) {
    let pixels_per_metre = pixels_per_metre_at(bus.metres_south);
    let (centre_x, foot_y) = project_to_pixel(bus.metres_east, bus.metres_south, 0.0);
    let saloon_is_lit = night > 0.5;
    let saloon_glass = if saloon_is_lit { rgb(120, 110, 84) } else { rgb(50, 66, 90) };
    let depth = bus.drawing_depth();
    let height = (BUS_HEIGHT * screen_rows_per_metre_of_height_at(bus.metres_south)).round() as i32;
    match bus.view {
        BusView::SideOn => {
            let half_length = (BUS_HALF_LENGTH * pixels_per_metre).round() as i32;
            let window_top = 2;
            let window_bottom = height / 2;
            let front_door_back_from_nose = half_length - (FRONT_DOOR_FROM_CENTRE * pixels_per_metre).round() as i32;
            let rear_door_back_from_nose = half_length - (REAR_DOOR_FROM_CENTRE * pixels_per_metre).round() as i32;
            let open_doorway = if saloon_is_lit { rgb(250, 226, 150) } else { rgb(22, 26, 34) };
            for row in 0..height {
                for along in -half_length..=half_length {
                    let back_from_nose = half_length - along;
                    let is_a_door = back_from_nose == front_door_back_from_nose
                        || back_from_nose == front_door_back_from_nose + 1
                        || back_from_nose == rear_door_back_from_nose
                        || back_from_nose == rear_door_back_from_nose + 1;
                    let is_window_row = row >= window_top && row <= window_bottom;
                    let colour = if row == 0 {
                        BUS_PANEL
                    } else if is_a_door && row >= window_top && row < height - 1 {
                        if bus.doors_open {
                            if row == height - 2 {
                                rgb(196, 198, 196)
                            } else {
                                open_doorway
                            }
                        } else if row <= window_bottom + 1 {
                            BUS_DARK_GLASS
                        } else {
                            BUS_PANEL
                        }
                    } else if is_window_row && (1..=2).contains(&back_from_nose) {
                        BUS_DARK_GLASS
                    } else if is_window_row
                        && back_from_nose > front_door_back_from_nose + 2
                        && along > -half_length + 1
                        && (back_from_nose - front_door_back_from_nose - 3) % 5 != 4
                    {
                        saloon_glass
                    } else if row == window_bottom + 2 || row == window_bottom + 3 {
                        BUS_DUKE_BLUE
                    } else if row == height - 1 {
                        let is_front_wheel = (back_from_nose - front_door_back_from_nose - 4).abs() <= 1;
                        let is_rear_wheel = (back_from_nose - (2 * half_length - 6)).abs() <= 1;
                        if is_front_wheel || is_rear_wheel {
                            BUS_TYRE
                        } else {
                            BUS_SKIRT
                        }
                    } else {
                        BUS_WHITE
                    };
                    painter.put(centre_x + along, foot_y - height + row, depth, colour);
                }
            }
            for along in -half_length / 3..=half_length / 3 {
                painter.put(centre_x + along, foot_y - height - 1, depth, BUS_PANEL);
            }
            painter.put(centre_x + half_length - 1, foot_y - height + 1, depth, BUS_DESTINATION_SIGN);
            painter.put(centre_x + half_length - 2, foot_y - height + 1, depth, BUS_DESTINATION_SIGN);
            painter.put(centre_x + half_length, foot_y - 3, depth, BUS_HEADLAMP);
            let tail_light = if bus.braking { BUS_BRAKE_LIGHT } else { BUS_TAIL_LIGHT };
            painter.put(centre_x - half_length, foot_y - 3, depth, tail_light);
            painter.put(centre_x - half_length, foot_y - 4, depth, tail_light);
            if bus.hazard_lights_on && bus.stopped {
                painter.put(centre_x + half_length, foot_y - 5, depth, BUS_AMBER);
                painter.put(centre_x - half_length, foot_y - 5, depth, BUS_AMBER);
            }
            let pane_count = ((2 * half_length - front_door_back_from_nose - 4) / 5).max(1);
            let riders_per_pane = bus.riders_aboard / pane_count as f32;
            let is_packed = riders_per_pane >= 3.0;
            let window_bottom_y = foot_y - height + window_bottom;
            for pane in 0..pane_count {
                let pane_left_x = centre_x + half_length - (front_door_back_from_nose + 3 + pane * 5) - 3;
                if pane_left_x < centre_x - half_length + 2 {
                    continue;
                }
                if is_packed {
                    for column in 0..4 {
                        let look = hash_position(pane * 5 + column, bus.id as i32, 0xB0D);
                        painter.put(pane_left_x + column, window_bottom_y, depth, SKIN_TONES[(look % 4) as usize]);
                        let second_row = if column & 1 == 0 {
                            HAIR_COLOURS[((look >> 3) % 5) as usize]
                        } else {
                            SKIN_TONES[((look >> 5) % 4) as usize]
                        };
                        painter.put(pane_left_x + column, window_bottom_y - 1, depth, second_row);
                        if window_bottom - 2 >= window_top {
                            let third_row = if column & 1 == 1 {
                                HAIR_COLOURS[((look >> 8) % 5) as usize]
                            } else {
                                SHIRTS[((look >> 6) % 12) as usize]
                            };
                            painter.put(pane_left_x + column, window_bottom_y - 2, depth, third_row);
                        }
                    }
                    continue;
                }
                let pane_hash = hash_position(pane, bus.id as i32, 0xB0B);
                let heads_in_pane =
                    (riders_per_pane + (pane_hash % 100) as f32 / 100.0 - 0.3).floor().clamp(0.0, 3.0) as i32;
                for head in 0..heads_in_pane {
                    let look = hash_position(pane * 5 + head, bus.id as i32, 0xB0C);
                    let column = pane_left_x + (head * 2 + (pane_hash >> 8) as i32 % 2).min(3);
                    painter.put(column, window_bottom_y, depth, SKIN_TONES[(look % 4) as usize]);
                    painter.put(column, window_bottom_y - 1, depth, HAIR_COLOURS[((look >> 3) % 5) as usize]);
                }
            }
            for along in -half_length..=half_length {
                painter.blend(centre_x + along, foot_y, depth, rgb(10, 12, 24), 0.4);
            }
        }
        BusView::Turning => {
            let (heading_cos, heading_sin) = (bus.heading.cos(), bus.heading.sin());
            let (along_east, along_south) = (heading_cos, -heading_sin);
            let (door_side_east, door_side_south) = (heading_sin, heading_cos);
            let heading_north = heading_sin > 0.0;
            let metres_per_sample = 0.42 / pixels_per_metre;
            let samples_along = (2.0 * BUS_HALF_LENGTH / metres_per_sample).ceil() as i32;
            let samples_across = (2.6 / metres_per_sample).ceil() as i32;
            let samples_up = (BUS_HEIGHT * screen_rows_per_metre_of_height_at(bus.metres_south) * 1.6).ceil() as i32;
            let pixel_at = |along: f32, across: f32, up: f32| {
                project_to_pixel(
                    bus.metres_east + along * along_east + across * door_side_east,
                    bus.metres_south + along * along_south + across * door_side_south,
                    up,
                )
            };
            for along_sample in 0..=samples_along {
                let along = -BUS_HALF_LENGTH + 2.0 * BUS_HALF_LENGTH * along_sample as f32 / samples_along as f32;
                for across_sample in 0..=samples_across {
                    let across = -BUS_HALF_WIDTH + 2.6 * across_sample as f32 / samples_across as f32;
                    let (x, y) = pixel_at(along, across, BUS_HEIGHT);
                    let is_roof_unit = along.abs() < 2.2 && across.abs() < 0.7;
                    let is_roof_edge = across.abs() > 1.05 || along.abs() > BUS_HALF_LENGTH - 0.3;
                    painter.put(x, y, depth, if is_roof_unit || is_roof_edge { BUS_PANEL } else { BUS_WHITE });
                }
            }
            let visible_end = if heading_north { -BUS_HALF_LENGTH } else { BUS_HALF_LENGTH };
            for across_sample in 0..=samples_across {
                let across = -BUS_HALF_WIDTH + 2.6 * across_sample as f32 / samples_across as f32;
                for up_sample in 0..=samples_up {
                    let up = BUS_HEIGHT * up_sample as f32 / samples_up as f32;
                    let colour = if up > 1.45 && up < 2.7 && across.abs() < 1.05 {
                        if heading_north {
                            saloon_glass
                        } else {
                            BUS_DARK_GLASS
                        }
                    } else if up >= 2.7 && !heading_north && across.abs() < 1.0 {
                        BUS_DESTINATION_SIGN
                    } else if up > 0.45 && up < 0.95 && across.abs() > 0.75 {
                        if !heading_north {
                            BUS_HEADLAMP
                        } else if bus.braking {
                            BUS_BRAKE_LIGHT
                        } else {
                            BUS_TAIL_LIGHT
                        }
                    } else if up > 0.95 && up < 1.3 {
                        BUS_DUKE_BLUE
                    } else if up < 0.35 {
                        BUS_BUMPER
                    } else {
                        BUS_WHITE
                    };
                    let (x, y) = pixel_at(visible_end, across, up);
                    painter.put(x, y, depth, colour);
                }
            }
            for along_sample in 0..=samples_along {
                let along = -BUS_HALF_LENGTH + 2.0 * BUS_HALF_LENGTH * along_sample as f32 / samples_along as f32;
                let is_a_door =
                    (along - FRONT_DOOR_FROM_CENTRE).abs() < 0.38 || (along - REAR_DOOR_FROM_CENTRE).abs() < 0.38;
                let is_a_pane = ((along + BUS_HALF_LENGTH) / 1.5).fract() > 0.16
                    && along < FRONT_DOOR_FROM_CENTRE - 0.6
                    && along > -BUS_HALF_LENGTH + 0.4;
                let is_a_wheel = (along - 3.4).abs() < 0.55 || (along + 3.9).abs() < 0.55;
                for up_sample in 0..=samples_up {
                    let up = BUS_HEIGHT * up_sample as f32 / samples_up as f32;
                    let is_window_band = up > 1.3 && up < 2.55;
                    let colour = if up > 2.8 {
                        BUS_PANEL
                    } else if (is_a_door && up > 0.35 && up < 2.55)
                        || (is_window_band && along > BUS_HALF_LENGTH - 0.75)
                    {
                        BUS_DARK_GLASS
                    } else if is_window_band && is_a_pane {
                        saloon_glass
                    } else if up > 0.4 && up < 1.0 {
                        BUS_DUKE_BLUE
                    } else if up <= 0.4 {
                        if is_a_wheel {
                            BUS_TYRE
                        } else {
                            BUS_SKIRT
                        }
                    } else {
                        BUS_WHITE
                    };
                    let (x, y) = pixel_at(along, BUS_HALF_WIDTH, up);
                    painter.put(x, y, depth, colour);
                }
            }
        }
        BusView::TailOn | BusView::NoseOn => {
            let half_width = (BUS_HALF_WIDTH * pixels_per_metre).round() as i32;
            let roof_rows = (11.0 * pixels_per_metre * ground_foreshortening_at(bus.metres_south)).round() as i32;
            let nose_on = bus.view == BusView::NoseOn;
            for row in 0..roof_rows {
                for across in -half_width..=half_width {
                    painter.put(
                        centre_x + across,
                        foot_y - height - roof_rows + row,
                        depth,
                        if across.abs() == half_width { BUS_PANEL } else { BUS_WHITE },
                    );
                }
            }
            for row in 0..height {
                for across in -half_width..=half_width {
                    let colour = if row >= 1 && row <= height / 2 && across.abs() < half_width {
                        if nose_on {
                            BUS_DARK_GLASS
                        } else {
                            saloon_glass
                        }
                    } else if row == height / 2 + 2 {
                        BUS_DUKE_BLUE
                    } else if row == height - 2 && across.abs() == half_width - 1 {
                        if nose_on {
                            BUS_HEADLAMP
                        } else {
                            BUS_BRAKE_LIGHT
                        }
                    } else if row == height - 1 {
                        BUS_BUMPER
                    } else {
                        BUS_WHITE
                    };
                    painter.put(centre_x + across, foot_y - height + row, depth, colour);
                }
            }
            if nose_on {
                for across in -half_width + 1..half_width {
                    painter.put(centre_x + across, foot_y - height, depth, BUS_DESTINATION_SIGN);
                }
            }
        }
    }
}
