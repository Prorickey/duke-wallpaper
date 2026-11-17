mod ambient;
mod cloud_shadows;
mod drawing;
mod fog;
mod lightning;
mod rain;
mod simulation;
mod snow;

#[cfg(test)]
mod tests;

use crate::{apply_lighting, Canvas, Environment, RandomNumberGenerator, Scene, SceneInfo, Season, Weather};
use drawing::{smoothstep, EffectState};
use simulation::{LightningStrike, LIGHTNING_RANDOM_SEED_MIX, WEATHER_RANDOM_SEED_MIX};

const DEFAULT_WEATHER_MINUTES: f32 = 12.0;
const NOON_HOUR: f32 = 12.0;
const SIMULATION_TIME_TOLERANCE_SECONDS: f64 = 1e-6;
const RAIN_AMOUNT_IN_STEADY_RAIN: f32 = 0.68;
const STORM_INTENSITY_WHERE_GUSTS_BEGIN: f32 = 0.15;
const STORM_INTENSITY_WHERE_GUSTS_ARE_FULL: f32 = 0.8;
const SNOWFALL_WHERE_FOREGROUND_FLAKES_BEGIN: f32 = 0.3;
const SNOWFALL_WHERE_FOREGROUND_FLAKES_ARE_FULL: f32 = 0.9;

const RAIN_AMOUNT_AS_A_STORM_BEGINS: f32 = 0.7;
const RAIN_AMOUNT_GAINED_BY_FULL_STORM: f32 = 0.3;

#[derive(Copy, Clone, Debug)]
pub struct WallClock {
    pub month: u32,
    pub day: u32,
    pub hour: f32,
}

#[derive(Clone, Debug)]
pub struct Controls {
    pub season: Option<Season>,
    pub weather: Option<Weather>,
    pub hour: Option<f32>,
    pub weather_spell_minutes: f32,
}

impl Default for Controls {
    fn default() -> Self {
        Controls { season: None, weather: None, hour: None, weather_spell_minutes: DEFAULT_WEATHER_MINUTES }
    }
}

#[derive(Clone, Debug)]
pub struct SimulationState {
    pub weather: Weather,
    pub upcoming_weather: Option<Weather>,
    pub weather_intensity: f32,
    pub spell_seconds_remaining: f32,
    pub wind: f32,
    pub snow_cover: f32,
    pub wetness: f32,
    pub lightning_flash: f32,
    pub seconds_until_next_strike: Option<f32>,
    pub simulation_seconds: f64,
}

pub struct Atmosphere {
    world_seed: u64,
    weather_random: RandomNumberGenerator,
    lightning_random: RandomNumberGenerator,
    simulation_seconds: f64,
    has_started: bool,

    current_weather: Weather,
    transition_progress: f32,
    spell_strength: f32,
    target_spell_strength: f32,
    upcoming_weather: Option<Weather>,
    upcoming_spell_strength: f32,
    spell_seconds_remaining: f64,
    spell_minutes_setting: f32,
    pinned_weather: Option<Weather>,
    next_weather_requested: bool,

    snow_cover: f32,
    wetness: f32,
    snow_cover_requested_before_start: Option<f32>,
    wetness_requested_before_start: Option<f32>,
    displayed_hour: f32,
    wind: f32,
    cloud_deck_overcast: f32,

    lightning_strike: Option<LightningStrike>,
    next_strike_at_seconds: Option<f64>,
    strike_requested: bool,
    flash_measured_up_to_seconds: f64,
    flash: f32,
    bolt_brightness: f32,

    effect_state: EffectState,
}

impl Atmosphere {
    pub fn new(seed: u64) -> Self {
        Atmosphere {
            world_seed: seed,
            weather_random: RandomNumberGenerator::new(seed ^ WEATHER_RANDOM_SEED_MIX),
            lightning_random: RandomNumberGenerator::new(seed ^ LIGHTNING_RANDOM_SEED_MIX),
            simulation_seconds: 0.0,
            has_started: false,
            current_weather: Weather::Clear,
            transition_progress: 1.0,
            spell_strength: 1.0,
            target_spell_strength: 1.0,
            upcoming_weather: None,
            upcoming_spell_strength: 1.0,
            spell_seconds_remaining: 0.0,
            spell_minutes_setting: DEFAULT_WEATHER_MINUTES,
            pinned_weather: None,
            next_weather_requested: false,
            snow_cover: 0.0,
            wetness: 0.0,
            snow_cover_requested_before_start: None,
            wetness_requested_before_start: None,
            displayed_hour: NOON_HOUR,
            wind: 0.0,
            cloud_deck_overcast: 0.0,
            lightning_strike: None,
            next_strike_at_seconds: None,
            strike_requested: false,
            flash_measured_up_to_seconds: 0.0,
            flash: 0.0,
            bolt_brightness: 0.0,
            effect_state: EffectState::default(),
        }
    }

    pub fn draw_overlay(&mut self, environment: &Environment, scene: &SceneInfo, frame: &mut Canvas) {
        if frame.width <= 0 || frame.height <= 0 {
            return;
        }
        self.effect_state.follow_environment(environment);
        let effect_state = &mut self.effect_state;
        let intensity = environment.weather_intensity.clamp(0.0, 1.0);
        let mut rain_amount = 0.0;
        let mut storminess = 0.0;
        let mut snowfall = 0.0;
        if environment.weather == Weather::Rain {
            rain_amount = RAIN_AMOUNT_IN_STEADY_RAIN * intensity;
        }
        if environment.weather == Weather::Storm {
            rain_amount = intensity * (RAIN_AMOUNT_AS_A_STORM_BEGINS + RAIN_AMOUNT_GAINED_BY_FULL_STORM * intensity);
            storminess = smoothstep(STORM_INTENSITY_WHERE_GUSTS_BEGIN, STORM_INTENSITY_WHERE_GUSTS_ARE_FULL, intensity);
        }
        if environment.weather == Weather::Snow {
            snowfall = intensity;
        }
        let is_raining = rain_amount > 0.0;
        let is_snowing = snowfall > 0.0;

        ambient::draw_bird_shadows(environment, frame);

        if is_raining {
            rain::draw_rain_layer(effect_state, environment, frame, &rain::FAR_RAIN, rain_amount, storminess);
            rain::draw_rain_layer(effect_state, environment, frame, &rain::MIDDLE_RAIN, rain_amount, storminess);
        }
        if is_snowing {
            snow::draw_snow_layer(effect_state, environment, frame, &snow::FAR_SNOW, snowfall);
        }

        fog::draw_fog(effect_state, environment, frame);
        if storminess > 0.0 {
            rain::draw_storm_sheets(effect_state, environment, frame, rain_amount, storminess);
        }

        if let Some(strike) = &self.lightning_strike {
            let strike_age = (environment.animation_seconds - strike.started_at_seconds) as f32;
            let came_from_this_simulation =
                (environment.animation_seconds - self.simulation_seconds).abs() < SIMULATION_TIME_TOLERANCE_SECONDS;
            if came_from_this_simulation {
                lightning::draw_lightning_bolt(effect_state, strike, strike_age, self.bolt_brightness, scene, frame);
            }
        }

        if is_raining {
            rain::draw_rain_layer(effect_state, environment, frame, &rain::NEAR_RAIN, rain_amount, storminess);
        }
        if is_snowing {
            snow::draw_snow_layer(effect_state, environment, frame, &snow::MIDDLE_SNOW, snowfall);
            snow::draw_snow_layer(effect_state, environment, frame, &snow::NEAR_SNOW, snowfall);
        }

        ambient::draw_leaves(effect_state, environment, frame);
        ambient::draw_spring_petals_and_pollen(effect_state, environment, frame);
        ambient::draw_fireflies(environment, frame);
        ambient::draw_birds(environment, frame);

        if is_snowing {
            let foreground_snowfall =
                smoothstep(SNOWFALL_WHERE_FOREGROUND_FLAKES_BEGIN, SNOWFALL_WHERE_FOREGROUND_FLAKES_ARE_FULL, snowfall);
            snow::draw_snow_layer(effect_state, environment, frame, &snow::FOREGROUND_SNOW, foreground_snowfall);
        }
    }

    pub fn simulation_state(&self) -> SimulationState {
        let seconds_until_next_strike =
            self.next_strike_at_seconds.map(|strike_at| (strike_at - self.simulation_seconds).max(0.0) as f32);
        SimulationState {
            weather: self.current_weather,
            upcoming_weather: self.upcoming_weather,
            weather_intensity: self.intensity(),
            spell_seconds_remaining: self.spell_seconds_remaining.max(0.0) as f32,
            wind: self.wind,
            snow_cover: self.snow_cover,
            wetness: self.wetness,
            lightning_flash: self.flash,
            seconds_until_next_strike,
            simulation_seconds: self.simulation_seconds,
        }
    }

    pub fn status_summary(&self) -> String {
        let state = self.simulation_state();
        let mut description = format!("{} {:.2}", state.weather.name(), state.weather_intensity);
        if let Some(upcoming_weather) = state.upcoming_weather {
            description.push_str(&format!(" -> {}", upcoming_weather.name()));
        }
        description.push_str(&format!(
            " | spell {:.0}s | wind {:+.2} | snow {:.2} wet {:.2}",
            state.spell_seconds_remaining, state.wind, state.snow_cover, state.wetness
        ));
        if let Some(seconds_until_next_strike) = state.seconds_until_next_strike {
            description.push_str(&format!(" | strike in {seconds_until_next_strike:.1}s"));
        }
        if state.lightning_flash > 0.0 {
            description.push_str(&format!(" | flash {:.2}", state.lightning_flash));
        }
        description
    }
}

pub fn compose_frame(
    scene: &mut dyn Scene,
    atmosphere: &mut Atmosphere,
    environment: &Environment,
    frame: &mut Canvas,
) {
    scene.render(environment, frame);
    atmosphere.draw_cloud_shadows(environment, frame);
    apply_lighting(frame, &environment.lighting());
    scene.render_lights(environment, frame);
    let scene_info = scene.info();
    atmosphere.draw_overlay(environment, &scene_info, frame);
}
