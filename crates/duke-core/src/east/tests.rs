use crate::east::bus::{self, Timetable};
use crate::east::cycle_memory::CycleMemory;
use crate::east::walkers::how_busy_the_quad_is;
use crate::east::EastCampusScene;
use crate::preview::render_without_weather;
use crate::{Canvas, Environment, Scene, Season, Weather};

fn render_sequence(
    scene: &mut EastCampusScene,
    first_environment: &Environment,
    frame_count: u32,
    seconds_between_frames: f64,
) -> Vec<Vec<u32>> {
    let mut environment = first_environment.clone();
    let mut canvas = Canvas::full_size();
    let mut frames = Vec::new();
    for _ in 0..frame_count {
        render_without_weather(scene, &environment, &mut canvas);
        frames.push(canvas.pixels.clone());
        environment.animation_seconds += seconds_between_frames;
        environment.hour += (seconds_between_frames / 3600.0) as f32;
    }
    frames
}

#[test]
fn the_same_seed_and_sequence_always_draw_the_same_frames() {
    let mut environment = Environment { animation_seconds: 470.0, seed: 42, ..Environment::default() };
    let first_run = render_sequence(&mut EastCampusScene::new(), &environment, 6, 7.0);
    let second_run = render_sequence(&mut EastCampusScene::new(), &environment, 6, 7.0);
    assert!(first_run == second_run);

    environment.seed = 43;
    let other_seed = render_sequence(&mut EastCampusScene::new(), &environment, 1, 0.0);
    assert!(other_seed[0] != first_run[0]);
}

fn odd_environments() -> Vec<Environment> {
    let mut environments = Vec::new();
    let odd_values = [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, -1.0e9, 1.0e9, -3.0, 0.0, 0.5, 1.0, 30.0];
    let odd_times = [f64::NAN, f64::INFINITY, -1.0e15, -0.001, 0.0, 4.0e9, 9.0e15];
    for (index, value) in odd_values.iter().enumerate() {
        environments.push(Environment {
            season: Season::ALL[index % 4],
            weather: Weather::ALL[index % 6],
            season_progress: *value,
            weather_intensity: *value,
            hour: *value,
            wind: *value,
            snow_cover: *value,
            wetness: *value,
            lightning_flash: *value,
            animation_seconds: odd_times[index % odd_times.len()],
            seed: (index as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15),
        });
    }
    environments
}

#[test]
fn odd_environments_and_canvas_sizes_never_panic() {
    let mut scene = EastCampusScene::new();
    for environment in odd_environments() {
        for (width, height) in [(640, 360), (0, 0), (0, 5), (5, 0), (1, 1), (13, 7), (1280, 720), (639, 361)] {
            let mut canvas = Canvas::new(width, height);
            render_without_weather(&mut scene, &environment, &mut canvas);
        }
    }
}

#[test]
fn every_pixel_of_a_non_standard_canvas_is_drawn_with_life_and_lights() {
    let environment = Environment { hour: 22.0, animation_seconds: 470.0, ..Environment::default() };
    let mut scene = EastCampusScene::new();
    let mut standard = Canvas::full_size();
    render_without_weather(&mut scene, &environment, &mut standard);
    let mut doubled = Canvas::new(1280, 720);
    doubled.clear(0x123456);
    render_without_weather(&mut scene, &environment, &mut doubled);
    for y in 0..360 {
        for x in 0..640 {
            assert_eq!(doubled.pixel_at(2 * x + 1, 2 * y + 1), standard.pixel_at(x, y));
        }
    }
}

#[test]
fn a_pinned_clock_never_changes_a_bus_cycle_rush_decision_half_way() {
    let seed = 1924;
    for pinned_hour in [9.86, 9.87, 9.88, 9.89, 9.9, 13.12, 13.13, 16.5] {
        let mut environment = Environment { hour: pinned_hour, ..Environment::default() };
        let mut memory = CycleMemory::default();
        let mut decisions: Vec<(i64, bool, usize)> = Vec::new();
        for step in 0..1000 {
            environment.animation_seconds = 30.0 + step as f64 * 0.5;
            let timetable = Timetable::now(&environment, seed, how_busy_the_quad_is(&environment));
            for cycle in timetable.current_cycle..timetable.current_cycle + 3 {
                let bus_cycle = timetable.cycle(cycle, &mut memory);
                match decisions.iter().find(|decision| decision.0 == cycle) {
                    Some(earlier) => assert!(
                        earlier.1 == bus_cycle.is_rush && earlier.2 == bus_cycle.riders_waiting,
                        "cycle {cycle} changed its mind at hour {pinned_hour}"
                    ),
                    None => decisions.push((cycle, bus_cycle.is_rush, bus_cycle.riders_waiting)),
                }
            }
        }
    }
}

#[test]
fn a_running_clock_still_brings_the_class_change_rush() {
    let seed = 1924;
    let mut memory = CycleMemory::default();
    let mut environment = Environment::default();
    let mut rush_cycles_seen = 0;
    for step in 0..400 {
        environment.animation_seconds = step as f64 * 2.0;
        environment.hour = bus::RUSH_HOURS[1] - 0.1 + (environment.animation_seconds / 3600.0) as f32;
        let timetable = Timetable::now(&environment, seed, how_busy_the_quad_is(&environment));
        let bus_cycle = timetable.cycle(timetable.current_cycle, &mut memory);
        if bus_cycle.is_rush && !bus::is_timed_rush(timetable.current_cycle) {
            rush_cycles_seen += 1;
        }
    }
    assert!(rush_cycles_seen > 0);
}

#[test]
fn every_rider_in_a_rush_gets_on_a_bus_and_the_load_adds_up() {
    let seed = 1924;
    let mut memory = CycleMemory::default();
    let environment = Environment::default();
    let busyness = how_busy_the_quad_is(&environment);
    let mut rush_cycles_checked = 0;
    for cycle in 0..40 {
        let timetable = Timetable {
            environment: &environment,
            seed,
            busyness,
            current_cycle: cycle,
            seconds_into_current_cycle: 0.0,
        };
        let bus_cycle = timetable.cycle(cycle, &mut memory);
        let total_boarders: usize = bus_cycle.buses.iter().map(|times| times.boarders).sum();
        assert_eq!(total_boarders, bus_cycle.riders_waiting);
        if bus_cycle.is_rush {
            rush_cycles_checked += 1;
            assert_eq!(bus_cycle.buses.len(), 3);
        }
        let inside_the_bus = |times: &bus::BusTimes, place_in_queue: usize| {
            times.doors_open
                + bus::FIRST_BOARDER_AFTER_DOORS_OPEN
                + place_in_queue as f32 * bus::SECONDS_BETWEEN_BOARDERS
                + bus::SECONDS_TO_WALK_INTO_THE_BUS
        };
        for (bus_index, times) in bus_cycle.buses.iter().enumerate() {
            for place_in_queue in 0..times.boarders {
                assert!(
                    inside_the_bus(times, place_in_queue) <= times.doors_close,
                    "rider {place_in_queue} of bus {bus_index} is shut out"
                );
            }
            let mut seconds = times.enters;
            while seconds <= times.leaves {
                if let Some(state) = bus::bus_on_the_road(times, bus_index, seconds, 0) {
                    let stepped_off = (0..times.alighters)
                        .filter(|rider| {
                            times.doors_open
                                + bus::FIRST_ALIGHTER_AFTER_DOORS_OPEN
                                + *rider as f32 * bus::SECONDS_BETWEEN_ALIGHTERS
                                <= seconds
                        })
                        .count();
                    let got_on = (0..times.boarders)
                        .filter(|place_in_queue| inside_the_bus(times, *place_in_queue) <= seconds)
                        .count();
                    assert_eq!(state.riders_aboard, (times.aboard_on_arrival - stepped_off + got_on) as f32);
                }
                seconds += 0.05;
            }
        }
    }
    assert!(rush_cycles_checked > 0);
}

#[test]
fn lightning_strike_points_are_inside_the_canvas() {
    let information = EastCampusScene::new().info();
    assert_eq!(information.id, "east");
    assert_eq!(information.title, "East Campus");
    assert!(information.lightning_strike_points.len() >= 10);
    for (x, y) in information.lightning_strike_points {
        assert!((0..640).contains(&x) && (0..360).contains(&y));
    }
}

#[test]
fn the_static_layer_is_only_rebuilt_when_its_key_changes() {
    let mut scene = EastCampusScene::new();
    let mut canvas = Canvas::full_size();
    let mut environment = Environment::default();
    render_without_weather(&mut scene, &environment, &mut canvas);
    assert_eq!(scene.times_static_layer_was_built, 1);
    for _ in 0..20 {
        environment.animation_seconds += 3.0;
        environment.wind = -environment.wind;
        environment.lightning_flash = 0.0;
        render_without_weather(&mut scene, &environment, &mut canvas);
    }
    environment.weather = Weather::Cloudy;
    environment.weather_intensity = 0.0;
    render_without_weather(&mut scene, &environment, &mut canvas);
    assert_eq!(scene.times_static_layer_was_built, 1);
    environment.snow_cover = 0.5;
    render_without_weather(&mut scene, &environment, &mut canvas);
    assert_eq!(scene.times_static_layer_was_built, 2);
}
