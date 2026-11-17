use crate::east::animation::Actor;
use crate::east::campus_layout::{
    cross_walks_south, LAWN_HALF_WIDTH, NORTH_LAWN_CAP_SOUTH, QUAD_HALLS, SOUTH_LAWN_CAP_SOUTH, UNION_CIRCLE_SOUTH,
};
use crate::east::cycle_memory::{cycle_and_seconds_into_it, CycleMemory};
use crate::east::figures::{stride_frame, Person, Pose, LOOK_NO_BACKPACK, LOOK_WEARS_SHORTS_MASK};
use crate::east::paths::{path_length, place_along, Point};
use crate::east::props::{Prop, PropKind, Rider};
use crate::east::shading_tools::unit_hash;
use crate::east::street_furniture::bench_positions;
use crate::{hash_position, Environment, Weather};
use std::f32::consts::{FRAC_PI_2, PI, TAU};
use std::sync::OnceLock;

const CLASS_CHANGE_HOURS: [f32; 8] = [8.3, 9.9, 11.6, 13.1, 14.75, 16.4, 18.0, 19.3];

const WALKING_PARTY_SLOTS: i32 = 110;
const JOGGER_SLOTS_START: usize = 110;
const BIKE_SLOTS_START: usize = 114;
pub const GROUNDS_CART_SLOT: usize = 123;
pub const MARKHAM_CAR_SLOTS_START: usize = 124;

pub fn how_busy_the_quad_is(environment: &Environment) -> f32 {
    let fair_weather_share = match environment.weather {
        Weather::Clear => 1.0,
        Weather::Cloudy => 0.92,
        Weather::Fog => 0.65,
        Weather::Rain => 0.45,
        Weather::Storm => 0.14,
        Weather::Snow => 0.5,
    };
    let weather_factor = 1.0 + (fair_weather_share - 1.0) * environment.weather_intensity.clamp(0.0, 1.0);
    let hour = environment.hour.rem_euclid(24.0);
    let small_hours_factor = if hour < 5.5 { 0.3 } else { 1.0 };
    let daylight_base = 0.2 + 0.5 * environment.daylight();
    let class_change_bump = CLASS_CHANGE_HOURS
        .iter()
        .map(|bell| {
            let minutes_after_the_bell = (hour - bell) * 60.0;
            if !(-6.0..=11.0).contains(&minutes_after_the_bell) {
                0.0
            } else if minutes_after_the_bell < 0.0 {
                1.0 + minutes_after_the_bell / 6.0
            } else {
                1.0 - minutes_after_the_bell / 11.0
            }
        })
        .fold(0.0, f32::max);
    let class_change_factor = 1.0 + 0.55 * class_change_bump * (0.4 + 0.6 * environment.daylight());
    (weather_factor * daylight_base * class_change_factor * small_hours_factor).clamp(0.03, 1.0)
}

fn drive_centreline(side: f32, outward_offset: f32) -> Vec<Point> {
    let half_width = LAWN_HALF_WIDTH + 3.25 + outward_offset;
    let squareness = 2.0 / 2.6f32;
    let mut points = Vec::new();
    for step in 0..=14 {
        let angle = FRAC_PI_2 * (1.0 - step as f32 / 14.0);
        points.push((
            side * half_width * angle.cos().powf(squareness),
            NORTH_LAWN_CAP_SOUTH - (half_width + 0.5) * angle.sin().powf(squareness),
        ));
    }
    let circle_radius = 31.9 + outward_offset;
    let first_angle = (half_width / circle_radius).clamp(-1.0, 1.0).asin();
    for step in 0..=12 {
        let angle = first_angle + (PI - 2.0 * first_angle) * step as f32 / 12.0;
        points.push((side * circle_radius * angle.sin(), UNION_CIRCLE_SOUTH - circle_radius * angle.cos()));
    }
    for step in 0..=14 {
        let angle = FRAC_PI_2 * step as f32 / 14.0;
        points.push((
            side * half_width * angle.cos().powf(squareness),
            SOUTH_LAWN_CAP_SOUTH + half_width * angle.sin().powf(squareness),
        ));
    }
    points
}

#[derive(Copy, Clone)]
pub struct Seat {
    pub distance_along: f32,
    pub seat_height: f32,
    pub shortest_stay: f32,
    pub longest_stay: f32,
}

pub struct Route {
    pub points: Vec<Point>,
    pub length: f32,
    pub seat: Option<Seat>,
}

impl Route {
    fn walk(points: Vec<Point>) -> Route {
        Route { length: path_length(&points), points, seat: None }
    }

    fn calling_at_a_seat(
        points: Vec<Point>,
        seat_point_index: usize,
        seat_height: f32,
        shortest_stay: f32,
        longest_stay: f32,
    ) -> Route {
        let distance_along = path_length(&points[..=seat_point_index]);
        Route {
            length: path_length(&points),
            points,
            seat: Some(Seat { distance_along, seat_height, shortest_stay, longest_stay }),
        }
    }
}

pub struct Routes {
    pub walks: Vec<Route>,
    pub laps_round_the_lawns: [Route; 2],
}

const DESIRE_LINES_ACROSS_THE_LAWNS: [&[Point]; 4] = [
    &[
        (-46.0, 69.0),
        (-27.0, 69.0),
        (-21.5, 62.0),
        (-2.0, 68.2),
        (2.0, 70.8),
        (20.5, 96.0),
        (27.5, 139.0),
        (46.0, 139.0),
    ],
    &[(-46.0, 136.0), (-27.5, 136.0), (-22.5, 150.0), (22.5, 182.0), (27.5, 194.3), (46.0, 194.3)],
    &[(-46.0, 259.3), (-27.0, 259.6), (27.0, 259.6), (46.0, 261.2)],
    &[(-46.0, 191.5), (-27.0, 191.5), (-19.5, 206.0), (18.0, 252.0), (27.0, 261.2), (46.0, 261.2)],
];

const WALKS_ROUND_BALDWIN: [&[Point]; 5] = [
    &[(-33.0, 13.5), (-17.0, 13.5), (-12.0, 27.5), (0.0, 30.0), (12.0, 27.5), (17.0, 14.0), (33.0, 14.0)],
    &[
        (-46.0, 69.0),
        (-28.3, 69.0),
        (-28.3, 52.0),
        (-27.6, 47.0),
        (-27.6, 16.5),
        (-24.0, 13.6),
        (-17.0, 13.5),
        (-5.0, 26.2),
    ],
    &[(46.0, 71.5), (28.3, 71.5), (28.0, 47.0), (27.6, 23.2), (17.0, 24.2), (7.4, 30.4), (3.4, 28.5), (3.0, 25.9)],
    &[(-2.5, 25.9), (-3.0, 40.0), (0.0, 47.5), (9.0, 48.5), (19.0, 55.0), (24.5, 66.0), (28.0, 71.5), (46.0, 71.5)],
    &[
        (32.4, 37.9),
        (27.6, 37.9),
        (27.6, 30.0),
        (10.0, 34.0),
        (-10.0, 34.0),
        (-27.6, 30.0),
        (-27.6, 35.5),
        (-32.4, 35.5),
    ],
];

const WALKS_ROUND_THE_BUS_PLAZA: [&[Point]; 6] = [
    &[
        (-38.4, 398.0),
        (-38.4, 345.0),
        (-35.0, 335.4),
        (-22.0, 333.5),
        (8.0, 333.3),
        (27.5, 331.0),
        (28.4, 300.0),
        (28.2, 261.2),
        (46.0, 261.2),
    ],
    &[
        (-46.0, 259.3),
        (-27.0, 259.3),
        (-24.5, 266.0),
        (-24.5, 291.0),
        (-17.0, 309.5),
        (-9.0, 315.0),
        (-5.8, 319.5),
        (-5.8, 332.4),
        (-12.0, 333.3),
        (-18.6, 334.8),
        (-18.6, 343.4),
        (-11.4, 349.8),
        (-18.6, 398.0),
    ],
    &[
        (46.0, 261.2),
        (27.0, 261.2),
        (24.5, 268.0),
        (24.5, 291.0),
        (17.0, 309.5),
        (9.0, 315.0),
        (5.8, 319.5),
        (5.8, 332.4),
        (12.0, 333.3),
        (18.6, 334.8),
        (18.6, 343.4),
        (11.4, 349.8),
        (16.8, 398.0),
    ],
    &[(-18.6, 398.0), (-11.4, 349.8), (0.0, 346.0), (11.4, 349.8), (16.8, 398.0)],
    &[
        (62.0, 333.3),
        (9.0, 333.3),
        (5.8, 331.0),
        (5.8, 320.0),
        (12.0, 314.0),
        (24.5, 300.0),
        (24.5, 266.0),
        (27.0, 261.2),
        (46.0, 261.2),
    ],
    &[
        (-62.0, 333.3),
        (-9.0, 333.3),
        (-5.8, 331.0),
        (-5.8, 320.0),
        (-12.0, 314.0),
        (-24.5, 300.0),
        (-24.5, 266.0),
        (-27.0, 259.3),
        (-46.0, 259.3),
    ],
];

const BALDWIN_STEP_SEATS_EAST_AND_HEIGHT: [(f32, f32); 4] = [(-4.2, 0.75), (4.8, 0.9), (-6.6, 0.45), (6.9, 0.55)];

pub fn routes() -> &'static Routes {
    static ROUTES: OnceLock<Routes> = OnceLock::new();
    ROUTES.get_or_init(|| {
        let mut walks: Vec<Route> = Vec::new();
        for side in [-1.0f32, 1.0] {
            let mut down_the_whole_quad = vec![(side * 2.2, 25.9), (side * 2.6, 44.0), (side * 9.0, 48.2)];
            down_the_whole_quad.extend(
                drive_centreline(side, 1.8).into_iter().filter(|point| point.0.abs() > 12.0 || point.1 > 200.0),
            );
            down_the_whole_quad.extend([
                (side * 5.8, 320.0),
                (side * 5.8, 332.4),
                (side * 14.0, 333.3),
                (side * 35.0, 334.6),
                (side * 38.4, 345.0),
                (side * 38.4, 398.0),
            ]);
            walks.push(Route::walk(down_the_whole_quad));
        }
        for (hall_index, hall) in QUAD_HALLS.iter().enumerate() {
            let side = hall.side_of_quad;
            let door = hall.entrance_south();
            let gaps_this_side = cross_walks_south(side);
            let gaps_across_the_quad = cross_walks_south(-side);
            let nearest_gap = gaps_this_side[hall_index % 4];
            let other_gap = gaps_this_side[(hall_index + 3) % 4];
            walks.push(Route::walk(vec![
                (side * 32.4, door),
                (side * 28.2, door),
                (side * 28.2, nearest_gap),
                (side * 46.0, nearest_gap),
            ]));
            walks.push(Route::walk(vec![
                (side * 32.4, door),
                (side * 28.2, door),
                (side * 28.2, other_gap),
                (side * 46.0, other_gap),
            ]));
            if hall_index % 4 != 1 {
                let gap_opposite = gaps_across_the_quad[hall_index % 4];
                walks.push(Route::walk(vec![
                    (side * 32.4, door),
                    (side * 24.0, door),
                    (-side * 24.0, door + 6.0),
                    (-side * 28.2, door + 6.0),
                    (-side * 28.2, gap_opposite),
                    (-side * 46.0, gap_opposite),
                ]));
            }
        }
        let mut round_union_circle = vec![(-43.0, UNION_CIRCLE_SOUTH)];
        for step in 0..=10 {
            let angle = PI * (1.0 + step as f32 / 10.0);
            round_union_circle.push((33.6 * angle.cos(), UNION_CIRCLE_SOUTH - 33.6 * angle.sin()));
        }
        round_union_circle.push((37.0, UNION_CIRCLE_SOUTH));
        walks.push(Route::walk(round_union_circle));
        for fixed_walk in DESIRE_LINES_ACROSS_THE_LAWNS
            .iter()
            .chain(WALKS_ROUND_BALDWIN.iter())
            .chain(WALKS_ROUND_THE_BUS_PLAZA.iter())
        {
            walks.push(Route::walk(fixed_walk.to_vec()));
        }
        for &(bench_east, bench_south) in bench_positions().iter().filter(|bench| bench.1 <= 330.0) {
            let side = if bench_east < 0.0 { -1.0 } else { 1.0 };
            let mut gaps_nearest_first = cross_walks_south(side).to_vec();
            gaps_nearest_first
                .sort_by(|first, second| (first - bench_south).abs().total_cmp(&(second - bench_south).abs()));
            let (gap_in, gap_out) = (gaps_nearest_first[0], gaps_nearest_first[1]);
            let lane = side * 28.2;
            let points = vec![
                (side * 46.0, gap_in),
                (lane, gap_in),
                (lane, bench_south),
                (bench_east, bench_south),
                (lane, bench_south + 0.4),
                (lane, gap_out),
                (side * 46.0, gap_out),
            ];
            walks.push(Route::calling_at_a_seat(points, 3, 0.45, 35.0, 130.0));
        }
        for (step_east, step_height) in BALDWIN_STEP_SEATS_EAST_AND_HEIGHT {
            let side = if step_east < 0.0 { -1.0 } else { 1.0 };
            let first_gap = cross_walks_south(side)[0];
            let points = vec![
                (side * 46.0, first_gap),
                (side * 27.5, first_gap),
                (side * 22.0, 52.0),
                (side * 6.0, 45.0),
                (step_east, 26.0),
                (step_east, 24.3),
                (step_east * 0.9, 26.2),
                (side * 5.0, 44.5),
                (side * 21.0, 51.0),
                (side * 27.8, 60.0),
                (side * 28.2, first_gap),
                (side * 46.0, first_gap),
            ];
            walks.push(Route::calling_at_a_seat(points, 5, step_height, 40.0, 150.0));
        }
        let lap_of = |outward_offset: f32| {
            let mut lap = drive_centreline(-1.0, outward_offset);
            lap.extend(drive_centreline(1.0, outward_offset).into_iter().rev());
            Route::walk(lap)
        };
        Routes { walks, laps_round_the_lawns: [lap_of(0.0), lap_of(-1.5)] }
    })
}

#[derive(Copy, Clone)]
pub struct Plan {
    pub turns_up: bool,
    pub hurrying: bool,
}

pub fn remember_plan(
    memory: &mut CycleMemory<Plan>,
    slot: usize,
    cycle: i32,
    seconds_into_cycle: f32,
    environment: &Environment,
    seed: u32,
    decide: impl FnOnce() -> Plan,
) -> Plan {
    memory.recall_or_decide(
        slot,
        cycle as i64,
        environment.hour - seconds_into_cycle / 3600.0,
        environment,
        seed,
        decide,
    )
}

pub fn remember_whether_it_turns_up(
    memory: &mut CycleMemory<Plan>,
    slot: usize,
    cycle: i32,
    seconds_into_cycle: f32,
    environment: &Environment,
    seed: u32,
    decide: impl FnOnce() -> bool,
) -> bool {
    remember_plan(memory, slot, cycle, seconds_into_cycle, environment, seed, || Plan {
        turns_up: decide(),
        hurrying: false,
    })
    .turns_up
}

pub fn add_walkers(
    environment: &Environment,
    seed: u32,
    busyness: f32,
    actors: &mut Vec<Actor>,
    memory: &mut CycleMemory<Plan>,
) {
    let routes = routes();
    let raining = matches!(environment.weather, Weather::Rain | Weather::Storm) && environment.weather_intensity > 0.25;
    let seats_are_dry = !raining && environment.snow_cover < 0.3 && environment.wetness < 0.6;

    for party_slot in 0..WALKING_PARTY_SLOTS {
        let party_hash = hash_position(party_slot, 1, seed ^ 0xA11);
        let party_number = |purpose: i32| unit_hash(party_slot, purpose, seed ^ 0xA11);
        let route = &routes.walks[(party_hash as usize) % routes.walks.len()];
        let party_size = match (party_hash >> 8) % 100 {
            0..=57 => 1,
            58..=85 => 2,
            86..=95 => 3,
            _ => 4,
        };
        let dry_walking_speed = 1.1 + 0.45 * party_number(2);
        let (pause_at, pause_seconds, seat_height) = match route.seat {
            Some(seat) => (
                seat.distance_along,
                seat.shortest_stay + (seat.longest_stay - seat.shortest_stay) * party_number(5),
                Some(seat.seat_height),
            ),
            None => {
                let stops_to_chat = (party_size > 1 && (party_hash >> 16).is_multiple_of(3))
                    || (party_size == 1 && (party_hash >> 16).is_multiple_of(9));
                if stops_to_chat {
                    ((0.2 + 0.6 * party_number(6)) * route.length, 7.0 + 14.0 * party_number(7), None)
                } else {
                    (0.0, 0.0, None)
                }
            }
        };
        let gap_before_next_party = 8.0 + 46.0 * party_number(3);
        let cycle_seconds = (route.length / dry_walking_speed + pause_seconds + gap_before_next_party) as f64;
        let (cycle, seconds_into_cycle) = cycle_and_seconds_into_it(
            environment.animation_seconds + (party_number(4) as f64) * cycle_seconds,
            cycle_seconds,
        );
        let plan = remember_plan(memory, party_slot as usize, cycle, seconds_into_cycle, environment, seed, || Plan {
            turns_up: unit_hash(party_slot, cycle, seed ^ 0xB22) <= busyness * 1.05
                && (route.seat.is_none() || seats_are_dry),
            hurrying: raining,
        });
        if !plan.turns_up {
            continue;
        }
        let walking_speed = if plan.hurrying { dry_walking_speed * 1.35 } else { dry_walking_speed };
        let pause_seconds = if plan.hurrying { 0.0 } else { pause_seconds };
        if seconds_into_cycle > route.length / walking_speed + pause_seconds {
            continue;
        }
        let pause_starts = pause_at / walking_speed;
        let (distance_walked, is_paused) = if pause_seconds > 0.0 && seconds_into_cycle >= pause_starts {
            if seconds_into_cycle < pause_starts + pause_seconds {
                (pause_at, true)
            } else {
                ((seconds_into_cycle - pause_seconds) * walking_speed, false)
            }
        } else {
            (seconds_into_cycle * walking_speed, false)
        };
        let walks_it_backwards = seat_height.is_none() && hash_position(party_slot, cycle, seed ^ 0xC33) & 1 == 1;
        let place = if walks_it_backwards {
            place_along(&route.points, route.length - distance_walked).turned_around()
        } else {
            place_along(&route.points, distance_walked)
        };
        let heading_length =
            (place.heading_east * place.heading_east + place.heading_south * place.heading_south).sqrt().max(1e-3);
        let (sideways_east, sideways_south) =
            (-place.heading_south / heading_length, place.heading_east / heading_length);
        for member in 0..party_size {
            let look = hash_position(party_slot * 7 + member, cycle, seed ^ 0xD44);
            let mut person = Person::new(place.metres_east, place.metres_south, look, Pose::Walk);
            let place_in_the_row = (member as f32 - (party_size - 1) as f32 * 0.5) * 0.75;
            match (is_paused, seat_height) {
                (true, Some(height)) => {
                    person.pose = Pose::Sit;
                    person.metres_up = height;
                    person.metres_east = place.metres_east + place_in_the_row;
                }
                (true, None) => {
                    person.pose = Pose::Stand;
                    let angle_round_the_knot = member as f32 / party_size.max(2) as f32 * TAU;
                    person.metres_east = place.metres_east + angle_round_the_knot.cos() * 0.6;
                    person.metres_south = place.metres_south + angle_round_the_knot.sin() * 0.9;
                    person.heading_across_screen = if angle_round_the_knot.cos() > 0.0 { -1 } else { 1 };
                }
                (false, _) => {
                    let beside = (member as f32 - (party_size - 1) as f32 * 0.5) * 0.85;
                    let behind = (member % 2) as f32 * 0.55;
                    person.metres_east =
                        place.metres_east + sideways_east * beside - place.heading_east / heading_length * behind;
                    person.metres_south =
                        place.metres_south + sideways_south * beside - place.heading_south / heading_length * behind;
                    person.heading_across_screen = place.facing_on_screen();
                    person.stride_frame = stride_frame(distance_walked + member as f32 * 0.3, 0.62);
                }
            }
            actors.push(Actor::Person(person));
        }
    }

    for jogger in 0..4 {
        let jogger_number = |purpose: i32| unit_hash(jogger, purpose, seed ^ 0x106);
        let lap = &routes.laps_round_the_lawns[(jogger as usize) & 1];
        let running_speed = 2.9 + 0.7 * jogger_number(2);
        let cycle_seconds = (lap.length / running_speed + 30.0 + 90.0 * jogger_number(3)) as f64;
        let (cycle, seconds_into_cycle) = cycle_and_seconds_into_it(
            environment.animation_seconds + (jogger_number(4) as f64) * cycle_seconds,
            cycle_seconds,
        );
        let turns_up = remember_whether_it_turns_up(
            memory,
            JOGGER_SLOTS_START + jogger as usize,
            cycle,
            seconds_into_cycle,
            environment,
            seed,
            || {
                !raining
                    && environment.snow_cover < 0.3
                    && unit_hash(jogger, cycle, seed ^ 0x107) <= busyness * 0.8 + 0.1
            },
        );
        if !turns_up {
            continue;
        }
        let distance_run = seconds_into_cycle * running_speed;
        if distance_run > lap.length {
            continue;
        }
        let runs_it_backwards = jogger & 2 == 2;
        let place = if runs_it_backwards {
            place_along(&lap.points, lap.length - distance_run).turned_around()
        } else {
            place_along(&lap.points, distance_run)
        };
        let mut person =
            Person::new(place.metres_east, place.metres_south, hash_position(jogger, cycle, seed ^ 0x108), Pose::Run);
        person.look &= !LOOK_WEARS_SHORTS_MASK;
        person.look |= LOOK_NO_BACKPACK;
        person.heading_across_screen = place.facing_on_screen();
        person.stride_frame = stride_frame(distance_run, 0.9);
        actors.push(Actor::Person(person));
    }

    for rider_slot in 0..9 {
        let rider_number = |purpose: i32| unit_hash(rider_slot, purpose, seed ^ 0xB1C);
        let lap = &routes.laps_round_the_lawns[(rider_slot as usize) & 1];
        let rides_a_scooter = rider_slot % 3 == 2;
        let riding_speed = if rides_a_scooter { 5.2 } else { 4.2 + 1.6 * rider_number(2) };
        let cycle_seconds = (lap.length / riding_speed + 18.0 + 60.0 * rider_number(3)) as f64;
        let (cycle, seconds_into_cycle) = cycle_and_seconds_into_it(
            environment.animation_seconds + (rider_number(4) as f64) * cycle_seconds,
            cycle_seconds,
        );
        let turns_up = remember_whether_it_turns_up(
            memory,
            BIKE_SLOTS_START + rider_slot as usize,
            cycle,
            seconds_into_cycle,
            environment,
            seed,
            || {
                environment.snow_cover < 0.4
                    && environment.weather != Weather::Storm
                    && unit_hash(rider_slot, cycle, seed ^ 0xB2C) <= busyness * 0.85
            },
        );
        if !turns_up {
            continue;
        }
        let distance_ridden = seconds_into_cycle * riding_speed;
        if distance_ridden > lap.length {
            continue;
        }
        let rides_it_backwards = rider_slot & 2 == 2;
        let place = if rides_it_backwards {
            place_along(&lap.points, lap.length - distance_ridden).turned_around()
        } else {
            place_along(&lap.points, distance_ridden)
        };
        let rider = Rider {
            facing: place.facing_on_screen(),
            pedal_frame: (seconds_into_cycle * 6.0) as i32 & 1,
            look: hash_position(rider_slot, cycle, seed ^ 0xB3C) & 0x7fff,
        };
        let kind = if rides_a_scooter { PropKind::Scooter(rider) } else { PropKind::Bike(rider) };
        actors.push(Actor::Prop(Prop {
            metres_east: place.metres_east,
            metres_south: place.metres_south,
            metres_up: 0.0,
            kind,
        }));
    }
}
