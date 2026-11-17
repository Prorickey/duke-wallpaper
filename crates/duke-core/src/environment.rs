use crate::canvas::Canvas;
use crate::colour::{colour_channels, interpolate};

const HOURS_PER_DAY: f32 = 24.0;
const NOON: f32 = 12.0;
const YEAR_TRINITY_COLLEGE_BECAME_DUKE_UNIVERSITY: u64 = 1924;

const DAWN_FADE_BEFORE_SUNRISE_HOURS: f32 = 0.8;
const DAWN_FADE_AFTER_SUNRISE_HOURS: f32 = 0.9;
const DUSK_FADE_BEFORE_SUNSET_HOURS: f32 = 0.9;
const DUSK_FADE_AFTER_SUNSET_HOURS: f32 = 0.8;
const GOLDEN_HOUR_PEAK_OFFSET_HOURS: f32 = 0.4;
const GOLDEN_HOUR_HALF_WIDTH_HOURS: f32 = 1.3;

const SHORTEST_SHADOW_LENGTH: f32 = 0.35;
const SHADOW_LENGTH_GAINED_AT_THE_HORIZON: f32 = 1.1;
const SHADOW_LENGTH_CURVE_EXPONENT: f32 = 1.5;
const SHADOW_DROP_TOWARD_THE_VIEWER: f32 = 0.35;
const SHADOW_FADE_WITH_OVERCAST_EXPONENT: f32 = 1.5;

const NIGHT_TINT: [f32; 3] = [0.26, 0.33, 0.56];
const GOLDEN_HOUR_TINT: [f32; 3] = [1.06, 0.84, 0.64];
const GOLDEN_HOUR_TINT_SHARE: f32 = 0.8;
const OVERCAST_DARKENING: f32 = 0.38;
const OVERCAST_RED_LOSS: f32 = 0.06;
const OVERCAST_BLUE_GAIN: f32 = 0.05;
const OVERCAST_DESATURATION: f32 = 0.55;
const NIGHT_DESATURATION: f32 = 0.25;
const STRONGEST_DESATURATION: f32 = 0.8;

const RED_LUMINANCE_WEIGHT: f32 = 0.299;
const GREEN_LUMINANCE_WEIGHT: f32 = 0.587;
const BLUE_LUMINANCE_WEIGHT: f32 = 0.114;
const FLASH_SHADOW_LIFT: f32 = 40.0;
const FLASH_BRIGHTENING: f32 = 0.6;
const FLASH_RED_SHARE_OF_LUMINANCE: f32 = 0.92;
const FLASH_GREEN_SHARE_OF_LUMINANCE: f32 = 0.97;
const FLASH_BLUE_SHARE_OF_LUMINANCE: f32 = 1.10;
const BRIGHTEST_CHANNEL: f32 = 255.0;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Season {
    Spring,
    Summer,
    Autumn,
    Winter,
}

impl Season {
    pub const ALL: [Season; 4] = [Season::Spring, Season::Summer, Season::Autumn, Season::Winter];

    pub fn name(self) -> &'static str {
        match self {
            Season::Spring => "spring",
            Season::Summer => "summer",
            Season::Autumn => "autumn",
            Season::Winter => "winter",
        }
    }

    pub fn parse(text: &str) -> Option<Season> {
        match text.to_ascii_lowercase().as_str() {
            "spring" => Some(Season::Spring),
            "summer" => Some(Season::Summer),
            "autumn" | "fall" => Some(Season::Autumn),
            "winter" => Some(Season::Winter),
            _ => None,
        }
    }

    pub fn sunrise_and_sunset_hours(self) -> (f32, f32) {
        match self {
            Season::Spring => (6.6, 19.6),
            Season::Summer => (6.0, 20.5),
            Season::Autumn => (7.2, 18.4),
            Season::Winter => (7.4, 17.3),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Weather {
    Clear,
    Cloudy,
    Rain,
    Storm,
    Snow,
    Fog,
}

impl Weather {
    pub const ALL: [Weather; 6] =
        [Weather::Clear, Weather::Cloudy, Weather::Rain, Weather::Storm, Weather::Snow, Weather::Fog];

    pub fn name(self) -> &'static str {
        match self {
            Weather::Clear => "clear",
            Weather::Cloudy => "cloudy",
            Weather::Rain => "rain",
            Weather::Storm => "storm",
            Weather::Snow => "snow",
            Weather::Fog => "fog",
        }
    }

    pub fn parse(text: &str) -> Option<Weather> {
        match text.to_ascii_lowercase().as_str() {
            "clear" | "sunny" => Some(Weather::Clear),
            "cloudy" | "overcast" => Some(Weather::Cloudy),
            "rain" | "rainy" => Some(Weather::Rain),
            "storm" | "thunderstorm" | "lightning" => Some(Weather::Storm),
            "snow" | "snowy" => Some(Weather::Snow),
            "fog" | "mist" => Some(Weather::Fog),
            _ => None,
        }
    }

    pub fn overcast_at_full_intensity(self) -> f32 {
        match self {
            Weather::Clear => 0.0,
            Weather::Cloudy => 0.45,
            Weather::Rain => 0.6,
            Weather::Storm => 0.85,
            Weather::Snow => 0.4,
            Weather::Fog => 0.35,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Environment {
    pub season: Season,
    pub season_progress: f32,
    pub weather: Weather,
    pub weather_intensity: f32,
    pub hour: f32,
    pub animation_seconds: f64,
    pub wind: f32,
    pub snow_cover: f32,
    pub wetness: f32,
    pub lightning_flash: f32,
    pub seed: u64,
}

impl Default for Environment {
    fn default() -> Self {
        Environment {
            season: Season::Summer,
            season_progress: 0.5,
            weather: Weather::Clear,
            weather_intensity: 1.0,
            hour: 13.0,
            animation_seconds: 0.0,
            wind: 0.2,
            snow_cover: 0.0,
            wetness: 0.0,
            lightning_flash: 0.0,
            seed: YEAR_TRINITY_COLLEGE_BECAME_DUKE_UNIVERSITY,
        }
    }
}

fn smoothstep(edge_start: f32, edge_end: f32, value: f32) -> f32 {
    let fraction = ((value - edge_start) / (edge_end - edge_start)).clamp(0.0, 1.0);
    fraction * fraction * (3.0 - 2.0 * fraction)
}

impl Environment {
    fn hour_of_day(&self) -> f32 {
        if self.hour.is_finite() {
            self.hour.rem_euclid(HOURS_PER_DAY)
        } else {
            NOON
        }
    }

    pub fn daylight(&self) -> f32 {
        let (sunrise, sunset) = self.season.sunrise_and_sunset_hours();
        let hour = self.hour_of_day();
        let dawn = smoothstep(sunrise - DAWN_FADE_BEFORE_SUNRISE_HOURS, sunrise + DAWN_FADE_AFTER_SUNRISE_HOURS, hour);
        let dusk = smoothstep(sunset - DUSK_FADE_BEFORE_SUNSET_HOURS, sunset + DUSK_FADE_AFTER_SUNSET_HOURS, hour);
        dawn * (1.0 - dusk)
    }

    pub fn darkness(&self) -> f32 {
        1.0 - self.daylight()
    }

    pub fn golden_hour_strength(&self) -> f32 {
        let (sunrise, sunset) = self.season.sunrise_and_sunset_hours();
        let hour = self.hour_of_day();
        let closeness_to =
            |peak_hour: f32| (1.0 - ((hour - peak_hour).abs() / GOLDEN_HOUR_HALF_WIDTH_HOURS)).clamp(0.0, 1.0);
        closeness_to(sunrise + GOLDEN_HOUR_PEAK_OFFSET_HOURS).max(closeness_to(sunset - GOLDEN_HOUR_PEAK_OFFSET_HOURS))
    }

    pub fn overcast(&self) -> f32 {
        (self.weather.overcast_at_full_intensity() * self.weather_intensity).clamp(0.0, 1.0)
    }

    pub fn shadow_direction(&self) -> (f32, f32) {
        let (sunrise, sunset) = self.season.sunrise_and_sunset_hours();
        let progress_through_the_day = ((self.hour - sunrise) / (sunset - sunrise)).clamp(0.0, 1.0);
        let distance_from_noon = (2.0 * progress_through_the_day - 1.0).abs();
        let length = SHORTEST_SHADOW_LENGTH
            + SHADOW_LENGTH_GAINED_AT_THE_HORIZON * distance_from_noon.powf(SHADOW_LENGTH_CURVE_EXPONENT);
        let across_the_screen = interpolate(-1.0, 1.0, progress_through_the_day) * length;
        (across_the_screen, SHADOW_DROP_TOWARD_THE_VIEWER * length)
    }

    pub fn shadow_strength(&self) -> f32 {
        self.daylight() * (1.0 - self.overcast()).powf(SHADOW_FADE_WITH_OVERCAST_EXPONENT)
    }

    pub fn lighting(&self) -> Lighting {
        let daylight = self.daylight();
        let overcast = self.overcast();
        let golden_hour = self.golden_hour_strength() * (1.0 - overcast);
        let mut tint = [0.0f32; 3];
        for channel in 0..3 {
            let sunlit = interpolate(1.0, GOLDEN_HOUR_TINT[channel], golden_hour * GOLDEN_HOUR_TINT_SHARE);
            tint[channel] = interpolate(NIGHT_TINT[channel], sunlit, daylight);
        }
        let overcast_brightness = 1.0 - OVERCAST_DARKENING * overcast;
        tint[0] *= overcast_brightness * (1.0 - OVERCAST_RED_LOSS * overcast);
        tint[1] *= overcast_brightness;
        tint[2] *= overcast_brightness * (1.0 + OVERCAST_BLUE_GAIN * overcast);
        Lighting {
            tint,
            desaturation: (OVERCAST_DESATURATION * overcast + NIGHT_DESATURATION * (1.0 - daylight))
                .clamp(0.0, STRONGEST_DESATURATION),
            lightning_flash: self.lightning_flash,
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Lighting {
    pub tint: [f32; 3],
    pub desaturation: f32,
    pub lightning_flash: f32,
}

pub fn apply_lighting(canvas: &mut Canvas, lighting: &Lighting) {
    let changes_nothing =
        lighting.desaturation == 0.0 && lighting.lightning_flash == 0.0 && lighting.tint == [1.0, 1.0, 1.0];
    if changes_nothing {
        return;
    }
    let flash = lighting.lightning_flash.clamp(0.0, 1.0);
    let desaturation = lighting.desaturation;
    let [red_tint, green_tint, blue_tint] = lighting.tint;
    for pixel in canvas.pixels.iter_mut() {
        let (red, green, blue) = colour_channels(*pixel);
        let (red, green, blue) = (red as f32, green as f32, blue as f32);
        let luminance = RED_LUMINANCE_WEIGHT * red + GREEN_LUMINANCE_WEIGHT * green + BLUE_LUMINANCE_WEIGHT * blue;
        let mut red = (red + (luminance - red) * desaturation) * red_tint;
        let mut green = (green + (luminance - green) * desaturation) * green_tint;
        let mut blue = (blue + (luminance - blue) * desaturation) * blue_tint;
        if flash > 0.0 {
            let lift_toward_cold_white = |channel: f32, share_of_luminance: f32| {
                channel
                    + (luminance * share_of_luminance + FLASH_SHADOW_LIFT - channel).max(0.0) * flash
                    + channel * flash * FLASH_BRIGHTENING
            };
            red = lift_toward_cold_white(red, FLASH_RED_SHARE_OF_LUMINANCE);
            green = lift_toward_cold_white(green, FLASH_GREEN_SHARE_OF_LUMINANCE);
            blue = lift_toward_cold_white(blue, FLASH_BLUE_SHARE_OF_LUMINANCE);
        }
        *pixel = ((red.clamp(0.0, BRIGHTEST_CHANNEL) as u32) << 16)
            | ((green.clamp(0.0, BRIGHTEST_CHANNEL) as u32) << 8)
            | blue.clamp(0.0, BRIGHTEST_CHANNEL) as u32;
    }
}
