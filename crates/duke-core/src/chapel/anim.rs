use crate::chapel::art::{Layer, FLOODLIT_TOWER_STONE, NOTHING_DRAWN};
use crate::chapel::palette::{smooth_step, Look};
use crate::chapel::proj::*;
use crate::chapel::props::{BENCHES, LAMP_POSTS};
use crate::hash_fraction;
use crate::{blend_colours, hash_position, Canvas, Colour, Environment, Season, Weather, CANVAS_HEIGHT, CANVAS_WIDTH};

pub const CLOCK_WRAP_SECONDS: f32 = 21_600.0;
pub const WALKING_METRES_PER_SECOND: f32 = 1.3;
pub const STRIDES_PER_SECOND: f32 = 2.4;

pub fn seconds_on_wrapped_clock(environment: &Environment) -> f32 {
    environment.animation_seconds.rem_euclid(CLOCK_WRAP_SECONDS as f64) as f32
}

pub fn angle_after_whole_cycles(seconds: f32, cycles_per_clock_wrap: f32) -> f32 {
    seconds / CLOCK_WRAP_SECONDS * cycles_per_clock_wrap * std::f32::consts::TAU
}

pub fn presence(suitability: f32, threshold: f32) -> f32 {
    const FADE_OVER_SUITABILITY: f32 = 0.05;
    ((suitability - threshold * (1.0 - FADE_OVER_SUITABILITY)) / FADE_OVER_SUITABILITY).clamp(0.0, 1.0)
}

pub fn stride_frame(seconds_walked: f32) -> bool {
    (seconds_walked * STRIDES_PER_SECOND).floor() as i64 % 2 == 0
}

pub fn busyness_at_hour(hour: f32) -> f32 {
    const NIGHT: f32 = 0.07;
    const DAY: f32 = 0.67;
    const LATE_EVENING: f32 = 0.17;
    let hour = hour.rem_euclid(24.0);
    if hour < 5.5 {
        return NIGHT;
    }
    if hour < 8.5 {
        return NIGHT + (hour - 5.5) / 3.0 * (DAY - NIGHT);
    }
    if hour < 18.0 {
        return DAY;
    }
    if hour < 23.0 {
        return DAY - (hour - 18.0) / 5.0 * (DAY - LATE_EVENING);
    }
    LATE_EVENING - (hour - 23.0) * (LATE_EVENING - NIGHT)
}

#[derive(Clone, Copy)]
pub struct Conditions {
    pub busyness: f32,
    pub fair_weather: f32,
    pub rain: f32,
    pub snowfall: f32,
    pub snow_cover: f32,
    pub coldness: f32,
    pub daylight: f32,
    pub night: f32,
    pub shadow_strength: f32,
}

impl Conditions {
    pub fn of(environment: &Environment) -> Conditions {
        let intensity = environment.weather_intensity.clamp(0.0, 1.0);
        let fair_weather = match environment.weather {
            Weather::Clear => 1.0,
            Weather::Cloudy => 1.0 - 0.1 * intensity,
            Weather::Fog => 1.0 - 0.35 * intensity,
            Weather::Rain => 1.0 - 0.6 * intensity,
            Weather::Storm => 1.0 - 0.88 * intensity,
            Weather::Snow => 1.0 - 0.45 * intensity,
        };
        let rain = if matches!(environment.weather, Weather::Rain | Weather::Storm) { intensity } else { 0.0 };
        let snowfall = if environment.weather == Weather::Snow { intensity } else { 0.0 };
        let coldness_of_the_season = match environment.season {
            Season::Winter => 1.0,
            Season::Autumn => smooth_step(0.7, 0.9, environment.season_progress),
            Season::Spring => 1.0 - smooth_step(0.0, 0.15, environment.season_progress),
            Season::Summer => 0.0,
        };
        let coldness_of_the_snow = smooth_step(0.1, 0.25, environment.snow_cover);
        Conditions {
            busyness: (busyness_at_hour(environment.hour) * fair_weather).clamp(0.04, 1.0),
            fair_weather,
            rain,
            snowfall,
            snow_cover: environment.snow_cover,
            coldness: coldness_of_the_season.max(coldness_of_the_snow),
            daylight: environment.daylight(),
            night: environment.darkness(),
            shadow_strength: environment.shadow_strength(),
        }
    }

    pub fn dryness(&self) -> f32 {
        1.0 - smooth_step(0.05, 0.2, self.rain)
    }

    pub fn has_umbrella(&self, person: u32) -> bool {
        let reluctance = hash_fraction(person as i32, 21, 0x6A2B);
        self.rain > 0.1 + 0.25 * reluctance || self.snowfall > 0.55 + 0.3 * reluctance
    }
}

const SHIRT_COLOURS: [u32; 13] = [
    0x2b4c9b, 0xd9d9de, 0xb5352f, 0x2f7d4f, 0xe0b23a, 0x6b4a8a, 0x1f2430, 0xd87a2e, 0x4aa3c8, 0x8a8f96, 0xe58aa8,
    0x173a8c, 0x173a8c,
];
const COAT_COLOURS: [u32; 8] = [0x2a2f3a, 0x3d4a5a, 0x5a3a34, 0x2f4a3a, 0x173a8c, 0x6a6258, 0x8a2f2f, 0x30303a];
const TROUSER_COLOURS: [u32; 5] = [0x2a3348, 0x1d1f26, 0x5a5f6a, 0x7a6a50, 0x34425a];
const HAIR_COLOURS: [u32; 6] = [0x2a2018, 0x4a3524, 0x15130f, 0x8a6a3a, 0xb9b3aa, 0x5a2e1c];
const HAT_COLOURS: [u32; 5] = [0x173a8c, 0xb5352f, 0xd9d9de, 0x2f7d4f, 0x1f2430];
const SKIN_COLOURS: [u32; 4] = [0xe6bf9c, 0xc99872, 0x8d5f3f, 0xf0d0b0];
const BACKPACK_COLOURS: [u32; 6] = [0x20242c, 0x8a2f2f, 0x2f4f8f, 0x4a5a3a, 0x6a4a2a, 0x555a66];
const UMBRELLA_COLOURS: [u32; 7] = [0x1b1d24, 0x173a8c, 0xb8322c, 0xe0c23a, 0x2f7d4f, 0xd9d9de, 0x7a3f8f];
pub const DUKE_BLUE: Colour = 0x173a8c;
const CAST_SHADOW_COLOUR: Colour = 0x10141a;

#[derive(Clone, Copy)]
pub struct Outfit {
    pub top: Colour,
    pub trousers: Colour,
    pub hair_or_hat: Colour,
    pub skin: Colour,
    pub backpack: Option<Colour>,
    pub umbrella: Colour,
}

pub fn outfit_of(person: u32, coldness: f32) -> Outfit {
    let choose = |colours: &[u32], trait_number: i32| {
        colours[hash_position(person as i32, trait_number, 0x6A2B) as usize % colours.len()]
    };
    let feels_the_cold_at = 0.25 + 0.5 * hash_fraction(person as i32, 15, 0x6A2B);
    let wears_a_coat = coldness > feels_the_cold_at;
    let wears_a_hat = wears_a_coat && !hash_position(person as i32, 12, 0x6A2B).is_multiple_of(3);
    let carries_a_backpack = hash_position(person as i32, 13, 0x6A2B) % 5 < 3;
    Outfit {
        top: if wears_a_coat { choose(&COAT_COLOURS, 5) } else { choose(&SHIRT_COLOURS, 5) },
        trousers: choose(&TROUSER_COLOURS, 6),
        hair_or_hat: if wears_a_hat { choose(&HAT_COLOURS, 11) } else { choose(&HAIR_COLOURS, 7) },
        skin: choose(&SKIN_COLOURS, 8),
        backpack: if carries_a_backpack { Some(choose(&BACKPACK_COLOURS, 14)) } else { None },
        umbrella: choose(&UMBRELLA_COLOURS, 9),
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum Pose {
    Walking { stride: bool },
    Running { stride: bool },
    Standing,
    Sitting,
    LyingDown,
    ArmsRaised,
    Cycling { stride: bool },
}

#[derive(Clone, Copy)]
pub enum SpriteKind {
    Person { outfit: Outfit, pose: Pose, has_umbrella: bool },
    Ball { colour: Colour, is_large: bool },
    VolleyballNet { metres_wide: f32, raised: f32 },
    PicnicBlanket { colour: Colour, unrolled: f32 },
    Snowman { balls_stacked: u8 },
    SnowFort,
    Car { heading_east: f32, heading_north: f32, body_colour: Colour },
}

#[derive(Clone, Copy)]
pub struct Sprite {
    pub metres_east: f32,
    pub metres_north: f32,
    pub metres_up: f32,
    pub opacity: f32,
    pub kind: SpriteKind,
}

impl Sprite {
    pub fn person(position: (f32, f32), person: u32, pose: Pose, conditions: &Conditions) -> Sprite {
        Sprite {
            metres_east: position.0,
            metres_north: position.1,
            metres_up: 0.0,
            opacity: 1.0,
            kind: SpriteKind::Person {
                outfit: outfit_of(person, conditions.coldness),
                pose,
                has_umbrella: conditions.has_umbrella(person),
            },
        }
    }

    pub fn on_the_ground(position: (f32, f32), kind: SpriteKind) -> Sprite {
        Sprite { metres_east: position.0, metres_north: position.1, metres_up: 0.0, opacity: 1.0, kind }
    }

    pub fn faded(mut self, opacity: f32) -> Sprite {
        self.opacity *= opacity.clamp(0.0, 1.0);
        self
    }

    fn painting_order(&self) -> f32 {
        const BLANKETS_GO_UNDER_EVERYTHING: f32 = 50.0;
        match self.kind {
            SpriteKind::PicnicBlanket { .. } => self.metres_north + BLANKETS_GO_UNDER_EVERYTHING,
            _ => self.metres_north,
        }
    }
}

pub fn distance_between(from: (f32, f32), to: (f32, f32)) -> f32 {
    ((to.0 - from.0).powi(2) + (to.1 - from.1).powi(2)).sqrt()
}

pub fn path_metres(corners: &[(f32, f32)]) -> f32 {
    let mut metres = 0.0;
    for leg in corners.windows(2) {
        metres += distance_between(leg[0], leg[1]);
    }
    metres
}

pub fn position_and_heading_along_path(corners: &[(f32, f32)], metres_along: f32) -> ((f32, f32), (f32, f32)) {
    let mut metres_left = metres_along.max(0.0);
    let last_leg = corners.len() - 2;
    for (leg, ends) in corners.windows(2).enumerate() {
        let (from, to) = (ends[0], ends[1]);
        let leg_metres = distance_between(from, to).max(0.001);
        if metres_left <= leg_metres || leg == last_leg {
            let fraction = (metres_left / leg_metres).min(1.0);
            let position = (from.0 + (to.0 - from.0) * fraction, from.1 + (to.1 - from.1) * fraction);
            let heading = ((to.0 - from.0) / leg_metres, (to.1 - from.1) / leg_metres);
            return (position, heading);
        }
        metres_left -= leg_metres;
    }
    (corners[0], (0.0, 0.0))
}

pub fn position_along_path(corners: &[(f32, f32)], metres_along: f32) -> (f32, f32) {
    position_and_heading_along_path(corners, metres_along).0
}

pub const WALKING_ROUTES: &[&[(f32, f32)]] = &[
    &[(-24.1, -43.0), (-24.1, -10.3), (24.5, -10.9), (24.5, -43.0)],
    &[(-5.4, -43.0), (-5.7, -10.1), (-3.0, -5.6), (-1.6, 1.4)],
    &[(6.7, -43.0), (6.6, -10.4), (3.0, -5.6), (1.6, 1.4)],
    &[(-24.1, -43.0), (-24.1, -10.3), (-5.7, -10.1), (-3.0, -5.6), (-1.6, 1.4)],
    &[
        (-24.1, -43.0),
        (-24.1, -10.3),
        (-24.3, 5.2),
        (-24.8, 12.8),
        (-25.2, 30.6),
        (-28.5, 40.4),
        (-35.4, 40.9),
        (-35.7, 63.5),
        (-29.1, 79.7),
        (-29.0, 84.2),
        (-29.6, 98.0),
        (-32.7, 110.0),
        (-36.1, 123.8),
        (-34.0, 136.3),
    ],
    &[
        (24.5, -43.0),
        (24.5, -10.9),
        (24.4, 11.2),
        (31.8, 17.9),
        (32.0, 47.1),
        (32.4, 60.4),
        (32.8, 73.6),
        (29.7, 78.5),
        (29.4, 83.0),
        (30.5, 105.6),
        (36.3, 125.0),
        (41.4, 138.0),
    ],
    &[
        (24.5, -43.0),
        (24.5, -10.9),
        (24.4, 11.2),
        (16.1, 12.0),
        (16.6, 46.5),
        (19.6, 49.6),
        (23.1, 52.5),
        (27.5, 52.3),
        (30.0, 50.1),
        (32.0, 47.1),
        (32.4, 60.4),
        (46.0, 60.2),
    ],
    &[
        (-108.0, -43.0),
        (-108.3, -10.1),
        (-107.9, 24.1),
        (-78.5, 24.8),
        (-70.0, 31.4),
        (-64.6, 37.3),
        (-64.8, 47.8),
        (-66.5, 65.3),
        (-66.6, 73.6),
        (-66.1, 76.5),
        (-78.4, 79.3),
        (-104.6, 79.3),
    ],
    &[
        (-83.4, -43.0),
        (-83.2, -34.1),
        (-83.4, 2.3),
        (-46.9, 2.3),
        (-49.3, 4.4),
        (-58.8, 19.2),
        (-63.0, 27.9),
        (-64.6, 37.3),
        (-55.4, 51.9),
        (-51.0, 58.4),
        (-47.3, 58.4),
        (-43.5, 64.7),
        (-40.3, 68.2),
        (-29.1, 79.7),
        (-17.2, 89.5),
        (5.0, 93.0),
        (19.9, 85.5),
        (25.7, 80.2),
        (32.8, 73.6),
        (67.1, 74.7),
        (74.7, 74.0),
        (82.7, 70.8),
        (85.8, 68.9),
        (86.3, 65.4),
        (85.6, 56.1),
        (87.6, 48.1),
        (93.7, 41.9),
        (101.7, 39.3),
        (107.9, 36.8),
        (113.8, 27.8),
        (123.1, 16.4),
    ],
    &[
        (-35.7, 63.5),
        (-35.4, 40.9),
        (-49.1, 24.6),
        (-49.3, 4.4),
        (-46.9, 2.3),
        (-83.4, 2.3),
        (-83.2, -34.1),
        (-83.4, -43.0),
    ],
    &[
        (-34.0, 136.3),
        (-36.1, 123.8),
        (-39.4, 111.4),
        (-45.0, 87.1),
        (-46.6, 83.5),
        (-52.0, 78.7),
        (-59.2, 76.5),
        (-66.1, 76.5),
        (-66.6, 73.6),
        (-61.5, 72.1),
        (-57.6, 69.5),
        (-51.7, 63.1),
        (-47.3, 58.4),
        (-43.5, 53.2),
        (-40.0, 48.9),
        (-35.4, 40.9),
    ],
    &[
        (-39.4, 111.4),
        (-28.7, 109.5),
        (-11.1, 107.2),
        (0.9, 100.4),
        (12.8, 101.7),
        (30.5, 105.6),
        (53.5, 107.9),
        (77.7, 107.4),
        (100.5, 107.4),
        (115.9, 108.7),
        (132.0, 110.5),
    ],
];
const ROUTE_POPULARITY: [u32; 12] = [9, 6, 6, 5, 6, 6, 3, 6, 5, 3, 3, 2];
const ROUTES_ACROSS_THE_COURT: usize = 4;
const ROUTES_WITH_JOGGERS: [usize; 3] = [4, 5, 8];
const ROUTES_WITH_CYCLISTS: [usize; 2] = [0, 7];
const EVERYDAY_WALKERS: i32 = 90;
const EXTRA_WALKERS_IN_A_RUSH: i32 = 60;
pub(crate) const PAUSE_CYCLES_THAT_DIVIDE_THE_CLOCK: [f32; 8] = [36.0, 40.0, 45.0, 48.0, 50.0, 54.0, 60.0, 72.0];
const CHAT_PAUSE_SECONDS: f32 = 7.0;

#[derive(Clone, Copy, PartialEq)]
enum Gait {
    Walking,
    WalkingAndStoppingToChat,
    Jogging,
    Cycling,
}

#[derive(Clone, Copy)]
struct Walker {
    route: usize,
    gait: Gait,
    round_trips_per_clock_wrap: f32,
    starting_point_in_round_trip: f32,
    pause_cycle_seconds: f32,
    pause_seconds: f32,
    metres_right_of_centre_line: f32,
    only_in_a_rush: bool,
    appears_above_level: f32,
    person: u32,
}

pub const STEP_SEATS: [(f32, f32); 5] = [(-4.6, -3.6), (-3.4, -3.6), (4.2, -3.6), (5.6, -4.0), (-6.2, -4.2)];
pub const SUNBATHING_SPOTS: [(f32, f32); 6] =
    [(-70.0, 12.0), (-62.0, 8.0), (-74.0, 20.0), (-18.0, 98.0), (24.0, 22.0), (25.5, 38.0)];
pub const PUDDLE_COLOUR: Colour = 0x5d6e84;

pub struct World {
    walkers: Vec<Walker>,
    puddle_pixels: Vec<(i16, i16)>,
    pub seed: u32,
    pub projection: Projection,
}

impl World {
    pub fn new(look: &Look, ground_pixels: &[Colour], objects: &Layer, projection: Projection) -> World {
        let seed = look.seed;
        let mut walkers: Vec<Walker> = Vec::new();
        let popularity_of_all_routes: u32 = ROUTE_POPULARITY.iter().sum();
        let popularity_of_court_routes: u32 = ROUTE_POPULARITY[..ROUTES_ACROSS_THE_COURT].iter().sum();
        for number in 0..EVERYDAY_WALKERS + EXTRA_WALKERS_IN_A_RUSH {
            let only_in_a_rush = number >= EVERYDAY_WALKERS;
            let person = 1000 + number as u32;

            let mut ticket = hash_position(number, 1, seed)
                % if only_in_a_rush { popularity_of_court_routes } else { popularity_of_all_routes };
            let mut route = 0;
            while ticket >= ROUTE_POPULARITY[route] {
                ticket -= ROUTE_POPULARITY[route];
                route += 1;
            }

            let roll = hash_position(number, 15, seed) % 100;
            let gait = if only_in_a_rush {
                Gait::Walking
            } else if roll < 6 && ROUTES_WITH_JOGGERS.contains(&route) {
                Gait::Jogging
            } else if roll < 11 && ROUTES_WITH_CYCLISTS.contains(&route) {
                Gait::Cycling
            } else if roll < 45 {
                Gait::WalkingAndStoppingToChat
            } else {
                Gait::Walking
            };

            let walks_beside_the_previous_walker = number > 0
                && hash_position(number, 16, seed).is_multiple_of(3)
                && matches!(gait, Gait::Walking | Gait::WalkingAndStoppingToChat);
            if walks_beside_the_previous_walker {
                let companion = walkers.last().unwrap();
                if companion.only_in_a_rush == only_in_a_rush
                    && matches!(companion.gait, Gait::Walking | Gait::WalkingAndStoppingToChat)
                {
                    let beside = Walker {
                        metres_right_of_centre_line: companion.metres_right_of_centre_line + 0.95,
                        person,
                        ..*companion
                    };
                    walkers.push(beside);
                    continue;
                }
            }

            let metres_per_second = match gait {
                Gait::Jogging => 3.0 + hash_fraction(number, 2, seed) * 0.6,
                Gait::Cycling => 4.6 + hash_fraction(number, 2, seed) * 1.2,
                _ => 1.05 + hash_fraction(number, 2, seed) * 0.55,
            };
            let pause_cycle_seconds = PAUSE_CYCLES_THAT_DIVIDE_THE_CLOCK
                [hash_position(number, 4, seed) as usize % PAUSE_CYCLES_THAT_DIVIDE_THE_CLOCK.len()];
            let pause_seconds = if gait == Gait::WalkingAndStoppingToChat { CHAT_PAUSE_SECONDS } else { 0.0 };
            let moving_seconds_per_clock_wrap =
                CLOCK_WRAP_SECONDS * (pause_cycle_seconds - pause_seconds) / pause_cycle_seconds;
            let round_trip_metres = 2.0 * path_metres(WALKING_ROUTES[route]);
            walkers.push(Walker {
                route,
                gait,
                round_trips_per_clock_wrap: (moving_seconds_per_clock_wrap * metres_per_second / round_trip_metres)
                    .round()
                    .max(1.0),
                starting_point_in_round_trip: hash_fraction(number, 3, seed),
                pause_cycle_seconds,
                pause_seconds,
                metres_right_of_centre_line: 0.5 + hash_fraction(number, 5, seed) * 0.6,
                only_in_a_rush,
                appears_above_level: (hash_position(number, 10, seed) % 1000) as f32 / 1000.0,
                person,
            });
        }

        let mut puddle_pixels = Vec::new();
        for pixel_y in 0..CANVAS_HEIGHT {
            for pixel_x in 0..CANVAS_WIDTH {
                let index = (pixel_y * CANVAS_WIDTH + pixel_x) as usize;
                let is_open_puddle = ground_pixels[index] == PUDDLE_COLOUR && objects.depth[index] == NOTHING_DRAWN;
                if is_open_puddle && hash_position(pixel_x, pixel_y, 0x9D).is_multiple_of(3) {
                    puddle_pixels.push((pixel_x as i16, pixel_y as i16));
                }
            }
        }
        World { walkers, puddle_pixels, seed, projection }
    }
}

pub struct Painter<'a> {
    pub canvas: &'a mut Canvas,
    pub objects: &'a Layer,
    pub depth_tag: i16,
    pub opacity: f32,
    pub projection: Projection,
}

impl<'a> Painter<'a> {
    fn is_hidden(&self, pixel_x: i32, pixel_y: i32) -> bool {
        if pixel_x < 0 || pixel_y < 0 || pixel_x >= CANVAS_WIDTH || pixel_y >= CANVAS_HEIGHT {
            return true;
        }
        let depth_of_what_is_there = self.objects.depth[(pixel_y * CANVAS_WIDTH + pixel_x) as usize];
        depth_of_what_is_there != NOTHING_DRAWN && depth_of_what_is_there < self.depth_tag
    }

    pub fn plot(&mut self, pixel_x: i32, pixel_y: i32, colour: Colour) {
        self.blend(pixel_x, pixel_y, colour, 1.0);
    }

    pub fn blend(&mut self, pixel_x: i32, pixel_y: i32, colour: Colour, strength: f32) {
        if self.is_hidden(pixel_x, pixel_y) {
            return;
        }
        let index = (pixel_y * CANVAS_WIDTH + pixel_x) as usize;
        let strength = strength * self.opacity;
        if strength >= 0.999 {
            self.canvas.pixels[index] = colour;
        } else {
            self.canvas.pixels[index] = blend_colours(self.canvas.pixels[index], colour, strength);
        }
    }
}

fn draw_person(
    painter: &mut Painter,
    feet_x: i32,
    feet_y: i32,
    outfit: &Outfit,
    pose: Pose,
    has_umbrella: bool,
    cast_shadow: f32,
) {
    if cast_shadow > 0.02 {
        painter.blend(feet_x + 1, feet_y + 1, CAST_SHADOW_COLOUR, 0.34 * cast_shadow);
        painter.blend(feet_x + 2, feet_y + 1, CAST_SHADOW_COLOUR, 0.26 * cast_shadow);
    }
    match pose {
        Pose::LyingDown => {
            painter.plot(feet_x - 2, feet_y, outfit.hair_or_hat);
            painter.plot(feet_x - 1, feet_y, outfit.skin);
            painter.plot(feet_x, feet_y, outfit.top);
            painter.plot(feet_x + 1, feet_y, outfit.top);
            painter.plot(feet_x + 2, feet_y, outfit.trousers);
            painter.plot(feet_x + 3, feet_y, outfit.trousers);
            return;
        }
        Pose::Sitting => {
            painter.plot(feet_x, feet_y - 3, outfit.hair_or_hat);
            painter.plot(feet_x, feet_y - 2, outfit.skin);
            painter.plot(feet_x - 1, feet_y - 1, outfit.top);
            painter.plot(feet_x, feet_y - 1, outfit.top);
            painter.plot(feet_x + 1, feet_y - 1, outfit.top);
            painter.plot(feet_x, feet_y, outfit.trousers);
            painter.plot(feet_x + 1, feet_y, outfit.trousers);
            painter.plot(feet_x + 2, feet_y, outfit.trousers);
            return;
        }
        Pose::Cycling { stride } => {
            const TYRE_COLOUR: Colour = 0x1c1d21;
            const BICYCLE_FRAME_COLOUR: Colour = 0x8a8f96;
            painter.plot(feet_x - 2, feet_y, TYRE_COLOUR);
            painter.plot(feet_x + 2, feet_y, TYRE_COLOUR);
            painter.plot(feet_x - 1, feet_y - 1, BICYCLE_FRAME_COLOUR);
            painter.plot(feet_x, feet_y - 1, BICYCLE_FRAME_COLOUR);
            painter.plot(feet_x + 1, feet_y - 1, BICYCLE_FRAME_COLOUR);
            let pedalling_leg_x = if stride { feet_x } else { feet_x - 1 };
            painter.plot(pedalling_leg_x, feet_y - 1, outfit.trousers);
            painter.plot(feet_x, feet_y - 2, outfit.top);
            painter.plot(feet_x + 1, feet_y - 2, outfit.top);
            painter.plot(feet_x + 1, feet_y - 3, outfit.skin);
            painter.plot(feet_x + 1, feet_y - 4, outfit.hair_or_hat);
            return;
        }
        _ => {}
    }

    match pose {
        Pose::Walking { stride: true } | Pose::Running { stride: true } => {
            painter.plot(feet_x - 1, feet_y, outfit.trousers);
            painter.plot(feet_x + 1, feet_y, outfit.trousers);
        }
        Pose::Running { stride: false } => {
            painter.plot(feet_x, feet_y, outfit.trousers);
            painter.plot(feet_x + 1, feet_y - 1, outfit.trousers);
        }
        _ => painter.plot(feet_x, feet_y, outfit.trousers),
    }
    painter.plot(feet_x, feet_y - 1, outfit.trousers);

    if pose == Pose::ArmsRaised {
        painter.plot(feet_x, feet_y - 2, outfit.top);
        painter.plot(feet_x - 1, feet_y - 3, outfit.skin);
        painter.plot(feet_x + 1, feet_y - 3, outfit.skin);
        painter.plot(feet_x - 1, feet_y - 4, outfit.skin);
        painter.plot(feet_x + 1, feet_y - 4, outfit.skin);
    } else {
        painter.plot(feet_x - 1, feet_y - 2, outfit.top);
        painter.plot(feet_x, feet_y - 2, outfit.top);
        painter.plot(feet_x + 1, feet_y - 2, outfit.backpack.unwrap_or(outfit.top));
        if pose == (Pose::Running { stride: true }) {
            painter.plot(feet_x - 1, feet_y - 3, outfit.skin);
        }
    }
    painter.plot(feet_x, feet_y - 3, outfit.skin);
    painter.plot(feet_x, feet_y - 4, outfit.hair_or_hat);

    if has_umbrella && pose != Pose::Sitting {
        let canopy_highlight = blend_colours(outfit.umbrella, 0xffffff, 0.25);
        for offset_x in -2..=2 {
            painter.plot(feet_x + offset_x, feet_y - 4, outfit.umbrella);
        }
        for offset_x in -1..=1 {
            painter.plot(feet_x + offset_x, feet_y - 5, canopy_highlight);
        }
    }
}

pub fn car_pixels(
    projection: Projection,
    (centre_x, wheels_y): (i32, i32),
    heading_east: f32,
    heading_north: f32,
    body_colour: Colour,
    night: f32,
    plot: &mut dyn FnMut(i32, i32, Colour),
) {
    const WINDSCREEN_COLOUR: Colour = 0x27303d;
    const TYRE_COLOUR: Colour = 0x1c1d21;
    const TAIL_LIGHT_COLOUR: Colour = 0xb8322c;
    let roof_colour = blend_colours(body_colour, 0xffffff, 0.22);
    let sill_colour = blend_colours(body_colour, 0x000000, 0.35);
    let headlight_colour = if night > 0.3 { 0xfff6d0 } else { 0xf3e9c0 };

    let drives_across_the_view = heading_east.abs() * projection.pixels_per_metre_across(0.0)
        >= heading_north.abs() * projection.pixels_per_metre_of_depth(0.0);
    if drives_across_the_view {
        let forward = if heading_east >= 0.0 { 1 } else { -1 };
        for offset_x in -6..=6i32 {
            let pixel_x = centre_x + offset_x;
            let along_the_car = offset_x * forward;
            let is_bumper = offset_x.abs() == 6;
            plot(pixel_x, wheels_y - 4, if is_bumper { sill_colour } else { body_colour });
            for rows_up in [3, 2] {
                let colour = if is_bumper {
                    body_colour
                } else if along_the_car == 2 || along_the_car == 3 || along_the_car == -4 {
                    WINDSCREEN_COLOUR
                } else if (-3..=1).contains(&along_the_car) {
                    roof_colour
                } else {
                    body_colour
                };
                plot(pixel_x, wheels_y - rows_up, colour);
            }
            plot(pixel_x, wheels_y - 1, body_colour);
            plot(pixel_x, wheels_y, if offset_x.abs() == 4 { TYRE_COLOUR } else { sill_colour });
        }
        for rows_up in [3, 1] {
            plot(centre_x + 6 * forward, wheels_y - rows_up, headlight_colour);
            plot(centre_x - 6 * forward, wheels_y - rows_up, TAIL_LIGHT_COLOUR);
        }
        return;
    }

    let drives_toward_the_viewer = heading_north < 0.0;
    for row in 0..11i32 {
        let pixel_y = wheels_y - 10 + row;
        let from_the_front = if drives_toward_the_viewer { 10 - row } else { row };
        for offset_x in -3..=2i32 {
            let is_side = offset_x == -3 || offset_x == 2;
            let colour = if row == 0 || row == 10 {
                if is_side {
                    sill_colour
                } else {
                    body_colour
                }
            } else if (from_the_front == 3 || from_the_front == 8) && !is_side {
                WINDSCREEN_COLOUR
            } else if (4..=7).contains(&from_the_front) && !is_side {
                roof_colour
            } else {
                body_colour
            };
            plot(centre_x + offset_x, pixel_y, colour);
        }
    }
    let (front_y, back_y) =
        if drives_toward_the_viewer { (wheels_y, wheels_y - 10) } else { (wheels_y - 10, wheels_y) };
    for offset_x in [-3, 2] {
        plot(centre_x + offset_x, front_y, headlight_colour);
        plot(centre_x + offset_x, back_y, TAIL_LIGHT_COLOUR);
    }
}

fn draw_sprite(painter: &mut Painter, sprite: &Sprite, conditions: &Conditions) {
    let projection = painter.projection;
    const SNOW_HIGHLIGHT: Colour = 0xf6f9fd;
    const SNOW_MIDTONE: Colour = 0xdde6f2;
    const SNOW_SHADE: Colour = 0xbac8de;
    let (pixel_x, ground_y) = projection.ground_pixel((sprite.metres_east, sprite.metres_north));
    painter.depth_tag = depth_tag(sprite.metres_north);
    painter.opacity = sprite.opacity;
    let cast_shadow = conditions.shadow_strength;
    match sprite.kind {
        SpriteKind::Person { outfit, pose, has_umbrella } => {
            let pixels_off_the_ground = (sprite.metres_up * 2.0).round() as i32;
            if pixels_off_the_ground > 0 {
                painter.blend(pixel_x, ground_y, CAST_SHADOW_COLOUR, 0.3 * cast_shadow);
                draw_person(painter, pixel_x, ground_y - pixels_off_the_ground, &outfit, pose, has_umbrella, 0.0);
            } else {
                draw_person(painter, pixel_x, ground_y, &outfit, pose, has_umbrella, cast_shadow);
            }
        }
        SpriteKind::Ball { colour, is_large } => {
            let ball_y = projection.pixel_y(sprite.metres_north, sprite.metres_up);
            let shadow_on_the_grass = 0.35 * cast_shadow.max(0.3);
            painter.blend(pixel_x, ground_y, CAST_SHADOW_COLOUR, shadow_on_the_grass);
            painter.plot(pixel_x, ball_y, colour);
            if is_large {
                painter.blend(pixel_x + 1, ground_y, CAST_SHADOW_COLOUR, shadow_on_the_grass);
                painter.plot(pixel_x + 1, ball_y, blend_colours(colour, 0x8090a8, 0.25));
                painter.plot(pixel_x, ball_y - 1, colour);
                painter.plot(pixel_x + 1, ball_y - 1, colour);
            }
        }
        SpriteKind::VolleyballNet { metres_wide, raised } => {
            const POST_COLOUR: Colour = 0x4a4f58;
            const POST_FOOT_COLOUR: Colour = 0x2d3138;
            const TAPE_COLOUR: Colour = 0xf4f4ee;
            const MESH_COLOUR: Colour = 0xe8e8e0;
            let left_post_x = projection.pixel_x(sprite.metres_east - metres_wide / 2.0, sprite.metres_north);
            let right_post_x = projection.pixel_x(sprite.metres_east + metres_wide / 2.0, sprite.metres_north);
            let tape_y = projection.pixel_y(sprite.metres_north, 2.4 * raised);
            for post_x in [left_post_x, right_post_x] {
                painter.plot(post_x, ground_y, POST_FOOT_COLOUR);
                for post_y in tape_y..ground_y {
                    painter.plot(post_x, post_y, POST_COLOUR);
                }
            }
            for net_x in left_post_x + 1..right_post_x {
                painter.plot(net_x, tape_y, TAPE_COLOUR);
                for rows_below_tape in 1..=2 {
                    let is_a_knot = (net_x + rows_below_tape) % 2 == 0;
                    if is_a_knot && tape_y + rows_below_tape < ground_y {
                        painter.blend(net_x, tape_y + rows_below_tape, MESH_COLOUR, 0.7);
                    }
                }
            }
        }
        SpriteKind::PicnicBlanket { colour, unrolled } => {
            painter.depth_tag = depth_tag(sprite.metres_north + 2.0);
            let half_width = (1.5 * projection.pixels_per_metre_across(sprite.metres_north) * unrolled).round() as i32;
            let check_colour = blend_colours(colour, 0xffffff, 0.45);
            let hem_colour = blend_colours(colour, 0x000000, 0.2);
            for blanket_y in ground_y - 2..=ground_y + 1 {
                for blanket_x in pixel_x - half_width..=pixel_x + half_width {
                    let is_a_check = (blanket_x + blanket_y) % 2 == 0;
                    let is_the_hem = blanket_y == ground_y + 1;
                    painter.plot(
                        blanket_x,
                        blanket_y,
                        if is_a_check {
                            check_colour
                        } else if is_the_hem {
                            hem_colour
                        } else {
                            colour
                        },
                    );
                }
            }
        }
        SpriteKind::Snowman { balls_stacked } => {
            const TWIG_COLOUR: Colour = 0x5a4634;
            const HAT_COLOUR: Colour = 0x1d1f26;
            const CARROT_COLOUR: Colour = 0xe8782a;
            const SCARF_COLOUR: Colour = 0xb5352f;
            const SHADOW_ON_SNOW: Colour = 0x394a66;
            if balls_stacked == 0 {
                return;
            }
            painter.blend(pixel_x + 2, ground_y, SHADOW_ON_SNOW, 0.35);
            painter.blend(pixel_x + 3, ground_y, SHADOW_ON_SNOW, 0.25);
            let base_ball = [
                (-1, 0, SNOW_SHADE),
                (0, 0, SNOW_MIDTONE),
                (1, 0, SNOW_SHADE),
                (-1, -1, SNOW_HIGHLIGHT),
                (0, -1, SNOW_HIGHLIGHT),
                (1, -1, SNOW_MIDTONE),
                (0, -2, SNOW_HIGHLIGHT),
            ];
            for (offset_x, offset_y, colour) in base_ball {
                painter.plot(pixel_x + offset_x, ground_y + offset_y, colour);
            }
            if balls_stacked >= 2 {
                let middle_ball = [
                    (-1, -3, SNOW_HIGHLIGHT),
                    (0, -3, SNOW_HIGHLIGHT),
                    (1, -3, SNOW_MIDTONE),
                    (0, -4, SNOW_HIGHLIGHT),
                    (-1, -2, SNOW_HIGHLIGHT),
                    (1, -2, SNOW_SHADE),
                ];
                for (offset_x, offset_y, colour) in middle_ball {
                    painter.plot(pixel_x + offset_x, ground_y + offset_y, colour);
                }
                painter.plot(pixel_x - 2, ground_y - 3, TWIG_COLOUR);
                painter.plot(pixel_x + 2, ground_y - 3, TWIG_COLOUR);
            }
            if balls_stacked >= 3 {
                painter.plot(pixel_x, ground_y - 5, SNOW_HIGHLIGHT);
                painter.plot(pixel_x, ground_y - 6, SNOW_HIGHLIGHT);
                painter.plot(pixel_x + 1, ground_y - 5, CARROT_COLOUR);
                painter.plot(pixel_x - 1, ground_y - 7, HAT_COLOUR);
                painter.plot(pixel_x, ground_y - 7, HAT_COLOUR);
                painter.plot(pixel_x + 1, ground_y - 7, HAT_COLOUR);
                painter.plot(pixel_x, ground_y - 8, HAT_COLOUR);
                painter.plot(pixel_x, ground_y - 4, SCARF_COLOUR);
            }
        }
        SpriteKind::SnowFort => {
            for offset_x in -5..=5i32 {
                painter.plot(pixel_x + offset_x, ground_y, SNOW_SHADE);
                painter.plot(pixel_x + offset_x, ground_y - 1, SNOW_MIDTONE);
                if offset_x.abs() < 5 {
                    painter.plot(pixel_x + offset_x, ground_y - 2, SNOW_HIGHLIGHT);
                }
            }
        }
        SpriteKind::Car { heading_east, heading_north, body_colour } => {
            painter.depth_tag = depth_tag(sprite.metres_north - 1.0);
            let night = conditions.night;
            car_pixels(
                painter.projection,
                (pixel_x, ground_y),
                heading_east,
                heading_north,
                body_colour,
                night,
                &mut |plot_x, plot_y, colour| painter.plot(plot_x, plot_y, colour),
            );
        }
    }
}

fn add_walkers(world: &World, seconds: f32, rush_level: f32, conditions: &Conditions, sprites: &mut Vec<Sprite>) {
    const FADE_IN_OVER_LEVEL: f32 = 0.05;
    let everyday_level = (conditions.busyness + 0.3 * rush_level).min(1.0);
    let good_for_exercise = conditions.dryness()
        * (1.0 - smooth_step(0.25, 0.4, conditions.snow_cover))
        * (1.0 - smooth_step(0.6, 0.8, conditions.night));
    for walker in &world.walkers {
        let (mut level, appears_above_level) = if walker.only_in_a_rush {
            (rush_level, walker.appears_above_level * 0.8)
        } else {
            (everyday_level, walker.appears_above_level)
        };
        if matches!(walker.gait, Gait::Jogging | Gait::Cycling) {
            level *= good_for_exercise;
        }
        if level <= appears_above_level {
            continue;
        }
        let opacity = (level - appears_above_level) / FADE_IN_OVER_LEVEL;
        if opacity <= 0.004 {
            continue;
        }

        let pauses_so_far = (seconds / walker.pause_cycle_seconds).floor();
        let seconds_into_cycle = seconds - pauses_so_far * walker.pause_cycle_seconds;
        let moving_seconds_per_cycle = walker.pause_cycle_seconds - walker.pause_seconds;
        let is_paused = seconds_into_cycle > moving_seconds_per_cycle;
        let moving_seconds =
            pauses_so_far * moving_seconds_per_cycle + seconds_into_cycle.min(moving_seconds_per_cycle);
        let moving_seconds_per_clock_wrap = CLOCK_WRAP_SECONDS * moving_seconds_per_cycle / walker.pause_cycle_seconds;
        let round_trips_walked = moving_seconds / moving_seconds_per_clock_wrap * walker.round_trips_per_clock_wrap;
        let point_in_round_trip = (walker.starting_point_in_round_trip + round_trips_walked).rem_euclid(1.0);
        let is_outbound = point_in_round_trip < 0.5;
        let fraction_along_route =
            if is_outbound { point_in_round_trip * 2.0 } else { 2.0 - point_in_round_trip * 2.0 };

        let route = WALKING_ROUTES[walker.route];
        let (centre_line, heading) = position_and_heading_along_path(route, fraction_along_route * path_metres(route));
        let direction = if is_outbound { 1.0 } else { -1.0 };
        let keep_right = walker.metres_right_of_centre_line * direction;
        let position = (centre_line.0 + heading.1 * keep_right, centre_line.1 - heading.0 * keep_right);

        let stride = stride_frame(seconds + walker.person as f32 * 0.37);
        let pose = if is_paused {
            Pose::Standing
        } else {
            match walker.gait {
                Gait::Jogging => Pose::Running { stride },
                Gait::Cycling => Pose::Cycling { stride },
                _ => Pose::Walking { stride },
            }
        };
        let mut sprite = Sprite::person(position, walker.person, pose, conditions).faded(opacity);
        if let (Gait::Jogging | Gait::Cycling, SpriteKind::Person { has_umbrella, .. }) =
            (walker.gait, &mut sprite.kind)
        {
            *has_umbrella = false;
        }
        sprites.push(sprite);
    }
}

fn add_people_sitting_about(world: &World, seconds: f32, conditions: &Conditions, sprites: &mut Vec<Sprite>) {
    const SPELLS_THAT_DIVIDE_THE_CLOCK: [f32; 8] = [90.0, 96.0, 100.0, 108.0, 120.0, 135.0, 144.0, 150.0];
    const FADE_SECONDS: f32 = 2.5;
    let dry_seats = conditions.dryness()
        * (1.0 - smooth_step(0.2, 0.35, conditions.snow_cover))
        * smooth_step(0.2, 0.4, conditions.daylight);
    let sunbathing_weather = dry_seats
        * (1.0 - smooth_step(0.2, 0.5, conditions.coldness))
        * smooth_step(0.7, 0.85, conditions.fair_weather);
    let mut seat_number = 0;
    let mut maybe_seat_someone = |position: (f32, f32), pose: Pose, opacity: f32, sprites: &mut Vec<Sprite>| {
        seat_number += 1;
        if opacity <= 0.004 {
            return;
        }
        let spell_seconds = SPELLS_THAT_DIVIDE_THE_CLOCK[seat_number % SPELLS_THAT_DIVIDE_THE_CLOCK.len()];
        let staggered_seconds = (seconds + seat_number as f32 * 37.0).rem_euclid(CLOCK_WRAP_SECONDS);
        let spell = (staggered_seconds / spell_seconds).floor();
        let seconds_into_spell = staggered_seconds - spell * spell_seconds;
        let wish_to_sit = hash_fraction(seat_number as i32, spell as i32, world.seed ^ 0x51);
        let seat_is_taken = (((0.5 * conditions.busyness + 0.1) * opacity - wish_to_sit) / 0.02).clamp(0.0, 1.0);
        if seat_is_taken <= 0.0 {
            return;
        }
        let settling = (seconds_into_spell / FADE_SECONDS).min((spell_seconds - seconds_into_spell) / FADE_SECONDS);
        let person = 2000 + seat_number as u32 + spell as u32 * 31;
        let mut sprite = Sprite::person(position, person, pose, conditions).faded(settling * seat_is_taken);
        if let SpriteKind::Person { has_umbrella, .. } = &mut sprite.kind {
            *has_umbrella = false;
        }
        sprites.push(sprite);
    };
    for seat in STEP_SEATS {
        maybe_seat_someone(seat, Pose::Sitting, dry_seats, sprites);
    }
    for (bench_east, bench_north) in BENCHES {
        maybe_seat_someone((bench_east + 0.2, bench_north - 0.3), Pose::Sitting, dry_seats, sprites);
    }
    for &(table_east, table_north) in &crate::chapel::props::PLAZA_TABLES {
        maybe_seat_someone((table_east - 1.3, table_north - 0.2), Pose::Sitting, dry_seats, sprites);
    }
    for spot in SUNBATHING_SPOTS {
        maybe_seat_someone(spot, Pose::LyingDown, sunbathing_weather, sprites);
        maybe_seat_someone((spot.0 + 1.6, spot.1 + 0.6), Pose::Sitting, sunbathing_weather, sprites);
    }
}

fn add_tour_group(world: &World, seconds: f32, rush_level: f32, conditions: &Conditions, sprites: &mut Vec<Sprite>) {
    const TOUR_SECONDS: f32 = 240.0;
    const FADE_SECONDS: f32 = 4.0;
    const VISITORS: i32 = 8;
    let tour = (seconds / TOUR_SECONDS).floor();
    let seconds_into_tour = seconds - tour * TOUR_SECONDS;
    let a_tour_is_booked = hash_fraction(tour as i32, 41, world.seed) < 0.6;
    let touring_weather = smooth_step(0.5, 0.7, conditions.daylight)
        * smooth_step(0.25, 0.35, conditions.busyness)
        * conditions.dryness()
        * (1.0 - rush_level * 5.0).clamp(0.0, 1.0);
    let gathering = (seconds_into_tour / FADE_SECONDS).min((TOUR_SECONDS - seconds_into_tour) / FADE_SECONDS);
    let opacity = gathering.clamp(0.0, 1.0) * presence(touring_weather, hash_fraction(tour as i32, 44, world.seed));
    if !a_tour_is_booked || opacity <= 0.004 {
        return;
    }
    let group_east = -3.0 + hash_fraction(tour as i32, 42, world.seed) * 6.0;

    let guide_is_pointing = (seconds * 0.7) as i32 % 3 == 0;
    let guide_pose = if guide_is_pointing { Pose::ArmsRaised } else { Pose::Standing };
    let mut guide = Sprite::person((group_east, -6.6), 3000 + tour as u32 * 17, guide_pose, conditions).faded(opacity);
    if let SpriteKind::Person { outfit, .. } = &mut guide.kind {
        outfit.top = DUKE_BLUE;
    }
    sprites.push(guide);

    for visitor in 0..VISITORS {
        let column = (visitor % 3) as f32;
        let row = (visitor / 3) as f32;
        let position =
            (group_east + column * 1.2 - 1.2 + hash_fraction(visitor, 43, tour as u32) * 0.5, -8.6 - row * 1.1);
        sprites.push(
            Sprite::person(position, 3001 + visitor as u32 + tour as u32 * 17, Pose::Standing, conditions)
                .faded(opacity),
        );
    }
}

fn draw_rain_rings_on_puddles(painter: &mut Painter, world: &World, seconds: f32, conditions: &Conditions) {
    const RING_COLOUR: Colour = 0xc8d6e6;
    const SPLASH_COLOUR: Colour = 0xdfe9f4;
    if conditions.rain <= 0.15 || world.puddle_pixels.is_empty() {
        return;
    }
    painter.depth_tag = NOTHING_DRAWN;
    painter.opacity = 1.0;
    let animation_frame = (seconds * 7.0) as i32;
    let ring_radius = animation_frame % 3 + 1;
    let drops = (world.puddle_pixels.len() as f32 * 0.05 * conditions.rain) as usize;
    for drop in 0..drops {
        let chosen = hash_position(drop as i32, animation_frame / 3, 0x21BB) as usize % world.puddle_pixels.len();
        let (pixel_x, pixel_y) = (world.puddle_pixels[chosen].0 as i32, world.puddle_pixels[chosen].1 as i32);
        painter.blend(pixel_x - ring_radius, pixel_y, RING_COLOUR, 0.5);
        painter.blend(pixel_x + ring_radius, pixel_y, RING_COLOUR, 0.5);
        painter.blend(pixel_x, pixel_y, SPLASH_COLOUR, 0.6);
    }
}

fn draw_banners_on_lamp_posts(painter: &mut Painter, seconds: f32, wind: f32, snow_cover: f32) {
    let projection = painter.projection;
    const LAMP_POST_METRES: f32 = 3.9;
    const BANNER_ROWS: i32 = 5;
    const CREST_COLOUR: Colour = 0xe8e8ee;
    let banner_colour = blend_colours(DUKE_BLUE, 0x24459a, smooth_step(0.4, 0.6, snow_cover));
    let downwind = if wind < 0.0 { -1 } else { 1 };
    painter.opacity = 1.0;
    for (number, post) in LAMP_POSTS.iter().filter(|post| post.carries_a_banner).enumerate() {
        let (lamp_east, lamp_north) = (post.metres_east, post.metres_north);
        let (post_x, ground_y) = projection.ground_pixel((lamp_east, lamp_north));
        let top_y = ground_y - projection.height_in_pixels(lamp_north, LAMP_POST_METRES);
        let flutter = (angle_after_whole_cycles(seconds, 5500.0) + number as f32 * 1.7).sin() * (0.4 + wind.abs());
        let tail_swing = flutter.round() as i32 * downwind;
        painter.depth_tag = depth_tag(lamp_north);
        for row in 0..BANNER_ROWS {
            let swing = if row >= 3 { tail_swing } else { 0 };
            painter.plot(post_x + downwind + swing, top_y + 1 + row, banner_colour);
            painter.plot(
                post_x + 2 * downwind + swing,
                top_y + 1 + row,
                if row == 2 { CREST_COLOUR } else { banner_colour },
            );
        }
    }
}

fn draw_pigeons_round_the_tower(
    projection: Projection,
    canvas: &mut Canvas,
    objects: &Layer,
    seconds: f32,
    conditions: &Conditions,
) {
    const PIGEONS: i32 = 7;
    const WHITE_PIGEON: Colour = 0xe6e8ec;
    const GREY_PIGEON: Colour = 0x4a4f5a;
    const TOWER_CENTRE_NORTH: f32 = 5.8;
    let flying_weather = smooth_step(0.3, 0.4, conditions.daylight) * smooth_step(0.5, 0.6, conditions.fair_weather);
    if flying_weather <= 0.0 {
        return;
    }
    for pigeon in 0..PIGEONS {
        let number = pigeon as f32;
        let opacity = presence(flying_weather, (number + 0.5) / PIGEONS as f32);
        if opacity <= 0.0 {
            continue;
        }
        let angle = angle_after_whole_cycles(seconds, 1100.0 + 45.0 * number) + number * 0.9;
        let circle_metres = 13.0 + (pigeon % 3) as f32 * 3.0;
        let metres_up = 46.0 + number + angle_after_whole_cycles(seconds, 770.0 + 31.0 * number).sin() * 6.0;
        let metres_east = circle_metres * angle.cos();
        let metres_north = TOWER_CENTRE_NORTH + circle_metres * 0.8 * angle.sin();
        let (pixel_x, pixel_y) =
            (projection.pixel_x(metres_east, metres_north), projection.pixel_y(metres_north, metres_up));
        let colour = if pigeon % 3 == 0 { WHITE_PIGEON } else { GREY_PIGEON };
        let wings_are_up = ((seconds * 6.0 + number) as i32) % 2 == 0;
        let wing_y = if wings_are_up { pixel_y - 1 } else { pixel_y };
        for (plot_x, plot_y) in [(pixel_x, pixel_y), (pixel_x - 1, wing_y), (pixel_x + 1, wing_y)] {
            if plot_x < 0 || plot_y < 0 || plot_x >= CANVAS_WIDTH || plot_y >= CANVAS_HEIGHT {
                continue;
            }
            let index = (plot_y * CANVAS_WIDTH + plot_x) as usize;
            let is_behind_the_tower = metres_north > 0.0 && objects.flags[index] & FLOODLIT_TOWER_STONE != 0;
            if !is_behind_the_tower {
                canvas.pixels[index] = blend_colours(canvas.pixels[index], colour, opacity);
            }
        }
    }
}

pub fn draw(environment: &Environment, canvas: &mut Canvas, world: &World, objects: &Layer, sprites: &mut Vec<Sprite>) {
    let seconds = seconds_on_wrapped_clock(environment);
    let conditions = Conditions::of(environment);

    sprites.clear();
    let rush_level = crate::chapel::crowd::add_rush_crowd(environment, world, seconds, &conditions, sprites);
    add_walkers(world, seconds, rush_level, &conditions, sprites);
    add_people_sitting_about(world, seconds, &conditions, sprites);
    add_tour_group(world, seconds, rush_level, &conditions, sprites);
    crate::chapel::crowd::add_car_at_the_circle(environment, world, seconds, &conditions, sprites);
    crate::chapel::events::add_lawn_events(environment, world, seconds, &conditions, sprites);
    sprites.sort_by(|nearer, farther| farther.painting_order().total_cmp(&nearer.painting_order()));

    let mut painter = Painter { canvas, objects, depth_tag: NOTHING_DRAWN, opacity: 1.0, projection: world.projection };
    draw_rain_rings_on_puddles(&mut painter, world, seconds, &conditions);
    draw_banners_on_lamp_posts(&mut painter, seconds, environment.wind, environment.snow_cover);
    for sprite in sprites.iter() {
        if sprite.opacity > 0.004 {
            draw_sprite(&mut painter, sprite, &conditions);
        }
    }
    draw_pigeons_round_the_tower(world.projection, canvas, objects, seconds, &conditions);
}
