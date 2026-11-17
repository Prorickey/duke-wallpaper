use crate::atmosphere::drawing::{smoothstep, time_noise, triangle_bump};
use crate::atmosphere::{Atmosphere, Controls, WallClock};
use crate::{interpolate, Environment, RandomNumberGenerator, Season, Weather};

const LONGEST_SIMULATION_SLICE_SECONDS: f32 = 0.5;
const LONG_ABSENCE_SECONDS: f32 = 1800.0;
const LONGEST_CLOCK_ADVANCE_SECONDS: f32 = 30.0 * 86_400.0;
const HOURS_PER_DAY: f32 = 24.0;
const NOON_HOUR: f32 = 12.0;
const DAYS_PER_YEAR: f32 = 365.0;
const LAST_DAY_INDEX_OF_YEAR: u32 = 364;
const DAYS_BEFORE_EACH_MONTH: [u32; 12] = [0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334];
const SPRING_FIRST_DAY_INDEX: f32 = 59.0;
const SUMMER_FIRST_DAY_INDEX: f32 = 151.0;
const AUTUMN_FIRST_DAY_INDEX: f32 = 243.0;
const WINTER_FIRST_DAY_INDEX: f32 = 334.0;
const SPRING_DAYS: f32 = 92.0;
const SUMMER_DAYS: f32 = 92.0;
const AUTUMN_DAYS: f32 = 91.0;
const WINTER_DAYS: f32 = 90.0;
const MID_SEASON_PROGRESS_BY_SEASON: [f32; 4] = [0.3, 0.5, 0.55, 0.5];

const WEATHER_KIND_COUNT: usize = 6;
const CLEAR: usize = 0;
const CLOUDY: usize = 1;
const RAIN: usize = 2;
const STORM: usize = 3;
const SNOW: usize = 4;
const FOG: usize = 5;
const ODDS_OF_FOLLOWING_WEATHER: [[f32; WEATHER_KIND_COUNT]; WEATHER_KIND_COUNT] = [
    [1.00, 1.60, 0.50, 0.60, 0.50, 1.00],
    [1.20, 0.60, 1.80, 1.30, 1.80, 0.80],
    [0.80, 1.50, 0.50, 1.80, 0.60, 0.90],
    [0.80, 1.20, 2.50, 0.25, 0.20, 0.30],
    [1.00, 1.60, 0.30, 0.05, 0.70, 0.60],
    [1.50, 1.30, 0.50, 0.20, 0.50, 0.40],
];
const FADE_OUT_SECONDS_BY_WEATHER: [f32; WEATHER_KIND_COUNT] = [6.0, 16.0, 18.0, 20.0, 22.0, 28.0];
const FADE_IN_SECONDS_BY_WEATHER: [f32; WEATHER_KIND_COUNT] = [6.0, 22.0, 24.0, 26.0, 26.0, 40.0];
const SPELL_LENGTH_SCALE_BY_WEATHER: [f32; WEATHER_KIND_COUNT] = [1.25, 0.95, 0.9, 0.5, 1.0, 0.65];
const WEAKEST_SPELL_STRENGTH_BY_WEATHER: [f32; WEATHER_KIND_COUNT] = [1.0, 0.6, 0.5, 0.8, 0.45, 0.55];
const GUSTINESS_BY_WEATHER: [f32; WEATHER_KIND_COUNT] = [0.08, 0.2, 0.4, 1.0, 0.22, 0.0];

const DEFAULT_SPELL_MINUTES: f32 = 12.0;
const SHORTEST_SPELL_MINUTES: f32 = 0.25;
const LONGEST_SPELL_MINUTES: f32 = 720.0;
const SHORTEST_SPELL_SECONDS: f32 = 30.0;
const SPELL_MINUTES_FOR_FULL_LENGTH_FADES: f32 = 2.0;
const FASTEST_FADE_SCALE: f32 = 0.35;
const SECONDS_FOR_SPELL_STRENGTH_TO_CHANGE_FULLY: f32 = 45.0;
const CLOUD_DECK_CHANGE_PER_SECOND: f32 = 0.022;

const HOUR_DIFFERENCE_THAT_SNAPS: f32 = 0.05;
const HOUR_GLIDE_EASING_PER_SECOND: f32 = 2.5;
const SLOWEST_HOUR_GLIDE_HOURS_PER_SECOND: f32 = 3.0;

const SNOW_COVER_AT_PINNED_SNOWY_START: f32 = 0.4;
const WETNESS_AT_RAINY_START: f32 = 0.85;
const WETNESS_AT_FOGGY_START: f32 = 0.2;
const CHANCE_OF_OLD_SNOW_IN_MID_WINTER: f32 = 0.3;
const SNOW_COVER_SNOWFALL_APPROACHES: f32 = 1.15;
const SNOWFALL_SECONDS_SCALE: f32 = 170.0;
const WINTER_MELT_PER_SECOND: f32 = 1.0 / 1500.0;
const SPRING_AND_AUTUMN_MELT_PER_SECOND: f32 = 1.0 / 300.0;
const SUMMER_MELT_PER_SECOND: f32 = 1.0 / 90.0;
const SECONDS_OF_STORM_TO_SOAK_THE_GROUND: f32 = 16.0;
const SECONDS_OF_RAIN_TO_SOAK_THE_GROUND: f32 = 26.0;
const BASE_DRYING_PER_SECOND: f32 = 1.0 / 240.0;
const WETNESS_UNDER_MELTING_SNOW: f32 = 0.35;
const WETNESS_UNDER_FULL_FOG: f32 = 0.22;
const SECONDS_TO_DAMPEN_FULLY: f32 = 45.0;
const SNOWFALL_INTENSITY_THAT_STOPS_MELT_WATER: f32 = 0.3;

const STORM_INTENSITY_NEEDED_FOR_LIGHTNING: f32 = 0.4;
const CHANCE_OF_A_QUICK_SECOND_STRIKE: f32 = 0.15;
const CHANCE_OF_A_DISTANT_FLASH: f32 = 0.3;
const AFTERGLOW_BRIGHTNESS: f32 = 0.14;
const SECONDS_A_STRIKE_LINGERS_AFTER_ITS_LAST_PULSE: f32 = 2.2;
const BRANCH_SECONDS_FOR_A_SINGLE_STROKE: f32 = 0.14;
const FAINTEST_VISIBLE_FLASH: f32 = 0.004;
const AGE_MARGIN_FOR_PULSES_ON_THE_BOUNDARY: f32 = 1e-4;

const FOG_PEAK_HOURS_AFTER_SUNRISE: f32 = 0.2;
const FOG_HALF_WIDTH_HOURS: f32 = 2.2;
const AFTERNOON_BEGINS_HOUR: f32 = 12.5;
const AFTERNOON_IN_FULL_SWING_HOUR: f32 = 15.5;
const EVENING_FADES_FROM_HOUR: f32 = 21.0;
const EVENING_OVER_HOUR: f32 = 23.5;
const FEWEST_RETURN_STROKES: i32 = 2;
const MOST_RETURN_STROKES: i32 = 4;
const FEWEST_DISTANT_FLICKERS: i32 = 2;
const MOST_DISTANT_FLICKERS: i32 = 4;
const FIRST_STROKE_BRIGHTNESS_RANGE: (f32, f32) = (0.62, 0.82);
const LATER_STROKE_SHARE_OF_FIRST_RANGE: (f32, f32) = (0.45, 0.9);
const STROKE_DECAY_SECONDS_RANGE: (f32, f32) = (0.045, 0.085);
const SECONDS_BETWEEN_STROKES_RANGE: (f32, f32) = (0.08, 0.17);
const AFTERGLOW_DECAY_SECONDS_RANGE: (f32, f32) = (0.3, 0.5);
const DISTANT_FLICKER_BRIGHTNESS_RANGE: (f32, f32) = (0.12, 0.3);
const SECONDS_BETWEEN_DISTANT_FLICKERS_RANGE: (f32, f32) = (0.12, 0.3);
const DISTANT_FLICKER_RISE_SECONDS_RANGE: (f32, f32) = (0.03, 0.06);
const DISTANT_FLICKER_DECAY_SECONDS_RANGE: (f32, f32) = (0.1, 0.22);
const SECONDS_PER_MINUTE: f32 = 60.0;
const SPELL_LENGTH_VARIATION_RANGE: (f32, f32) = (0.65, 1.4);
const SHARE_OF_FIRST_SPELL_LEFT_RANGE: (f32, f32) = (0.35, 1.0);
const SNOW_COVER_AT_AUTOMATIC_SNOWY_START_RANGE: (f32, f32) = (0.25, 0.5);
const MID_WINTER_BEGINS_AT_PROGRESS: f32 = 0.1;
const MID_WINTER_ENDS_AT_PROGRESS: f32 = 0.9;
const OLD_SNOW_COVER_RANGE: (f32, f32) = (0.15, 0.45);
const SHARE_OF_SPELL_LEFT_AFTER_UNPINNING_RANGE: (f32, f32) = (0.15, 0.5);
const SECONDS_UNTIL_FIRST_STRIKE_RANGE: (f32, f32) = (1.5, 5.0);
const QUICK_SECOND_STRIKE_SECONDS_RANGE: (f32, f32) = (1.2, 2.6);
const SHORTEST_STRIKE_GAP_SECONDS: f32 = 4.0;
const STRIKE_GAP_SPREAD_SECONDS: f32 = 11.0;
const STRIKE_RATE_IN_A_WEAK_STORM: f32 = 0.55;
const STRIKE_RATE_GAINED_IN_A_FULL_STORM: f32 = 0.45;
const EXTRA_MELT_IN_FULL_SUN: f32 = 1.5;
const EXTRA_MELT_IN_FULL_RAIN: f32 = 2.5;
const SUMMER_DRYING_SCALE: f32 = 1.25;
const WINTER_DRYING_SCALE: f32 = 0.75;
const DRYING_LOST_IN_DAMP_AIR: f32 = 0.7;
const DRYING_SCALE_WITHOUT_SUN: f32 = 0.6;
const DRYING_GAINED_IN_FULL_SUN: f32 = 0.9;
const EXTRA_DRYING_IN_FULL_WIND: f32 = 0.5;
const SNOW_COVER_WHERE_MELT_WATER_BEGINS: f32 = 0.02;
const SNOW_COVER_WHERE_MELT_WATER_IS_FULL: f32 = 0.2;
const GUSTINESS_BETWEEN_WEATHERS: f32 = 0.1;
const WIND_LOST_IN_FULL_FOG: f32 = 0.55;
const PREVAILING_DIRECTION_NOISE_SECONDS: f64 = 1500.0;
const PREVAILING_DIRECTION_SEED_MIX: u32 = 0x51;
const PREVAILING_DIRECTION_SHARPNESS: f32 = 9.0;
const CALMEST_WIND_STRENGTH: f32 = 0.08;
const CALM_WIND_STRENGTH_RANGE: f32 = 0.42;
const CALM_WIND_NOISE_SECONDS: f64 = 47.0;
const CALM_WIND_SEED_MIX: u32 = 0x52;
const WEAKEST_STORM_WIND_STRENGTH: f32 = 0.6;
const STORM_WIND_STRENGTH_RANGE: f32 = 0.22;
const STORM_WIND_NOISE_SECONDS: f64 = 19.0;
const STORM_WIND_SEED_MIX: u32 = 0x53;
const SLOW_GUST_NOISE_SECONDS: f64 = 7.3;
const SLOW_GUST_SEED_MIX: u32 = 0x54;
const QUICK_GUST_NOISE_SECONDS: f64 = 2.1;
const QUICK_GUST_SEED_MIX: u32 = 0x55;
const SLOW_GUST_STRENGTH_WHEN_CALM: f32 = 0.06;
const SLOW_GUST_STRENGTH_GAINED_IN_STORM: f32 = 0.2;
const QUICK_GUST_STRENGTH_WHEN_CALM: f32 = 0.02;
const QUICK_GUST_STRENGTH_GAINED_IN_STORM: f32 = 0.1;

pub(crate) const WEATHER_RANDOM_SEED_MIX: u64 = 0xA7_05F3;
pub(crate) const LIGHTNING_RANDOM_SEED_MIX: u64 = 0x11_6477;

pub(crate) fn season_and_progress(month: u32, day: u32, hour: f32) -> (Season, f32) {
    let month = month.clamp(1, 12);
    let day = day.clamp(1, 31);
    let fraction_of_day = if hour.is_finite() { (hour / HOURS_PER_DAY).clamp(0.0, 1.0) } else { 0.5 };
    let whole_days_into_year = (DAYS_BEFORE_EACH_MONTH[(month - 1) as usize] + day - 1).min(LAST_DAY_INDEX_OF_YEAR);
    let days_into_year = whole_days_into_year as f32 + fraction_of_day;
    let (season, first_day_index, season_days) = match month {
        3..=5 => (Season::Spring, SPRING_FIRST_DAY_INDEX, SPRING_DAYS),
        6..=8 => (Season::Summer, SUMMER_FIRST_DAY_INDEX, SUMMER_DAYS),
        9..=11 => (Season::Autumn, AUTUMN_FIRST_DAY_INDEX, AUTUMN_DAYS),
        _ => (Season::Winter, WINTER_FIRST_DAY_INDEX, WINTER_DAYS),
    };
    let days_into_season = (days_into_year - first_day_index).rem_euclid(DAYS_PER_YEAR);
    let season_progress = (days_into_season / season_days).clamp(0.0, 1.0);
    (season, season_progress)
}

pub(crate) fn weather_weights(
    season: Season,
    season_progress: f32,
    hour: f32,
    weather_before: Option<Weather>,
) -> [f32; WEATHER_KIND_COUNT] {
    let (sunrise_hour, _) = season.sunrise_and_sunset_hours();
    let hour = hour.rem_euclid(HOURS_PER_DAY);
    let nearness_to_dawn = triangle_bump(hour, sunrise_hour + FOG_PEAK_HOURS_AFTER_SUNRISE, FOG_HALF_WIDTH_HOURS);
    let afternoon_and_evening = smoothstep(AFTERNOON_BEGINS_HOUR, AFTERNOON_IN_FULL_SWING_HOUR, hour)
        * (1.0 - smoothstep(EVENING_FADES_FROM_HOUR, EVENING_OVER_HOUR, hour));
    let mut weights = [0.0; WEATHER_KIND_COUNT];
    match season {
        Season::Spring => {
            weights[CLEAR] = 42.0;
            weights[CLOUDY] = 22.0;
            weights[RAIN] = 20.0;
            weights[STORM] = 5.0 * (0.35 + 1.7 * afternoon_and_evening);
            weights[SNOW] = if season_progress < 0.1 { 2.0 * (1.0 - season_progress / 0.1) } else { 0.0 };
            weights[FOG] = 2.5 * (0.3 + 3.5 * nearness_to_dawn);
        }
        Season::Summer => {
            weights[CLEAR] = 46.0;
            weights[CLOUDY] = 17.0;
            weights[RAIN] = 11.0;
            weights[STORM] = 14.0 * (0.2 + 2.0 * afternoon_and_evening);
            weights[SNOW] = 0.0;
            weights[FOG] = 4.0 * nearness_to_dawn;
        }
        Season::Autumn => {
            weights[CLEAR] = 46.0;
            weights[CLOUDY] = 22.0;
            weights[RAIN] = 15.0;
            weights[STORM] = 2.5 * (0.35 + 1.7 * afternoon_and_evening);
            weights[SNOW] = if season_progress > 0.92 { 1.5 * (season_progress - 0.92) / 0.08 } else { 0.0 };
            weights[FOG] = 7.0 * (0.25 + 3.5 * nearness_to_dawn);
        }
        Season::Winter => {
            weights[CLEAR] = 38.0;
            weights[CLOUDY] = 24.0;
            weights[RAIN] = 12.0;
            weights[STORM] = 0.6;
            weights[SNOW] = 15.0 * (0.55 + 0.45 * triangle_bump(season_progress, 0.5, 0.6));
            weights[FOG] = 6.0 * (0.3 + 3.0 * nearness_to_dawn);
        }
    }
    if let Some(weather_before) = weather_before {
        let follow_on_odds = ODDS_OF_FOLLOWING_WEATHER[weather_before as usize];
        for (weight, odds_of_following) in weights.iter_mut().zip(follow_on_odds) {
            *weight *= odds_of_following;
        }
    }
    weights
}

fn pick_weather(random: &mut RandomNumberGenerator, weights: &[f32; WEATHER_KIND_COUNT]) -> Weather {
    let mut total_weight = 0.0;
    for weight in weights {
        total_weight += weight;
    }
    if total_weight <= 0.0 {
        return Weather::Clear;
    }
    let mut remaining_roll = random.next_fraction() * total_weight;
    for (weather, weight) in Weather::ALL.iter().zip(weights) {
        if remaining_roll < *weight {
            return *weather;
        }
        remaining_roll -= weight;
    }
    Weather::Clear
}

fn sane_spell_minutes(requested_minutes: f32) -> f32 {
    if requested_minutes.is_finite() && requested_minutes > 0.0 {
        requested_minutes.clamp(SHORTEST_SPELL_MINUTES, LONGEST_SPELL_MINUTES)
    } else {
        DEFAULT_SPELL_MINUTES
    }
}

fn wrap_hour(hour: f32) -> f32 {
    let wrapped_hour = hour.rem_euclid(HOURS_PER_DAY);
    if wrapped_hour < HOURS_PER_DAY {
        wrapped_hour
    } else {
        0.0
    }
}

struct LightningPulse {
    starts_at_age: f32,
    peak_brightness: f32,
    rise_seconds: f32,
    decay_seconds: f32,
}

impl LightningPulse {
    fn brightness_at(&self, strike_age: f32) -> f32 {
        let pulse_age = strike_age - self.starts_at_age;
        if pulse_age < 0.0 {
            0.0
        } else if pulse_age < self.rise_seconds {
            self.peak_brightness * pulse_age / self.rise_seconds
        } else {
            self.peak_brightness * (-(pulse_age - self.rise_seconds) / self.decay_seconds).exp()
        }
    }
}

pub(crate) struct LightningStrike {
    pub(crate) started_at_seconds: f64,
    pub(crate) shape_seed: u32,
    pub(crate) has_bolt: bool,
    pulses: Vec<LightningPulse>,
    return_stroke_count: usize,
    pub(crate) branches_visible_until_age: f32,
    pub(crate) last_return_stroke_age: f32,
    visible_seconds: f32,
}

impl LightningStrike {
    fn new(started_at_seconds: f64, random: &mut RandomNumberGenerator, must_have_bolt: bool) -> LightningStrike {
        let shape_seed = random.next_u32();
        let has_bolt = must_have_bolt || !random.succeeds_with_probability(CHANCE_OF_A_DISTANT_FLASH);
        let mut pulses = Vec::with_capacity(6);
        let mut return_stroke_count = 0;
        if has_bolt {
            return_stroke_count = random.integer_between(FEWEST_RETURN_STROKES, MOST_RETURN_STROKES) as usize;
            let first_stroke_brightness =
                random.float_between(FIRST_STROKE_BRIGHTNESS_RANGE.0, FIRST_STROKE_BRIGHTNESS_RANGE.1);
            let mut starts_at_age = 0.0;
            for stroke_index in 0..return_stroke_count {
                let peak_brightness = if stroke_index == 0 {
                    first_stroke_brightness
                } else {
                    first_stroke_brightness
                        * random.float_between(LATER_STROKE_SHARE_OF_FIRST_RANGE.0, LATER_STROKE_SHARE_OF_FIRST_RANGE.1)
                };
                let decay_seconds = random.float_between(STROKE_DECAY_SECONDS_RANGE.0, STROKE_DECAY_SECONDS_RANGE.1);
                pulses.push(LightningPulse { starts_at_age, peak_brightness, rise_seconds: 0.0, decay_seconds });
                starts_at_age += random.float_between(SECONDS_BETWEEN_STROKES_RANGE.0, SECONDS_BETWEEN_STROKES_RANGE.1);
            }
            let afterglow_decay_seconds =
                random.float_between(AFTERGLOW_DECAY_SECONDS_RANGE.0, AFTERGLOW_DECAY_SECONDS_RANGE.1);
            pulses.push(LightningPulse {
                starts_at_age: 0.0,
                peak_brightness: AFTERGLOW_BRIGHTNESS,
                rise_seconds: 0.0,
                decay_seconds: afterglow_decay_seconds,
            });
        } else {
            let flicker_count = random.integer_between(FEWEST_DISTANT_FLICKERS, MOST_DISTANT_FLICKERS);
            let mut starts_at_age = 0.0;
            for _ in 0..flicker_count {
                let peak_brightness =
                    random.float_between(DISTANT_FLICKER_BRIGHTNESS_RANGE.0, DISTANT_FLICKER_BRIGHTNESS_RANGE.1);
                let rise_seconds =
                    random.float_between(DISTANT_FLICKER_RISE_SECONDS_RANGE.0, DISTANT_FLICKER_RISE_SECONDS_RANGE.1);
                let decay_seconds =
                    random.float_between(DISTANT_FLICKER_DECAY_SECONDS_RANGE.0, DISTANT_FLICKER_DECAY_SECONDS_RANGE.1);
                pulses.push(LightningPulse { starts_at_age, peak_brightness, rise_seconds, decay_seconds });
                starts_at_age += random
                    .float_between(SECONDS_BETWEEN_DISTANT_FLICKERS_RANGE.0, SECONDS_BETWEEN_DISTANT_FLICKERS_RANGE.1);
            }
        }
        let branches_visible_until_age =
            if return_stroke_count >= 2 { pulses[1].starts_at_age } else { BRANCH_SECONDS_FOR_A_SINGLE_STROKE };
        let last_return_stroke_age =
            if return_stroke_count >= 1 { pulses[return_stroke_count - 1].starts_at_age } else { 0.0 };
        let mut age_of_last_peak = 0.0f32;
        for pulse in &pulses {
            age_of_last_peak = age_of_last_peak.max(pulse.starts_at_age + pulse.rise_seconds);
        }
        let visible_seconds = age_of_last_peak + SECONDS_A_STRIKE_LINGERS_AFTER_ITS_LAST_PULSE;
        LightningStrike {
            started_at_seconds,
            shape_seed,
            has_bolt,
            pulses,
            return_stroke_count,
            branches_visible_until_age,
            last_return_stroke_age,
            visible_seconds,
        }
    }

    fn flash_at(&self, strike_age: f32) -> f32 {
        if strike_age < 0.0 {
            return 0.0;
        }
        let mut darkness_left = 1.0;
        for pulse in &self.pulses {
            darkness_left *= 1.0 - pulse.brightness_at(strike_age).min(1.0);
        }
        1.0 - darkness_left
    }

    fn brightest_flash_between(&self, age_at_start: f32, age_at_end: f32) -> f32 {
        let mut brightest = self.flash_at(age_at_end);
        for pulse in &self.pulses {
            let age_at_peak = pulse.starts_at_age + pulse.rise_seconds;
            if age_at_peak > age_at_start && age_at_peak <= age_at_end {
                brightest = brightest.max(self.flash_at(age_at_peak));
            }
        }
        brightest
    }

    fn brightest_bolt_between(&self, age_at_start: f32, age_at_end: f32) -> f32 {
        let return_strokes = &self.pulses[..self.return_stroke_count];
        let mut brightest = 0.0f32;
        if age_at_end >= 0.0 {
            for stroke in return_strokes {
                brightest = brightest.max(stroke.brightness_at(age_at_end));
            }
            brightest = brightest.min(1.0);
        }
        for stroke in return_strokes {
            if stroke.starts_at_age > age_at_start && stroke.starts_at_age <= age_at_end {
                brightest = brightest.max(stroke.peak_brightness.min(1.0));
            }
        }
        brightest
    }
}

impl Atmosphere {
    pub fn step(&mut self, seconds_elapsed: f32, clock: &WallClock, controls: &Controls) -> Environment {
        let seconds_elapsed =
            if seconds_elapsed.is_finite() { seconds_elapsed.clamp(0.0, LONGEST_CLOCK_ADVANCE_SECONDS) } else { 0.0 };

        let (calendar_season, calendar_progress) = season_and_progress(clock.month, clock.day, clock.hour);
        let (season, season_progress) = match controls.season {
            Some(pinned_season) if pinned_season != calendar_season => {
                (pinned_season, MID_SEASON_PROGRESS_BY_SEASON[pinned_season as usize])
            }
            Some(pinned_season) => (pinned_season, calendar_progress),
            None => (calendar_season, calendar_progress),
        };
        let requested_hour = controls.hour.unwrap_or(clock.hour);
        let target_hour = if requested_hour.is_finite() { wrap_hour(requested_hour) } else { NOON_HOUR };

        let seconds_before_this_step = self.simulation_seconds;
        if !self.has_started {
            self.begin_weather(season, season_progress, target_hour, controls);
            self.simulation_seconds += seconds_elapsed as f64;
            self.flash_measured_up_to_seconds = seconds_before_this_step;
        } else if seconds_elapsed > LONG_ABSENCE_SECONDS {
            self.simulation_seconds += seconds_elapsed as f64;
            self.begin_weather(season, season_progress, target_hour, controls);
        } else {
            let mut hours_to_target = (target_hour - self.displayed_hour).rem_euclid(HOURS_PER_DAY);
            if hours_to_target > HOURS_PER_DAY / 2.0 {
                hours_to_target -= HOURS_PER_DAY;
            }
            if hours_to_target.abs() < HOUR_DIFFERENCE_THAT_SNAPS {
                self.displayed_hour = target_hour;
            } else {
                let eased_hours = hours_to_target * (1.0 - (-seconds_elapsed * HOUR_GLIDE_EASING_PER_SECOND).exp());
                let slowest_hours = (SLOWEST_HOUR_GLIDE_HOURS_PER_SECOND * seconds_elapsed).min(hours_to_target.abs());
                let hours_moved = eased_hours.abs().max(slowest_hours) * hours_to_target.signum();
                self.displayed_hour = wrap_hour(self.displayed_hour + hours_moved);
            }
            let mut seconds_left = seconds_elapsed;
            while seconds_left > 0.0 {
                let slice_seconds = seconds_left.min(LONGEST_SIMULATION_SLICE_SECONDS);
                self.advance_simulation(slice_seconds, season, season_progress, controls);
                seconds_left -= slice_seconds;
            }
        }

        if self.strike_requested {
            self.strike_requested = false;
            let strike_time = seconds_before_this_step.max(self.flash_measured_up_to_seconds);
            self.lightning_strike = Some(LightningStrike::new(strike_time, &mut self.lightning_random, true));
        }

        self.wind = self.wind_at(self.simulation_seconds);

        self.flash = 0.0;
        self.bolt_brightness = 0.0;
        if let Some(strike) = &self.lightning_strike {
            let age_now = (self.simulation_seconds - strike.started_at_seconds) as f32;
            let age_at_last_measurement = (self.flash_measured_up_to_seconds - strike.started_at_seconds) as f32
                - AGE_MARGIN_FOR_PULSES_ON_THE_BOUNDARY;
            if age_now > strike.visible_seconds {
                self.lightning_strike = None;
            } else {
                self.flash = strike.brightest_flash_between(age_at_last_measurement, age_now).clamp(0.0, 1.0);
                self.bolt_brightness = strike.brightest_bolt_between(age_at_last_measurement, age_now);
                if self.flash < FAINTEST_VISIBLE_FLASH {
                    self.flash = 0.0;
                }
            }
        }
        self.flash_measured_up_to_seconds = self.simulation_seconds;

        Environment {
            season,
            season_progress,
            weather: self.current_weather,
            weather_intensity: self.intensity(),
            hour: self.displayed_hour,
            animation_seconds: self.simulation_seconds,
            wind: self.wind,
            snow_cover: self.snow_cover,
            wetness: self.wetness,
            lightning_flash: self.flash,
            seed: self.world_seed,
        }
    }

    pub fn trigger_lightning_strike(&mut self) {
        self.strike_requested = true;
    }

    pub fn skip_to_next_weather(&mut self) {
        self.next_weather_requested = true;
    }

    pub fn finish_weather_transition(&mut self) {
        if let Some(upcoming_weather) = self.upcoming_weather.take() {
            self.current_weather = upcoming_weather;
            self.spell_strength = self.upcoming_spell_strength;
            self.target_spell_strength = self.upcoming_spell_strength;
            self.next_strike_at_seconds = None;
        }
        self.transition_progress = 1.0;
        self.spell_strength = self.target_spell_strength;
        self.cloud_deck_overcast = self.current_weather.overcast_at_full_intensity() * self.spell_strength;
    }

    pub fn set_ground_conditions(&mut self, snow_cover: Option<f32>, wetness: Option<f32>) {
        if let Some(requested_snow_cover) = snow_cover {
            if requested_snow_cover.is_finite() {
                self.snow_cover = requested_snow_cover.clamp(0.0, 1.0);
                if !self.has_started {
                    self.snow_cover_requested_before_start = Some(self.snow_cover);
                }
            }
        }
        if let Some(requested_wetness) = wetness {
            if requested_wetness.is_finite() {
                self.wetness = requested_wetness.clamp(0.0, 1.0);
                if !self.has_started {
                    self.wetness_requested_before_start = Some(self.wetness);
                }
            }
        }
    }

    pub(crate) fn intensity(&self) -> f32 {
        let eased_progress = smoothstep(0.0, 1.0, self.transition_progress);
        (eased_progress * self.spell_strength).clamp(0.0, 1.0)
    }

    fn roll_spell_strength(&mut self, weather: Weather, is_pinned: bool) -> f32 {
        if is_pinned || weather == Weather::Clear {
            return 1.0;
        }
        self.weather_random.float_between(WEAKEST_SPELL_STRENGTH_BY_WEATHER[weather as usize], 1.0)
    }

    fn roll_spell_seconds(&mut self, weather: Weather, requested_minutes: f32) -> f32 {
        let typical_seconds = sane_spell_minutes(requested_minutes)
            * SECONDS_PER_MINUTE
            * SPELL_LENGTH_SCALE_BY_WEATHER[weather as usize];
        (typical_seconds
            * self.weather_random.float_between(SPELL_LENGTH_VARIATION_RANGE.0, SPELL_LENGTH_VARIATION_RANGE.1))
        .max(SHORTEST_SPELL_SECONDS)
    }

    fn begin_weather(&mut self, season: Season, season_progress: f32, hour: f32, controls: &Controls) {
        self.has_started = true;
        self.pinned_weather = controls.weather;
        let is_pinned = controls.weather.is_some();
        let weather = match controls.weather {
            Some(pinned_weather) => pinned_weather,
            None => pick_weather(&mut self.weather_random, &weather_weights(season, season_progress, hour, None)),
        };
        self.current_weather = weather;
        self.upcoming_weather = None;
        self.transition_progress = 1.0;
        self.spell_strength = self.roll_spell_strength(weather, is_pinned);
        self.target_spell_strength = self.spell_strength;
        self.upcoming_spell_strength = self.spell_strength;
        let whole_spell_seconds = self.roll_spell_seconds(weather, controls.weather_spell_minutes);
        let fraction_of_spell_left =
            self.weather_random.float_between(SHARE_OF_FIRST_SPELL_LEFT_RANGE.0, SHARE_OF_FIRST_SPELL_LEFT_RANGE.1);
        self.spell_seconds_remaining = (whole_spell_seconds * fraction_of_spell_left) as f64;
        self.spell_minutes_setting = sane_spell_minutes(controls.weather_spell_minutes);
        self.cloud_deck_overcast = weather.overcast_at_full_intensity() * self.spell_strength;
        self.displayed_hour = hour;

        self.snow_cover = 0.0;
        self.wetness = 0.0;
        if weather == Weather::Snow {
            self.snow_cover = if is_pinned {
                SNOW_COVER_AT_PINNED_SNOWY_START
            } else {
                self.weather_random.float_between(
                    SNOW_COVER_AT_AUTOMATIC_SNOWY_START_RANGE.0,
                    SNOW_COVER_AT_AUTOMATIC_SNOWY_START_RANGE.1,
                )
            };
        }
        if weather == Weather::Rain || weather == Weather::Storm {
            self.wetness = WETNESS_AT_RAINY_START;
        }
        if weather == Weather::Fog {
            self.wetness = WETNESS_AT_FOGGY_START;
        }
        let is_mid_winter = season == Season::Winter
            && season_progress > MID_WINTER_BEGINS_AT_PROGRESS
            && season_progress < MID_WINTER_ENDS_AT_PROGRESS;
        if is_mid_winter && weather != Weather::Snow {
            let old_snow_is_lying = self.weather_random.succeeds_with_probability(CHANCE_OF_OLD_SNOW_IN_MID_WINTER);
            let old_snow_cover = self.weather_random.float_between(OLD_SNOW_COVER_RANGE.0, OLD_SNOW_COVER_RANGE.1);
            let rain_would_have_washed_it_away = weather == Weather::Rain || weather == Weather::Storm;
            if old_snow_is_lying && !rain_would_have_washed_it_away {
                self.snow_cover = old_snow_cover;
            }
        }
        if let Some(requested_snow_cover) = self.snow_cover_requested_before_start.take() {
            self.snow_cover = requested_snow_cover;
        }
        if let Some(requested_wetness) = self.wetness_requested_before_start.take() {
            self.wetness = requested_wetness;
        }

        self.lightning_strike = None;
        self.next_strike_at_seconds = None;
        self.flash_measured_up_to_seconds = self.simulation_seconds;
    }

    fn advance_simulation(&mut self, slice_seconds: f32, season: Season, season_progress: f32, controls: &Controls) {
        self.simulation_seconds += slice_seconds as f64;
        let hour = self.displayed_hour;
        let spell_minutes = sane_spell_minutes(controls.weather_spell_minutes);
        let fade_scale = (spell_minutes / SPELL_MINUTES_FOR_FULL_LENGTH_FADES).clamp(FASTEST_FADE_SCALE, 1.0);
        if spell_minutes != self.spell_minutes_setting {
            self.spell_seconds_remaining *= (spell_minutes / self.spell_minutes_setting) as f64;
            self.spell_minutes_setting = spell_minutes;
        }

        if controls.weather != self.pinned_weather {
            self.pinned_weather = controls.weather;
            if let Some(newly_pinned_weather) = controls.weather {
                self.upcoming_spell_strength = 1.0;
                if newly_pinned_weather == self.current_weather {
                    self.upcoming_weather = None;
                    self.target_spell_strength = 1.0;
                } else {
                    self.upcoming_weather = Some(newly_pinned_weather);
                }
            } else {
                self.upcoming_weather = None;
                let odds_right_now = weather_weights(season, season_progress, hour, None);
                let current_weather_is_possible = odds_right_now[self.current_weather as usize] > 0.0;
                if current_weather_is_possible {
                    let whole_spell_seconds =
                        self.roll_spell_seconds(self.current_weather, controls.weather_spell_minutes);
                    let fraction_of_spell_left = self.weather_random.float_between(
                        SHARE_OF_SPELL_LEFT_AFTER_UNPINNING_RANGE.0,
                        SHARE_OF_SPELL_LEFT_AFTER_UNPINNING_RANGE.1,
                    );
                    self.spell_seconds_remaining = (whole_spell_seconds * fraction_of_spell_left) as f64;
                } else {
                    self.spell_seconds_remaining = 0.0;
                }
                self.target_spell_strength = self.roll_spell_strength(self.current_weather, false);
            }
        }
        if let Some(pinned_weather) = self.pinned_weather {
            if self.upcoming_weather.is_none() && self.current_weather != pinned_weather {
                self.upcoming_weather = Some(pinned_weather);
                self.upcoming_spell_strength = 1.0;
            }
        }

        let weather_is_automatic = self.pinned_weather.is_none();
        if weather_is_automatic && self.upcoming_weather.is_none() {
            self.spell_seconds_remaining -= slice_seconds as f64;
            let mut odds_of_next_weather = weather_weights(season, season_progress, hour, Some(self.current_weather));
            let current_weather_is_impossible = odds_of_next_weather[self.current_weather as usize] <= 0.0;
            if self.next_weather_requested || current_weather_is_impossible {
                self.spell_seconds_remaining = 0.0;
            }
            if self.spell_seconds_remaining <= 0.0 {
                if self.next_weather_requested {
                    odds_of_next_weather[self.current_weather as usize] = 0.0;
                }
                let chosen_weather = pick_weather(&mut self.weather_random, &odds_of_next_weather);
                self.spell_seconds_remaining =
                    self.roll_spell_seconds(chosen_weather, controls.weather_spell_minutes) as f64;
                let chosen_strength = self.roll_spell_strength(chosen_weather, false);
                if chosen_weather == self.current_weather {
                    self.target_spell_strength = chosen_strength;
                } else {
                    self.upcoming_weather = Some(chosen_weather);
                    self.upcoming_spell_strength = chosen_strength;
                }
            }
        }
        self.next_weather_requested = false;

        let weather_to_fade_into = self.upcoming_weather.filter(|upcoming| *upcoming != self.current_weather);
        if let Some(upcoming_weather) = weather_to_fade_into {
            let fade_out_seconds = FADE_OUT_SECONDS_BY_WEATHER[self.current_weather as usize] * fade_scale;
            self.transition_progress -= slice_seconds / fade_out_seconds;
            if self.transition_progress <= 0.0 {
                self.transition_progress = 0.0;
                self.current_weather = upcoming_weather;
                self.upcoming_weather = None;
                self.spell_strength = self.upcoming_spell_strength;
                self.target_spell_strength = self.upcoming_spell_strength;
                self.next_strike_at_seconds = None;
            }
        } else {
            self.upcoming_weather = None;
            let fade_in_seconds = FADE_IN_SECONDS_BY_WEATHER[self.current_weather as usize] * fade_scale;
            self.transition_progress = (self.transition_progress + slice_seconds / fade_in_seconds).min(1.0);
        }
        let strength_still_to_change = self.target_spell_strength - self.spell_strength;
        let largest_strength_change = slice_seconds / SECONDS_FOR_SPELL_STRENGTH_TO_CHANGE_FULLY;
        self.spell_strength += strength_still_to_change.clamp(-largest_strength_change, largest_strength_change);

        let intensity = self.intensity();

        let weather_ahead = self.upcoming_weather.unwrap_or(self.current_weather);
        let strength_ahead =
            if self.upcoming_weather.is_some() { self.upcoming_spell_strength } else { self.target_spell_strength };
        let cloud_deck_target = weather_ahead.overcast_at_full_intensity() * strength_ahead;
        let largest_deck_change = slice_seconds * (CLOUD_DECK_CHANGE_PER_SECOND / fade_scale);
        self.cloud_deck_overcast +=
            (cloud_deck_target - self.cloud_deck_overcast).clamp(-largest_deck_change, largest_deck_change);

        let wind = self.wind_at(self.simulation_seconds);
        self.update_ground(slice_seconds, season, hour, intensity, wind);

        let storm_is_electric =
            self.current_weather == Weather::Storm && intensity > STORM_INTENSITY_NEEDED_FOR_LIGHTNING;
        if !storm_is_electric {
            self.next_strike_at_seconds = None;
            return;
        }
        let Some(next_strike_at_seconds) = self.next_strike_at_seconds else {
            let seconds_until_first_strike = self
                .lightning_random
                .float_between(SECONDS_UNTIL_FIRST_STRIKE_RANGE.0, SECONDS_UNTIL_FIRST_STRIKE_RANGE.1);
            self.next_strike_at_seconds = Some(self.simulation_seconds + seconds_until_first_strike as f64);
            return;
        };
        if self.simulation_seconds < next_strike_at_seconds {
            return;
        }
        self.lightning_strike = Some(LightningStrike::new(self.simulation_seconds, &mut self.lightning_random, false));
        let seconds_until_next_strike =
            if self.lightning_random.succeeds_with_probability(CHANCE_OF_A_QUICK_SECOND_STRIKE) {
                self.lightning_random
                    .float_between(QUICK_SECOND_STRIKE_SECONDS_RANGE.0, QUICK_SECOND_STRIKE_SECONDS_RANGE.1)
            } else {
                let roll = self.lightning_random.next_fraction();
                (SHORTEST_STRIKE_GAP_SECONDS + STRIKE_GAP_SPREAD_SECONDS * roll * roll.sqrt())
                    / (STRIKE_RATE_IN_A_WEAK_STORM + STRIKE_RATE_GAINED_IN_A_FULL_STORM * intensity)
            };
        self.next_strike_at_seconds = Some(self.simulation_seconds + seconds_until_next_strike as f64);
    }

    fn update_ground(&mut self, slice_seconds: f32, season: Season, hour: f32, intensity: f32, wind: f32) {
        let weather = self.current_weather;
        let snowfall = if weather == Weather::Snow { intensity } else { 0.0 };
        let rainfall = if weather == Weather::Rain || weather == Weather::Storm { intensity } else { 0.0 };
        let sky = Environment { season, hour, weather, weather_intensity: intensity, ..Environment::default() };
        let sunshine = sky.daylight() * (1.0 - sky.overcast());

        if snowfall > 0.0 {
            let room_left_to_build = SNOW_COVER_SNOWFALL_APPROACHES - self.snow_cover;
            self.snow_cover += slice_seconds * snowfall * room_left_to_build / SNOWFALL_SECONDS_SCALE;
        }
        let seasonal_melt_per_second = match season {
            Season::Winter => WINTER_MELT_PER_SECOND,
            Season::Spring | Season::Autumn => SPRING_AND_AUTUMN_MELT_PER_SECOND,
            Season::Summer => SUMMER_MELT_PER_SECOND,
        };
        let melt_per_second = seasonal_melt_per_second
            * (1.0 + EXTRA_MELT_IN_FULL_SUN * sunshine)
            * (1.0 + EXTRA_MELT_IN_FULL_RAIN * rainfall)
            * (1.0 - snowfall)
            * (1.0 - snowfall);
        let snow_cover_before_melting = self.snow_cover;
        self.snow_cover = (self.snow_cover - slice_seconds * melt_per_second).clamp(0.0, 1.0);

        if rainfall > 0.0 {
            let seconds_to_soak = if weather == Weather::Storm {
                SECONDS_OF_STORM_TO_SOAK_THE_GROUND
            } else {
                SECONDS_OF_RAIN_TO_SOAK_THE_GROUND
            };
            self.wetness += slice_seconds * rainfall / seconds_to_soak;
        }
        let seasonal_drying = match season {
            Season::Summer => SUMMER_DRYING_SCALE,
            Season::Winter => WINTER_DRYING_SCALE,
            _ => 1.0,
        };
        let drying_in_damp_air = if weather == Weather::Fog {
            1.0 - DRYING_LOST_IN_DAMP_AIR * intensity
        } else {
            1.0 - DRYING_LOST_IN_DAMP_AIR * snowfall
        };
        let drying_per_second = BASE_DRYING_PER_SECOND
            * (DRYING_SCALE_WITHOUT_SUN + DRYING_GAINED_IN_FULL_SUN * sunshine)
            * seasonal_drying
            * (1.0 + EXTRA_DRYING_IN_FULL_WIND * wind.abs())
            * (1.0 - rainfall)
            * drying_in_damp_air;
        let snow_is_melting = snow_cover_before_melting > 0.0 && snowfall < SNOWFALL_INTENSITY_THAT_STOPS_MELT_WATER;
        let wetness_from_melt = if snow_is_melting {
            WETNESS_UNDER_MELTING_SNOW
                * smoothstep(
                    SNOW_COVER_WHERE_MELT_WATER_BEGINS,
                    SNOW_COVER_WHERE_MELT_WATER_IS_FULL,
                    snow_cover_before_melting,
                )
                * (1.0 - snowfall / SNOWFALL_INTENSITY_THAT_STOPS_MELT_WATER)
        } else {
            0.0
        };
        let wetness_from_fog = if weather == Weather::Fog { WETNESS_UNDER_FULL_FOG * intensity } else { 0.0 };
        let lowest_wetness = wetness_from_melt.max(wetness_from_fog);
        if self.wetness >= lowest_wetness {
            self.wetness = (self.wetness - slice_seconds * drying_per_second).max(lowest_wetness);
        } else {
            self.wetness = (self.wetness + slice_seconds / SECONDS_TO_DAMPEN_FULLY).min(lowest_wetness);
        }
        self.wetness = self.wetness.clamp(0.0, 1.0);
    }

    fn wind_at(&self, seconds: f64) -> f32 {
        let noise_seed = self.world_seed as u32;
        let intensity = self.intensity();
        let gustiness =
            interpolate(GUSTINESS_BETWEEN_WEATHERS, GUSTINESS_BY_WEATHER[self.current_weather as usize], intensity);
        let stillness_of_fog =
            if self.current_weather == Weather::Fog { 1.0 - WIND_LOST_IN_FULL_FOG * intensity } else { 1.0 };
        let prevailing_direction =
            ((time_noise(seconds / PREVAILING_DIRECTION_NOISE_SECONDS, noise_seed ^ PREVAILING_DIRECTION_SEED_MIX)
                - 0.5)
                * PREVAILING_DIRECTION_SHARPNESS)
                .clamp(-1.0, 1.0);
        let calm_strength = CALMEST_WIND_STRENGTH
            + CALM_WIND_STRENGTH_RANGE * time_noise(seconds / CALM_WIND_NOISE_SECONDS, noise_seed ^ CALM_WIND_SEED_MIX);
        let stormy_strength = WEAKEST_STORM_WIND_STRENGTH
            + STORM_WIND_STRENGTH_RANGE
                * time_noise(seconds / STORM_WIND_NOISE_SECONDS, noise_seed ^ STORM_WIND_SEED_MIX);
        let slow_gust = (time_noise(seconds / SLOW_GUST_NOISE_SECONDS, noise_seed ^ SLOW_GUST_SEED_MIX) - 0.5) * 2.0;
        let quick_gust = (time_noise(seconds / QUICK_GUST_NOISE_SECONDS, noise_seed ^ QUICK_GUST_SEED_MIX) - 0.5) * 2.0;
        let steady_strength = interpolate(calm_strength, stormy_strength, gustiness);
        let wind = prevailing_direction * steady_strength
            + slow_gust * (SLOW_GUST_STRENGTH_WHEN_CALM + SLOW_GUST_STRENGTH_GAINED_IN_STORM * gustiness)
            + quick_gust * (QUICK_GUST_STRENGTH_WHEN_CALM + QUICK_GUST_STRENGTH_GAINED_IN_STORM * gustiness);
        (wind * stillness_of_fog).clamp(-1.0, 1.0)
    }
}
