use crate::east::animation::Actor;
use crate::east::campus_layout::{cross_walks_south, UNION_CIRCLE_SOUTH};
use crate::east::cycle_memory::cycle_and_seconds_into_it;
use crate::east::figures::{stride_frame, Person, Pose};
use crate::east::paths::{path_length, place_along, PlaceOnPath, Point};
use crate::east::props::{Prop, PropKind};
use crate::east::shading_tools::{smoothstep, unit_hash};
use crate::{hash_position, Environment, Season, Weather};
use std::f32::consts::{FRAC_PI_2, TAU};

const GAME_SLOT_SECONDS: f64 = 210.0;
const QUIET_SLOT_SECONDS: f64 = 420.0;
const SECONDS_TO_WALK_ON: f32 = 34.0;
const SECONDS_TO_WALK_OFF: f32 = 34.0;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Game {
    Nothing,
    Football,
    Volleyball,
    Frisbee,
    Spikeball,
    SnowballFight,
    Snowman,
}

#[derive(Copy, Clone)]
pub struct Pitch {
    pub centre_east: f32,
    pub centre_south: f32,
    pub half_width: f32,
    pub half_depth: f32,
}

const fn pitch(centre_east: f32, centre_south: f32, half_width: f32, half_depth: f32) -> Pitch {
    Pitch { centre_east, centre_south, half_width, half_depth }
}

pub const PITCHES: [Pitch; 4] = [
    pitch(0.0, 101.0, 13.0, 20.0),
    pitch(0.0, UNION_CIRCLE_SOUTH, 16.0, 16.0),
    pitch(0.0, 229.0, 13.0, 18.0),
    pitch(0.0, 284.0, 13.0, 13.0),
];

const QUIET_SPOTS_AT_THE_LAWN_EDGES: [Point; 8] = [
    (-15.0, 86.0),
    (14.5, 114.0),
    (-13.0, 152.0),
    (12.5, 181.0),
    (-15.0, 214.0),
    (15.0, 243.0),
    (-14.5, 277.0),
    (14.0, 301.0),
];

fn pitch_clock_offset(pitch_index: usize) -> f64 {
    pitch_index as f64 * 53.0 + 11.0
}

fn game_in_slot(pitch_index: usize, slot: i32, seed: u32, lawns_are_snowy: bool) -> Game {
    let roll = hash_position(pitch_index as i32, slot, seed ^ 0x6A3E) % 100;
    if lawns_are_snowy {
        return match (pitch_index, roll) {
            (0, 0..=44) | (2, 0..=34) => Game::SnowballFight,
            (1, 0..=34) | (3, 0..=29) => Game::Snowman,
            _ => Game::Nothing,
        };
    }
    match (pitch_index, roll) {
        (0, 0..=13) => Game::Football,
        (0, 14..=25) => Game::Frisbee,
        (0, 26..=35) => Game::Volleyball,
        (0, 36..=43) => Game::Spikeball,
        (1, 0..=15) => Game::Volleyball,
        (1, 16..=27) => Game::Frisbee,
        (1, 28..=37) => Game::Spikeball,
        (2, 0..=15) => Game::Football,
        (2, 16..=27) => Game::Volleyball,
        (2, 28..=37) => Game::Frisbee,
        (3.., 0..=13) => Game::Frisbee,
        (3.., 14..=25) => Game::Spikeball,
        (3.., 26..=35) => Game::Football,
        _ => Game::Nothing,
    }
}

fn how_inviting_the_lawns_are(environment: &Environment) -> f32 {
    let weather_factor = match environment.weather {
        Weather::Clear => 1.0,
        Weather::Cloudy => 1.0 - 0.15 * environment.weather_intensity,
        Weather::Fog => 1.0 - 0.5 * environment.weather_intensity,
        Weather::Snow => 1.0 - 0.3 * environment.weather_intensity,
        Weather::Rain | Weather::Storm => 1.0 - environment.weather_intensity.clamp(0.0, 1.0),
    };
    let dryness = 1.0 - smoothstep(0.35, 0.7, environment.wetness);
    smoothstep(0.4, 0.7, environment.daylight()) * weather_factor * dryness
}

fn part_way(from: f32, to: f32, fraction: f32) -> f32 {
    from + (to - from) * fraction.clamp(0.0, 1.0)
}

fn part_way_between(from: Point, to: Point, fraction: f32) -> Point {
    (part_way(from.0, to.0, fraction), part_way(from.1, to.1, fraction))
}

#[derive(Default)]
struct Player {
    number: i32,
    home: Point,
    standing_at: Point,
    pose: Pose,
    facing: i8,
    team_bib: u8,
    stride_frame: u8,
}

struct LawnCast<'a> {
    actors: &'a mut Vec<Actor>,
    how_inviting: f32,
    seed: u32,
    game_tag: i32,
    seconds_into_slot: f32,
    slot_seconds: f32,
}

impl LawnCast<'_> {
    fn player(&mut self, player: Player) {
        let finds_it_too_dark_or_damp =
            self.how_inviting < 0.42 + 0.3 * unit_hash(player.number, self.game_tag, self.seed ^ 0xFA1);
        if finds_it_too_dark_or_damp {
            return;
        }
        let look = hash_position(player.number, self.game_tag, self.seed ^ 0x100C);
        let comes_from_side = if (look >> 7) & 1 == 0 { -1.0 } else { 1.0 };
        let seconds_late = (look >> 9) as f32 % 5.0;
        let mut person = Person::new(player.standing_at.0, player.standing_at.1, look, player.pose);
        person.heading_across_screen = player.facing;
        person.team_bib = player.team_bib;
        person.stride_frame = player.stride_frame;
        if let Some((place, distance_walked)) = self.walking_on_or_off(player.home, comes_from_side, seconds_late) {
            person.metres_east = place.metres_east;
            person.metres_south = place.metres_south;
            person.pose = Pose::Walk;
            person.heading_across_screen = place.facing_on_screen();
            person.stride_frame = stride_frame(distance_walked, 0.62);
        }
        self.actors.push(Actor::Person(person));
    }

    fn walking_on_or_off(&self, home: Point, side: f32, seconds_late: f32) -> Option<(PlaceOnPath, f32)> {
        let gaps = cross_walks_south(side);
        let nearest_gap = gaps.iter().cloned().fold(gaps[0], |nearest, gap| {
            if (gap - home.1).abs() < (nearest - home.1).abs() {
                gap
            } else {
                nearest
            }
        });
        let way_on = [
            (side * 46.0, nearest_gap),
            (side * 28.0, nearest_gap),
            (side * 22.0, home.1 * 0.7 + nearest_gap * 0.3),
            home,
        ];
        let way_length = path_length(&way_on);
        let seconds = self.seconds_into_slot;
        if seconds < SECONDS_TO_WALK_ON {
            let progress = ((seconds - seconds_late) / (SECONDS_TO_WALK_ON - 6.0)).clamp(0.0, 1.0);
            if progress >= 1.0 {
                return None;
            }
            return Some((place_along(&way_on, way_length * progress), way_length * progress));
        }
        let seconds_left = self.slot_seconds - seconds;
        if seconds_left < SECONDS_TO_WALK_OFF {
            let progress =
                ((SECONDS_TO_WALK_OFF - seconds_left - seconds_late) / (SECONDS_TO_WALK_OFF - 6.0)).clamp(0.0, 1.0);
            if progress <= 0.0 {
                return None;
            }
            return Some((place_along(&way_on, way_length * (1.0 - progress)).turned_around(), way_length * progress));
        }
        None
    }

    fn prop(&mut self, at: Point, metres_up: f32, kind: PropKind) {
        self.actors.push(Actor::Prop(Prop { metres_east: at.0, metres_south: at.1, metres_up, kind }));
    }

    fn game_is_under_way(&self) -> bool {
        self.seconds_into_slot > SECONDS_TO_WALK_ON + 2.0
            && self.seconds_into_slot < self.slot_seconds - SECONDS_TO_WALK_OFF
    }
}

fn touch_football(cast: &mut LawnCast, pitch: Pitch, seconds_playing: f32) {
    let (centre_east, centre_south, half_width) = (pitch.centre_east, pitch.centre_south, pitch.half_width);
    let seconds_per_play = 15.0;
    let play = (seconds_playing / seconds_per_play).floor() as i32;
    let into_play = seconds_playing - play as f32 * seconds_per_play;
    let series = play.div_euclid(4);
    let down = play.rem_euclid(4);
    let attacking_direction = if series & 1 == 0 { 1.0 } else { -1.0 };
    let line_of_scrimmage = centre_east + attacking_direction * (-half_width * 0.75 + down as f32 * half_width * 0.38);
    let next_line_of_scrimmage =
        centre_east + attacking_direction * (-half_width * 0.75 + ((down + 1) % 4) as f32 * half_width * 0.38);
    let play_hash = hash_position(play, cast.game_tag, cast.seed ^ 0xF007);
    let target_receiver = (play_hash % 2) as usize;
    let pass_is_complete = (play_hash >> 3) % 10 < 7 || down == 3;
    let (offence_bib, defence_bib) = if series & 1 == 0 { (1u8, 2u8) } else { (2u8, 1u8) };
    let offence_line_up = [
        (line_of_scrimmage - attacking_direction * 0.6, centre_south),
        (line_of_scrimmage - attacking_direction * 3.4, centre_south),
        (line_of_scrimmage - attacking_direction * 0.6, centre_south - 7.0),
        (line_of_scrimmage - attacking_direction * 0.6, centre_south + 7.0),
    ];
    let defence_line_up = [
        (line_of_scrimmage + attacking_direction * 1.7, centre_south),
        (line_of_scrimmage + attacking_direction * 4.5, centre_south - 7.0),
        (line_of_scrimmage + attacking_direction * 4.5, centre_south + 7.0),
        (line_of_scrimmage + attacking_direction * 9.0, centre_south),
    ];
    let huddle = (line_of_scrimmage - attacking_direction * 5.5, centre_south);
    let receiver_routes = [
        (
            line_of_scrimmage + attacking_direction * (11.0 + (play_hash >> 6) as f32 % 4.0),
            centre_south - 5.0 + (play_hash >> 9) as f32 % 4.0,
        ),
        (
            line_of_scrimmage + attacking_direction * (8.0 + (play_hash >> 12) as f32 % 4.0),
            centre_south + 3.0 + (play_hash >> 15) as f32 % 4.0,
        ),
    ];
    let catch_point = receiver_routes[target_receiver];
    let tagged_point = (catch_point.0 + attacking_direction * if down == 3 { 9.0 } else { 3.5 }, catch_point.1);
    let (snap, throw, ball_lands) = (5.0f32, 7.0f32, 8.15f32);
    let tagged = ball_lands + 1.5;
    let offence_facing = if attacking_direction > 0.0 { 1i8 } else { -1 };
    let drift_to_the_next_huddle = |position: Point, pose: Pose, next_huddle_spot: Point| {
        if into_play > 11.5 {
            (part_way_between(position, next_huddle_spot, (into_play - 11.5) / 3.2), Pose::Walk)
        } else {
            (position, pose)
        }
    };
    let stride = |pose: Pose| ((into_play * if pose == Pose::Run { 7.0 } else { 3.4 }) as i32 & 1) as u8;

    for (player_index, &line_up_spot) in offence_line_up.iter().enumerate() {
        let spread = player_index as f32 - 1.5;
        let (position, pose) = if into_play < 4.0 {
            let huddle_spot = (huddle.0 + spread * 0.3, huddle.1 + spread * 1.6);
            let walked = ((into_play - 0.8) / 3.0).clamp(0.0, 1.0);
            (
                part_way_between(huddle_spot, line_up_spot, walked),
                if walked > 0.0 && walked < 1.0 { Pose::Walk } else { Pose::Stand },
            )
        } else if into_play < snap {
            (line_up_spot, if player_index == 0 { Pose::Crouch } else { Pose::Stand })
        } else {
            let route_progress = (into_play - snap) / (ball_lands - snap);
            match player_index {
                0 => (line_up_spot, Pose::Stand),
                1 => {
                    let drop_back_spot = (line_up_spot.0 - attacking_direction * 2.2, line_up_spot.1);
                    let position =
                        (part_way(line_up_spot.0, drop_back_spot.0, (into_play - snap) / 1.2), drop_back_spot.1);
                    (position, if (into_play - throw).abs() < 0.3 { Pose::ArmsUp } else { Pose::Stand })
                }
                _ => {
                    let receiver = player_index - 2;
                    let route_end = receiver_routes[receiver];
                    let catches_it = receiver == target_receiver && pass_is_complete;
                    if into_play < ball_lands || !catches_it {
                        let pose = if route_progress < 1.0 {
                            Pose::Run
                        } else if catches_it {
                            Pose::ArmsUp
                        } else {
                            Pose::Stand
                        };
                        (part_way_between(line_up_spot, route_end, route_progress), pose)
                    } else if into_play < tagged {
                        let run_progress = (into_play - ball_lands) / (tagged - ball_lands);
                        ((part_way(route_end.0, tagged_point.0, run_progress), route_end.1), Pose::Run)
                    } else {
                        (tagged_point, if down == 3 && into_play < tagged + 3.0 { Pose::ArmsUp } else { Pose::Stand })
                    }
                }
            }
        };
        let next_huddle_spot =
            (next_line_of_scrimmage - attacking_direction * 5.5 + spread * 0.3, centre_south + spread * 1.6);
        let (position, pose) = drift_to_the_next_huddle(position, pose, next_huddle_spot);
        cast.player(Player {
            number: player_index as i32,
            home: line_up_spot,
            standing_at: position,
            pose,
            facing: offence_facing,
            team_bib: offence_bib,
            stride_frame: stride(pose),
        });
    }

    for (player_index, &line_up_spot) in defence_line_up.iter().enumerate() {
        let spread = player_index as f32 - 1.5;
        let mut position = line_up_spot;
        let mut pose = Pose::Stand;
        if into_play < 4.0 {
            let huddle_spot = (line_of_scrimmage + attacking_direction * 5.0, centre_south + spread * 1.8);
            let walked = ((into_play - 1.2) / 2.6).clamp(0.0, 1.0);
            position = part_way_between(huddle_spot, line_up_spot, walked);
            pose = if walked > 0.0 && walked < 1.0 { Pose::Walk } else { Pose::Stand };
        } else if into_play >= snap {
            let chase_progress = ((into_play - snap - 0.35) / (ball_lands - snap)).clamp(0.0, 1.0);
            match player_index {
                0 => {
                    position = (
                        part_way(line_up_spot.0, line_of_scrimmage - attacking_direction * 3.0, chase_progress * 0.8),
                        centre_south + 1.5,
                    );
                    pose = Pose::Run;
                }
                1 | 2 => {
                    let receiver = player_index - 1;
                    let covering_spot =
                        (receiver_routes[receiver].0 - attacking_direction * 1.2, receiver_routes[receiver].1 + 1.0);
                    position = part_way_between(line_up_spot, covering_spot, chase_progress);
                    pose = if chase_progress < 1.0 { Pose::Run } else { Pose::Stand };
                    if receiver == target_receiver && pass_is_complete && into_play >= ball_lands {
                        let tackle_progress = ((into_play - ball_lands) / (tagged - ball_lands)).clamp(0.0, 1.0);
                        position = part_way_between(
                            covering_spot,
                            (tagged_point.0 - attacking_direction * 0.6, tagged_point.1 + 0.6),
                            tackle_progress,
                        );
                        pose = if tackle_progress < 1.0 { Pose::Run } else { Pose::Stand };
                    }
                }
                _ => {
                    position = part_way_between(
                        line_up_spot,
                        (catch_point.0 + attacking_direction * 2.0, catch_point.1),
                        chase_progress * 0.7,
                    );
                    pose = if chase_progress < 1.0 { Pose::Run } else { Pose::Stand };
                }
            }
        }
        let next_huddle_spot = (next_line_of_scrimmage + attacking_direction * 5.0, centre_south + spread * 1.8);
        let (position, pose) = drift_to_the_next_huddle(position, pose, next_huddle_spot);
        cast.player(Player {
            number: 10 + player_index as i32,
            home: line_up_spot,
            standing_at: position,
            pose,
            facing: -offence_facing,
            team_bib: defence_bib,
            stride_frame: stride(pose),
        });
    }

    if !cast.game_is_under_way() || into_play >= 11.5 {
        return;
    }
    let quarterback = (offence_line_up[1].0 - attacking_direction * 2.2, offence_line_up[1].1);
    let (ball_east, ball_south, ball_up) = if into_play < snap {
        (offence_line_up[0].0 + attacking_direction * 0.5, centre_south, 0.2)
    } else if into_play < throw {
        (quarterback.0 + attacking_direction * 0.4, quarterback.1, 1.3)
    } else if into_play < ball_lands {
        let flight = (into_play - throw) / (ball_lands - throw);
        let lands_at = if pass_is_complete {
            catch_point
        } else {
            (catch_point.0 + attacking_direction * 3.0, catch_point.1 - 2.0)
        };
        (
            part_way(quarterback.0, lands_at.0, flight),
            part_way(quarterback.1, lands_at.1, flight),
            1.6 + 4.2 * 4.0 * flight * (1.0 - flight),
        )
    } else if pass_is_complete {
        let run_progress = ((into_play - ball_lands) / (tagged - ball_lands)).clamp(0.0, 1.0);
        (part_way(catch_point.0, tagged_point.0, run_progress) + attacking_direction * 0.5, catch_point.1, 1.2)
    } else {
        (catch_point.0 + attacking_direction * (3.0 + (into_play - ball_lands).min(1.5)), catch_point.1 - 2.0, 0.15)
    };
    cast.prop((ball_east, ball_south + 0.3), ball_up, PropKind::Football);
}

fn volleyball(cast: &mut LawnCast, pitch: Pitch, seconds_playing: f32) {
    let (centre_east, centre_south) = (pitch.centre_east, pitch.centre_south);
    let seed = cast.seed;
    let game_tag = cast.game_tag;
    if cast.seconds_into_slot > 14.0 && cast.seconds_into_slot < cast.slot_seconds - 12.0 {
        cast.prop((centre_east, centre_south), 0.0, PropKind::VolleyballNet);
    }
    let seconds_per_touch = 0.92;
    let touches_per_rally = 14;
    let touch = (seconds_playing / seconds_per_touch).floor() as i32;
    let into_touch = seconds_playing / seconds_per_touch - touch as f32;
    let rally = touch.div_euclid(touches_per_rally);
    let touch_in_rally = touch.rem_euclid(touches_per_rally);
    let side_of_touch =
        |touch_in_rally: i32| if (touch_in_rally.div_euclid(2) + rally) & 1 == 0 { -1.0f32 } else { 1.0 };
    let touch_spot = |touch_in_rally: i32| {
        let side = side_of_touch(touch_in_rally);
        let spot_hash = hash_position(rally * 31 + touch_in_rally, game_tag, seed ^ 0x70B);
        let is_near_the_net = touch_in_rally & 1 == 1;
        let distance_from_net =
            if is_near_the_net { 2.6 + (spot_hash % 3) as f32 * 0.6 } else { 5.8 + (spot_hash % 4) as f32 * 0.7 };
        let touching_player = (spot_hash >> 9) as usize % 3;
        (centre_east - 3.6 + (spot_hash >> 4) as f32 % 7.2, centre_south + side * distance_from_net, touching_player)
    };
    let point_is_over = touch_in_rally >= touches_per_rally - 3;
    let (this_touch, next_touch) = (touch_spot(touch_in_rally), touch_spot(touch_in_rally + 1));
    for side in [-1.0f32, 1.0] {
        for player_in_team in 0..3usize {
            let home = (
                centre_east - 3.4 + player_in_team as f32 * 3.4,
                centre_south + side * (3.6 + (player_in_team % 2) as f32 * 3.0),
            );
            let mut position = home;
            let mut pose = Pose::Stand;
            if !point_is_over && side_of_touch(touch_in_rally + 1) == side && next_touch.2 == player_in_team {
                position = (
                    part_way(home.0, next_touch.0, into_touch * 1.3),
                    part_way(home.1, next_touch.1, into_touch * 1.3),
                );
                pose = if into_touch > 0.72 { Pose::ArmsUp } else { Pose::Walk };
            } else if !point_is_over && side_of_touch(touch_in_rally) == side && this_touch.2 == player_in_team {
                position = (part_way(this_touch.0, home.0, into_touch), part_way(this_touch.1, home.1, into_touch));
                pose = if into_touch < 0.25 { Pose::ArmsUp } else { Pose::Stand };
            } else {
                position.0 += ((seconds_playing * 1.7 + player_in_team as f32 * 2.1 + side).sin() * 0.9).round() * 0.5;
            }
            let team_offset = if side < 0.0 { 0 } else { 3 };
            cast.player(Player {
                number: (team_offset + player_in_team) as i32,
                home,
                standing_at: position,
                pose,
                facing: if next_touch.0 > position.0 { 1 } else { -1 },
                team_bib: if side < 0.0 { 3 } else { 2 },
                stride_frame: stride_frame(seconds_playing * 4.0, 1.0),
            });
        }
    }
    if !cast.game_is_under_way() {
        return;
    }
    if point_is_over {
        let last_touch = touch_spot(touches_per_rally - 3);
        let rolled = ((touch_in_rally - (touches_per_rally - 3)) as f32 + into_touch) / 3.0;
        cast.prop((last_touch.0 + 2.0 + rolled * 3.0, last_touch.1), 0.15, PropKind::Volleyball);
        return;
    }
    let goes_over_the_net = side_of_touch(touch_in_rally) != side_of_touch(touch_in_rally + 1);
    let peak_height = if goes_over_the_net { 5.0 } else { 3.6 };
    let ball_up = 1.9 + (peak_height - 1.9) * 4.0 * into_touch * (1.0 - into_touch);
    cast.prop(
        (part_way(this_touch.0, next_touch.0, into_touch), part_way(this_touch.1, next_touch.1, into_touch) + 0.3),
        ball_up,
        PropKind::Volleyball,
    );
}

fn frisbee(cast: &mut LawnCast, pitch: Pitch, seconds_playing: f32) {
    let thrower_count = 3 + (hash_position(cast.game_tag, 5, cast.seed) % 2) as usize;
    let mut ring = [(0.0f32, 0.0f32); 4];
    for (thrower, spot) in ring.iter_mut().enumerate().take(thrower_count) {
        let angle = thrower as f32 / thrower_count as f32 * TAU + 0.6;
        *spot = (
            pitch.centre_east + angle.cos() * pitch.half_width * 0.8,
            pitch.centre_south + angle.sin() * pitch.half_depth * 0.55,
        );
    }
    let ring = &ring[..thrower_count];
    let seconds_per_throw = 2.5;
    let throw = (seconds_playing / seconds_per_throw).floor() as i32;
    let into_throw = seconds_playing / seconds_per_throw - throw as f32;
    let thrower = throw.rem_euclid(thrower_count as i32) as usize;
    let throws_across_the_ring =
        hash_position(throw, cast.game_tag, cast.seed ^ 0xD15C).is_multiple_of(3) && thrower_count > 3;
    let catcher = (thrower + if throws_across_the_ring { 2 } else { 1 }) % thrower_count;
    for (player_index, spot) in ring.iter().enumerate() {
        let reaches_up = (player_index == catcher && into_throw > 0.55 && into_throw < 0.75)
            || (player_index == thrower && into_throw < 0.12);
        let wobble = ((seconds_playing * 1.3 + player_index as f32).sin() * 0.4).round() * 0.4;
        cast.player(Player {
            number: player_index as i32,
            home: *spot,
            standing_at: (spot.0 + wobble, spot.1),
            pose: if reaches_up { Pose::ArmsUp } else { Pose::Stand },
            facing: if ring[(player_index + 1) % thrower_count].0 > spot.0 { 1 } else { -1 },
            ..Player::default()
        });
    }
    if cast.game_is_under_way() && into_throw < 0.7 {
        let flight = into_throw / 0.7;
        let disc_east = part_way(ring[thrower].0, ring[catcher].0, flight);
        let disc_south = part_way(ring[thrower].1, ring[catcher].1, flight);
        cast.prop((disc_east, disc_south + 0.3), 1.3 + 2.4 * 4.0 * flight * (1.0 - flight), PropKind::Frisbee);
    }
}

fn spikeball(cast: &mut LawnCast, pitch: Pitch, seconds_playing: f32) {
    let seed = cast.seed;
    let game_tag = cast.game_tag;
    let net_hash = hash_position(game_tag, 9, seed ^ 0x591);
    let net = (
        pitch.centre_east + ((net_hash % 9) as f32 - 4.0) * pitch.half_width * 0.12,
        pitch.centre_south + ((net_hash >> 5) % 7) as f32 - 3.0,
    );
    if cast.seconds_into_slot > 12.0 && cast.seconds_into_slot < cast.slot_seconds - 10.0 {
        cast.prop(net, 0.0, PropKind::SpikeballNet);
    }
    let seconds_per_bounce = 0.62;
    let bounce = (seconds_playing / seconds_per_bounce).floor() as i32;
    let into_bounce = seconds_playing / seconds_per_bounce - bounce as f32;
    let hitter = |bounce: i32| (hash_position(bounce.div_euclid(2), game_tag, seed ^ 0x592) % 4) as usize;
    let circling = seconds_playing * 0.12;
    let spot_of = |player_index: usize| {
        let angle = player_index as f32 * FRAC_PI_2 + circling;
        (net.0 + angle.cos() * 2.1, net.1 + angle.sin() * 2.6)
    };
    let ball_is_going_up = bounce & 1 == 1;
    for player_index in 0..4usize {
        let spot = spot_of(player_index);
        let is_hitting = (ball_is_going_up && hitter(bounce) == player_index && into_bounce > 0.6)
            || (!ball_is_going_up && hitter(bounce - 1) == player_index && into_bounce < 0.3);
        let pose = if is_hitting {
            Pose::ArmsUp
        } else if (seconds_playing * 2.0 + player_index as f32) as i32 % 5 == 0 {
            Pose::Crouch
        } else {
            Pose::Stand
        };
        cast.player(Player {
            number: player_index as i32,
            home: spot,
            standing_at: spot,
            pose,
            facing: if spot.0 < net.0 { 1 } else { -1 },
            ..Player::default()
        });
    }
    if !cast.game_is_under_way() {
        return;
    }
    let (ball_east, ball_south, ball_up) = if !ball_is_going_up {
        let hit_from = spot_of(hitter(bounce - 1));
        (
            part_way(hit_from.0, net.0, into_bounce),
            part_way(hit_from.1, net.1, into_bounce),
            1.3 * (1.0 - into_bounce) + 0.5,
        )
    } else {
        let next_hitter = spot_of(hitter(bounce));
        (
            part_way(net.0, next_hitter.0, into_bounce),
            part_way(net.1, next_hitter.1, into_bounce),
            0.5 + 1.6 * (into_bounce * (2.0 - into_bounce)),
        )
    };
    cast.prop((ball_east, ball_south + 0.2), ball_up, PropKind::Spikeball);
}

fn snowball_fight(cast: &mut LawnCast, pitch: Pitch, seconds_playing: f32) {
    let forts = [
        (pitch.centre_east - pitch.half_width * 0.62, pitch.centre_south),
        (pitch.centre_east + pitch.half_width * 0.62, pitch.centre_south),
    ];
    if cast.seconds_into_slot > 20.0 {
        for fort in forts {
            cast.prop(fort, 0.0, PropKind::SnowFort);
        }
    }
    let seconds_per_throw = 1.15;
    let throw = (seconds_playing / seconds_per_throw).floor() as i32;
    let into_throw = seconds_playing / seconds_per_throw - throw as f32;
    let throw_hash = hash_position(throw, cast.game_tag, cast.seed ^ 0x5B0);
    let throwing_side = (throw & 1) as usize;
    let thrower = (throw_hash % 3) as usize;
    let target = ((throw_hash >> 3) % 3) as usize;
    let hits = (throw_hash >> 6).is_multiple_of(3);
    let spot_behind_fort = |side: usize, player_in_team: usize| {
        let outward = if side == 0 { -1.0 } else { 1.0 };
        (
            forts[side].0 + outward * (1.2 + player_in_team as f32 * 1.3),
            forts[side].1 - 3.6 + player_in_team as f32 * 3.6,
        )
    };
    for side in 0..2usize {
        for player_in_team in 0..3usize {
            let home = spot_behind_fort(side, player_in_team);
            let mut pose = if (seconds_playing * 0.8 + player_in_team as f32 * 1.3 + side as f32).sin() > 0.72 {
                Pose::Crouch
            } else {
                Pose::Stand
            };
            if side == throwing_side && player_in_team == thrower && into_throw < 0.35 {
                pose = Pose::ArmsUp;
            }
            if side != throwing_side && player_in_team == target && hits && into_throw > 0.85 {
                pose = Pose::Crouch;
            }
            cast.player(Player {
                number: (side * 3 + player_in_team) as i32,
                home,
                standing_at: home,
                pose,
                facing: if side == 0 { 1 } else { -1 },
                ..Player::default()
            });
        }
    }
    if !cast.game_is_under_way() {
        return;
    }
    let outward = if throwing_side == 0 { -1.0 } else { 1.0 };
    let thrown_from = spot_behind_fort(throwing_side, thrower);
    let lands_at = (
        forts[1 - throwing_side].0 - outward * (1.2 + target as f32 * 1.3) + if hits { 0.0 } else { -outward * 3.0 },
        forts[1 - throwing_side].1 - 3.6 + target as f32 * 3.6,
    );
    let ball_up = 1.5 + 3.0 * 4.0 * into_throw * (1.0 - into_throw);
    cast.prop(
        (part_way(thrown_from.0, lands_at.0, into_throw), part_way(thrown_from.1, lands_at.1, into_throw) + 0.3),
        ball_up,
        PropKind::Snowball,
    );
}

fn building_a_snowman(cast: &mut LawnCast, pitch: Pitch, seconds_playing: f32) {
    let snowman = (pitch.centre_east + pitch.half_width * 0.25, pitch.centre_south);
    let seconds = cast.seconds_into_slot;
    let stage = if seconds < SECONDS_TO_WALK_ON + 10.0 {
        0
    } else if seconds < SECONDS_TO_WALK_ON + 45.0 {
        1
    } else if seconds < SECONDS_TO_WALK_ON + 80.0 {
        2
    } else {
        3
    };
    if stage > 0 {
        cast.prop(snowman, 0.0, PropKind::Snowman { stage });
    }
    let rolling_angle = seconds_playing * 0.35;
    let roller = (snowman.0 - 5.0 + rolling_angle.cos() * 3.0, snowman.1 + rolling_angle.sin() * 5.0);
    let still_rolling = stage < 3;
    cast.player(Player {
        number: 0,
        home: roller,
        standing_at: roller,
        pose: if still_rolling { Pose::Crouch } else { Pose::ArmsUp },
        facing: 1,
        ..Player::default()
    });
    if still_rolling && cast.game_is_under_way() {
        cast.prop((roller.0 + 0.9, roller.1 + 0.2), 0.0, PropKind::SnowballBeingRolled);
    }
    let admirer = (snowman.0 + 1.4, snowman.1 + 1.0);
    cast.player(Player {
        number: 1,
        home: admirer,
        standing_at: admirer,
        pose: if (seconds_playing * 0.9).sin() > 0.3 { Pose::ArmsUp } else { Pose::Stand },
        facing: -1,
        ..Player::default()
    });
    let patter = (snowman.0 - 1.5, snowman.1 - 1.2);
    cast.player(Player {
        number: 2,
        home: patter,
        standing_at: patter,
        pose: if (seconds_playing * 0.6 + 1.0).sin() > 0.5 { Pose::Crouch } else { Pose::Stand },
        facing: 1,
        ..Player::default()
    });
}

fn quiet_company_at_the_lawn_edges(environment: &Environment, seed: u32, how_inviting: f32, actors: &mut Vec<Actor>) {
    for (spot_index, &(spot_east, spot_south)) in QUIET_SPOTS_AT_THE_LAWN_EDGES.iter().enumerate() {
        let (slot, seconds_into_slot) =
            cycle_and_seconds_into_it(environment.animation_seconds + spot_index as f64 * 61.0, QUIET_SLOT_SECONDS);
        let slot_hash = hash_position(spot_index as i32, slot, seed ^ 0xA3B1);
        let what_happens = slot_hash % 100;
        if what_happens >= 52 {
            continue;
        }
        let slot_seconds = QUIET_SLOT_SECONDS as f32;
        let game_tag = 4000 + spot_index as i32 * 97 + (slot & 0xFFFF);
        let mut cast = LawnCast { actors, how_inviting, seed, game_tag, seconds_into_slot, slot_seconds };
        let everyone_has_settled =
            seconds_into_slot > SECONDS_TO_WALK_ON && seconds_into_slot < slot_seconds - SECONDS_TO_WALK_OFF;
        if what_happens < 24 {
            let picnickers = 2 + (slot_hash >> 8) as usize % 3;
            if seconds_into_slot > SECONDS_TO_WALK_ON - 4.0
                && seconds_into_slot < slot_seconds - SECONDS_TO_WALK_OFF + 4.0
            {
                cast.prop(
                    (spot_east, spot_south),
                    0.0,
                    PropKind::PicnicBlanket { pattern: (slot_hash >> 12) as usize & 3 },
                );
            }
            for (picnicker, offset) in
                [(-1.2f32, 0.0f32), (1.2, 0.2), (0.0, -1.0), (0.2, 1.1)].into_iter().enumerate().take(picnickers)
            {
                let seat = (spot_east + offset.0, spot_south + offset.1);
                cast.player(Player {
                    number: picnicker as i32,
                    home: seat,
                    standing_at: seat,
                    pose: Pose::Sit,
                    facing: if offset.0 < 0.0 { 1 } else { -1 },
                    ..Player::default()
                });
            }
        } else if what_happens < 40 {
            let sunbathers = 1 + (slot_hash >> 8) as usize % 2;
            for sunbather in 0..sunbathers {
                let towel = (spot_east + sunbather as f32 * 0.4, spot_south + sunbather as f32 * 3.0);
                if everyone_has_settled {
                    cast.prop(
                        towel,
                        0.0,
                        PropKind::BeachTowel { colour: ((slot_hash >> (10 + sunbather * 2)) & 3) as usize },
                    );
                }
                cast.player(Player {
                    number: sunbather as i32,
                    home: towel,
                    standing_at: towel,
                    pose: Pose::Lie,
                    ..Player::default()
                });
            }
        } else {
            let reader = (spot_east, spot_south);
            cast.player(Player {
                number: 0,
                home: reader,
                standing_at: reader,
                pose: Pose::Sit,
                facing: 1,
                ..Player::default()
            });
            if everyone_has_settled {
                cast.prop((spot_east + 0.6, spot_south + 0.1), 0.5, PropKind::Book);
            }
        }
    }
}

pub fn add_lawn_life(environment: &Environment, seed: u32, actors: &mut Vec<Actor>) {
    let how_inviting = how_inviting_the_lawns_are(environment);
    if how_inviting < 0.4 {
        return;
    }
    let lawns_are_snowy = environment.snow_cover > 0.3;
    let summer_games_are_off = !lawns_are_snowy && environment.snow_cover > 0.12;
    for (pitch_index, &pitch) in PITCHES.iter().enumerate() {
        let (slot, seconds_into_slot) = cycle_and_seconds_into_it(
            environment.animation_seconds + pitch_clock_offset(pitch_index),
            GAME_SLOT_SECONDS,
        );
        let game = game_in_slot(pitch_index, slot, seed, lawns_are_snowy);
        if game == Game::Nothing || summer_games_are_off {
            continue;
        }
        let too_cold_for_most = environment.season == Season::Winter
            && !lawns_are_snowy
            && hash_position(pitch_index as i32, slot, seed ^ 0xC01D).is_multiple_of(2);
        if too_cold_for_most {
            continue;
        }
        let game_tag = pitch_index as i32 * 1000 + (slot & 0xFFFF);
        let seconds_playing = (seconds_into_slot - SECONDS_TO_WALK_ON).max(0.0);
        let mut cast = LawnCast {
            actors,
            how_inviting,
            seed,
            game_tag,
            seconds_into_slot,
            slot_seconds: GAME_SLOT_SECONDS as f32,
        };
        match game {
            Game::Football => touch_football(&mut cast, pitch, seconds_playing),
            Game::Volleyball => volleyball(&mut cast, pitch, seconds_playing),
            Game::Frisbee => frisbee(&mut cast, pitch, seconds_playing),
            Game::Spikeball => spikeball(&mut cast, pitch, seconds_playing),
            Game::SnowballFight => snowball_fight(&mut cast, pitch, seconds_playing),
            Game::Snowman => building_a_snowman(&mut cast, pitch, seconds_playing),
            Game::Nothing => {}
        }
    }
    if !lawns_are_snowy && environment.snow_cover < 0.12 && environment.season != Season::Winter {
        quiet_company_at_the_lawn_edges(environment, seed, how_inviting, actors);
    }
}
