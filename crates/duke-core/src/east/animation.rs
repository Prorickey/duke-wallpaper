use crate::east::bus::{add_bus_stop_crowd, BusState, Crowd, Timetable};
use crate::east::cycle_memory::{cycle_and_seconds_into_it, CycleMemory};
use crate::east::figures::{draw_person, FramePainter, PeopleConditions, Person};
use crate::east::geometry::CANVAS_WIDTH;
use crate::east::lawn_games::add_lawn_life;
use crate::east::paths::place_along;
use crate::east::projection::*;
use crate::east::props::{draw_prop, Prop, PropKind};
use crate::east::shading_tools::*;
use crate::east::static_layer::StaticLayer;
use crate::east::vehicles::{draw_bus, draw_car_side_on, markham_avenue_cars, service_loop_car};
use crate::east::walkers::{
    add_walkers, how_busy_the_quad_is, remember_whether_it_turns_up, routes, Plan, GROUNDS_CART_SLOT,
};
use crate::{blend_colours, fold_seed_to_32_bits, hash_position, rgb, Canvas, Environment, Season, Weather};

pub enum Actor {
    Person(Person),
    Bus(BusState),
    Prop(Prop),
}

impl Actor {
    fn depth(&self) -> f32 {
        match self {
            Actor::Person(person) => person.metres_south,
            Actor::Bus(bus) => bus.drawing_depth(),
            Actor::Prop(prop) => prop.metres_south,
        }
    }
}

#[derive(Default)]
pub struct Memories {
    pub who_turns_up: CycleMemory<Plan>,
    pub bus_stop_crowds: CycleMemory<Crowd>,
}

const SQUIRREL_HOMES: [(f32, f32); 5] = [(-16.5, 80.0), (15.5, 100.0), (-16.0, 228.0), (15.0, 275.0), (-16.0, 300.0)];
const BIRD_FLOCK_CYCLE_SECONDS: f32 = 38.0;
const SQUIRREL_DASH_SECONDS: f64 = 3.4;
const GROUNDS_CART_SPEED: f32 = 2.6;

pub fn render(environment: &Environment, static_layer: &StaticLayer, canvas: &mut Canvas, memories: &mut Memories) {
    rustle_the_foliage(environment, static_layer, canvas);
    let seed = fold_seed_to_32_bits(environment.seed);
    let time = environment.animation_seconds;
    let busyness = how_busy_the_quad_is(environment);
    let raining = matches!(environment.weather, Weather::Rain | Weather::Storm) && environment.weather_intensity > 0.25;
    let (shadow_direction_x, _) = environment.shadow_direction();
    let shadow_strength = environment.shadow_strength();
    let people_conditions = PeopleConditions {
        wrapped_up_for_winter: environment.season == Season::Winter || environment.snow_cover > 0.2,
        raining,
        snowing: environment.weather == Weather::Snow && environment.weather_intensity > 0.3,
        shadow_direction_x,
        shadow_strength,
    };
    let mut painter = FramePainter { canvas, depths: &static_layer.depths };

    for car in markham_avenue_cars(environment, seed, busyness, &mut memories.who_turns_up).iter().flatten() {
        draw_car_side_on(&mut painter, car);
    }
    if let Some(car) = service_loop_car(environment) {
        draw_car_side_on(&mut painter, &car);
    }

    if raining && !static_layer.puddle_pixels.is_empty() {
        let ring_count = (static_layer.puddle_pixels.len() as f32 * 0.02 * environment.weather_intensity) as i32 + 6;
        let ripple = rgb(206, 216, 230);
        for ring in 0..ring_count {
            let ring_seconds = 0.9 + (ring % 5) as f64 * 0.13;
            let (drop, seconds_since_drop) = cycle_and_seconds_into_it(time + ring as f64 * 0.37, ring_seconds);
            let spread = seconds_since_drop / (0.9 + (ring % 5) as f32 * 0.13);
            let pixel = static_layer.puddle_pixels
                [(hash_position(ring, drop, seed ^ 0xD20) as usize) % static_layer.puddle_pixels.len()]
                as usize;
            let (x, y) = ((pixel % CANVAS_WIDTH) as i32, (pixel / CANVAS_WIDTH) as i32);
            let depth = static_layer.depths[pixel];
            if spread < 0.3 {
                painter.blend(x, y, depth, ripple, 0.9);
            } else if spread < 0.65 {
                painter.blend(x - 1, y, depth, ripple, 0.6);
                painter.blend(x + 1, y, depth, ripple, 0.6);
            } else {
                painter.blend(x - 2, y, depth, ripple, 0.3);
                painter.blend(x + 2, y, depth, ripple, 0.3);
            }
        }
    }

    if busyness > 0.3 && !raining && environment.snow_cover < 0.6 {
        for (squirrel, home) in SQUIRREL_HOMES.iter().enumerate() {
            let squirrel = squirrel as i32;
            let (dash, seconds_into_dash) =
                cycle_and_seconds_into_it(time + squirrel as f64 * 1.05, SQUIRREL_DASH_SECONDS);
            let progress = (seconds_into_dash / SQUIRREL_DASH_SECONDS as f32 * 2.2).min(1.0);
            let offset_at = |dash: i32| {
                (unit_hash(dash, squirrel, seed ^ 0x501) - 0.5, unit_hash(dash, squirrel + 9, seed ^ 0x501) - 0.5)
            };
            let (from, to) = (offset_at(dash), offset_at(dash + 1));
            let eased = progress * progress * (3.0 - 2.0 * progress);
            let metres_east = home.0 + (from.0 + (to.0 - from.0) * eased) * 6.0;
            let metres_south = home.1 + (from.1 + (to.1 - from.1) * eased) * 18.0;
            let (x, y) = project_to_pixel(metres_east, metres_south, 0.0);
            let forward = if to.0 > from.0 { 1 } else { -1 };
            let hop = if progress < 1.0 && ((progress * 6.0) as i32) & 1 == 1 { 1 } else { 0 };
            painter.put(x, y - hop, metres_south, rgb(112, 96, 86));
            painter.put(x + forward, y - hop, metres_south, rgb(84, 70, 64));
            painter.put(x - forward, y - 1 - hop, metres_south, rgb(150, 136, 124));
        }
    }

    let mut actors: Vec<Actor> = Vec::with_capacity(256);
    add_walkers(environment, seed, busyness, &mut actors, &mut memories.who_turns_up);
    add_bus_stop_crowd(&Timetable::now(environment, seed, busyness), &mut actors, &mut memories.bus_stop_crowds);
    add_lawn_life(environment, seed, &mut actors);
    let cart_route = &routes().laps_round_the_lawns[0];
    let (round, seconds_into_round) =
        cycle_and_seconds_into_it(time + 40.0, (cart_route.length / GROUNDS_CART_SPEED + 70.0) as f64);
    let distance_driven = seconds_into_round * GROUNDS_CART_SPEED;
    let cart_turns_up = distance_driven < cart_route.length
        && remember_whether_it_turns_up(
            &mut memories.who_turns_up,
            GROUNDS_CART_SLOT,
            round,
            seconds_into_round,
            environment,
            seed,
            || busyness > 0.3 && environment.daylight() > 0.3,
        );
    if cart_turns_up {
        let place = place_along(&cart_route.points, distance_driven);
        let seen_side_on = place.heading_east.abs() > place.heading_south.abs() * 0.6;
        actors.push(Actor::Prop(Prop {
            metres_east: place.metres_east,
            metres_south: place.metres_south,
            metres_up: 0.0,
            kind: PropKind::GroundsCart { seen_side_on },
        }));
    }
    actors.sort_by(|first, second| first.depth().total_cmp(&second.depth()));
    let night = environment.darkness();
    for actor in actors.iter() {
        match actor {
            Actor::Person(person) => draw_person(&mut painter, person, &people_conditions),
            Actor::Bus(bus) => draw_bus(&mut painter, bus, night),
            Actor::Prop(prop) => draw_prop(&mut painter, prop),
        }
    }

    if environment.daylight() > 0.3 && environment.weather != Weather::Storm {
        let (flock, seconds_into_flight) = cycle_and_seconds_into_it(time, BIRD_FLOCK_CYCLE_SECONDS as f64);
        let share_of_cycle = seconds_into_flight / BIRD_FLOCK_CYCLE_SECONDS;
        if share_of_cycle < 0.45 {
            let bird_count = 3 + (hash_position(flock, 1, seed ^ 0xB12D) % 4) as i32;
            let first_bird_y = 60.0 + 220.0 * unit_hash(flock, 2, seed ^ 0xB12D);
            let flies_left_to_right = hash_position(flock, 3, seed ^ 0xB12D) & 1 == 0;
            let progress = share_of_cycle / 0.45;
            let bird_colour = rgb(34, 34, 46);
            let canvas = &mut painter.canvas;
            for bird in 0..bird_count {
                let formation_x = (bird as f32) * 9.0 * if flies_left_to_right { -1.0 } else { 1.0 }
                    + (unit_hash(bird, flock, 7) - 0.5) * 6.0;
                let formation_y =
                    ((bird as f32) - bird_count as f32 * 0.5).abs() * 5.0 + (unit_hash(bird, flock, 8) - 0.5) * 6.0;
                let flight_x = if flies_left_to_right { -60.0 + 760.0 * progress } else { 700.0 - 760.0 * progress };
                let x = (flight_x + formation_x) as i32;
                let y =
                    (first_bird_y + formation_y + (progress * 9.0 + bird as f32).sin() * 5.0 - progress * 30.0) as i32;
                let wings_up = (seconds_into_flight * 7.0 + bird as f32 * 1.7).sin() > 0.0;
                if shadow_strength > 0.2 {
                    let shadow_x = x + 14 + (shadow_direction_x * 20.0) as i32;
                    canvas.blend_pixel(shadow_x, y + 16, rgb(16, 22, 40), 0.22 * shadow_strength);
                    canvas.blend_pixel(shadow_x + 1, y + 16, rgb(16, 22, 40), 0.22 * shadow_strength);
                }
                canvas.set_pixel(x, y, bird_colour);
                if wings_up {
                    canvas.set_pixel(x - 1, y - 1, bird_colour);
                    canvas.set_pixel(x + 1, y - 1, bird_colour);
                } else {
                    canvas.set_pixel(x - 1, y, bird_colour);
                    canvas.set_pixel(x + 1, y, bird_colour);
                    canvas.set_pixel(x - 2, y + 1, blend_colours(bird_colour, 0xFFFFFF, 0.25));
                    canvas.set_pixel(x + 2, y + 1, blend_colours(bird_colour, 0xFFFFFF, 0.25));
                }
            }
        }
    }
}

fn rustle_the_foliage(environment: &Environment, static_layer: &StaticLayer, canvas: &mut Canvas) {
    let wind = environment.wind.clamp(-1.0, 1.0);
    let gust = 0.5
        + 0.5
            * (((environment.animation_seconds % 1000.0) * 0.23).sin()
                * ((environment.animation_seconds % 1000.0) * 0.071 + 1.3).sin()) as f32;
    let strength = 0.28 + wind.abs() * (0.9 + 0.8 * gust);
    let wave_clock = (environment.animation_seconds % (std::f64::consts::TAU * 1000.0)) as f32 * 1.25;
    let downwind = if wind >= 0.0 { 1.0 } else { -1.0 };
    let lean = downwind * wind.abs() * 0.35;
    let mut current_tree_and_row = (0u16, usize::MAX);
    let mut borrow_from_columns_upwind = 0i32;
    for &(pixel, tree_id) in static_layer.swaying_foliage.iter() {
        let pixel = pixel as usize;
        let (column, row) = (pixel % CANVAS_WIDTH, pixel / CANVAS_WIDTH);
        if (tree_id, row) != current_tree_and_row {
            let sway = static_layer.tree_sway[tree_id as usize];
            let bough_height = ((sway.crown_foot_y - row as f32) / 14.0).clamp(0.25, 1.3);
            let wave = (wave_clock + sway.phase + row as f32 * 0.21 - column as f32 * 0.07 * downwind).sin();
            borrow_from_columns_upwind = (wave * strength * sway.amplitude * bough_height + lean).round() as i32;
            current_tree_and_row = (tree_id, row);
        }
        if borrow_from_columns_upwind == 0 {
            continue;
        }
        let source_column = column as i32 - borrow_from_columns_upwind * downwind as i32;
        if source_column < 0 || source_column >= CANVAS_WIDTH as i32 {
            continue;
        }
        let source_pixel = pixel - column + source_column as usize;
        if static_layer.tree_ids[source_pixel] == tree_id {
            canvas.pixels[pixel] = static_layer.colours[source_pixel];
        }
    }
}
