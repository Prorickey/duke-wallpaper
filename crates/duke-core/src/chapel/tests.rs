use crate::chapel::ChapelScene;
use crate::preview::render_without_weather;
use crate::{Canvas, Environment, Scene, Season, Weather};

fn frame_of(scene: &mut ChapelScene, environment: &Environment) -> Vec<u32> {
    let mut canvas = Canvas::full_size();
    render_without_weather(scene, environment, &mut canvas);
    canvas.pixels
}

#[test]
fn a_used_scene_draws_exactly_what_a_fresh_scene_draws() {
    let mut used_scene = ChapelScene::new();
    let mut environment = Environment::default();
    for (wind, hour) in [(-0.7f32, 14.0f32), (0.7, 14.03), (0.6, 14.05), (0.6, 14.051)] {
        environment.wind = wind;
        environment.hour = hour;
        environment.animation_seconds += 3.3;
        frame_of(&mut used_scene, &environment);
    }
    let used_frame = frame_of(&mut used_scene, &environment);
    let fresh_frame = frame_of(&mut ChapelScene::new(), &environment);
    let differing = used_frame.iter().zip(&fresh_frame).filter(|(used, fresh)| used != fresh).count();
    assert_eq!(differing, 0);
}

fn opacity_on_the_canvas(scene: &ChapelScene) -> f32 {
    const MARGIN_PIXELS: f32 = 20.0;
    let inside = |pixel: f32, end: f32| {
        ((pixel + MARGIN_PIXELS) / MARGIN_PIXELS).min((end + MARGIN_PIXELS - pixel) / MARGIN_PIXELS).clamp(0.0, 1.0)
    };
    let mut total = 0.0;
    for sprite in &scene.cache.as_ref().unwrap().sprites {
        let pixel_x =
            crate::chapel::proj::Projection::Perspective.pixel_x(sprite.metres_east, sprite.metres_north) as f32;
        let pixel_y = crate::chapel::proj::Projection::Perspective.exact_pixel_y(sprite.metres_north, sprite.metres_up);
        total += sprite.opacity.clamp(0.0, 1.0) * inside(pixel_x, 640.0) * inside(pixel_y, 360.0);
    }
    total
}

type ConditionSweep = (&'static str, i32, Box<dyn Fn(f32) -> Environment>);

#[test]
fn sprite_opacity_never_jumps_as_the_conditions_drift() {
    let mut scene = ChapelScene::new();
    let mut canvas = Canvas::full_size();
    const SECONDS_IN_A_DAY: i32 = 86400;
    let sweeps: Vec<ConditionSweep> = vec![
        (
            "hour",
            SECONDS_IN_A_DAY,
            Box::new(|step| Environment { hour: step * 24.0, animation_seconds: 500.0, ..Environment::default() }),
        ),
        (
            "rain",
            20000,
            Box::new(|step| Environment {
                weather: Weather::Rain,
                weather_intensity: step,
                hour: 15.0,
                animation_seconds: 500.0,
                ..Environment::default()
            }),
        ),
        (
            "snowfall",
            20000,
            Box::new(|step| Environment {
                season: Season::Winter,
                weather: Weather::Snow,
                weather_intensity: step,
                snow_cover: 0.6,
                hour: 12.0,
                animation_seconds: 500.0,
                ..Environment::default()
            }),
        ),
        (
            "snow cover",
            20000,
            Box::new(|step| Environment {
                season: Season::Winter,
                snow_cover: step,
                hour: 12.0,
                animation_seconds: 500.0,
                ..Environment::default()
            }),
        ),
    ];
    for (name, steps, environment_at) in sweeps {
        let mut previous_total: Option<f32> = None;
        for step in 0..=steps {
            let environment = environment_at(step as f32 / steps as f32);
            let near_a_class_change =
                crate::chapel::crowd::CLASS_CHANGE_HOURS.iter().any(|hour| (environment.hour - hour).abs() < 0.1);
            if name == "hour" && near_a_class_change {
                previous_total = None;
                continue;
            }
            scene.render(&environment, &mut canvas);
            let total = opacity_on_the_canvas(&scene);
            if let Some(previous) = previous_total {
                assert!(
                    (total - previous).abs() < 0.6,
                    "{name} step {step}: sprite opacity jumped from {previous} to {total}"
                );
            }
            previous_total = Some(total);
        }
    }
}

#[test]
fn odd_environments_and_canvas_sizes_never_panic() {
    let odd_values = [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, -1.0e9, 1.0e9, -3.0, 0.0, 0.5, 1.0, 30.0];
    let odd_times = [f64::NAN, f64::INFINITY, -1.0e15, -0.001, 0.0, 4.0e9, 9.0e15];
    let mut scene = ChapelScene::new();
    for (index, value) in odd_values.iter().enumerate() {
        let environment = Environment {
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
        };
        for (width, height) in [(640, 360), (0, 0), (0, 5), (5, 0), (1, 1), (13, 7), (1280, 720), (639, 361)] {
            let mut canvas = Canvas::new(width, height);
            render_without_weather(&mut scene, &environment, &mut canvas);
            assert_eq!((canvas.width, canvas.height), (width, height));
        }
    }
}

#[test]
fn a_canvas_of_another_size_gets_the_whole_frame_with_its_lights() {
    let environment = Environment { hour: 22.0, animation_seconds: 470.0, ..Environment::default() };
    let standard = frame_of(&mut ChapelScene::new(), &environment);
    let mut doubled = Canvas::new(1280, 720);
    doubled.clear(0x123456);
    render_without_weather(&mut ChapelScene::new(), &environment, &mut doubled);
    for row in 0..360 {
        for column in 0..640 {
            assert_eq!(doubled.pixel_at(2 * column + 1, 2 * row + 1), standard[(row * 640 + column) as usize]);
        }
    }
}

fn static_layers(season: Season, season_progress: f32, snow_cover: f32) -> Vec<u32> {
    let look =
        crate::chapel::palette::Look { season, season_progress, snow_cover, wetness: 0.0, sun_side: -0.5, seed: 1924 };
    let palette = crate::chapel::palette::Palette::new(&look);
    let mut ground = crate::chapel::art::Layer::new(640, 360);
    let context = crate::chapel::tex::DrawingContext {
        projection: crate::chapel::proj::Projection::Perspective,
        look: &look,
        palette: &palette,
        sway: 0.0,
        distant_stonework: false,
    };
    crate::chapel::ground::draw_ground(&mut ground, &context);
    let (objects, _) = crate::chapel::build_objects(
        &look,
        &palette,
        0.0,
        &crate::chapel::trees::plant(1924),
        crate::chapel::proj::Projection::Perspective,
    );
    let mut pixels = ground.pixels.clone();
    for (index, pixel) in pixels.iter_mut().enumerate() {
        if objects.depth[index] != crate::chapel::art::NOTHING_DRAWN {
            *pixel = objects.pixels[index];
        }
    }
    pixels
}

#[test]
fn nothing_jumps_where_one_season_turns_into_the_next() {
    for (ending, beginning) in [
        (Season::Spring, Season::Summer),
        (Season::Summer, Season::Autumn),
        (Season::Autumn, Season::Winter),
        (Season::Winter, Season::Spring),
    ] {
        let last_day = static_layers(ending, 1.0, 0.0);
        let first_day = static_layers(beginning, 0.0, 0.0);
        let differing = last_day.iter().zip(&first_day).filter(|(last, first)| last != first).count();
        assert_eq!(differing, 0, "{ending:?} into {beginning:?}: {differing} pixels change");
    }
}

#[test]
fn every_lightning_strike_point_is_on_a_drawn_pixel_inside_the_canvas() {
    let mut scene = ChapelScene::new();
    frame_of(&mut scene, &Environment::default());
    let objects = &scene.cache.as_ref().unwrap().resting_objects;
    let strike_points = scene.info().lightning_strike_points;
    assert_eq!(strike_points.len(), 4 + crate::chapel::trees::TREES_LIGHTNING_CAN_STRIKE.len());
    for (pixel_x, pixel_y) in strike_points {
        assert!((0..640).contains(&pixel_x) && (0..360).contains(&pixel_y), "({pixel_x}, {pixel_y}) is off the canvas");
        assert_ne!(
            objects.depth[(pixel_y * 640 + pixel_x) as usize],
            crate::chapel::art::NOTHING_DRAWN,
            "({pixel_x}, {pixel_y}) is on no drawn object"
        );
    }
}

#[test]
fn the_static_layers_are_built_once_while_their_look_holds() {
    let mut scene = ChapelScene::new();
    let mut environment = Environment { hour: 9.0, ..Environment::default() };
    for _ in 0..200 {
        environment.animation_seconds += 0.05;
        environment.hour += 0.0002;
        environment.wind = (environment.animation_seconds as f32).sin();
        frame_of(&mut scene, &environment);
    }
    assert_eq!(scene.times_rebuilt, 1);
    environment.season = Season::Autumn;
    frame_of(&mut scene, &environment);
    assert_eq!(scene.times_rebuilt, 2);
}

#[test]
fn the_scene_is_drawn_in_perspective_unless_the_plan_view_is_asked_for() {
    assert_eq!(ChapelScene::new().projection, crate::chapel::proj::Projection::Perspective);
    let plan_view =
        crate::chapel::proj::PlanView { centre_metres_east: 0.0, centre_metres_north: 40.0, pixels_per_metre: 2.0 };
    assert_eq!(
        ChapelScene::seen_straight_down(plan_view).projection,
        crate::chapel::proj::Projection::StraightDown(plan_view)
    );
}

#[test]
fn the_same_seed_draws_the_same_frame_and_another_seed_draws_another() {
    let environment = Environment { hour: 13.0, animation_seconds: 1234.5, ..Environment::default() };
    let first = frame_of(&mut ChapelScene::new(), &environment);
    let second = frame_of(&mut ChapelScene::new(), &environment);
    assert_eq!(first, second);
    let reseeded =
        frame_of(&mut ChapelScene::new(), &Environment { seed: environment.seed ^ 0xBEEF, ..environment.clone() });
    assert_ne!(first, reseeded);
}

#[test]
fn nothing_jumps_where_the_clock_wraps_round() {
    let mut scene = ChapelScene::new();
    let mut canvas = Canvas::full_size();
    for hour in [8.0f32, 12.5, 17.0] {
        let mut previous_total: Option<f32> = None;
        for step in -40..=40 {
            let environment =
                Environment { hour, animation_seconds: 21600.0 * 3.0 + step as f64 * 0.025, ..Environment::default() };
            scene.render(&environment, &mut canvas);
            let total = opacity_on_the_canvas(&scene);
            if let Some(previous) = previous_total {
                assert!(
                    (total - previous).abs() < 0.6,
                    "hour {hour} step {step}: sprite opacity jumped from {previous} to {total}"
                );
            }
            previous_total = Some(total);
        }
    }
}

#[test]
fn every_repeating_cycle_fits_a_whole_number_of_times_into_the_clock_wrap() {
    let mut cycles = vec![
        crate::chapel::crowd::RUSH_CYCLE_SECONDS,
        crate::chapel::crowd::CAR_CALLS_EVERY_SECONDS,
        crate::chapel::events::SLOT_SECONDS,
        crate::chapel::events::VISIT_EVERY_SECONDS,
    ];
    cycles.extend(crate::chapel::anim::PAUSE_CYCLES_THAT_DIVIDE_THE_CLOCK);
    for cycle_seconds in cycles {
        let repeats = crate::chapel::anim::CLOCK_WRAP_SECONDS / cycle_seconds;
        assert_eq!(repeats, repeats.round(), "{cycle_seconds} s does not divide the clock wrap");
    }
}
