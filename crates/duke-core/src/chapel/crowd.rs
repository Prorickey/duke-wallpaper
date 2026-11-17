use crate::chapel::anim::{
    busyness_at_hour, path_metres, position_along_path, stride_frame, Conditions, Pose, Sprite, SpriteKind, World,
    DUKE_BLUE, WALKING_METRES_PER_SECOND,
};
use crate::chapel::palette::smooth_step;
use crate::chapel::props::CAR_COLOURS;
use crate::hash_fraction;
use crate::Environment;

pub const CLASS_CHANGE_HOURS: [f32; 7] = [8.3, 9.9, 11.45, 13.1, 14.7, 16.25, 17.9];
pub(crate) const RUSH_CYCLE_SECONDS: f32 = 720.0;
const FIRST_RUSH_STARTS_AT_SECONDS: f32 = 240.0;
const CROWD_PEAKS_SECONDS_INTO_RUSH: f32 = 120.0;

const PEOPLE_IN_A_FULL_RUSH: usize = 132;
const LATEST_SETTING_OUT_SECONDS: f32 = 60.0;
const EARLIEST_LEAVING_SECONDS: f32 = 150.0;
const LATEST_LEAVING_SECONDS: f32 = 220.0;
const RUSH_IS_OVER_SECONDS: f32 = 270.0;
const FADE_AT_THE_DOOR_SECONDS: f32 = 1.5;

const CHAPEL_DOOR_NORTH: f32 = 1.3;
const FOOT_OF_STEPS_NORTH: f32 = -2.6;
const ARCADE_WALK_EAST: f32 = 24.3;
const UNDER_THE_ARCADE_NORTH: f32 = 7.4;
const CROSS_WALK_NORTH: f32 = -10.4;
const CENTRE_WALK_EAST: f32 = 5.6;
const BELOW_THE_FRAME_NORTH: f32 = -43.0;

#[derive(Clone, Copy, PartialEq)]
enum StandingPlace {
    ChapelSteps,
    Terrace,
    CrossWalk,
}

fn seconds_into_each_rush(environment: &Environment, seconds: f32) -> [f32; 2] {
    let seconds_into_timed_rush = (seconds - FIRST_RUSH_STARTS_AT_SECONDS).rem_euclid(RUSH_CYCLE_SECONDS);

    let mut hours_after_nearest_class_change = f32::INFINITY;
    for class_change in CLASS_CHANGE_HOURS {
        let hours_after = environment.hour - class_change;
        if hours_after.abs() < hours_after_nearest_class_change.abs() {
            hours_after_nearest_class_change = hours_after;
        }
    }
    let seconds_into_class_change_rush = hours_after_nearest_class_change * 3600.0 + CROWD_PEAKS_SECONDS_INTO_RUSH;
    [seconds_into_timed_rush, seconds_into_class_change_rush]
}

fn crowd_level(seconds_into_rush: f32) -> f32 {
    if !(0.0..RUSH_IS_OVER_SECONDS).contains(&seconds_into_rush) {
        return 0.0;
    }
    let building = smooth_step(0.0, LATEST_SETTING_OUT_SECONDS + 20.0, seconds_into_rush);
    let draining = smooth_step(EARLIEST_LEAVING_SECONDS, LATEST_LEAVING_SECONDS + 30.0, seconds_into_rush);
    building * (1.0 - draining)
}

pub fn add_rush_crowd(
    environment: &Environment,
    world: &World,
    seconds: f32,
    conditions: &Conditions,
    sprites: &mut Vec<Sprite>,
) -> f32 {
    let turnout = smooth_step(0.12, 0.42, busyness_at_hour(environment.hour)) * conditions.fair_weather.powf(1.5);
    let people_who_turn_out = PEOPLE_IN_A_FULL_RUSH as f32 * turnout;
    let mut highest_level: f32 = 0.0;

    for (rush_number, seconds_into_rush) in seconds_into_each_rush(environment, seconds).into_iter().enumerate() {
        let level = crowd_level(seconds_into_rush) * turnout;
        highest_level = highest_level.max(level);
        if level <= 0.0 {
            continue;
        }
        for member in 0..PEOPLE_IN_A_FULL_RUSH {
            let turnout_opacity = (people_who_turn_out - member as f32).clamp(0.0, 1.0);
            if turnout_opacity <= 0.0 {
                break;
            }
            let chance = |trait_number: i32| {
                hash_fraction(member as i32, trait_number, world.seed ^ 0xC20D ^ (rush_number as u32 * 0x5151))
            };

            let standing_place = match member % 4 {
                0 => StandingPlace::ChapelSteps,
                1 | 2 => StandingPlace::Terrace,
                _ => StandingPlace::CrossWalk,
            };
            let spot = match standing_place {
                StandingPlace::ChapelSteps => ((chance(1) - 0.5) * 12.6, -0.9 - chance(2) * 3.6),
                StandingPlace::Terrace => ((chance(1) - 0.5) * 16.6, -5.4 - chance(2) * 3.2),
                StandingPlace::CrossWalk => ((chance(1) - 0.5) * 45.0, -9.0 - chance(2) * 2.4),
            };
            let side = if spot.0 < 0.0 { -1.0 } else { 1.0 };
            let chapel_door = (side * (0.5 + chance(4)), CHAPEL_DOOR_NORTH);
            let foot_of_steps = (spot.0 * 0.5, FOOT_OF_STEPS_NORTH);

            let comes_out_of_the_chapel = standing_place == StandingPlace::ChapelSteps || chance(3) < 0.42;
            let way_in = if comes_out_of_the_chapel {
                [chapel_door, foot_of_steps, spot]
            } else {
                [
                    (side * ARCADE_WALK_EAST, UNDER_THE_ARCADE_NORTH),
                    (side * ARCADE_WALK_EAST, CROSS_WALK_NORTH + chance(5)),
                    spot,
                ]
            };
            let goes_back_inside = standing_place == StandingPlace::ChapelSteps && chance(6) < 0.4;
            let way_out = if goes_back_inside {
                [spot, foot_of_steps, chapel_door]
            } else if chance(6) < 0.7 {
                let walk_east = side * (CENTRE_WALK_EAST + chance(7) * 1.2);
                [spot, (walk_east, CROSS_WALK_NORTH), (walk_east, BELOW_THE_FRAME_NORTH)]
            } else {
                let walk_east = side * (ARCADE_WALK_EAST - 0.4 + chance(7));
                [spot, (walk_east, CROSS_WALK_NORTH), (walk_east, BELOW_THE_FRAME_NORTH)]
            };

            let metres_per_second = WALKING_METRES_PER_SECOND + chance(11) * 0.4;
            let sets_out_at = chance(10) * LATEST_SETTING_OUT_SECONDS;
            let arrives_at = sets_out_at + path_metres(&way_in) / metres_per_second;
            let leaves_at = EARLIEST_LEAVING_SECONDS + chance(12) * (LATEST_LEAVING_SECONDS - EARLIEST_LEAVING_SECONDS);
            let is_gone_at = leaves_at + path_metres(&way_out) / metres_per_second;
            if seconds_into_rush < sets_out_at || seconds_into_rush > is_gone_at {
                continue;
            }

            let person = 5000 + rush_number as u32 * 500 + member as u32;
            let (position, pose) = if seconds_into_rush < arrives_at {
                let seconds_walked = seconds_into_rush - sets_out_at;
                (
                    position_along_path(&way_in, seconds_walked * metres_per_second),
                    Pose::Walking { stride: stride_frame(seconds_walked) },
                )
            } else if seconds_into_rush < leaves_at {
                let sits_on_the_steps =
                    standing_place == StandingPlace::ChapelSteps && chance(9) < 0.4 && conditions.rain < 0.1;
                let waves = hash_fraction(member as i32, (seconds_into_rush / 2.5) as i32, world.seed ^ 0xC2) < 0.03;
                let pose = if sits_on_the_steps {
                    Pose::Sitting
                } else if waves {
                    Pose::ArmsRaised
                } else {
                    Pose::Standing
                };
                (spot, pose)
            } else {
                let seconds_walked = seconds_into_rush - leaves_at;
                (
                    position_along_path(&way_out, seconds_walked * metres_per_second),
                    Pose::Walking { stride: stride_frame(seconds_walked) },
                )
            };

            let appearing = (seconds_into_rush - sets_out_at) / FADE_AT_THE_DOOR_SECONDS;
            let mut sprite =
                Sprite::person(position, person, pose, conditions).faded(appearing.min(1.0) * turnout_opacity);
            if let SpriteKind::Person { outfit, .. } = &mut sprite.kind {
                if member % 9 == 0 {
                    outfit.top = DUKE_BLUE;
                }
            }
            sprites.push(sprite);
        }
    }
    highest_level
}

const CIRCLE_LANE_RADIUS_METRES: f32 = 13.7;
const CAR_PARK_DRIVE: [(f32, f32); 4] = [(-38.0, 165.0), (-41.2, 153.4), (-52.2, 111.6), (-56.5, 106.5)];
const DRIVE_JOINS_CIRCLE_AT_DEGREES: f32 = 56.0;
const CAR_STOPS_AT_DEGREES: f32 = 255.0;
pub(crate) const CAR_CALLS_EVERY_SECONDS: f32 = 150.0;
const CAR_REACHES_CIRCLE_AT: f32 = 12.0;
const CAR_STOPS_AT: f32 = 22.0;
const CAR_PULLS_AWAY_AT: f32 = 36.0;
const CAR_LEAVES_CIRCLE_AT: f32 = 46.0;
const CAR_IS_GONE_AT: f32 = 58.0;
const PASSENGER_STEPS_OUT_AT: f32 = 26.0;
const PASSENGER_WALK_TO_TRANSEPT_DOOR: [(f32, f32); 7] =
    [(-60.0, 78.4), (-57.6, 69.5), (-47.9, 73.5), (-39.9, 77.3), (-29.1, 79.7), (-22.0, 66.0), (-14.4, 54.8)];

pub struct CircleCar {
    pub metres_east: f32,
    pub metres_north: f32,
    pub heading_east: f32,
    pub heading_north: f32,
    pub body_colour: u32,
    pub presence: f32,
}

fn car_call_now(environment: &Environment, world: &World, seconds: f32) -> Option<(f32, f32, f32)> {
    let call_number = (seconds / CAR_CALLS_EVERY_SECONDS).floor();
    let seconds_into_call = seconds - call_number * CAR_CALLS_EVERY_SECONDS;
    let hour_when_the_call_began = environment.hour - seconds_into_call / 3600.0;
    let it_is_a_busy_hour = smooth_step(0.22, 0.28, busyness_at_hour(hour_when_the_call_began));
    let a_late_car_calls_anyway = hash_fraction(call_number as i32, 1, world.seed ^ 0xCA2) < 0.35;
    let presence = if a_late_car_calls_anyway { 1.0 } else { it_is_a_busy_hour };
    if presence > 0.0 {
        Some((call_number, seconds_into_call, presence))
    } else {
        None
    }
}

pub fn car_at_the_circle(environment: &Environment, world: &World, seconds: f32) -> Option<CircleCar> {
    let (call_number, seconds_into_call, presence) = car_call_now(environment, world, seconds)?;
    if seconds_into_call > CAR_IS_GONE_AT {
        return None;
    }
    let circle_centre = crate::chapel::ground::DROP_OFF_CIRCLE.0;
    let on_the_circle_at = |degrees: f32| {
        let angle = degrees.to_radians();
        let position = (
            circle_centre.0 + CIRCLE_LANE_RADIUS_METRES * angle.cos(),
            circle_centre.1 + CIRCLE_LANE_RADIUS_METRES * angle.sin(),
        );
        (position, (-angle.sin(), angle.cos()))
    };
    let drive_metres = path_metres(&CAR_PARK_DRIVE);
    let full_turn_back_at_the_drive = DRIVE_JOINS_CIRCLE_AT_DEGREES + 360.0;

    let (position, heading) = if seconds_into_call < CAR_REACHES_CIRCLE_AT {
        crate::chapel::anim::position_and_heading_along_path(
            &CAR_PARK_DRIVE,
            drive_metres * seconds_into_call / CAR_REACHES_CIRCLE_AT,
        )
    } else if seconds_into_call < CAR_STOPS_AT {
        let fraction = smooth_step(CAR_REACHES_CIRCLE_AT, CAR_STOPS_AT, seconds_into_call);
        on_the_circle_at(
            DRIVE_JOINS_CIRCLE_AT_DEGREES + (CAR_STOPS_AT_DEGREES - DRIVE_JOINS_CIRCLE_AT_DEGREES) * fraction,
        )
    } else if seconds_into_call < CAR_PULLS_AWAY_AT {
        on_the_circle_at(CAR_STOPS_AT_DEGREES)
    } else if seconds_into_call < CAR_LEAVES_CIRCLE_AT {
        let fraction = smooth_step(CAR_PULLS_AWAY_AT, CAR_LEAVES_CIRCLE_AT, seconds_into_call);
        on_the_circle_at(CAR_STOPS_AT_DEGREES + (full_turn_back_at_the_drive - CAR_STOPS_AT_DEGREES) * fraction)
    } else {
        let fraction = (seconds_into_call - CAR_LEAVES_CIRCLE_AT) / (CAR_IS_GONE_AT - CAR_LEAVES_CIRCLE_AT);
        let (position, heading) =
            crate::chapel::anim::position_and_heading_along_path(&CAR_PARK_DRIVE, drive_metres * (1.0 - fraction));
        (position, (-heading.0, -heading.1))
    };
    let body_colour = CAR_COLOURS
        [(hash_fraction(call_number as i32, 2, world.seed) * CAR_COLOURS.len() as f32) as usize % CAR_COLOURS.len()];
    Some(CircleCar {
        metres_east: position.0,
        metres_north: position.1,
        heading_east: heading.0,
        heading_north: heading.1,
        body_colour,
        presence,
    })
}

pub fn add_car_at_the_circle(
    environment: &Environment,
    world: &World,
    seconds: f32,
    conditions: &Conditions,
    sprites: &mut Vec<Sprite>,
) {
    if let Some(car) = car_at_the_circle(environment, world, seconds) {
        let kind = SpriteKind::Car {
            heading_east: car.heading_east,
            heading_north: car.heading_north,
            body_colour: car.body_colour,
        };
        sprites.push(Sprite::on_the_ground((car.metres_east, car.metres_north), kind).faded(car.presence));
    }
    let Some((call_number, seconds_into_call, presence)) = car_call_now(environment, world, seconds) else {
        return;
    };
    let seconds_walked = seconds_into_call - PASSENGER_STEPS_OUT_AT;
    let metres_walked = seconds_walked * WALKING_METRES_PER_SECOND;
    if seconds_walked < 0.0 || metres_walked > path_metres(&PASSENGER_WALK_TO_TRANSEPT_DOOR) {
        return;
    }
    let pose = Pose::Walking { stride: stride_frame(seconds_walked) };
    let stepping_out = seconds_walked / 1.5;
    let position = position_along_path(&PASSENGER_WALK_TO_TRANSEPT_DOOR, metres_walked);
    sprites.push(
        Sprite::person(position, 7000 + call_number as u32, pose, conditions).faded(stepping_out.min(1.0) * presence),
    );
}
