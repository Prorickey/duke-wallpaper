use crate::{apply_lighting, Canvas, Environment, Scene, Season, Weather};

pub fn render_without_weather(scene: &mut dyn Scene, environment: &Environment, canvas: &mut Canvas) {
    scene.render(environment, canvas);
    apply_lighting(canvas, &environment.lighting());
    scene.render_lights(environment, canvas);
}

pub fn standard_shots() -> Vec<(&'static str, Environment)> {
    let shot = |season: Season, weather: Weather, hour: f32, adjust: &dyn Fn(&mut Environment)| {
        let mut environment = Environment { season, weather, hour, ..Environment::default() };
        adjust(&mut environment);
        environment
    };
    let unchanged = |_: &mut Environment| {};
    vec![
        (
            "spring-clear-day",
            shot(Season::Spring, Weather::Clear, 11.0, &|environment| environment.season_progress = 0.25),
        ),
        ("summer-clear-day", shot(Season::Summer, Weather::Clear, 13.0, &unchanged)),
        ("summer-golden-hour", shot(Season::Summer, Weather::Clear, 19.6, &unchanged)),
        ("summer-night", shot(Season::Summer, Weather::Clear, 23.0, &unchanged)),
        (
            "summer-storm",
            shot(Season::Summer, Weather::Storm, 17.0, &|environment| {
                environment.wetness = 1.0;
                environment.wind = 0.8;
            }),
        ),
        (
            "summer-storm-flash",
            shot(Season::Summer, Weather::Storm, 21.5, &|environment| {
                environment.wetness = 1.0;
                environment.lightning_flash = 0.9;
            }),
        ),
        (
            "autumn-early-clear",
            shot(Season::Autumn, Weather::Clear, 15.0, &|environment| environment.season_progress = 0.15),
        ),
        (
            "autumn-peak-clear",
            shot(Season::Autumn, Weather::Clear, 16.0, &|environment| environment.season_progress = 0.6),
        ),
        (
            "autumn-rain",
            shot(Season::Autumn, Weather::Rain, 12.0, &|environment| {
                environment.season_progress = 0.7;
                environment.wetness = 0.9;
            }),
        ),
        ("autumn-fog-dawn", shot(Season::Autumn, Weather::Fog, 7.6, &|environment| environment.season_progress = 0.8)),
        ("winter-clear-day", shot(Season::Winter, Weather::Clear, 12.0, &unchanged)),
        ("winter-light-snow", shot(Season::Winter, Weather::Snow, 10.0, &|environment| environment.snow_cover = 0.3)),
        ("winter-deep-snow", shot(Season::Winter, Weather::Snow, 14.0, &|environment| environment.snow_cover = 1.0)),
        ("winter-snow-night", shot(Season::Winter, Weather::Snow, 20.0, &|environment| environment.snow_cover = 0.9)),
    ]
}
