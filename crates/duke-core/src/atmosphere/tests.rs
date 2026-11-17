use crate::atmosphere::ambient::draw_leaves;
use crate::atmosphere::drawing::EffectState;
use crate::atmosphere::rain::{draw_rain_landing, RainLanding};
use crate::atmosphere::simulation::{season_and_progress, weather_weights};
use crate::atmosphere::{Atmosphere, Controls, WallClock};
use crate::{Canvas, Environment, SceneInfo, Season, Weather};

const FRAME_SECONDS: f32 = 0.05;
const SNOW_WEIGHT_INDEX: usize = 4;
const STORM_WEIGHT_INDEX: usize = 3;
const FOG_WEIGHT_INDEX: usize = 5;
const SUMMER_AFTERNOON: WallClock = WallClock { month: 7, day: 15, hour: 13.0 };
const WINTER_AFTERNOON: WallClock = WallClock { month: 1, day: 15, hour: 13.0 };

fn pinned_to(weather: Weather) -> Controls {
    Controls { weather: Some(weather), ..Controls::default() }
}

fn run_for(
    atmosphere: &mut Atmosphere,
    seconds: f32,
    step_seconds: f32,
    clock: &WallClock,
    controls: &Controls,
) -> Environment {
    let mut environment = atmosphere.step(0.0, clock, controls);
    let mut seconds_run = 0.0;
    while seconds_run < seconds {
        environment = atmosphere.step(step_seconds, clock, controls);
        seconds_run += step_seconds;
    }
    environment
}

fn assert_environment_is_sane(environment: &Environment) {
    let unit_range = 0.0..=1.0;
    assert!(unit_range.contains(&environment.weather_intensity), "intensity {}", environment.weather_intensity);
    assert!(unit_range.contains(&environment.season_progress), "season progress {}", environment.season_progress);
    assert!((0.0..24.0).contains(&environment.hour), "hour {}", environment.hour);
    assert!((-1.0..=1.0).contains(&environment.wind), "wind {}", environment.wind);
    assert!(unit_range.contains(&environment.snow_cover), "snow cover {}", environment.snow_cover);
    assert!(unit_range.contains(&environment.wetness), "wetness {}", environment.wetness);
    assert!(unit_range.contains(&environment.lightning_flash), "flash {}", environment.lightning_flash);
    assert!(
        environment.animation_seconds.is_finite() && environment.animation_seconds >= 0.0,
        "time {}",
        environment.animation_seconds
    );
}

fn environments_are_identical(first: &Environment, second: &Environment) -> bool {
    first.season == second.season
        && first.weather == second.weather
        && first.season_progress.to_bits() == second.season_progress.to_bits()
        && first.weather_intensity.to_bits() == second.weather_intensity.to_bits()
        && first.hour.to_bits() == second.hour.to_bits()
        && first.animation_seconds.to_bits() == second.animation_seconds.to_bits()
        && first.wind.to_bits() == second.wind.to_bits()
        && first.snow_cover.to_bits() == second.snow_cover.to_bits()
        && first.wetness.to_bits() == second.wetness.to_bits()
        && first.lightning_flash.to_bits() == second.lightning_flash.to_bits()
        && first.seed == second.seed
}

fn share_of_pixels_changed(frame: &Canvas, background: u32) -> f32 {
    let changed_pixels = frame.pixels.iter().filter(|pixel| **pixel != background).count();
    changed_pixels as f32 / frame.pixels.len() as f32
}

#[test]
fn seasons_follow_the_calendar() {
    assert_eq!(season_and_progress(3, 1, 0.0).0, Season::Spring);
    assert_eq!(season_and_progress(5, 31, 23.0).0, Season::Spring);
    assert_eq!(season_and_progress(6, 1, 0.0).0, Season::Summer);
    assert_eq!(season_and_progress(9, 1, 0.0).0, Season::Autumn);
    assert_eq!(season_and_progress(11, 30, 12.0).0, Season::Autumn);
    assert_eq!(season_and_progress(12, 1, 0.0).0, Season::Winter);
    assert_eq!(season_and_progress(2, 28, 12.0).0, Season::Winter);

    assert!(season_and_progress(3, 1, 0.0).1 < 0.01);
    assert!(season_and_progress(5, 31, 23.9).1 > 0.99);
    assert!(season_and_progress(12, 1, 0.0).1 < 0.01);
    let progress_in_mid_january = season_and_progress(1, 15, 12.0).1;
    let progress_at_end_of_february = season_and_progress(2, 28, 23.9).1;
    assert!(progress_in_mid_january > 0.45 && progress_in_mid_january < 0.55, "{progress_in_mid_january}");
    assert!(progress_at_end_of_february > 0.98, "{progress_at_end_of_february}");

    let mut previous_progress = -1.0;
    for (month, day) in [(9, 1), (9, 20), (10, 10), (10, 31), (11, 16), (11, 30)] {
        let progress = season_and_progress(month, day, 12.0).1;
        assert!(progress > previous_progress && (0.0..=1.0).contains(&progress));
        previous_progress = progress;
    }

    for (nonsense_month, nonsense_day) in [(0, 0), (13, 40), (2, 31), (u32::MAX, u32::MAX)] {
        let (_, progress) = season_and_progress(nonsense_month, nonsense_day, f32::NAN);
        assert!((0.0..=1.0).contains(&progress));
    }
}

#[test]
fn pinned_season_and_hour_are_honoured() {
    let july_morning = WallClock { month: 7, day: 4, hour: 9.0 };
    let autumn_evening = Controls { season: Some(Season::Autumn), hour: Some(21.0), ..Controls::default() };
    let environment = Atmosphere::new(3).step(FRAME_SECONDS, &july_morning, &autumn_evening);
    assert_eq!(environment.season, Season::Autumn);
    assert_eq!(environment.hour, 21.0);
    assert!((environment.season_progress - 0.55).abs() < 1e-6, "out of season should look mid-season");

    let summer_in_summer = Controls { season: Some(Season::Summer), ..Controls::default() };
    let environment = Atmosphere::new(3).step(FRAME_SECONDS, &july_morning, &summer_in_summer);
    assert!((environment.season_progress - season_and_progress(7, 4, 9.0).1).abs() < 1e-6);
    assert_eq!(environment.hour, 9.0);
}

#[test]
fn pinning_the_hour_mid_run_glides() {
    let mut atmosphere = Atmosphere::new(5);
    let mut controls = pinned_to(Weather::Clear);
    run_for(&mut atmosphere, 2.0, FRAME_SECONDS, &SUMMER_AFTERNOON, &controls);
    controls.hour = Some(23.0);
    let mut previous_hour = 13.0f32;
    let mut frames_taken = 0;
    loop {
        let environment = atmosphere.step(FRAME_SECONDS, &SUMMER_AFTERNOON, &controls);
        let hours_moved = (environment.hour - previous_hour).abs();
        assert!(hours_moved <= 1.3, "hour jumped by {hours_moved}");
        previous_hour = environment.hour;
        frames_taken += 1;
        if environment.hour == 23.0 {
            break;
        }
        assert!(frames_taken < 200, "never arrived");
    }
    assert!(frames_taken > 10, "snapped in {frames_taken} frames");
}

#[test]
fn hour_never_reaches_twenty_four() {
    for awkward_hour in [-1.0e-9f32, -1.0e-7, 24.0, 47.999_999, -24.0] {
        let pinned_hour = Controls { hour: Some(awkward_hour), ..pinned_to(Weather::Clear) };
        let environment = Atmosphere::new(1).step(FRAME_SECONDS, &SUMMER_AFTERNOON, &pinned_hour);
        assert!((0.0..24.0).contains(&environment.hour), "pinned hour {awkward_hour} became {}", environment.hour);
        let awkward_clock = WallClock { month: 7, day: 4, hour: awkward_hour };
        let environment = Atmosphere::new(1).step(FRAME_SECONDS, &awkward_clock, &pinned_to(Weather::Clear));
        assert!((0.0..24.0).contains(&environment.hour), "clock hour {awkward_hour} became {}", environment.hour);
    }

    let mut atmosphere = Atmosphere::new(1);
    let mut controls = Controls { hour: Some(0.3), ..pinned_to(Weather::Clear) };
    atmosphere.step(FRAME_SECONDS, &SUMMER_AFTERNOON, &controls);
    controls.hour = Some(23.2);
    for _ in 0..400 {
        let environment = atmosphere.step(0.013, &SUMMER_AFTERNOON, &controls);
        assert!((0.0..24.0).contains(&environment.hour), "gliding back across midnight reached {}", environment.hour);
    }
}

#[test]
fn pinned_weather_starts_at_full_strength_with_ground_state() {
    for weather in Weather::ALL {
        for month in [1, 7] {
            let clock = WallClock { month, day: 15, hour: 14.0 };
            let mut atmosphere = Atmosphere::new(11);
            let environment = atmosphere.step(FRAME_SECONDS, &clock, &pinned_to(weather));
            assert_eq!(environment.weather, weather);
            assert_eq!(environment.weather_intensity, 1.0, "{weather:?} in month {month}");
            if weather == Weather::Snow {
                assert!(environment.snow_cover >= 0.3, "snow cover {}", environment.snow_cover);
            }
            if weather == Weather::Rain || weather == Weather::Storm {
                assert!(environment.wetness >= 0.8, "wetness {}", environment.wetness);
            }
            let environment = run_for(&mut atmosphere, 120.0, 0.25, &clock, &pinned_to(weather));
            assert_eq!(environment.weather, weather, "a pinned weather must stay");
            assert!(environment.weather_intensity > 0.99);
        }
    }
}

#[test]
fn pinning_mid_run_transitions_smoothly() {
    let mut atmosphere = Atmosphere::new(21);
    let mut controls = pinned_to(Weather::Rain);
    let mut previous = run_for(&mut atmosphere, 5.0, FRAME_SECONDS, &SUMMER_AFTERNOON, &controls);
    controls.weather = Some(Weather::Snow);
    let mut switched_after_seconds = None;
    let mut seconds_run = 0.0;
    while seconds_run < 120.0 {
        let environment = atmosphere.step(FRAME_SECONDS, &SUMMER_AFTERNOON, &controls);
        assert_environment_is_sane(&environment);
        assert!((environment.weather_intensity - previous.weather_intensity).abs() < 0.02, "intensity snapped");
        if environment.weather != previous.weather {
            let switched_while_faint = environment.weather_intensity < 0.02 && previous.weather_intensity < 0.02;
            assert!(switched_while_faint, "switched at intensity {}", previous.weather_intensity);
            assert_eq!(environment.weather, Weather::Snow);
            switched_after_seconds = Some(seconds_run);
        }
        previous = environment;
        seconds_run += FRAME_SECONDS;
    }
    let switched_after_seconds = switched_after_seconds.expect("never switched to the pinned weather");
    assert!(switched_after_seconds > 5.0 && switched_after_seconds < 60.0, "switch after {switched_after_seconds}s");
    assert_eq!(previous.weather, Weather::Snow);
    assert!(previous.weather_intensity > 0.99);

    controls.weather = None;
    let mut left_the_snow = false;
    let mut seconds_run = 0.0;
    while seconds_run < 240.0 {
        let environment = atmosphere.step(FRAME_SECONDS, &SUMMER_AFTERNOON, &controls);
        assert!(
            (environment.weather_intensity - previous.weather_intensity).abs() < 0.02,
            "intensity snapped on unpin"
        );
        if environment.weather != previous.weather {
            assert!(environment.weather_intensity < 0.02);
            left_the_snow = true;
        }
        previous = environment;
        seconds_run += FRAME_SECONDS;
    }
    assert!(left_the_snow, "snow in July should not survive being unpinned");
    assert_ne!(previous.weather, Weather::Snow);
}

#[test]
fn everything_stays_in_range_and_continuous() {
    for (seed, month) in [(1u64, 1u32), (2, 4), (3, 7), (4, 10), (5, 12)] {
        let mut atmosphere = Atmosphere::new(seed);
        let one_minute_spells = Controls { weather_spell_minutes: 1.0, ..Controls::default() };
        let mut clock = WallClock { month, day: 10, hour: 5.0 };
        let mut previous = atmosphere.step(0.0, &clock, &one_minute_spells);
        let mut weather_changes = 0;
        let step_count = 3600 * 20;
        for step_index in 0..step_count {
            clock.hour = (5.0 + step_index as f32 * 20.0 / step_count as f32).rem_euclid(24.0);
            let environment = atmosphere.step(0.1, &clock, &one_minute_spells);
            assert_environment_is_sane(&environment);
            assert!((environment.weather_intensity - previous.weather_intensity).abs() < 0.06, "intensity step");
            let wind_change = (environment.wind - previous.wind).abs();
            assert!(wind_change < 0.12, "wind step {wind_change}");
            assert!((environment.snow_cover - previous.snow_cover).abs() < 0.01, "snow step");
            assert!((environment.wetness - previous.wetness).abs() < 0.02, "wetness step");
            if environment.weather != previous.weather {
                weather_changes += 1;
                assert!(
                    environment.weather_intensity < 0.04,
                    "weather switched at intensity {}",
                    environment.weather_intensity
                );
            }
            let cloud_deck = atmosphere.cloud_deck_overcast;
            assert!((-1e-4..=1.0).contains(&cloud_deck), "cloud deck {cloud_deck}");
            previous = environment;
        }
        assert!(weather_changes >= 5, "only {weather_changes} weather changes in two hours of 1-minute spells");
    }
}

#[test]
fn no_snow_in_summer_and_storms_like_summer_evenings() {
    for season_progress in [0.0, 0.3, 0.7, 1.0] {
        for hour in 0..24 {
            for weather_before in [None, Some(Weather::Cloudy), Some(Weather::Snow)] {
                let weights = weather_weights(Season::Summer, season_progress, hour as f32, weather_before);
                assert_eq!(weights[SNOW_WEIGHT_INDEX], 0.0);
            }
        }
    }
    assert_eq!(weather_weights(Season::Spring, 0.5, 12.0, None)[SNOW_WEIGHT_INDEX], 0.0);
    assert_eq!(weather_weights(Season::Autumn, 0.5, 12.0, None)[SNOW_WEIGHT_INDEX], 0.0);
    assert!(weather_weights(Season::Winter, 0.5, 12.0, None)[SNOW_WEIGHT_INDEX] > 0.0);

    let share_of_weather = |weight_index: usize, season: Season, hour: f32| {
        let weights = weather_weights(season, 0.5, hour, None);
        weights[weight_index] / weights.iter().sum::<f32>()
    };
    let summer_evening_storms = share_of_weather(STORM_WEIGHT_INDEX, Season::Summer, 17.0);
    assert!(summer_evening_storms > 3.0 * share_of_weather(STORM_WEIGHT_INDEX, Season::Summer, 8.0));
    assert!(summer_evening_storms > 10.0 * share_of_weather(STORM_WEIGHT_INDEX, Season::Winter, 17.0));
    let autumn_dawn_fog = share_of_weather(FOG_WEIGHT_INDEX, Season::Autumn, 7.4);
    assert!(autumn_dawn_fog > 3.0 * share_of_weather(FOG_WEIGHT_INDEX, Season::Autumn, 15.0));
    assert!(autumn_dawn_fog > 3.0 * share_of_weather(FOG_WEIGHT_INDEX, Season::Summer, 15.0));

    for seed in 0..4u64 {
        let mut atmosphere = Atmosphere::new(seed);
        let two_minute_spells = Controls { weather_spell_minutes: 2.0, ..Controls::default() };
        let mut clock = WallClock { month: 7, day: 20, hour: 0.0 };
        for second in 0..36000 {
            clock.hour = (second as f32 / 1500.0).rem_euclid(24.0);
            let environment = atmosphere.step(1.0, &clock, &two_minute_spells);
            assert_ne!(environment.weather, Weather::Snow, "snow in July (seed {seed})");
            assert_eq!(environment.snow_cover, 0.0);
        }
    }
}

#[test]
fn soak_days_of_weather_is_mostly_fair() {
    for (month, season) in [(4u32, Season::Spring), (7, Season::Summer), (10, Season::Autumn), (1, Season::Winter)] {
        let mut seconds_of_each_weather = [0.0f64; 6];
        let mut total_seconds = 0.0f64;
        for seed in 0..3u64 {
            let mut atmosphere = Atmosphere::new(100 + seed);
            let controls = Controls::default();
            let mut clock = WallClock { month, day: 15, hour: 0.0 };
            let step_seconds = 0.5f32;
            let step_count = (4.0 * 86400.0 / step_seconds) as u32;
            for step_index in 0..step_count {
                clock.hour = ((step_index as f32 * step_seconds) / 3600.0).rem_euclid(24.0);
                let environment = atmosphere.step(step_seconds, &clock, &controls);
                if step_index % 97 == 0 {
                    assert_environment_is_sane(&environment);
                }
                assert_eq!(environment.season, season);
                seconds_of_each_weather[environment.weather as usize] += step_seconds as f64;
                total_seconds += step_seconds as f64;
            }
        }
        let share = |weather: Weather| seconds_of_each_weather[weather as usize] / total_seconds;
        println!(
            "{season:?}: clear {:.2} cloudy {:.2} rain {:.2} storm {:.2} snow {:.2} fog {:.2}",
            share(Weather::Clear),
            share(Weather::Cloudy),
            share(Weather::Rain),
            share(Weather::Storm),
            share(Weather::Snow),
            share(Weather::Fog)
        );
        assert!(share(Weather::Clear) > 0.38, "{season:?}: only {:.2} clear", share(Weather::Clear));
        assert!(share(Weather::Clear) + share(Weather::Cloudy) > 0.6, "{season:?}: the art is hidden too often");
        assert!(share(Weather::Storm) < 0.15);
        if season == Season::Winter {
            assert!(share(Weather::Snow) > 0.03, "winter never snows");
        } else {
            assert_eq!(share(Weather::Snow), 0.0, "{season:?}: mid-season snow");
        }
    }
}

#[test]
fn spell_countdown_is_frame_rate_independent() {
    let twelve_hour_spells = Controls { weather_spell_minutes: 720.0, ..Controls::default() };
    let mut long_spells_checked = 0;
    for seed in 0..40u64 {
        let mut atmosphere = Atmosphere::new(seed);
        atmosphere.step(0.0, &SUMMER_AFTERNOON, &twelve_hour_spells);
        let remaining_before = atmosphere.simulation_state().spell_seconds_remaining;
        if remaining_before < 40_000.0 {
            continue;
        }
        for _ in 0..20_000 {
            atmosphere.step(0.001, &SUMMER_AFTERNOON, &twelve_hour_spells);
        }
        let remaining_after = atmosphere.simulation_state().spell_seconds_remaining;
        let seconds_counted = remaining_before - remaining_after;
        assert!(
            (seconds_counted - 20.0).abs() < 0.5,
            "20 s passed but the spell went from {remaining_before} to {remaining_after}"
        );
        long_spells_checked += 1;
    }
    assert!(long_spells_checked >= 3, "no long spells among the seeds tried");
}

#[test]
fn changing_spell_length_rescales_the_current_spell() {
    let mut controls = Controls { weather_spell_minutes: 600.0, ..Controls::default() };
    let mut atmosphere = Atmosphere::new(9);
    run_for(&mut atmosphere, 1.0, FRAME_SECONDS, &SUMMER_AFTERNOON, &controls);
    let remaining_at_ten_hours = atmosphere.simulation_state().spell_seconds_remaining;
    assert!(remaining_at_ten_hours > 3000.0, "expected a long first spell, got {remaining_at_ten_hours}");

    controls.weather_spell_minutes = 0.5;
    atmosphere.step(FRAME_SECONDS, &SUMMER_AFTERNOON, &controls);
    let remaining_at_half_a_minute = atmosphere.simulation_state().spell_seconds_remaining;
    assert!(
        remaining_at_half_a_minute < 60.0,
        "half-minute spells requested but {remaining_at_half_a_minute} s still remain"
    );

    controls.weather_spell_minutes = 600.0;
    atmosphere.step(FRAME_SECONDS, &SUMMER_AFTERNOON, &controls);
    let remaining_after_restoring = atmosphere.simulation_state().spell_seconds_remaining;
    let seconds_lost = remaining_at_ten_hours - remaining_after_restoring;
    assert!(
        (0.0..130.0).contains(&seconds_lost),
        "{remaining_at_ten_hours} became {remaining_after_restoring} after one short frame"
    );
}

#[test]
fn impossible_weather_ends_when_the_season_changes() {
    let mut controls = pinned_to(Weather::Snow);
    let mut atmosphere = Atmosphere::new(6);
    run_for(&mut atmosphere, 2.0, FRAME_SECONDS, &WINTER_AFTERNOON, &controls);
    controls.weather = None;
    let environment = run_for(&mut atmosphere, 1.0, FRAME_SECONDS, &WINTER_AFTERNOON, &controls);
    assert_eq!(environment.weather, Weather::Snow, "winter snow should carry on when unpinned");

    controls.season = Some(Season::Summer);
    let mut previous = environment;
    let mut stopped_snowing = false;
    let mut seconds_run = 0.0;
    while seconds_run < 30.0 {
        let environment = atmosphere.step(FRAME_SECONDS, &WINTER_AFTERNOON, &controls);
        assert!((environment.weather_intensity - previous.weather_intensity).abs() < 0.02, "snapped out of the snow");
        if environment.weather != Weather::Snow {
            stopped_snowing = true;
        }
        previous = environment;
        seconds_run += FRAME_SECONDS;
    }
    assert!(stopped_snowing, "still snowing 30 s into a pinned summer");
}

#[test]
fn forced_strikes_do_not_change_the_weather_sequence() {
    let late_afternoon = WallClock { month: 7, day: 15, hour: 16.0 };
    let half_minute_spells = Controls { weather_spell_minutes: 0.5, ..Controls::default() };
    let mut undisturbed = Atmosphere::new(21);
    let mut struck = Atmosphere::new(21);
    let mut weather_changes = 0;
    let mut previous_weather = None;
    for step_index in 0..12_000u32 {
        if step_index % 50 == 25 {
            struck.trigger_lightning_strike();
        }
        let undisturbed_environment = undisturbed.step(0.1, &late_afternoon, &half_minute_spells);
        let struck_environment = struck.step(0.1, &late_afternoon, &half_minute_spells);
        assert_eq!(
            undisturbed_environment.weather, struck_environment.weather,
            "weather diverged at step {step_index}"
        );
        assert_eq!(
            undisturbed_environment.weather_intensity.to_bits(),
            struck_environment.weather_intensity.to_bits(),
            "intensity diverged at step {step_index}"
        );
        if previous_weather != Some(undisturbed_environment.weather) {
            weather_changes += 1;
            previous_weather = Some(undisturbed_environment.weather);
        }
    }
    assert!(weather_changes > 8, "only {weather_changes} weather changes: the comparison proves nothing");
}

#[test]
fn snow_accumulates_then_melts() {
    let mut atmosphere = Atmosphere::new(8);
    let mut controls = pinned_to(Weather::Clear);
    run_for(&mut atmosphere, 2.0, FRAME_SECONDS, &WINTER_AFTERNOON, &controls);
    atmosphere.set_ground_conditions(Some(0.0), Some(0.0));
    controls.weather = Some(Weather::Snow);
    let mut previous_cover = 0.0;
    let mut environment = atmosphere.step(FRAME_SECONDS, &WINTER_AFTERNOON, &controls);
    for minute in 1..=6 {
        environment = run_for(&mut atmosphere, 60.0, 0.1, &WINTER_AFTERNOON, &controls);
        assert!(environment.snow_cover >= previous_cover, "cover shrank while snowing");
        if minute >= 2 {
            assert!(environment.snow_cover > previous_cover, "cover stopped building in minute {minute}");
        }
        previous_cover = environment.snow_cover;
    }
    assert!(environment.snow_cover > 0.75, "only {:.2} after six minutes of snow", environment.snow_cover);
    let deep_cover = environment.snow_cover;

    controls.weather = Some(Weather::Clear);
    let environment = run_for(&mut atmosphere, 300.0, 0.1, &WINTER_AFTERNOON, &controls);
    assert!(environment.snow_cover < deep_cover, "no melt");
    let cover_lost_in_winter = deep_cover - environment.snow_cover;
    assert!(cover_lost_in_winter < 0.6, "winter melt too fast: {deep_cover:.2} -> {:.2}", environment.snow_cover);

    let mut summer_atmosphere = Atmosphere::new(8);
    run_for(&mut summer_atmosphere, 1.0, FRAME_SECONDS, &SUMMER_AFTERNOON, &pinned_to(Weather::Clear));
    summer_atmosphere.set_ground_conditions(Some(deep_cover), None);
    let environment = run_for(&mut summer_atmosphere, 300.0, 0.1, &SUMMER_AFTERNOON, &pinned_to(Weather::Clear));
    let cover_lost_in_summer = deep_cover - environment.snow_cover;
    assert!(cover_lost_in_summer > 2.0 * cover_lost_in_winter || environment.snow_cover == 0.0);
    assert!(environment.snow_cover < 0.05, "July snow still {:.2} after five minutes", environment.snow_cover);

    let mut melting_atmosphere = Atmosphere::new(9);
    run_for(&mut melting_atmosphere, 1.0, FRAME_SECONDS, &SUMMER_AFTERNOON, &pinned_to(Weather::Clear));
    melting_atmosphere.set_ground_conditions(Some(0.8), Some(0.0));
    let environment = run_for(&mut melting_atmosphere, 30.0, 0.1, &SUMMER_AFTERNOON, &pinned_to(Weather::Clear));
    assert!(environment.wetness > 0.15, "melting snow left no melt water: {}", environment.wetness);
}

#[test]
fn rain_wets_quickly_and_dries_slowly() {
    let mut atmosphere = Atmosphere::new(13);
    let mut controls = pinned_to(Weather::Clear);
    let environment = run_for(&mut atmosphere, 2.0, FRAME_SECONDS, &SUMMER_AFTERNOON, &controls);
    assert_eq!(environment.wetness, 0.0);

    controls.weather = Some(Weather::Rain);
    let environment = run_for(&mut atmosphere, 75.0, FRAME_SECONDS, &SUMMER_AFTERNOON, &controls);
    assert!(environment.wetness > 0.8, "wetness {:.2} after about 30 s of full rain", environment.wetness);

    controls.weather = Some(Weather::Clear);
    let environment = run_for(&mut atmosphere, 60.0, FRAME_SECONDS, &SUMMER_AFTERNOON, &controls);
    assert!(environment.wetness > 0.4, "dried in under a minute: {:.2}", environment.wetness);
    let environment = run_for(&mut atmosphere, 420.0, 0.1, &SUMMER_AFTERNOON, &controls);
    assert!(environment.wetness < 0.05, "still wet after eight minutes: {:.2}", environment.wetness);
}

#[test]
fn fog_keeps_ground_damp_over_thin_snow() {
    let winter_night = WallClock { month: 1, day: 15, hour: 1.0 };
    let mut atmosphere = Atmosphere::new(3);
    atmosphere.step(0.0, &winter_night, &pinned_to(Weather::Fog));
    atmosphere.set_ground_conditions(Some(0.019), Some(0.0));
    let environment = run_for(&mut atmosphere, 15.0, FRAME_SECONDS, &winter_night, &pinned_to(Weather::Fog));
    assert!(environment.snow_cover > 0.0, "the thin snow was meant to outlast this test");
    assert!(environment.wetness > 0.15, "fog over a trace of snow left the ground at wetness {}", environment.wetness);
}

#[test]
fn ground_override_before_first_step_is_kept() {
    let mut atmosphere = Atmosphere::new(5);
    atmosphere.set_ground_conditions(Some(0.7), Some(0.3));
    let environment = atmosphere.step(0.0, &SUMMER_AFTERNOON, &pinned_to(Weather::Clear));
    assert_eq!(environment.snow_cover, 0.7);
    assert_eq!(environment.wetness, 0.3);

    let mut atmosphere = Atmosphere::new(5);
    atmosphere.set_ground_conditions(None, Some(0.1));
    let environment = atmosphere.step(0.0, &WINTER_AFTERNOON, &pinned_to(Weather::Snow));
    assert_eq!(environment.snow_cover, 0.4, "an untouched value keeps its natural start");
    assert_eq!(environment.wetness, 0.1);
}

#[test]
fn ground_state_is_frame_rate_independent() {
    let mut results = Vec::new();
    for step_seconds in [0.016f32, 0.05, 0.25, 0.5] {
        let mut atmosphere = Atmosphere::new(77);
        let mut controls = pinned_to(Weather::Clear);
        atmosphere.step(0.0, &WINTER_AFTERNOON, &controls);
        atmosphere.set_ground_conditions(Some(0.0), Some(0.0));
        controls.weather = Some(Weather::Snow);
        let step_count = (200.0 / step_seconds).round() as u32;
        let mut environment = atmosphere.step(0.0, &WINTER_AFTERNOON, &controls);
        for _ in 0..step_count {
            environment = atmosphere.step(step_seconds, &WINTER_AFTERNOON, &controls);
        }
        results.push((environment.snow_cover, environment.weather_intensity, environment.animation_seconds));
    }
    let (reference_cover, reference_intensity, reference_time) = results[0];
    for (snow_cover, intensity, time) in &results[1..] {
        assert!((snow_cover - reference_cover).abs() < 0.01, "snow cover depends on frame rate: {results:?}");
        assert!((intensity - reference_intensity).abs() < 0.01, "intensity depends on frame rate: {results:?}");
        assert!((time - reference_time).abs() < 0.2);
    }
}

#[test]
fn storms_strike_every_few_seconds() {
    let summer_evening = WallClock { month: 7, day: 15, hour: 18.0 };
    let mut atmosphere = Atmosphere::new(31);
    let mut environment = atmosphere.step(0.0, &summer_evening, &pinned_to(Weather::Storm));
    let mut flash_count = 0;
    let mut bright_flash_count = 0;
    let mut dark_frame_count = 0;
    let mut is_flashing = false;
    let mut peak_of_this_flash = 0.0f32;
    let mut frame_of_last_flash: Option<u32> = None;
    let mut longest_gap_seconds = 0.0f32;
    let frame_count = (240.0 / FRAME_SECONDS) as u32;
    for frame_index in 0..frame_count {
        environment = atmosphere.step(FRAME_SECONDS, &summer_evening, &pinned_to(Weather::Storm));
        assert_environment_is_sane(&environment);
        if environment.lightning_flash > 0.0 {
            if !is_flashing {
                flash_count += 1;
                if let Some(last_frame) = frame_of_last_flash {
                    longest_gap_seconds = longest_gap_seconds.max((frame_index - last_frame) as f32 * FRAME_SECONDS);
                }
                frame_of_last_flash = Some(frame_index);
                peak_of_this_flash = 0.0;
            }
            peak_of_this_flash = peak_of_this_flash.max(environment.lightning_flash);
            is_flashing = true;
            continue;
        }
        if is_flashing && peak_of_this_flash > 0.55 {
            bright_flash_count += 1;
        }
        is_flashing = false;
        dark_frame_count += 1;
    }
    assert_eq!(environment.weather, Weather::Storm);
    assert!((14..=70).contains(&flash_count), "{flash_count} flashes in four minutes");
    assert!(bright_flash_count >= flash_count / 3, "{bright_flash_count} of {flash_count} flashes were bright");
    assert!(bright_flash_count < flash_count, "no distant, bolt-less flashes at all");
    assert!(dark_frame_count as f32 > 0.6 * frame_count as f32, "the storm flashes most of the time");
    assert!(longest_gap_seconds < 20.0, "a {longest_gap_seconds}s lull");

    let mut rainy_atmosphere = Atmosphere::new(31);
    for _ in 0..2000 {
        let environment = rainy_atmosphere.step(FRAME_SECONDS, &summer_evening, &pinned_to(Weather::Rain));
        assert_eq!(environment.lightning_flash, 0.0, "lightning outside a storm");
    }
}

#[test]
fn forced_strike_fires_immediately_even_with_zero_time_step() {
    let mut atmosphere = Atmosphere::new(2);
    run_for(&mut atmosphere, 1.0, FRAME_SECONDS, &SUMMER_AFTERNOON, &pinned_to(Weather::Clear));
    atmosphere.trigger_lightning_strike();
    let environment = atmosphere.step(0.0, &SUMMER_AFTERNOON, &pinned_to(Weather::Clear));
    assert!(environment.lightning_flash > 0.55, "flash {}", environment.lightning_flash);

    let mut slow_atmosphere = Atmosphere::new(2);
    run_for(&mut slow_atmosphere, 1.0, FRAME_SECONDS, &SUMMER_AFTERNOON, &pinned_to(Weather::Clear));
    slow_atmosphere.trigger_lightning_strike();
    let environment = slow_atmosphere.step(1.0, &SUMMER_AFTERNOON, &pinned_to(Weather::Clear));
    assert!(environment.lightning_flash > 0.55, "peak lost at one frame per second: {}", environment.lightning_flash);

    let environment = run_for(&mut slow_atmosphere, 5.0, FRAME_SECONDS, &SUMMER_AFTERNOON, &pinned_to(Weather::Clear));
    assert_eq!(environment.lightning_flash, 0.0, "the flash never died away");
}

#[test]
fn each_scene_gets_a_bolt_to_its_own_strike_point() {
    let summer_night = WallClock { month: 7, day: 15, hour: 23.0 };
    let left_scene = SceneInfo { id: "shared name", title: "left", lightning_strike_points: vec![(100, 200)] };
    let right_scene = SceneInfo { id: "shared name", title: "right", lightning_strike_points: vec![(500, 100)] };
    let mut atmosphere = Atmosphere::new(12);
    run_for(&mut atmosphere, 2.0, FRAME_SECONDS, &summer_night, &pinned_to(Weather::Clear));
    atmosphere.trigger_lightning_strike();
    let environment = atmosphere.step(FRAME_SECONDS, &summer_night, &pinned_to(Weather::Clear));
    let mut left_frame = Canvas::full_size();
    let mut right_frame = Canvas::full_size();
    atmosphere.draw_overlay(&environment, &left_scene, &mut left_frame);
    atmosphere.draw_overlay(&environment, &right_scene, &mut right_frame);
    let red_channel = |pixel: u32| (pixel >> 16) & 0xFF;
    assert!(red_channel(left_frame.pixel_at(100, 200)) > 100, "no impact at the left scene's strike point");
    assert!(red_channel(right_frame.pixel_at(500, 100)) > 100, "no impact at the right scene's strike point");
    assert!(red_channel(right_frame.pixel_at(100, 200)) < 100, "the right scene was struck at the left scene's point");
}

#[test]
fn deterministic_for_a_seed() {
    let run_script = |atmosphere: &mut Atmosphere| -> (Vec<Environment>, Canvas) {
        let mut environments = Vec::new();
        let mut frame = Canvas::full_size();
        let scene = SceneInfo { id: "t", title: "t", lightning_strike_points: vec![(320, 100)] };
        let mut controls = Controls { weather_spell_minutes: 0.5, ..Controls::default() };
        let mut clock = WallClock { month: 7, day: 15, hour: 16.0 };
        for step_index in 0..4000u32 {
            if step_index == 1500 {
                controls.weather = Some(Weather::Storm);
            }
            if step_index == 3000 {
                controls.weather = None;
                atmosphere.trigger_lightning_strike();
            }
            clock.hour = 16.0 + step_index as f32 * 0.0005;
            let step_seconds = if step_index % 7 == 0 { 0.11 } else { 0.05 };
            let environment = atmosphere.step(step_seconds, &clock, &controls);
            if step_index % 50 == 0 {
                frame.clear(0x40_6040);
                atmosphere.draw_cloud_shadows(&environment, &mut frame);
                atmosphere.draw_overlay(&environment, &scene, &mut frame);
            }
            environments.push(environment);
        }
        (environments, frame)
    };
    let all_identical = |first: &[Environment], second: &[Environment]| {
        first.iter().zip(second).all(|(one, other)| environments_are_identical(one, other))
    };
    let (first_environments, first_frame) = run_script(&mut Atmosphere::new(42));
    let (second_environments, second_frame) = run_script(&mut Atmosphere::new(42));
    assert!(all_identical(&first_environments, &second_environments), "same seed diverged");
    assert!(first_frame.pixels == second_frame.pixels, "same seed drew different frames");
    let (other_seed_environments, _) = run_script(&mut Atmosphere::new(43));
    assert!(!all_identical(&first_environments, &other_seed_environments), "different seeds were identical");
}

#[test]
fn odd_time_steps_are_safe() {
    let controls = Controls::default();
    let mut atmosphere = Atmosphere::new(6);
    let first = atmosphere.step(0.0, &WINTER_AFTERNOON, &controls);
    assert_environment_is_sane(&first);
    let repeated = atmosphere.step(0.0, &WINTER_AFTERNOON, &controls);
    assert!(environments_are_identical(&first, &repeated), "a zero-length step changed something");

    for odd_step in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, -5.0, 0.0, 1e-9, 3.0, 59.0, 1799.0] {
        let environment = atmosphere.step(odd_step, &WINTER_AFTERNOON, &controls);
        assert_environment_is_sane(&environment);
    }

    let time_before_sleep = atmosphere.step(FRAME_SECONDS, &WINTER_AFTERNOON, &controls).animation_seconds;
    let eight_hours = 8.0 * 3600.0;
    let environment = atmosphere.step(eight_hours, &WINTER_AFTERNOON, &controls);
    assert_environment_is_sane(&environment);
    assert!(environment.animation_seconds >= time_before_sleep + eight_hours as f64 - 1.0);
    assert!(environment.weather_intensity > 0.4, "woke up mid-fade: {}", environment.weather_intensity);
    let thirty_days = 30.0 * 86400.0;
    let environment = atmosphere.step(thirty_days, &WINTER_AFTERNOON, &controls);
    assert_environment_is_sane(&environment);

    let mut frame = Canvas::full_size();
    let scene = SceneInfo::default();
    for _ in 0..200 {
        let environment = atmosphere.step(FRAME_SECONDS, &WINTER_AFTERNOON, &controls);
        assert_environment_is_sane(&environment);
        atmosphere.draw_cloud_shadows(&environment, &mut frame);
        atmosphere.draw_overlay(&environment, &scene, &mut frame);
    }

    let mut atmosphere = Atmosphere::new(6);
    let nonsense_controls = Controls { hour: Some(f32::NAN), weather_spell_minutes: f32::NAN, ..Controls::default() };
    let negative_spells = Controls { weather_spell_minutes: -3.0, ..Controls::default() };
    for _ in 0..50 {
        let nonsense_clock = WallClock { month: 0, day: 99, hour: -40.0 };
        assert_environment_is_sane(&atmosphere.step(FRAME_SECONDS, &nonsense_clock, &nonsense_controls));
        let endless_clock = WallClock { month: 99, day: 0, hour: f32::INFINITY };
        assert_environment_is_sane(&atmosphere.step(FRAME_SECONDS, &endless_clock, &negative_spells));
    }
}

#[test]
fn absurd_time_step_does_not_freeze_the_clock() {
    let controls = Controls::default();
    for absurd_step in [1.0e30f32, f32::MAX, 1.0e12] {
        let mut atmosphere = Atmosphere::new(4);
        atmosphere.step(FRAME_SECONDS, &SUMMER_AFTERNOON, &controls);
        let after_gap = atmosphere.step(absurd_step, &SUMMER_AFTERNOON, &controls);
        assert_environment_is_sane(&after_gap);
        let one_frame_later = atmosphere.step(FRAME_SECONDS, &SUMMER_AFTERNOON, &controls);
        let two_frames_later = atmosphere.step(FRAME_SECONDS, &SUMMER_AFTERNOON, &controls);
        let clock_still_runs = one_frame_later.animation_seconds > after_gap.animation_seconds
            && two_frames_later.animation_seconds > one_frame_later.animation_seconds;
        assert!(
            clock_still_runs,
            "clock frozen at {} after a step of {absurd_step}",
            one_frame_later.animation_seconds
        );
        assert!(
            two_frames_later.animation_seconds - after_gap.animation_seconds < 1.0,
            "clock lost its resolution after {absurd_step}"
        );
    }
}

#[test]
fn drawing_never_panics() {
    let scenes = [
        SceneInfo::default(),
        SceneInfo { id: "x", title: "x", lightning_strike_points: vec![(-50, -50), (10_000, 10_000), (3, 3)] },
    ];
    for (width, height) in [(640, 360), (320, 180), (97, 53), (7, 5), (1, 1)] {
        let mut frame = Canvas::new(width, height);
        for weather in Weather::ALL {
            for (month, hour) in [(1u32, 7.5f32), (4, 12.0), (7, 23.0), (10, 17.0)] {
                for scene in &scenes {
                    let mut atmosphere = Atmosphere::new(9);
                    let clock = WallClock { month, day: 12, hour };
                    for frame_index in 0..12 {
                        if frame_index == 3 {
                            atmosphere.trigger_lightning_strike();
                        }
                        let environment = atmosphere.step(0.07, &clock, &pinned_to(weather));
                        frame.clear(0x55_6655);
                        atmosphere.draw_cloud_shadows(&environment, &mut frame);
                        atmosphere.draw_overlay(&environment, scene, &mut frame);
                    }
                }
            }
        }
    }

    let mut atmosphere = Atmosphere::new(1);
    let mut frame = Canvas::full_size();
    let scene = SceneInfo::default();
    for (weather_index, weather) in Weather::ALL.iter().enumerate() {
        for season in Season::ALL {
            let hand_made_environment = Environment {
                season,
                season_progress: weather_index as f32 / 5.0,
                weather: *weather,
                weather_intensity: 1.0,
                hour: 3.0 * weather_index as f32 + 5.0,
                animation_seconds: 1.0e7 + weather_index as f64 * 977.0,
                wind: 1.0 - 0.4 * weather_index as f32,
                snow_cover: 1.0,
                wetness: 1.0,
                lightning_flash: 1.0,
                seed: u64::MAX,
            };
            atmosphere.draw_cloud_shadows(&hand_made_environment, &mut frame);
            atmosphere.draw_overlay(&hand_made_environment, &scene, &mut frame);
            let time_ran_backwards = Environment { animation_seconds: 5.0, ..hand_made_environment };
            atmosphere.draw_overlay(&time_ran_backwards, &scene, &mut frame);
            atmosphere.draw_overlay(&time_ran_backwards, &scene, &mut frame);
        }
    }
}

#[test]
fn empty_canvas_is_ignored() {
    let scene = SceneInfo { id: "e", title: "e", lightning_strike_points: vec![(3, 3)] };
    let summer_morning = WallClock { month: 7, day: 15, hour: 11.0 };
    for (width, height) in [(0, 0), (0, 5), (5, 0)] {
        let mut frame = Canvas::new(width, height);
        for weather in Weather::ALL {
            let mut atmosphere = Atmosphere::new(2);
            for frame_index in 0..400 {
                if frame_index % 40 == 0 {
                    atmosphere.trigger_lightning_strike();
                }
                let environment = atmosphere.step(0.25, &summer_morning, &pinned_to(weather));
                atmosphere.draw_cloud_shadows(&environment, &mut frame);
                atmosphere.draw_overlay(&environment, &scene, &mut frame);
            }
        }
    }
}

#[test]
fn non_finite_wind_does_not_poison_drift() {
    let scene = SceneInfo::default();
    let background = 0x20_3040u32;
    for broken_wind in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        let mut atmosphere = Atmosphere::new(3);
        let mut frame = Canvas::full_size();
        let broken_environment = Environment {
            weather: Weather::Snow,
            season: Season::Winter,
            wind: broken_wind,
            animation_seconds: 10.0,
            ..Environment::default()
        };
        atmosphere.draw_overlay(&broken_environment, &scene, &mut frame);
        let healthy_environment = Environment { wind: 0.3, animation_seconds: 10.05, ..broken_environment.clone() };
        frame.clear(background);
        atmosphere.draw_overlay(&healthy_environment, &scene, &mut frame);
        let mut columns_with_snow = 0;
        for pixel_x in 0..frame.width {
            let column_has_snow = (0..frame.height).any(|pixel_y| frame.pixel_at(pixel_x, pixel_y) != background);
            if column_has_snow {
                columns_with_snow += 1;
            }
        }
        assert!(columns_with_snow > 200, "after wind = {broken_wind} the snow only covers {columns_with_snow} columns");
    }
}

#[test]
fn clear_weather_adds_nothing_at_night_and_particles_show_in_rain() {
    let scene = SceneInfo::default();
    let background = 0x30_5030u32;
    let mut frame = Canvas::full_size();

    let winter_night = WallClock { month: 1, day: 15, hour: 1.0 };
    let mut atmosphere = Atmosphere::new(4);
    let environment = run_for(&mut atmosphere, 3.0, FRAME_SECONDS, &winter_night, &pinned_to(Weather::Clear));
    frame.clear(background);
    atmosphere.draw_cloud_shadows(&environment, &mut frame);
    assert_eq!(share_of_pixels_changed(&frame, background), 0.0, "cloud shadows at night");
    atmosphere.draw_overlay(&environment, &scene, &mut frame);
    assert_eq!(share_of_pixels_changed(&frame, background), 0.0, "a clear winter night should be empty");

    let summer_afternoon = WallClock { month: 7, day: 15, hour: 15.0 };
    let mut atmosphere = Atmosphere::new(4);
    let environment = run_for(&mut atmosphere, 3.0, FRAME_SECONDS, &summer_afternoon, &pinned_to(Weather::Storm));
    frame.clear(background);
    atmosphere.draw_overlay(&environment, &scene, &mut frame);
    let share_touched_by_storm = share_of_pixels_changed(&frame, background);
    assert!(share_touched_by_storm > 0.01, "storm drew almost nothing ({share_touched_by_storm})");
    assert!(share_touched_by_storm < 0.45, "storm is a grey wash over {share_touched_by_storm} of the frame");

    let winter_noon = WallClock { month: 1, day: 15, hour: 12.0 };
    let mut atmosphere = Atmosphere::new(4);
    let environment = run_for(&mut atmosphere, 3.0, FRAME_SECONDS, &winter_noon, &pinned_to(Weather::Snow));
    frame.clear(background);
    atmosphere.draw_overlay(&environment, &scene, &mut frame);
    let share_touched_by_snow = share_of_pixels_changed(&frame, background);
    assert!(share_touched_by_snow > 0.004 && share_touched_by_snow < 0.4, "snow covers {share_touched_by_snow}");
}

#[test]
fn storm_debris_does_not_recolour_autumn_leaves() {
    let leaves_at_storm_intensity = |intensity: f32| -> Canvas {
        let late_autumn_storm = Environment {
            season: Season::Autumn,
            season_progress: 0.97,
            weather: Weather::Storm,
            weather_intensity: intensity,
            hour: 12.0,
            animation_seconds: 321.0,
            wind: 0.0,
            ..Environment::default()
        };
        let mut effect_state = EffectState::default();
        effect_state.follow_environment(&late_autumn_storm);
        let mut canvas = Canvas::full_size();
        draw_leaves(&effect_state, &late_autumn_storm, &mut canvas);
        canvas
    };
    let red_and_green = |pixel: u32| ((pixel >> 16) & 0xFF, (pixel >> 8) & 0xFF);
    let before_debris = leaves_at_storm_intensity(0.70);
    let with_debris = leaves_at_storm_intensity(1.0);
    let mut brown_pixels_checked = 0;
    for (pixel_index, pixel) in before_debris.pixels.iter().enumerate() {
        let (red, green) = red_and_green(*pixel);
        if red < 60 {
            continue;
        }
        assert!(red > green, "a green leaf before the storm outnumbers the season");
        brown_pixels_checked += 1;
        let (red_later, green_later) = red_and_green(with_debris.pixels[pixel_index]);
        assert!(
            red_later > green_later,
            "pixel {pixel_index} turned from brown ({red},{green}) to green ({red_later},{green_later})"
        );
    }
    assert!(brown_pixels_checked >= 6, "only {brown_pixels_checked} leaf pixels to check");
    let green_debris_pixels = with_debris
        .pixels
        .iter()
        .filter(|pixel| {
            let (red, green) = red_and_green(**pixel);
            green > 60 && green > red
        })
        .count();
    assert!(green_debris_pixels > 0, "the storm added no green debris at all");
}

#[test]
fn rain_landing_just_off_canvas_draws_nothing() {
    let mut canvas = Canvas::new(16, 16);
    let white = 0xFF_FFFF;
    for (landing_x, landing_y) in [(-0.5f32, 8.0f32), (-0.01, 8.0), (8.0, -0.5), (-0.9, -0.9)] {
        draw_rain_landing(&mut canvas, landing_x, landing_y, 0.08, RainLanding::Speck, white, 1.0, 1.0);
        let canvas_is_untouched = canvas.pixels.iter().all(|pixel| *pixel == 0);
        assert!(canvas_is_untouched, "a speck landing at ({landing_x}, {landing_y}) was drawn on the canvas");
    }
    draw_rain_landing(&mut canvas, 0.5, 8.0, 0.08, RainLanding::Speck, white, 1.0, 1.0);
    assert!(canvas.pixel_at(0, 8) != 0, "a speck landing on the canvas was lost");
}

#[test]
fn drawing_soak_after_weeks_of_uptime() {
    let scene = SceneInfo { id: "soak", title: "soak", lightning_strike_points: vec![(160, 12), (40, 120)] };
    let mut frame = Canvas::new(320, 180);
    for (seed, month) in [(1u64, 1u32), (2, 7), (3, 10)] {
        let mut atmosphere = Atmosphere::new(seed);
        let half_minute_spells = Controls { weather_spell_minutes: 0.5, ..Controls::default() };
        let mut clock = WallClock { month, day: 20, hour: 6.0 };
        atmosphere.step(0.0, &clock, &half_minute_spells);
        let months_of_uptime = 1.0e7;
        let environment = atmosphere.step(months_of_uptime, &clock, &half_minute_spells);
        assert_environment_is_sane(&environment);
        let mut weathers_seen = [false; 6];
        let mut previous_time = environment.animation_seconds;
        for step_index in 0..12_000u32 {
            clock.hour = (6.0 + step_index as f32 * 0.0015).rem_euclid(24.0);
            if step_index % 500 == 250 {
                atmosphere.trigger_lightning_strike();
            }
            let environment = atmosphere.step(0.15, &clock, &half_minute_spells);
            assert!(
                environment.animation_seconds > previous_time,
                "the clock stalled at {}",
                environment.animation_seconds
            );
            previous_time = environment.animation_seconds;
            weathers_seen[environment.weather as usize] = true;
            frame.clear(0x48_6A48);
            atmosphere.draw_cloud_shadows(&environment, &mut frame);
            atmosphere.draw_overlay(&environment, &scene, &mut frame);
        }
        let kinds_of_weather_seen = weathers_seen.iter().filter(|seen| **seen).count();
        assert!(kinds_of_weather_seen >= 3, "weather barely changed: {weathers_seen:?}");
    }
}
