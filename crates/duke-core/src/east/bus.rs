use crate::east::animation::Actor;
use crate::east::campus_layout::{
    cross_walks_south, hall_entrances_south, BUS_PLAZA_BENCHES_EAST, BUS_PLAZA_BENCHES_SOUTH, BUS_PLAZA_SOUTH_EDGE,
    ENTRANCE_LOOP_SOUTH_KERB,
};
use crate::east::cycle_memory::CycleMemory;
use crate::east::figures::{stride_frame, Person, Pose};
use crate::east::paths::{path_length, place_along, Point};
use crate::east::shading_tools::unit_hash;
use crate::{hash_position, Environment, Weather};
use std::f32::consts::FRAC_PI_2;
use std::sync::OnceLock;

pub const CYCLE_SECONDS: f64 = 96.0;
const CYCLE_PHASE_SECONDS: f64 = 20.0;
pub const RUSH_HOURS: [f32; 6] = [8.22, 9.87, 11.57, 13.12, 14.82, 16.47];
const RIDERS_PER_BUS_IN_A_RUSH: usize = 21;
pub const SECONDS_BETWEEN_BOARDERS: f32 = 0.42;
pub const SECONDS_BETWEEN_ALIGHTERS: f32 = 0.55;
pub const FIRST_BOARDER_AFTER_DOORS_OPEN: f32 = 0.3;
pub const FIRST_ALIGHTER_AFTER_DOORS_OPEN: f32 = 0.6;
pub const SECONDS_TO_WALK_INTO_THE_BUS: f32 = 1.7;

const STOP_METRES_EAST: f32 = 8.6;
const HOLDING_PLACE_METRES_EAST: f32 = -4.7;
const LOOP_SIDE_LEGS_FROM_AXIS: f32 = 32.9;
const LOOP_CORNER_RADIUS: f32 = 16.0;
const BUSES_APPEAR_AT_SOUTH: f32 = 400.0;
const SECONDS_TO_DRIVE_IN: f32 = 14.0;
const SECONDS_TO_DRIVE_OUT: f32 = 11.5;
pub const BUS_HALF_LENGTH: f32 = 6.0;
pub const FRONT_DOOR_FROM_CENTRE: f32 = 4.5;
pub const REAR_DOOR_FROM_CENTRE: f32 = -0.8;
const LANE_SOUTH: f32 = ENTRANCE_LOOP_SOUTH_KERB - 1.5;
const FRONT_DOOR: Point = (STOP_METRES_EAST + FRONT_DOOR_FROM_CENTRE, ENTRANCE_LOOP_SOUTH_KERB + 0.45);
const REAR_DOOR: Point = (STOP_METRES_EAST + REAR_DOOR_FROM_CENTRE, ENTRANCE_LOOP_SOUTH_KERB + 0.45);

#[derive(Copy, Clone, Debug)]
pub struct BusTimes {
    pub enters: f32,
    pub reaches_holding_place: f32,
    pub moves_up: f32,
    pub stops: f32,
    pub doors_open: f32,
    pub doors_close: f32,
    pub leaves: f32,
    pub boarders: usize,
    pub alighters: usize,
    pub aboard_on_arrival: usize,
}

#[derive(Clone, Debug)]
pub struct BusCycle {
    pub is_rush: bool,
    pub riders_waiting: usize,
    pub raining: bool,
    pub buses: Vec<BusTimes>,
}

#[derive(Copy, Clone)]
pub struct Crowd {
    is_rush: bool,
    riders_waiting: usize,
    raining: bool,
}

pub fn cycle_start_time(cycle: i64) -> f64 {
    cycle as f64 * CYCLE_SECONDS - CYCLE_PHASE_SECONDS
}

pub fn is_timed_rush(cycle: i64) -> bool {
    cycle.rem_euclid(10) == 6
}

pub struct Timetable<'a> {
    pub environment: &'a Environment,
    pub seed: u32,
    pub busyness: f32,
    pub current_cycle: i64,
    pub seconds_into_current_cycle: f32,
}

impl<'a> Timetable<'a> {
    pub fn now(environment: &'a Environment, seed: u32, busyness: f32) -> Timetable<'a> {
        let cycle = ((environment.animation_seconds + CYCLE_PHASE_SECONDS) / CYCLE_SECONDS).floor();
        let seconds_into_current_cycle =
            (environment.animation_seconds + CYCLE_PHASE_SECONDS - cycle * CYCLE_SECONDS) as f32;
        Timetable { environment, seed, busyness, current_cycle: cycle as i64, seconds_into_current_cycle }
    }

    pub fn cycle(&self, cycle: i64, memory: &mut CycleMemory<Crowd>) -> BusCycle {
        let environment = self.environment;
        let seed = self.seed;
        let cycle_number = cycle as i32;
        let clock_hour_at_cycle_start = environment.hour - self.seconds_into_current_cycle / 3600.0
            + (cycle - self.current_cycle) as f32 * (CYCLE_SECONDS / 3600.0) as f32;
        let cycle_hash = hash_position(cycle_number, 77, seed ^ 0xB05);
        let crowd = memory.recall_or_decide(
            cycle.rem_euclid(8) as usize,
            cycle,
            clock_hour_at_cycle_start,
            environment,
            seed,
            || {
                let rush_is_possible = environment.daylight() > 0.35 && environment.weather != Weather::Storm;
                let clock_is_at_a_rush_hour = {
                    let hour = clock_hour_at_cycle_start.rem_euclid(24.0);
                    let cycle_hours = (CYCLE_SECONDS / 3600.0) as f32;
                    RUSH_HOURS.iter().any(|rush_hour| hour >= *rush_hour && hour < rush_hour + cycle_hours)
                };
                let is_rush = rush_is_possible && (is_timed_rush(cycle) || clock_is_at_a_rush_hour);
                let riders_waiting = if is_rush {
                    46 + (cycle_hash % 13) as usize
                } else {
                    let usual_queue = 2.0 + (cycle_hash % 7) as f32;
                    (usual_queue * (0.25 + 1.1 * self.busyness)).round().clamp(0.0, 12.0) as usize
                };
                let raining = matches!(environment.weather, Weather::Rain | Weather::Storm)
                    && environment.weather_intensity > 0.25;
                Crowd { is_rush, riders_waiting, raining }
            },
        );
        let bus_count = if crowd.is_rush {
            3
        } else if (cycle_hash >> 8).is_multiple_of(5) {
            2
        } else {
            1
        };
        let mut buses: Vec<BusTimes> = Vec::with_capacity(bus_count);
        let mut riders_still_waiting = crowd.riders_waiting;
        for place_in_line in 0..bus_count {
            let bus_hash = hash_position(cycle_number, place_in_line as i32, seed ^ 0xB07);
            let is_last_bus = place_in_line == bus_count - 1;
            let boarders = match (crowd.is_rush, place_in_line) {
                (true, _) if is_last_bus => riders_still_waiting,
                (true, _) => riders_still_waiting.min(RIDERS_PER_BUS_IN_A_RUSH),
                (false, 0) => riders_still_waiting,
                (false, _) => 0,
            };
            riders_still_waiting -= boarders;
            let alighters = if crowd.is_rush { 3 + (bus_hash % 3) as usize } else { (1 + bus_hash % 6) as usize };
            let staying_on =
                if crowd.is_rush { 6 + (bus_hash >> 4) as usize % 6 } else { 2 + (bus_hash >> 4) as usize % 9 };
            let (enters, reaches_holding_place, moves_up, stops) = match buses.last() {
                None => (0.0, SECONDS_TO_DRIVE_IN, SECONDS_TO_DRIVE_IN, SECONDS_TO_DRIVE_IN),
                Some(bus_in_front) => {
                    let enters = (bus_in_front.leaves - SECONDS_TO_DRIVE_IN - 3.0).max(bus_in_front.enters + 7.0);
                    let moves_up = bus_in_front.leaves + 1.2;
                    (enters, enters + SECONDS_TO_DRIVE_IN - 1.0, moves_up, moves_up + 3.6)
                }
            };
            let doors_open = stops + 1.0;
            let last_boarder_inside = if boarders > 0 {
                FIRST_BOARDER_AFTER_DOORS_OPEN
                    + (boarders - 1) as f32 * SECONDS_BETWEEN_BOARDERS
                    + SECONDS_TO_WALK_INTO_THE_BUS
            } else {
                0.0
            };
            let last_alighter_out = if alighters > 0 {
                FIRST_ALIGHTER_AFTER_DOORS_OPEN + (alighters - 1) as f32 * SECONDS_BETWEEN_ALIGHTERS
            } else {
                0.0
            };
            let doors_close = doors_open + last_boarder_inside.max(last_alighter_out).max(2.5) + 0.5;
            buses.push(BusTimes {
                enters,
                reaches_holding_place,
                moves_up,
                stops,
                doors_open,
                doors_close,
                leaves: doors_close + 0.8,
                boarders,
                alighters,
                aboard_on_arrival: alighters + staying_on,
            });
        }
        BusCycle { is_rush: crowd.is_rush, riders_waiting: crowd.riders_waiting, raining: crowd.raining, buses }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum BusView {
    SideOn,
    TailOn,
    Turning,
    NoseOn,
}

#[derive(Copy, Clone, Debug)]
pub struct BusState {
    pub metres_east: f32,
    pub metres_south: f32,
    pub view: BusView,
    pub heading: f32,
    pub braking: bool,
    pub stopped: bool,
    pub doors_open: bool,
    pub riders_aboard: f32,
    pub hazard_lights_on: bool,
    pub id: u32,
}

impl BusState {
    pub fn drawing_depth(&self) -> f32 {
        self.metres_south + 1.3
    }
}

const SIDE_LEG_LENGTH: f32 = BUSES_APPEAR_AT_SOUTH - (LANE_SOUTH + LOOP_CORNER_RADIUS);
const CORNER_LENGTH: f32 = LOOP_CORNER_RADIUS * FRAC_PI_2;
const NORTH_LEG_LENGTH: f32 = 2.0 * (LOOP_SIDE_LEGS_FROM_AXIS - LOOP_CORNER_RADIUS);

fn distance_round_the_loop_to(metres_east_on_the_north_leg: f32) -> f32 {
    SIDE_LEG_LENGTH + CORNER_LENGTH + (metres_east_on_the_north_leg + LOOP_SIDE_LEGS_FROM_AXIS - LOOP_CORNER_RADIUS)
}

fn place_round_the_loop(distance: f32) -> (f32, f32, f32) {
    if distance < SIDE_LEG_LENGTH {
        return (-LOOP_SIDE_LEGS_FROM_AXIS, BUSES_APPEAR_AT_SOUTH - distance, FRAC_PI_2);
    }
    let into_first_corner = distance - SIDE_LEG_LENGTH;
    if into_first_corner < CORNER_LENGTH {
        let angle = into_first_corner / LOOP_CORNER_RADIUS;
        return (
            -LOOP_SIDE_LEGS_FROM_AXIS + LOOP_CORNER_RADIUS - LOOP_CORNER_RADIUS * angle.cos(),
            LANE_SOUTH + LOOP_CORNER_RADIUS - LOOP_CORNER_RADIUS * angle.sin(),
            FRAC_PI_2 - angle,
        );
    }
    let into_north_leg = into_first_corner - CORNER_LENGTH;
    if into_north_leg < NORTH_LEG_LENGTH {
        return (-LOOP_SIDE_LEGS_FROM_AXIS + LOOP_CORNER_RADIUS + into_north_leg, LANE_SOUTH, 0.0);
    }
    let into_second_corner = into_north_leg - NORTH_LEG_LENGTH;
    if into_second_corner < CORNER_LENGTH {
        let angle = into_second_corner / LOOP_CORNER_RADIUS;
        return (
            LOOP_SIDE_LEGS_FROM_AXIS - LOOP_CORNER_RADIUS + LOOP_CORNER_RADIUS * angle.sin(),
            LANE_SOUTH + LOOP_CORNER_RADIUS - LOOP_CORNER_RADIUS * angle.cos(),
            -angle,
        );
    }
    (LOOP_SIDE_LEGS_FROM_AXIS, LANE_SOUTH + LOOP_CORNER_RADIUS + (into_second_corner - CORNER_LENGTH), -FRAC_PI_2)
}

pub fn bus_on_the_road(times: &BusTimes, place_in_line: usize, seconds: f32, id: u32) -> Option<BusState> {
    if seconds < times.enters || seconds > times.leaves + SECONDS_TO_DRIVE_OUT {
        return None;
    }
    let whole_loop = 2.0 * SIDE_LEG_LENGTH + 2.0 * CORNER_LENGTH + NORTH_LEG_LENGTH;
    let at_the_stop = distance_round_the_loop_to(STOP_METRES_EAST);
    let at_the_holding_place =
        if place_in_line == 0 { at_the_stop } else { distance_round_the_loop_to(HOLDING_PLACE_METRES_EAST) };
    let (distance, braking, stopped) = if seconds < times.reaches_holding_place {
        let progress = (seconds - times.enters) / (times.reaches_holding_place - times.enters);
        (at_the_holding_place * (1.0 - (1.0 - progress).powf(1.5)), progress > 0.6, false)
    } else if seconds < times.moves_up {
        (at_the_holding_place, true, true)
    } else if seconds < times.stops {
        let progress = (seconds - times.moves_up) / (times.stops - times.moves_up);
        let eased = progress.clamp(0.0, 1.0);
        let eased = eased * eased * (3.0 - 2.0 * eased);
        (at_the_holding_place + (at_the_stop - at_the_holding_place) * eased, progress > 0.6, false)
    } else if seconds < times.leaves {
        let settling_lurch = if seconds - times.stops < 0.25 { 0.3 } else { 0.0 };
        (at_the_stop + settling_lurch, true, true)
    } else {
        let progress = (seconds - times.leaves) / SECONDS_TO_DRIVE_OUT;
        (at_the_stop + (whole_loop - at_the_stop) * progress.powf(1.7), false, false)
    };
    let (metres_east, metres_south, heading) = place_round_the_loop(distance);
    let heading_degrees = heading.to_degrees();
    let view = if heading_degrees > 82.0 {
        BusView::TailOn
    } else if heading_degrees < -82.0 {
        BusView::NoseOn
    } else if heading_degrees.abs() < 4.0 {
        BusView::SideOn
    } else {
        BusView::Turning
    };
    let stepped_off =
        ((seconds - times.doors_open - FIRST_ALIGHTER_AFTER_DOORS_OPEN) / SECONDS_BETWEEN_ALIGHTERS).floor() + 1.0;
    let stepped_on = ((seconds - times.doors_open - FIRST_BOARDER_AFTER_DOORS_OPEN - SECONDS_TO_WALK_INTO_THE_BUS)
        / SECONDS_BETWEEN_BOARDERS)
        .floor()
        + 1.0;
    Some(BusState {
        metres_east,
        metres_south,
        view,
        heading,
        braking,
        stopped,
        doors_open: seconds >= times.doors_open && seconds < times.doors_close,
        riders_aboard: times.aboard_on_arrival as f32 - stepped_off.clamp(0.0, times.alighters as f32)
            + stepped_on.clamp(0.0, times.boarders as f32),
        hazard_lights_on: ((seconds * 2.6) as i32) & 1 == 0,
        id,
    })
}

pub fn buses_in_view(timetable: &Timetable, memory: &mut CycleMemory<Crowd>) -> Vec<BusState> {
    let mut buses = Vec::new();
    let current_cycle = timetable.current_cycle;
    for cycle in [current_cycle - 1, current_cycle] {
        let bus_cycle = timetable.cycle(cycle, memory);
        let seconds_into_cycle =
            timetable.seconds_into_current_cycle + (current_cycle - cycle) as f32 * CYCLE_SECONDS as f32;
        for (place_in_line, times) in bus_cycle.buses.iter().enumerate() {
            let id = (cycle as u32).wrapping_mul(4).wrapping_add(place_in_line as u32);
            buses.extend(bus_on_the_road(times, place_in_line, seconds_into_cycle, id));
        }
    }
    buses
}

fn waiting_spots_nearest_the_door_first() -> &'static [Point] {
    static SPOTS: OnceLock<Vec<Point>> = OnceLock::new();
    SPOTS.get_or_init(|| {
        let mut scored_spots: Vec<(Point, f32)> = Vec::new();
        let mut row = 0;
        let mut row_south = ENTRANCE_LOOP_SOUTH_KERB + 0.7;
        while row_south < 357.5 {
            let mut column_east = -24.0f32 + (row % 2) as f32 * 0.6;
            while column_east < 26.0 {
                let column_key = (column_east * 10.0) as i32;
                let metres_east = column_east + (unit_hash(row, column_key, 0x5707) - 0.5) * 0.5;
                let metres_south = row_south + (unit_hash(row, column_key, 0x5708) - 0.5) * 0.5;
                let is_on_a_bench = (metres_south - BUS_PLAZA_BENCHES_SOUTH).abs() < 0.7
                    && BUS_PLAZA_BENCHES_EAST.iter().any(|bench| (metres_east - bench).abs() < 1.2);
                if !is_on_a_bench {
                    let distance_from_the_door = (((metres_east - FRONT_DOOR.0 + 2.5) * 0.78).powi(2)
                        + ((metres_south - FRONT_DOOR.1) * 0.72).powi(2))
                    .sqrt();
                    let score = distance_from_the_door + unit_hash(row, (column_east * 7.0) as i32, 0x5709) * 1.6;
                    scored_spots.push(((metres_east, metres_south), score));
                }
                column_east += 1.3;
            }
            row_south += 1.15;
            row += 1;
        }
        scored_spots.sort_by(|first, second| first.1.total_cmp(&second.1));
        scored_spots.into_iter().take(64).map(|(spot, _)| spot).collect()
    })
}

fn ways_to_the_plaza() -> &'static [Vec<Point>] {
    static WAYS: OnceLock<Vec<Vec<Point>>> = OnceLock::new();
    WAYS.get_or_init(|| {
        let mut ways = Vec::new();
        for side in [-1.0f32, 1.0] {
            let last_gap = cross_walks_south(side)[3];
            let last_hall_door = hall_entrances_south(side)[3];
            let union_circle_gap = cross_walks_south(side)[2];
            let over_the_zebra = [
                (side * 5.8, 319.5),
                (side * 5.8, 332.4),
                (side * 12.0, 333.3),
                (side * 18.6, 334.8),
                (side * 18.6, ENTRANCE_LOOP_SOUTH_KERB + 0.9),
            ];
            let through_the_last_gap = [
                (side * 46.0, last_gap),
                (side * 27.0, last_gap),
                (side * 24.5, last_gap + 7.0),
                (side * 24.5, 291.0),
                (side * 17.0, 309.5),
                (side * 9.0, 315.0),
            ];
            let out_of_the_nearest_hall = [
                (side * 32.4, last_hall_door),
                (side * 25.0, last_hall_door),
                (side * 24.0, 300.0),
                (side * 17.0, 309.5),
                (side * 9.0, 315.0),
            ];
            let down_from_union_circle = [
                (side * 46.0, union_circle_gap),
                (side * 27.0, union_circle_gap),
                (side * 24.5, 205.0),
                (side * 24.5, 291.0),
                (side * 17.0, 309.5),
                (side * 9.0, 315.0),
            ];
            let foot_of_the_brick_walk = if side < 0.0 { -18.6 } else { 16.8 };
            ways.push([&through_the_last_gap[..], &over_the_zebra].concat());
            ways.push([&out_of_the_nearest_hall[..], &over_the_zebra].concat());
            ways.push([&down_from_union_circle[..], &over_the_zebra].concat());
            ways.push(vec![(foot_of_the_brick_walk, 398.0), (side * 11.4, BUS_PLAZA_SOUTH_EDGE + 0.4)]);
            ways.push(vec![
                (side * 38.4, 398.0),
                (side * 38.4, 345.0),
                (side * 35.0, 336.0),
                (side * 26.0, 334.2),
                (side * 18.6, 334.8),
                (side * 18.6, ENTRANCE_LOOP_SOUTH_KERB + 0.9),
            ]);
        }
        ways
    })
}

pub fn add_bus_stop_crowd(timetable: &Timetable, actors: &mut Vec<Actor>, memory: &mut CycleMemory<Crowd>) {
    let environment = timetable.environment;
    let seed = timetable.seed;
    let current_cycle = timetable.current_cycle;
    let spots = waiting_spots_nearest_the_door_first();
    let ways = ways_to_the_plaza();
    let mut path: Vec<Point> = Vec::with_capacity(16);

    actors.extend(buses_in_view(timetable, memory).into_iter().map(Actor::Bus));

    for cycle in current_cycle..current_cycle + 4 {
        let bus_cycle = timetable.cycle(cycle, memory);
        let seconds_into_cycle =
            timetable.seconds_into_current_cycle - (cycle - current_cycle) as f32 * CYCLE_SECONDS as f32;
        let cycle_number = cycle as i32;
        for rider in 0..bus_cycle.riders_waiting {
            let look = hash_position(cycle_number, rider as i32, seed ^ 0x51A);
            let (bus_index, place_in_queue) = if bus_cycle.is_rush {
                let bus_index = (rider / RIDERS_PER_BUS_IN_A_RUSH).min(bus_cycle.buses.len() - 1);
                (bus_index, rider - bus_index * RIDERS_PER_BUS_IN_A_RUSH)
            } else {
                (0, rider)
            };
            let bus = &bus_cycle.buses[bus_index];
            let turn_to_board =
                bus.doors_open + FIRST_BOARDER_AFTER_DOORS_OPEN + place_in_queue as f32 * SECONDS_BETWEEN_BOARDERS;
            if seconds_into_cycle >= turn_to_board + SECONDS_TO_WALK_INTO_THE_BUS {
                continue;
            }
            let share_of_the_queue = (rider as f32 + 0.5) / bus_cycle.riders_waiting as f32;
            let arrival_jitter = unit_hash(cycle_number, rider as i32 + 500, seed) - 0.5;
            let arrives = if bus_cycle.is_rush {
                -88.0 + 92.0 * share_of_the_queue.powf(0.8) + arrival_jitter * 3.0
            } else {
                -58.0 + 62.0 * share_of_the_queue + arrival_jitter * 6.0
            };
            let arrives = arrives.min(turn_to_board - 2.0);
            let sits_on_a_bench =
                !bus_cycle.is_rush && !bus_cycle.raining && rider < 8 && !(look >> 9).is_multiple_of(3);
            let spot = if sits_on_a_bench {
                let end_of_bench = if rider < 4 { -0.45 } else { 0.45 };
                (BUS_PLAZA_BENCHES_EAST[rider % 4] + end_of_bench, BUS_PLAZA_BENCHES_SOUTH + 0.1)
            } else {
                let spot_index = if bus_cycle.is_rush { rider } else { rider + (look as usize % 3) };
                spots[spot_index.min(spots.len() - 1)]
            };
            path.clear();
            path.extend_from_slice(&ways[(look >> 3) as usize % ways.len()]);
            path.push(spot);
            let walk_length = path_length(&path);
            let is_late = arrives > bus.stops - 6.0;
            let walking_speed = if is_late {
                2.9
            } else if bus_cycle.raining || bus_cycle.is_rush {
                1.55
            } else {
                1.3
            };
            let seconds_walking = walk_length / walking_speed;
            if seconds_into_cycle < arrives - seconds_walking {
                continue;
            }
            let lean_towards_the_door = if seconds_into_cycle > bus.stops && !sits_on_a_bench {
                0.22 * (1.0 - place_in_queue as f32 / RIDERS_PER_BUS_IN_A_RUSH as f32).max(0.0)
            } else {
                0.0
            };
            let standing_at = (
                spot.0 + (FRONT_DOOR.0 - spot.0) * lean_towards_the_door,
                spot.1 + (FRONT_DOOR.1 - spot.1) * lean_towards_the_door * 0.6,
            );
            let mut person = Person::new(standing_at.0, standing_at.1, look, Pose::Stand);
            if seconds_into_cycle < arrives {
                let distance_walked = (seconds_into_cycle - (arrives - seconds_walking)) * walking_speed;
                let place = place_along(&path, distance_walked);
                person.metres_east = place.metres_east;
                person.metres_south = place.metres_south;
                person.pose = if is_late { Pose::Run } else { Pose::Walk };
                person.heading_across_screen = place.facing_on_screen();
                person.stride_frame = stride_frame(distance_walked, 0.62);
            } else if seconds_into_cycle < turn_to_board {
                let shuffle = ((environment.animation_seconds * 0.9 + (look & 0xFF) as f64 * 0.13).sin() as f32 * 0.55)
                    .trunc()
                    * 0.3;
                person.metres_east += shuffle;
                person.heading_across_screen = if standing_at.0 < FRONT_DOOR.0 { 1 } else { -1 };
                if sits_on_a_bench && seconds_into_cycle < bus.stops - 1.0 {
                    person.pose = Pose::Sit;
                    person.metres_up = 0.45;
                }
            } else {
                let progress = ((seconds_into_cycle - turn_to_board) / SECONDS_TO_WALK_INTO_THE_BUS).clamp(0.0, 1.0);
                person.metres_east = standing_at.0 + (FRONT_DOOR.0 - standing_at.0) * progress;
                person.metres_south = standing_at.1 + (FRONT_DOOR.1 - standing_at.1) * progress;
                person.pose = Pose::Walk;
                person.heading_across_screen = if FRONT_DOOR.0 > standing_at.0 { 1 } else { -1 };
                person.stride_frame = stride_frame(seconds_into_cycle * 5.0, 1.0);
            }
            actors.push(Actor::Person(person));
        }
    }

    for cycle in [current_cycle - 2, current_cycle - 1, current_cycle] {
        let bus_cycle = timetable.cycle(cycle, memory);
        let seconds_into_cycle =
            timetable.seconds_into_current_cycle + (current_cycle - cycle) as f32 * CYCLE_SECONDS as f32;
        let cycle_number = cycle as i32;
        for (place_in_line, bus) in bus_cycle.buses.iter().enumerate() {
            for alighter in 0..bus.alighters {
                let steps_off =
                    bus.doors_open + FIRST_ALIGHTER_AFTER_DOORS_OPEN + alighter as f32 * SECONDS_BETWEEN_ALIGHTERS;
                if seconds_into_cycle < steps_off {
                    continue;
                }
                let look = hash_position(cycle_number * 5 + place_in_line as i32, alighter as i32 + 900, seed ^ 0x0FF);
                let way = &ways[(look >> 3) as usize % ways.len()];
                let step_aside = if way[0].0 < 0.0 { -1.0 } else { 1.0 };
                path.clear();
                path.push(REAR_DOOR);
                path.push((REAR_DOOR.0 + step_aside, REAR_DOOR.1 + 1.6));
                path.extend(way.iter().rev());
                let walk_length = path_length(&path);
                let walking_speed = if bus_cycle.raining {
                    1.6
                } else {
                    1.25 + 0.3 * unit_hash(cycle_number, alighter as i32 + 70, seed)
                };
                let distance_walked = (seconds_into_cycle - steps_off) * walking_speed;
                if distance_walked > walk_length {
                    continue;
                }
                let place = place_along(&path, distance_walked);
                let mut person = Person::new(place.metres_east, place.metres_south, look, Pose::Walk);
                person.heading_across_screen = place.facing_on_screen();
                person.stride_frame = stride_frame(distance_walked, 0.62);
                actors.push(Actor::Person(person));
            }
        }
    }
}
