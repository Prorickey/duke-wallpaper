use crate::chapel::anim::{
    presence, stride_frame, Conditions, Pose, Sprite, SpriteKind, World, CLOCK_WRAP_SECONDS, DUKE_BLUE,
};
use crate::chapel::palette::smooth_step;
use crate::hash_fraction;
use crate::Environment;

type Point = (f32, f32);

pub(crate) const SLOT_SECONDS: f32 = 360.0;
const SLOTS_PER_CLOCK_WRAP: i32 = (CLOCK_WRAP_SECONDS / SLOT_SECONDS) as i32;
const SLOT_ENDS_EARLY_BY_SECONDS: f32 = 8.0;
const WALK_ON_SECONDS: f32 = 26.0;
const WALK_OFF_SECONDS: f32 = 26.0;
const SECONDS_BETWEEN_PLAYERS_SETTING_OUT: f32 = 1.5;
const SETTING_UP_SECONDS: f32 = 8.0;

const GATE_BELOW_THE_FRAME_WEST: Point = (-6.2, -44.0);
const GATE_BELOW_THE_FRAME_EAST: Point = (6.2, -44.0);
const GATE_UNDER_THE_EAST_ARCADE: Point = (22.8, 7.4);

const WHITE_TEAM_SHIRT: u32 = 0xe4e4e8;
const FOOTBALL_COLOUR: u32 = 0x8a4b2a;
const VOLLEYBALL_COLOUR: u32 = 0xf6f4ea;
const FRISBEE_COLOUR: u32 = 0xf2742a;
const SNOWBALL_COLOUR: u32 = 0xf6f9fd;
const RED_BLANKET: u32 = 0xc8323a;
const BLUE_BLANKET: u32 = 0x2f62b0;

#[derive(Clone, Copy, PartialEq, Debug)]
enum LawnEvent {
    TouchFootball,
    Volleyball,
    Frisbee,
    Picnic,
    SnowballFight,
    SnowmanBuilding,
}

type Programme = &'static [(LawnEvent, f32)];

const PROGRAMMES_ON_GRASS: [Programme; 9] = [
    &[(LawnEvent::TouchFootball, 0.0)],
    &[(LawnEvent::Volleyball, 0.0)],
    &[(LawnEvent::Frisbee, 0.0)],
    &[(LawnEvent::Picnic, 0.0)],
    &[(LawnEvent::TouchFootball, 0.0), (LawnEvent::Picnic, 70.0)],
    &[(LawnEvent::Volleyball, 0.0), (LawnEvent::Frisbee, 90.0)],
    &[(LawnEvent::Frisbee, 0.0), (LawnEvent::Picnic, 60.0)],
    &[(LawnEvent::Volleyball, 30.0), (LawnEvent::Picnic, 120.0)],
    &[],
];

const PROGRAMMES_IN_SNOW: [Programme; 4] = [
    &[(LawnEvent::SnowballFight, 0.0)],
    &[(LawnEvent::SnowmanBuilding, 0.0)],
    &[(LawnEvent::SnowballFight, 0.0), (LawnEvent::SnowmanBuilding, 50.0)],
    &[],
];

#[derive(Clone, Copy)]
struct PlayerNow {
    position: Point,
    metres_up: f32,
    pose: Pose,
}

fn standing_at(position: Point) -> PlayerNow {
    PlayerNow { position, metres_up: 0.0, pose: Pose::Standing }
}

fn between(from: Point, to: Point, fraction: f32) -> Point {
    let fraction = fraction.clamp(0.0, 1.0);
    (from.0 + (to.0 - from.0) * fraction, from.1 + (to.1 - from.1) * fraction)
}

fn height_along_arc(fraction: f32, peak_metres: f32) -> f32 {
    4.0 * peak_metres * fraction * (1.0 - fraction)
}

fn programme_roll(slot: i32, seed: u32) -> usize {
    let slot_on_the_clock = slot.rem_euclid(SLOTS_PER_CLOCK_WRAP);
    (hash_fraction(slot_on_the_clock, 7, seed ^ 0xE7E) * 8.999) as usize
}

fn weather_needed_for(slot: i32, event_in_programme: usize, seed: u32) -> f32 {
    let slot_on_the_clock = slot.rem_euclid(SLOTS_PER_CLOCK_WRAP);
    hash_fraction(slot_on_the_clock, 8 + event_in_programme as i32, seed ^ 0xE7E)
}

pub fn add_lawn_events(
    environment: &Environment,
    world: &World,
    seconds: f32,
    conditions: &Conditions,
    sprites: &mut Vec<Sprite>,
) {
    let weather_for_playing = conditions.dryness() * smooth_step(0.6, 0.72, conditions.fair_weather);
    let good_for_playing = smooth_step(0.4, 0.5, conditions.daylight) * weather_for_playing;
    let snow_lies = smooth_step(0.3, 0.4, conditions.snow_cover);
    let grass_shows = 1.0 - smooth_step(0.2, 0.3, conditions.snow_cover);
    let warm_enough_to_sit_out = 1.0 - smooth_step(0.3, 0.6, conditions.coldness);

    let slot = (seconds / SLOT_SECONDS).floor() as i32;
    let seconds_into_slot = seconds - slot as f32 * SLOT_SECONDS;
    for (event_in_programme, &(event, starts_after)) in
        PROGRAMMES_ON_GRASS[programme_roll(slot, world.seed)].iter().enumerate()
    {
        let needs_warmth = matches!(event, LawnEvent::Picnic | LawnEvent::Volleyball);
        let suitability = good_for_playing * grass_shows * if needs_warmth { warm_enough_to_sit_out } else { 1.0 };
        let opacity = presence(suitability, weather_needed_for(slot, event_in_programme, world.seed));
        let event_seconds = SLOT_SECONDS - starts_after - SLOT_ENDS_EARLY_BY_SECONDS;
        stage_event(
            event,
            slot,
            seconds_into_slot - starts_after,
            event_seconds,
            opacity,
            world.seed,
            seconds,
            conditions,
            sprites,
        );
    }
    for slots_ago in [1, 0] {
        let earlier_slot = slot - slots_ago;
        for (event_in_programme, &(event, starts_after)) in
            PROGRAMMES_IN_SNOW[programme_roll(earlier_slot, world.seed) % PROGRAMMES_IN_SNOW.len()].iter().enumerate()
        {
            let weather_needed = weather_needed_for(earlier_slot, event_in_programme, world.seed);
            let seconds_into_event = seconds_into_slot + slots_ago as f32 * SLOT_SECONDS - starts_after;
            let event_seconds = SLOT_SECONDS - starts_after - SLOT_ENDS_EARLY_BY_SECONDS;
            if slots_ago == 0 {
                stage_event(
                    event,
                    slot,
                    seconds_into_event,
                    event_seconds,
                    presence(good_for_playing * snow_lies, weather_needed),
                    world.seed,
                    seconds,
                    conditions,
                    sprites,
                );
            }
            if event != LawnEvent::SnowmanBuilding {
                continue;
            }
            let hour_when_it_was_built =
                environment.hour - (seconds_into_event - WALK_ON_SECONDS - SNOWMAN_ROLLING_SECONDS) / 3600.0;
            let environment_when_it_was_built = Environment { hour: hour_when_it_was_built, ..environment.clone() };
            let builders_were_out =
                smooth_step(0.4, 0.5, environment_when_it_was_built.daylight()) * weather_for_playing;
            add_snowman_left_standing(
                earlier_slot,
                seconds_into_slot + slots_ago as f32 * SLOT_SECONDS,
                starts_after,
                presence(snow_lies * builders_were_out, weather_needed),
                sprites,
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn stage_event(
    event: LawnEvent,
    slot: i32,
    seconds_into_event: f32,
    event_seconds: f32,
    opacity: f32,
    seed: u32,
    seconds: f32,
    conditions: &Conditions,
    sprites: &mut Vec<Sprite>,
) {
    if opacity <= 0.004 || seconds_into_event < 0.0 || seconds_into_event > event_seconds {
        return;
    }
    let slot_on_the_clock = slot.rem_euclid(SLOTS_PER_CLOCK_WRAP);
    let event_seed = seed ^ (slot_on_the_clock as u32).wrapping_mul(0x9E37) ^ ((event as u32 + 1) * 7717);
    let playing_seconds = event_seconds - WALK_ON_SECONDS - WALK_OFF_SECONDS;
    let seconds_into_play = (seconds_into_event - WALK_ON_SECONDS).clamp(0.0, playing_seconds);
    let walk_off_begins = event_seconds - WALK_OFF_SECONDS;
    let is_walking_on = seconds_into_event < WALK_ON_SECONDS;
    let is_walking_off = seconds_into_event > walk_off_begins;
    let is_playing = !is_walking_on && !is_walking_off;

    let (players, gate) = match event {
        LawnEvent::TouchFootball => (6, GATE_BELOW_THE_FRAME_WEST),
        LawnEvent::Frisbee => (4, GATE_BELOW_THE_FRAME_EAST),
        LawnEvent::Picnic => (5, GATE_BELOW_THE_FRAME_WEST),
        LawnEvent::Volleyball => (6, GATE_UNDER_THE_EAST_ARCADE),
        LawnEvent::SnowballFight => (6, GATE_BELOW_THE_FRAME_WEST),
        LawnEvent::SnowmanBuilding => (3, GATE_BELOW_THE_FRAME_EAST),
    };
    let player_at = |player: usize, seconds_into_play: f32| match event {
        LawnEvent::TouchFootball => football_player(player, seconds_into_play, event_seed, seconds),
        LawnEvent::Volleyball => volleyball_player(player, seconds_into_play, event_seed),
        LawnEvent::Frisbee => frisbee_player(player, seconds_into_play, event_seed, seconds),
        LawnEvent::Picnic => picnicker(player, seconds_into_play),
        LawnEvent::SnowballFight => snowball_fighter(player, seconds_into_play, event_seed, seconds),
        LawnEvent::SnowmanBuilding => snowman_builder(player, seconds_into_play, slot),
    };

    for player in 0..players {
        let sets_out_after = player as f32 * SECONDS_BETWEEN_PLAYERS_SETTING_OUT;
        let place_in_the_gateway =
            (gate.0 + (player as f32 - players as f32 * 0.5) * 0.6, gate.1 - (player % 2) as f32 * 0.8);
        let walking_seconds = WALK_ON_SECONDS - 10.0;
        let now = if is_walking_on {
            let seconds_walked = seconds_into_event - sets_out_after;
            let fraction = seconds_walked / walking_seconds;
            let pose =
                if fraction < 1.0 { Pose::Walking { stride: stride_frame(seconds_walked) } } else { Pose::Standing };
            PlayerNow {
                position: between(place_in_the_gateway, player_at(player, 0.0).position, fraction),
                metres_up: 0.0,
                pose,
            }
        } else if is_walking_off {
            let seconds_walked = seconds_into_event - walk_off_begins - sets_out_after;
            let fraction = seconds_walked / walking_seconds;
            let pose =
                if fraction > 0.0 { Pose::Walking { stride: stride_frame(seconds_walked) } } else { Pose::Standing };
            PlayerNow {
                position: between(player_at(player, playing_seconds).position, place_in_the_gateway, fraction),
                metres_up: 0.0,
                pose,
            }
        } else {
            player_at(player, seconds_into_play)
        };

        let person = 9000 + (event_seed % 500) * 8 + player as u32;
        let mut sprite = Sprite::person(now.position, person, now.pose, conditions).faded(opacity);
        sprite.metres_up = now.metres_up;
        if let SpriteKind::Person { outfit, has_umbrella, .. } = &mut sprite.kind {
            outfit.backpack = None;
            *has_umbrella = false;
            if matches!(event, LawnEvent::TouchFootball | LawnEvent::Volleyball) {
                outfit.top = if player < players / 2 { DUKE_BLUE } else { WHITE_TEAM_SHIRT };
            }
        }
        sprites.push(sprite);
    }

    let set_up = smooth_step(WALK_ON_SECONDS - SETTING_UP_SECONDS, WALK_ON_SECONDS, seconds_into_event)
        * (1.0 - smooth_step(walk_off_begins, walk_off_begins + SETTING_UP_SECONDS, seconds_into_event));
    let mut add_prop = |position: Point, metres_up: f32, kind: SpriteKind| {
        let mut prop = Sprite::on_the_ground(position, kind).faded(opacity);
        prop.metres_up = metres_up;
        sprites.push(prop);
    };
    match event {
        LawnEvent::TouchFootball => {
            if let (true, Some((position, metres_up))) = (is_playing, football_position(seconds_into_play, event_seed))
            {
                add_prop(position, metres_up, SpriteKind::Ball { colour: FOOTBALL_COLOUR, is_large: false });
            }
        }
        LawnEvent::Volleyball => {
            if set_up > 0.05 {
                add_prop(
                    VOLLEYBALL_NET_CENTRE,
                    0.0,
                    SpriteKind::VolleyballNet { metres_wide: VOLLEYBALL_NET_METRES, raised: set_up },
                );
            }
            if is_playing {
                let (hit, fraction_to_next_hit) = volleyball_hit_now(seconds_into_play);
                let position = between(
                    volleyball_hit_point(hit, event_seed),
                    volleyball_hit_point(hit + 1, event_seed),
                    fraction_to_next_hit,
                );
                add_prop(
                    position,
                    1.9 + height_along_arc(fraction_to_next_hit, 3.4),
                    SpriteKind::Ball { colour: VOLLEYBALL_COLOUR, is_large: false },
                );
            }
        }
        LawnEvent::Frisbee => {
            let (throw, fraction_of_flight) = frisbee_throw_now(seconds_into_play);
            if is_playing && fraction_of_flight < 1.0 {
                let position = between(
                    frisbee_catch_point(throw - 1, event_seed),
                    frisbee_catch_point(throw, event_seed),
                    fraction_of_flight,
                );
                add_prop(
                    position,
                    1.3 + height_along_arc(fraction_of_flight, 0.9),
                    SpriteKind::Ball { colour: FRISBEE_COLOUR, is_large: false },
                );
            }
        }
        LawnEvent::Picnic => {
            if set_up > 0.05 {
                add_prop(RED_BLANKET_CENTRE, 0.0, SpriteKind::PicnicBlanket { colour: RED_BLANKET, unrolled: set_up });
                add_prop(
                    BLUE_BLANKET_CENTRE,
                    0.0,
                    SpriteKind::PicnicBlanket { colour: BLUE_BLANKET, unrolled: set_up },
                );
            }
        }
        LawnEvent::SnowballFight => {
            if set_up > 0.3 {
                add_prop(SOUTH_SNOW_FORT, 0.0, SpriteKind::SnowFort);
                add_prop(NORTH_SNOW_FORT, 0.0, SpriteKind::SnowFort);
            }
            let latest_throw = (seconds_into_play / SECONDS_BETWEEN_SNOWBALLS).floor() as i32;
            for throw in [latest_throw - 1, latest_throw] {
                let fraction_of_flight =
                    (seconds_into_play - throw as f32 * SECONDS_BETWEEN_SNOWBALLS) / SNOWBALL_FLIGHT_SECONDS;
                if is_playing && throw >= 0 && (0.0..1.0).contains(&fraction_of_flight) {
                    let (thrower, target) = snowball_thrower_and_target(throw, event_seed);
                    let position = between(
                        snowball_fighter_position(thrower, seconds),
                        snowball_fighter_position(target, seconds),
                        fraction_of_flight,
                    );
                    add_prop(
                        position,
                        1.5 + height_along_arc(fraction_of_flight, 2.0),
                        SpriteKind::Ball { colour: SNOWBALL_COLOUR, is_large: false },
                    );
                }
            }
        }
        LawnEvent::SnowmanBuilding => {
            if is_playing && seconds_into_play < SNOWMAN_ROLLING_SECONDS {
                let roller = snowman_builder(0, seconds_into_play, slot).position;
                let has_grown_large = seconds_into_play > SNOWMAN_ROLLING_SECONDS * 0.4;
                add_prop(
                    (roller.0 + 0.7, roller.1 - 0.2),
                    0.3,
                    SpriteKind::Ball { colour: SNOWBALL_COLOUR, is_large: has_grown_large },
                );
            }
        }
    }
}

const FOOTBALL_FIELD_CENTRE_EAST: f32 = -15.5;
const FOOTBALL_FIELD_SOUTH_END: f32 = -31.4;
const FOOTBALL_FIELD_NORTH_END: f32 = -13.6;
const FOOTBALL_FIELD_WEST_SIDE: f32 = -20.6;
const FOOTBALL_FIELD_EAST_SIDE: f32 = -10.4;
const SECONDS_PER_DOWN: f32 = 15.0;
const DOWNS_PER_DRIVE: i32 = 3;
const LINED_UP_BY: f32 = 7.0;
const SNAP_AT: f32 = 8.0;
const THROW_AT: f32 = 9.2;
const CATCH_AT: f32 = 10.4;
const TAGGED_AT: f32 = 12.2;
const QUARTERBACK: usize = 0;
const CARRIED_BALL_METRES_UP: f32 = 1.1;

struct Down {
    line_up: [Point; 6],
    routes_run: [Point; 6],
    tagged: [Point; 6],
    receiver_thrown_to: usize,
}

fn football_down(down: i32, seed: u32) -> Down {
    let chance = |trait_number: i32| hash_fraction(down, trait_number, seed ^ 0xF00B);
    let drive_heads_north = down.div_euclid(DOWNS_PER_DRIVE) % 2 == 0;
    let upfield = if drive_heads_north { 1.0 } else { -1.0 };
    let downs_into_drive = down.rem_euclid(DOWNS_PER_DRIVE) as f32;
    let scrimmage_north =
        if drive_heads_north { -29.0 + downs_into_drive * 4.0 } else { -16.0 - downs_into_drive * 4.0 };
    let on_the_field = |north: f32| north.clamp(FOOTBALL_FIELD_SOUTH_END, FOOTBALL_FIELD_NORTH_END);
    let centre = FOOTBALL_FIELD_CENTRE_EAST;

    let line_up = [
        (centre, on_the_field(scrimmage_north - upfield * 1.6)),
        (centre - 3.6, scrimmage_north),
        (centre + 3.6, scrimmage_north),
        (centre - 3.6, scrimmage_north + upfield * 1.5),
        (centre + 3.6, scrimmage_north + upfield * 1.5),
        (centre, on_the_field(scrimmage_north + upfield * 4.0)),
    ];
    let west_route_end =
        (centre - 3.6 + (chance(2) - 0.3) * 3.4, on_the_field(scrimmage_north + upfield * (6.0 + chance(3) * 3.5)));
    let east_route_end =
        (centre + 3.6 - (chance(4) - 0.3) * 3.4, on_the_field(scrimmage_north + upfield * (6.0 + chance(5) * 3.5)));
    let routes_run = [
        (centre + (chance(6) - 0.5) * 2.0, on_the_field(scrimmage_north - upfield * 3.2)),
        west_route_end,
        east_route_end,
        (west_route_end.0 + 0.9, west_route_end.1 - upfield * 1.1),
        (east_route_end.0 - 0.9, east_route_end.1 - upfield * 1.1),
        (centre, on_the_field(scrimmage_north + upfield * 7.0)),
    ];
    let receiver_thrown_to = 1 + (chance(1) * 1.999) as usize;
    let caught_at = routes_run[receiver_thrown_to];
    let run_ends = (
        (caught_at.0 + (chance(7) - 0.5) * 2.4).clamp(FOOTBALL_FIELD_WEST_SIDE, FOOTBALL_FIELD_EAST_SIDE),
        on_the_field(caught_at.1 + upfield * 3.6),
    );
    let mut tagged = routes_run;
    tagged[receiver_thrown_to] = run_ends;
    tagged[receiver_thrown_to + 2] = (run_ends.0 + 0.8, run_ends.1 - upfield * 0.5);
    Down { line_up, routes_run, tagged, receiver_thrown_to }
}

fn football_player(player: usize, seconds_into_play: f32, seed: u32, seconds: f32) -> PlayerNow {
    let down_number = (seconds_into_play / SECONDS_PER_DOWN).floor() as i32;
    let seconds_into_down = seconds_into_play - down_number as f32 * SECONDS_PER_DOWN;
    let down = football_down(down_number, seed);
    let running = Pose::Running { stride: stride_frame(seconds * 2.5 + player as f32 * 0.3) };
    let is_receiver_thrown_to = player == down.receiver_thrown_to;
    let is_their_defender = player == down.receiver_thrown_to + 2;

    if seconds_into_down < LINED_UP_BY {
        let came_from =
            if down_number == 0 { down.line_up[player] } else { football_down(down_number - 1, seed).tagged[player] };
        let fraction = smooth_step(0.5, LINED_UP_BY - 0.6, seconds_into_down);
        let is_walking_back = fraction < 1.0 && down_number > 0;
        let pose = if is_walking_back {
            Pose::Walking { stride: stride_frame(seconds + player as f32) }
        } else {
            Pose::Standing
        };
        return PlayerNow { position: between(came_from, down.line_up[player], fraction), metres_up: 0.0, pose };
    }
    if seconds_into_down < SNAP_AT {
        return standing_at(down.line_up[player]);
    }
    if seconds_into_down < CATCH_AT {
        let seconds_since_snap = seconds_into_down - SNAP_AT;
        if player == QUARTERBACK {
            let drop_back = seconds_since_snap / 0.9;
            let is_throwing = (THROW_AT - 0.2..THROW_AT + 0.3).contains(&seconds_into_down);
            let pose = if is_throwing {
                Pose::ArmsRaised
            } else if drop_back < 1.0 {
                running
            } else {
                Pose::Standing
            };
            return PlayerNow {
                position: between(down.line_up[player], down.routes_run[player], drop_back),
                metres_up: 0.0,
                pose,
            };
        }
        let position =
            between(down.line_up[player], down.routes_run[player], seconds_since_snap / (CATCH_AT - SNAP_AT));
        let is_reaching_for_the_catch = is_receiver_thrown_to && seconds_into_down > CATCH_AT - 0.3;
        if is_reaching_for_the_catch {
            return PlayerNow { position, metres_up: 0.4, pose: Pose::ArmsRaised };
        }
        return PlayerNow { position, metres_up: 0.0, pose: running };
    }
    if seconds_into_down < TAGGED_AT {
        let fraction = (seconds_into_down - CATCH_AT) / (TAGGED_AT - CATCH_AT);
        let pose = if is_receiver_thrown_to || is_their_defender { running } else { Pose::Standing };
        return PlayerNow {
            position: between(down.routes_run[player], down.tagged[player], fraction),
            metres_up: 0.0,
            pose,
        };
    }
    let is_celebrating = is_receiver_thrown_to && seconds_into_down < TAGGED_AT + 1.0;
    PlayerNow {
        position: down.tagged[player],
        metres_up: 0.0,
        pose: if is_celebrating { Pose::ArmsRaised } else { Pose::Standing },
    }
}

fn football_position(seconds_into_play: f32, seed: u32) -> Option<(Point, f32)> {
    let down_number = (seconds_into_play / SECONDS_PER_DOWN).floor() as i32;
    let seconds_into_down = seconds_into_play - down_number as f32 * SECONDS_PER_DOWN;
    let down = football_down(down_number, seed);
    let receiver = down.receiver_thrown_to;
    if seconds_into_down < LINED_UP_BY {
        return None;
    }
    if seconds_into_down < SNAP_AT {
        return Some((down.line_up[QUARTERBACK], CARRIED_BALL_METRES_UP));
    }
    if seconds_into_down < THROW_AT {
        let drop_back = (seconds_into_down - SNAP_AT) / 0.9;
        return Some((
            between(down.line_up[QUARTERBACK], down.routes_run[QUARTERBACK], drop_back),
            CARRIED_BALL_METRES_UP,
        ));
    }
    if seconds_into_down < CATCH_AT {
        let fraction_of_flight = (seconds_into_down - THROW_AT) / (CATCH_AT - THROW_AT);
        let position = between(down.routes_run[QUARTERBACK], down.routes_run[receiver], fraction_of_flight);
        return Some((position, 1.6 + height_along_arc(fraction_of_flight, 3.2)));
    }
    let fraction_of_run = (seconds_into_down - CATCH_AT) / (TAGGED_AT - CATCH_AT);
    Some((between(down.routes_run[receiver], down.tagged[receiver], fraction_of_run), CARRIED_BALL_METRES_UP))
}

const VOLLEYBALL_NET_CENTRE: Point = (24.3, 27.5);
const VOLLEYBALL_NET_METRES: f32 = 7.8;
const SECONDS_BETWEEN_VOLLEYBALL_HITS: f32 = 1.5;
const VOLLEYBALL_HOME_SPOTS: [Point; 6] =
    [(21.9, 23.6), (26.7, 23.6), (24.3, 20.9), (21.9, 31.4), (26.7, 31.4), (24.3, 34.1)];
const VOLLEYBALL_JUMP_METRES: f32 = 0.5;

fn volleyball_hit_now(seconds_into_play: f32) -> (i32, f32) {
    let hit = (seconds_into_play / SECONDS_BETWEEN_VOLLEYBALL_HITS).floor() as i32;
    (hit, seconds_into_play / SECONDS_BETWEEN_VOLLEYBALL_HITS - hit as f32)
}

fn volleyball_hit_is_on_far_side(hit: i32) -> bool {
    hit.rem_euclid(2) == 1
}

fn volleyball_hit_point(hit: i32, seed: u32) -> Point {
    let metres_from_net = 2.0 + hash_fraction(hit, 2, seed ^ 0xB011) * 4.2;
    let east = VOLLEYBALL_NET_CENTRE.0 + (hash_fraction(hit, 1, seed ^ 0xB011) - 0.5) * 6.0;
    if volleyball_hit_is_on_far_side(hit) {
        (east, VOLLEYBALL_NET_CENTRE.1 + metres_from_net)
    } else {
        (east, VOLLEYBALL_NET_CENTRE.1 - metres_from_net)
    }
}

fn volleyball_hitter(hit: i32, seed: u32) -> usize {
    let point = volleyball_hit_point(hit, seed);
    let is_deep = (point.1 - VOLLEYBALL_NET_CENTRE.1).abs() > 4.6;
    let position_on_side = if is_deep {
        2
    } else if point.0 < VOLLEYBALL_NET_CENTRE.0 {
        0
    } else {
        1
    };
    if volleyball_hit_is_on_far_side(hit) {
        position_on_side + 3
    } else {
        position_on_side
    }
}

fn volleyball_player(player: usize, seconds_into_play: f32, seed: u32) -> PlayerNow {
    let (hit, fraction_to_next_hit) = volleyball_hit_now(seconds_into_play);
    let home = VOLLEYBALL_HOME_SPOTS[player];
    if volleyball_hitter(hit, seed) == player {
        let has_just_hit = fraction_to_next_hit < 0.2;
        let position = between(volleyball_hit_point(hit, seed), home, smooth_step(0.1, 1.0, fraction_to_next_hit));
        if has_just_hit {
            return PlayerNow { position, metres_up: VOLLEYBALL_JUMP_METRES, pose: Pose::ArmsRaised };
        }
        return standing_at(position);
    }
    if volleyball_hitter(hit + 1, seed) == player {
        let is_jumping = fraction_to_next_hit > 0.86;
        let position = between(home, volleyball_hit_point(hit + 1, seed), smooth_step(0.0, 0.8, fraction_to_next_hit));
        if is_jumping {
            return PlayerNow { position, metres_up: VOLLEYBALL_JUMP_METRES, pose: Pose::ArmsRaised };
        }
        return standing_at(position);
    }
    let shuffle = (seconds_into_play * 1.3 + player as f32 * 2.1).sin() * 0.5;
    standing_at((home.0 + shuffle, home.1))
}

const SECONDS_BETWEEN_FRISBEE_THROWS: f32 = 2.7;
const FRISBEE_FLIGHT_SECONDS: f32 = 1.5;
const FRISBEE_PLAYER_SPOTS: [Point; 4] = [(11.4, -16.2), (19.8, -17.6), (19.4, -29.4), (11.2, -28.2)];

fn frisbee_throw_now(seconds_into_play: f32) -> (i32, f32) {
    let throw = (seconds_into_play / SECONDS_BETWEEN_FRISBEE_THROWS).floor() as i32;
    let fraction_of_flight =
        (seconds_into_play - throw as f32 * SECONDS_BETWEEN_FRISBEE_THROWS) / FRISBEE_FLIGHT_SECONDS;
    (throw, fraction_of_flight)
}

fn frisbee_catcher(throw: i32) -> usize {
    (throw * 3).rem_euclid(4) as usize
}

fn frisbee_catch_point(throw: i32, seed: u32) -> Point {
    let spot = FRISBEE_PLAYER_SPOTS[frisbee_catcher(throw)];
    (
        spot.0 + (hash_fraction(throw, 1, seed ^ 0xF215) - 0.5) * 3.6,
        spot.1 + (hash_fraction(throw, 2, seed ^ 0xF215) - 0.5) * 3.0,
    )
}

fn frisbee_player(player: usize, seconds_into_play: f32, seed: u32, seconds: f32) -> PlayerNow {
    let (throw, fraction_of_flight) = frisbee_throw_now(seconds_into_play);
    let spot = FRISBEE_PLAYER_SPOTS[player];
    if frisbee_catcher(throw) == player {
        let catch_point = frisbee_catch_point(throw, seed);
        if fraction_of_flight >= 1.0 {
            return standing_at(catch_point);
        }
        let position = between(spot, catch_point, smooth_step(0.1, 0.95, fraction_of_flight));
        let is_leaping = fraction_of_flight > 0.85;
        if is_leaping {
            return PlayerNow { position, metres_up: 0.4, pose: Pose::ArmsRaised };
        }
        return PlayerNow { position, metres_up: 0.0, pose: Pose::Running { stride: stride_frame(seconds * 2.5) } };
    }
    if frisbee_catcher(throw - 1) == player {
        let threw_from = frisbee_catch_point(throw - 1, seed);
        let is_following_through = fraction_of_flight < 0.15;
        let position = between(threw_from, spot, smooth_step(0.2, 1.6, fraction_of_flight));
        return PlayerNow {
            position,
            metres_up: 0.0,
            pose: if is_following_through { Pose::ArmsRaised } else { Pose::Standing },
        };
    }
    standing_at(spot)
}

const RED_BLANKET_CENTRE: Point = (-0.7, -19.4);
const BLUE_BLANKET_CENTRE: Point = (0.9, -27.2);
const PICNIC_SEATS: [(Point, Pose); 5] = [
    ((-1.7, -19.2), Pose::Sitting),
    ((0.2, -18.8), Pose::Sitting),
    ((-0.7, -20.3), Pose::LyingDown),
    ((0.2, -27.0), Pose::Sitting),
    ((1.8, -27.4), Pose::Sitting),
];
const PICNICKER_WHO_VISITS_THE_OTHER_BLANKET: usize = 1;
pub(crate) const VISIT_EVERY_SECONDS: f32 = 80.0;
const VISIT_SETS_OUT_AT: f32 = 50.0;
const VISIT_WALK_SECONDS: f32 = 8.0;
const VISIT_CHAT_SECONDS: f32 = 8.0;
const VISITING_SPOT: Point = (1.0, -25.6);

fn picnicker(player: usize, seconds_into_play: f32) -> PlayerNow {
    let (seat, pose) = PICNIC_SEATS[player];
    let seated = PlayerNow { position: seat, metres_up: 0.0, pose };
    if player != PICNICKER_WHO_VISITS_THE_OTHER_BLANKET {
        return seated;
    }
    let seconds_into_visit = seconds_into_play.rem_euclid(VISIT_EVERY_SECONDS) - VISIT_SETS_OUT_AT;
    let visit_seconds = 2.0 * VISIT_WALK_SECONDS + VISIT_CHAT_SECONDS;
    if seconds_into_visit < 0.0 || seconds_into_visit > visit_seconds {
        return seated;
    }
    if seconds_into_visit < VISIT_WALK_SECONDS {
        let pose = Pose::Walking { stride: stride_frame(seconds_into_visit) };
        return PlayerNow {
            position: between(seat, VISITING_SPOT, seconds_into_visit / VISIT_WALK_SECONDS),
            metres_up: 0.0,
            pose,
        };
    }
    if seconds_into_visit < VISIT_WALK_SECONDS + VISIT_CHAT_SECONDS {
        return standing_at(VISITING_SPOT);
    }
    let seconds_walking_back = seconds_into_visit - VISIT_WALK_SECONDS - VISIT_CHAT_SECONDS;
    let pose = Pose::Walking { stride: stride_frame(seconds_walking_back) };
    PlayerNow {
        position: between(VISITING_SPOT, seat, seconds_walking_back / VISIT_WALK_SECONDS),
        metres_up: 0.0,
        pose,
    }
}

const SECONDS_BETWEEN_SNOWBALLS: f32 = 0.85;
const SNOWBALL_FLIGHT_SECONDS: f32 = 0.75;
const SOUTH_SNOW_FORT: Point = (-15.5, -27.4);
const NORTH_SNOW_FORT: Point = (-15.5, -18.2);
const SNOWBALL_FIGHTER_SPOTS: [Point; 6] =
    [(-18.2, -28.6), (-15.5, -28.9), (-12.8, -28.6), (-18.2, -17.0), (-15.5, -16.7), (-12.8, -17.0)];
const FIGHTERS_PER_SIDE: usize = 3;

fn snowball_fighter_position(fighter: usize, seconds: f32) -> Point {
    let spot = SNOWBALL_FIGHTER_SPOTS[fighter];
    let dodge = (crate::chapel::anim::angle_after_whole_cycles(seconds, 3100.0) + fighter as f32 * 1.7).sin() * 0.9;
    (spot.0 + dodge, spot.1)
}

fn snowball_thrower_and_target(throw: i32, seed: u32) -> (usize, usize) {
    let south_side_throws = throw.rem_euclid(2) == 0;
    let thrower_on_side = throw.div_euclid(2).rem_euclid(FIGHTERS_PER_SIDE as i32) as usize;
    let target_on_side = (hash_fraction(throw, 1, seed ^ 0x5B) * 2.999) as usize;
    if south_side_throws {
        (thrower_on_side, FIGHTERS_PER_SIDE + target_on_side)
    } else {
        (FIGHTERS_PER_SIDE + thrower_on_side, target_on_side)
    }
}

fn snowball_fighter(fighter: usize, seconds_into_play: f32, seed: u32, seconds: f32) -> PlayerNow {
    let position = snowball_fighter_position(fighter, seconds);
    let latest_throw = (seconds_into_play / SECONDS_BETWEEN_SNOWBALLS).floor() as i32;
    let seconds_since_latest_throw = seconds_into_play - latest_throw as f32 * SECONDS_BETWEEN_SNOWBALLS;
    let is_throwing = snowball_thrower_and_target(latest_throw, seed).0 == fighter && seconds_since_latest_throw < 0.25;
    if is_throwing {
        return PlayerNow { position, metres_up: 0.0, pose: Pose::ArmsRaised };
    }
    for throw in [latest_throw - 1, latest_throw] {
        let seconds_since_throw = seconds_into_play - throw as f32 * SECONDS_BETWEEN_SNOWBALLS;
        let is_the_target = throw >= 0 && snowball_thrower_and_target(throw, seed).1 == fighter;
        if is_the_target && (0.55..1.2).contains(&seconds_since_throw) {
            return PlayerNow { position, metres_up: 0.0, pose: Pose::Sitting };
        }
    }
    standing_at(position)
}

const SNOWMAN_ROLLING_SECONDS: f32 = 70.0;
const SECONDS_TO_STACK_EACH_BALL: f32 = 15.0;
const SNOWMAN_MELTS_OVER_SECONDS: f32 = 90.0;
const SNOWMAN_STANDS_FOR_SECONDS: f32 = 2.0 * SLOT_SECONDS - SLOT_ENDS_EARLY_BY_SECONDS;
const BALLS_IN_A_SNOWMAN: f32 = 3.0;

fn snowman_spot(slot: i32) -> Point {
    if slot.rem_euclid(2) == 0 {
        (-1.4, -22.0)
    } else {
        (1.4, -26.0)
    }
}

fn snowman_rolling_position(builder: usize, seconds_into_play: f32, slot: i32) -> Point {
    let spot = snowman_spot(slot);
    let angle = seconds_into_play * 0.22 + builder as f32 * 0.35;
    let radius = 1.2 + 1.8 * (seconds_into_play / SNOWMAN_ROLLING_SECONDS) + builder as f32 * 0.5;
    (spot.0 + angle.cos() * radius, spot.1 + angle.sin() * radius * 1.4)
}

fn snowman_builder(builder: usize, seconds_into_play: f32, slot: i32) -> PlayerNow {
    if seconds_into_play < SNOWMAN_ROLLING_SECONDS {
        let pose = Pose::Walking { stride: stride_frame(seconds_into_play + builder as f32) };
        return PlayerNow {
            position: snowman_rolling_position(builder, seconds_into_play, slot),
            metres_up: 0.0,
            pose,
        };
    }
    let spot = snowman_spot(slot);
    let places_round_the_snowman =
        [(spot.0 - 1.5, spot.1 - 0.4), (spot.0 + 1.6, spot.1 - 0.2), (spot.0 + 0.4, spot.1 - 1.8)];
    let seconds_since_rolling = seconds_into_play - SNOWMAN_ROLLING_SECONDS;
    let fraction = seconds_since_rolling / 6.0;
    let position = between(
        snowman_rolling_position(builder, SNOWMAN_ROLLING_SECONDS, slot),
        places_round_the_snowman[builder],
        fraction,
    );
    if fraction < 1.0 {
        return PlayerNow {
            position,
            metres_up: 0.0,
            pose: Pose::Walking { stride: stride_frame(seconds_since_rolling) },
        };
    }
    let is_still_building = seconds_since_rolling < BALLS_IN_A_SNOWMAN * SECONDS_TO_STACK_EACH_BALL;
    let takes_a_turn_now = (seconds_into_play * 0.8) as usize % 3 == builder;
    if is_still_building && takes_a_turn_now {
        return PlayerNow { position, metres_up: 0.0, pose: Pose::ArmsRaised };
    }
    standing_at(position)
}

fn add_snowman_left_standing(
    slot_it_was_built_in: i32,
    seconds_since_the_slot_began: f32,
    builders_set_out_after: f32,
    opacity: f32,
    sprites: &mut Vec<Sprite>,
) {
    let seconds_since_rolling =
        seconds_since_the_slot_began - builders_set_out_after - WALK_ON_SECONDS - SNOWMAN_ROLLING_SECONDS;
    let seconds_until_gone = SNOWMAN_STANDS_FOR_SECONDS - seconds_since_the_slot_began;
    if seconds_since_rolling < 0.0 || seconds_until_gone <= 0.0 || opacity <= 0.004 {
        return;
    }
    let balls_built = 1.0 + (seconds_since_rolling / SECONDS_TO_STACK_EACH_BALL).floor();
    let balls_not_yet_melted = (seconds_until_gone / (SNOWMAN_MELTS_OVER_SECONDS / BALLS_IN_A_SNOWMAN)).ceil();
    let balls_stacked = balls_built.min(balls_not_yet_melted).min(BALLS_IN_A_SNOWMAN) as u8;
    let snowman = Sprite::on_the_ground(snowman_spot(slot_it_was_built_in), SpriteKind::Snowman { balls_stacked });
    sprites.push(snowman.faded(opacity));
}
