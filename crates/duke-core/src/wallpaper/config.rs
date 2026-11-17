use crate::{Season, Weather};
use std::path::{Path, PathBuf};

pub const SETTINGS_DIRECTORY_NAME: &str = "duke-pixel-art-background";
pub const CONFIG_FILE_NAME: &str = "config.toml";
pub const LOG_FILE_NAME: &str = "duke-wallpaper.log";
pub const SLOWEST_FRAMES_PER_SECOND: u32 = 5;
pub const FASTEST_FRAMES_PER_SECOND: u32 = 60;

const KEYS_THE_CONFIG_FILE_MAY_CONTAIN: &[&str] = &[
    "scene",
    "cycle_minutes",
    "transition_seconds",
    "season",
    "weather",
    "weather_minutes",
    "time",
    "seed",
    "fps",
    "fit",
    "letterbox_color",
    "pause_on_fullscreen",
    "pause_on_battery",
];

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum SceneChoice {
    Chapel,
    East,
    Auto,
}

impl SceneChoice {
    pub fn name(self) -> &'static str {
        match self {
            SceneChoice::Chapel => "chapel",
            SceneChoice::East => "east",
            SceneChoice::Auto => "auto",
        }
    }

    pub fn parse(text: &str) -> Option<Self> {
        match text.trim().to_ascii_lowercase().as_str() {
            "chapel" | "duke-chapel" | "west" => Some(SceneChoice::Chapel),
            "east" | "east-campus" | "eastcampus" => Some(SceneChoice::East),
            "auto" | "cycle" | "auto-cycle" => Some(SceneChoice::Auto),
            _ => None,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum TimeChoice {
    RealTime,
    Dawn,
    Midday,
    GoldenHour,
    Night,
    FixedHour(f32),
}

impl TimeChoice {
    pub fn parse(text: &str) -> Option<Self> {
        match text.trim().to_ascii_lowercase().as_str() {
            "real" | "auto" | "clock" | "realtime" | "real-time" => Some(TimeChoice::RealTime),
            "dawn" | "sunrise" => Some(TimeChoice::Dawn),
            "midday" | "noon" | "day" => Some(TimeChoice::Midday),
            "golden" | "golden-hour" | "goldenhour" | "sunset" | "dusk" => Some(TimeChoice::GoldenHour),
            "night" | "midnight" => Some(TimeChoice::Night),
            hour_as_text => hour_as_text.parse::<f32>().ok().and_then(Self::from_hour),
        }
    }

    pub fn from_hour(hour: f32) -> Option<Self> {
        if !hour.is_finite() || !(0.0..=24.0).contains(&hour) {
            return None;
        }
        let twenty_four_is_midnight = if hour >= 24.0 { 0.0 } else { hour };
        Some(TimeChoice::FixedHour(twenty_four_is_midnight))
    }

    pub fn pinned_hour(self, season: Season) -> Option<f32> {
        let (sunrise_hour, sunset_hour) = season.sunrise_and_sunset_hours();
        match self {
            TimeChoice::RealTime => None,
            TimeChoice::Dawn => Some(sunrise_hour + 0.35),
            TimeChoice::Midday => Some(12.5),
            TimeChoice::GoldenHour => Some(sunset_hour - 0.45),
            TimeChoice::Night => Some(23.0),
            TimeChoice::FixedHour(hour) => Some(hour),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Fit {
    Cover,
    Contain,
}

impl Fit {
    pub fn name(self) -> &'static str {
        match self {
            Fit::Cover => "cover",
            Fit::Contain => "contain",
        }
    }

    pub fn parse(text: &str) -> Option<Self> {
        match text.trim().to_ascii_lowercase().as_str() {
            "cover" | "fill" | "crop" => Some(Fit::Cover),
            "contain" | "fit" | "letterbox" => Some(Fit::Contain),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Config {
    pub scene: SceneChoice,
    pub cycle_minutes: f32,
    pub transition_seconds: f32,
    pub season: Option<Season>,
    pub weather: Option<Weather>,
    pub weather_minutes: f32,
    pub time_of_day: TimeChoice,
    pub world_seed: u64,
    pub frames_per_second: u32,
    pub fit: Fit,
    pub letterbox_colour: u32,
    pub pause_on_fullscreen: bool,
    pub pause_on_battery: bool,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            scene: SceneChoice::Auto,
            cycle_minutes: 15.0,
            transition_seconds: 2.0,
            season: None,
            weather: None,
            weather_minutes: 12.0,
            time_of_day: TimeChoice::RealTime,
            world_seed: 1924,
            frames_per_second: 20,
            fit: Fit::Cover,
            letterbox_colour: 0x0B0E14,
            pause_on_fullscreen: true,
            pause_on_battery: false,
        }
    }
}

fn number_as_toml_value(number: f32) -> String {
    let is_a_whole_number = number.fract() == 0.0 && number.abs() < 1e9;
    if is_a_whole_number {
        format!("{number:.1}")
    } else {
        format!("{number}")
    }
}

fn describe_toml_value(value: &toml::Value) -> String {
    match value {
        toml::Value::String(text) => format!("{text:?}"),
        toml::Value::Integer(integer) => integer.to_string(),
        toml::Value::Float(float) => float.to_string(),
        toml::Value::Boolean(boolean) => boolean.to_string(),
        other => format!("a {}", other.type_str()),
    }
}

fn parse_hex_colour(text: &str) -> Option<u32> {
    let hex_digits = text.trim().trim_start_matches('#');
    if !hex_digits.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    let value = u32::from_str_radix(hex_digits, 16).ok()?;
    match hex_digits.len() {
        6 => Some(value),
        3 => {
            const ONE_HEX_DIGIT_TO_FULL_CHANNEL: u32 = 17;
            let red = ((value >> 8) & 0xF) * ONE_HEX_DIGIT_TO_FULL_CHANNEL;
            let green = ((value >> 4) & 0xF) * ONE_HEX_DIGIT_TO_FULL_CHANNEL;
            let blue = (value & 0xF) * ONE_HEX_DIGIT_TO_FULL_CHANNEL;
            Some(red << 16 | green << 8 | blue)
        }
        _ => None,
    }
}

fn number_within(
    table: &toml::Table,
    key: &str,
    lowest: f64,
    highest: f64,
    default: f64,
    warnings: &mut Vec<String>,
) -> f64 {
    let Some(value) = table.get(key) else {
        return default;
    };
    let number = match value {
        toml::Value::Integer(integer) => Some(*integer as f64),
        toml::Value::Float(float) => Some(*float),
        _ => None,
    };
    match number {
        Some(number) if number.is_finite() && number >= lowest && number <= highest => number,
        _ => {
            let described = describe_toml_value(value);
            warnings.push(format!("`{key}` = {described} is not a number in {lowest}..={highest}; using {default}"));
            default
        }
    }
}

fn true_or_false(table: &toml::Table, key: &str, default: bool, warnings: &mut Vec<String>) -> bool {
    match table.get(key) {
        None => default,
        Some(toml::Value::Boolean(boolean)) => *boolean,
        Some(value) => {
            let described = describe_toml_value(value);
            warnings.push(format!("`{key}` = {described} is not true/false; using {default}"));
            default
        }
    }
}

fn named_choice<Choice>(
    table: &toml::Table,
    key: &str,
    accepted_values: &str,
    default: Choice,
    warnings: &mut Vec<String>,
    parse_name: impl Fn(&str) -> Option<Choice>,
) -> Choice {
    let Some(value) = table.get(key) else {
        return default;
    };
    let toml::Value::String(name) = value else {
        let described = describe_toml_value(value);
        warnings.push(format!("`{key}` = {described} should be a string ({accepted_values}); using the default"));
        return default;
    };
    match parse_name(name) {
        Some(choice) => choice,
        None => {
            warnings.push(format!("`{key}` = {name:?} is not one of {accepted_values}; using the default"));
            default
        }
    }
}

impl Config {
    pub fn parse(text: &str) -> (Config, Vec<String>) {
        let defaults = Config::default();
        let mut warnings: Vec<String> = Vec::new();
        let table: toml::Table = match text.parse() {
            Ok(table) => table,
            Err(syntax_error) => {
                let whole_message = syntax_error.to_string();
                let first_line = whole_message.lines().next().unwrap_or("syntax error");
                warnings.push(format!("config is not valid TOML ({first_line}); using defaults for everything"));
                return (defaults, warnings);
            }
        };
        for key in table.keys() {
            if !KEYS_THE_CONFIG_FILE_MAY_CONTAIN.contains(&key.as_str()) {
                warnings.push(format!("unknown key `{key}` ignored"));
            }
        }

        let scene = named_choice(
            &table,
            "scene",
            "\"chapel\", \"east\", \"auto\"",
            defaults.scene,
            &mut warnings,
            SceneChoice::parse,
        );
        let cycle_minutes =
            number_within(&table, "cycle_minutes", 0.25, 1440.0, defaults.cycle_minutes as f64, &mut warnings);
        let transition_seconds =
            number_within(&table, "transition_seconds", 0.0, 30.0, defaults.transition_seconds as f64, &mut warnings);
        let season = named_choice(
            &table,
            "season",
            "\"auto\", \"spring\", \"summer\", \"autumn\", \"winter\"",
            defaults.season,
            &mut warnings,
            |name| {
                if name.eq_ignore_ascii_case("auto") {
                    Some(None)
                } else {
                    Season::parse(name).map(Some)
                }
            },
        );
        let weather = named_choice(
            &table,
            "weather",
            "\"auto\", \"clear\", \"cloudy\", \"rain\", \"storm\", \"snow\", \"fog\"",
            defaults.weather,
            &mut warnings,
            |name| {
                if name.eq_ignore_ascii_case("auto") {
                    Some(None)
                } else {
                    Weather::parse(name).map(Some)
                }
            },
        );
        let weather_minutes =
            number_within(&table, "weather_minutes", 0.5, 1440.0, defaults.weather_minutes as f64, &mut warnings);

        let time_of_day_in_file = match table.get("time") {
            None => Some(defaults.time_of_day),
            Some(toml::Value::Integer(hour)) => TimeChoice::from_hour(*hour as f32),
            Some(toml::Value::Float(hour)) => TimeChoice::from_hour(*hour as f32),
            Some(toml::Value::String(name)) => TimeChoice::parse(name),
            Some(_) => None,
        };
        let time_of_day = match time_of_day_in_file {
            Some(time_of_day) => time_of_day,
            None => {
                let described = describe_toml_value(&table["time"]);
                warnings.push(format!(
                    "`time` = {described} is not \"real\", \"dawn\", \"midday\", \"golden\", \"night\" or an hour 0-24; using \"real\""
                ));
                defaults.time_of_day
            }
        };

        let world_seed = match table.get("seed") {
            None => defaults.world_seed,
            Some(toml::Value::Integer(seed)) if *seed >= 0 => *seed as u64,
            Some(value) => {
                let described = describe_toml_value(value);
                warnings
                    .push(format!("`seed` = {described} is not a non-negative integer; using {}", defaults.world_seed));
                defaults.world_seed
            }
        };

        let frames_per_second = number_within(
            &table,
            "fps",
            SLOWEST_FRAMES_PER_SECOND as f64,
            FASTEST_FRAMES_PER_SECOND as f64,
            defaults.frames_per_second as f64,
            &mut warnings,
        );
        let fit = named_choice(&table, "fit", "\"cover\", \"contain\"", defaults.fit, &mut warnings, Fit::parse);
        let letterbox_colour = named_choice(
            &table,
            "letterbox_color",
            "a \"#rrggbb\" colour",
            defaults.letterbox_colour,
            &mut warnings,
            parse_hex_colour,
        );
        let pause_on_fullscreen =
            true_or_false(&table, "pause_on_fullscreen", defaults.pause_on_fullscreen, &mut warnings);
        let pause_on_battery = true_or_false(&table, "pause_on_battery", defaults.pause_on_battery, &mut warnings);

        let config = Config {
            scene,
            cycle_minutes: cycle_minutes as f32,
            transition_seconds: transition_seconds as f32,
            season,
            weather,
            weather_minutes: weather_minutes as f32,
            time_of_day,
            world_seed,
            frames_per_second: frames_per_second.round() as u32,
            fit,
            letterbox_colour,
            pause_on_fullscreen,
            pause_on_battery,
        };
        (config, warnings)
    }

    pub fn to_toml(&self) -> String {
        let season = self.season.map(|season| season.name()).unwrap_or("auto");
        let weather = self.weather.map(|weather| weather.name()).unwrap_or("auto");
        let time_of_day = match self.time_of_day {
            TimeChoice::RealTime => "\"real\"".to_string(),
            TimeChoice::Dawn => "\"dawn\"".to_string(),
            TimeChoice::Midday => "\"midday\"".to_string(),
            TimeChoice::GoldenHour => "\"golden\"".to_string(),
            TimeChoice::Night => "\"night\"".to_string(),
            TimeChoice::FixedHour(hour) => number_as_toml_value(hour),
        };
        format!(
            r##"# Duke Pixel Art Background - configuration
#
# Edit this file, then choose "Reload config" in the tray icon's menu (or
# restart the wallpaper). Anything missing, unknown or invalid falls back to
# its default and is reported in duke-wallpaper.log next to this file.
# Changes made from the tray menu rewrite this file.

# ---- Scene ---------------------------------------------------------------

# Which scene to show: "chapel" (Duke Chapel), "east" (East Campus) or
# "auto" to cycle between them.
scene = "{scene}"

# Minutes between scene changes when scene = "auto".
cycle_minutes = {cycle_minutes}

# Length of the dithered dissolve between scenes, in seconds (0 = hard cut).
transition_seconds = {transition_seconds}

# ---- Atmosphere ----------------------------------------------------------

# "auto" follows the calendar, or pin "spring" | "summer" | "autumn" | "winter".
season = "{season}"

# "auto" simulates weather that drifts over time (with odds that depend on
# the season), or pin "clear" | "cloudy" | "rain" | "storm" | "snow" | "fog".
weather = "{weather}"

# With weather = "auto": roughly how many minutes a weather spell lasts.
weather_minutes = {weather_minutes}

# "real" follows the system clock. Or pin "dawn" | "midday" | "golden" |
# "night", or an exact hour of the day as a number, e.g. 17.5 for 5:30 pm.
time = {time_of_day}

# World seed: the same seed grows the same trees and the same weather.
seed = {world_seed}

# ---- Display -------------------------------------------------------------

# Frames per second, {slowest}-{fastest}. The art is animated for about 20; lower
# values save power.
fps = {frames_per_second}

# How the 640x360 canvas is scaled (always by a whole number, so pixels stay
# square and crisp):
#   "cover"   - smallest scale that fills the monitor; edges are cropped.
#   "contain" - largest scale that fits; the rest is letterboxed.
fit = "{fit}"

# Colour of the bars around a "contain" image, "#rrggbb".
letterbox_color = "#{letterbox_colour:06x}"

# ---- Power ---------------------------------------------------------------

# Stop animating a monitor while a maximized or fullscreen window covers it.
pause_on_fullscreen = {pause_on_fullscreen}

# Stop animating while the computer runs on battery.
pause_on_battery = {pause_on_battery}
"##,
            scene = self.scene.name(),
            cycle_minutes = number_as_toml_value(self.cycle_minutes),
            transition_seconds = number_as_toml_value(self.transition_seconds),
            season = season,
            weather = weather,
            weather_minutes = number_as_toml_value(self.weather_minutes),
            time_of_day = time_of_day,
            world_seed = self.world_seed,
            slowest = SLOWEST_FRAMES_PER_SECOND,
            fastest = FASTEST_FRAMES_PER_SECOND,
            frames_per_second = self.frames_per_second,
            fit = self.fit.name(),
            letterbox_colour = self.letterbox_colour & 0xFF_FFFF,
            pause_on_fullscreen = self.pause_on_fullscreen,
            pause_on_battery = self.pause_on_battery,
        )
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(directory) = path.parent() {
            if !directory.as_os_str().is_empty() {
                std::fs::create_dir_all(directory)?;
            }
        }
        let partly_written_file = path.with_extension("toml.tmp");
        std::fs::write(&partly_written_file, self.to_toml())?;
        std::fs::rename(&partly_written_file, path)
    }
}

pub struct ConfigFromDisk {
    pub config: Config,
    pub warnings: Vec<String>,
    pub was_just_created: bool,
}

pub fn load_or_create_with_defaults(path: &Path) -> ConfigFromDisk {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let config = Config::default();
            let created = config.save(path);
            let warnings = match &created {
                Ok(()) => Vec::new(),
                Err(error) => vec![format!("could not create {}: {error}", path.display())],
            };
            return ConfigFromDisk { config, warnings, was_just_created: created.is_ok() };
        }
        Err(error) => {
            let warning = format!("could not read {}: {error}; using defaults", path.display());
            return ConfigFromDisk { config: Config::default(), warnings: vec![warning], was_just_created: false };
        }
    };
    let (config, warnings) = Config::parse(&text);
    ConfigFromDisk { config, warnings, was_just_created: false }
}

pub fn save_keeping_a_backup_of_a_file_that_cannot_be_parsed(config: &Config, path: &Path) -> std::io::Result<()> {
    if let Ok(bytes_on_disk) = std::fs::read(path) {
        let file_can_be_parsed = match std::str::from_utf8(&bytes_on_disk) {
            Ok(text) => text.parse::<toml::Table>().is_ok(),
            Err(_) => false,
        };
        if !file_can_be_parsed {
            let _ = std::fs::write(path.with_extension("toml.bak"), bytes_on_disk);
        }
    }
    config.save(path)
}

pub fn config_in_file_or_else_in_memory(path: &Path, config_in_memory: &Config) -> Config {
    let Ok(text) = std::fs::read_to_string(path) else {
        return config_in_memory.clone();
    };
    if text.parse::<toml::Table>().is_err() {
        return config_in_memory.clone();
    }
    let (config_in_file, _warnings) = Config::parse(&text);
    config_in_file
}

pub fn default_config_path() -> PathBuf {
    let per_user_settings_root = if cfg!(windows) {
        std::env::var_os("APPDATA").map(PathBuf::from)
    } else if cfg!(target_os = "macos") {
        std::env::var_os("HOME").map(|home| PathBuf::from(home).join("Library").join("Application Support"))
    } else {
        let explicit_config_home = std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from);
        let config_under_home = std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config"));
        explicit_config_home.or(config_under_home)
    };
    per_user_settings_root.unwrap_or_else(|| PathBuf::from(".")).join(SETTINGS_DIRECTORY_NAME).join(CONFIG_FILE_NAME)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_scratch_directory(test_name: &str) -> PathBuf {
        let directory = std::env::temp_dir().join(format!("duke-wallpaper-{test_name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).unwrap();
        directory
    }

    #[test]
    fn defaults_survive_a_round_trip_through_the_file_format() {
        let defaults = Config::default();
        let (parsed, warnings) = Config::parse(&defaults.to_toml());
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(parsed, defaults);
    }

    #[test]
    fn every_setting_survives_a_round_trip_through_the_file_format() {
        let customised = Config {
            scene: SceneChoice::East,
            cycle_minutes: 2.5,
            transition_seconds: 0.0,
            season: Some(Season::Autumn),
            weather: Some(Weather::Storm),
            weather_minutes: 30.0,
            time_of_day: TimeChoice::FixedHour(17.5),
            world_seed: 42,
            frames_per_second: 30,
            fit: Fit::Contain,
            letterbox_colour: 0x102030,
            pause_on_fullscreen: false,
            pause_on_battery: true,
        };
        let (parsed, warnings) = Config::parse(&customised.to_toml());
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(parsed, customised);
    }

    #[test]
    fn each_bad_value_falls_back_to_its_default_with_a_warning() {
        let text = r#"
            scene = "moon"
            fps = 900
            fit = 3
            season = "fall"
            time = "teatime"
            pause_on_battery = "yes"
            letterbox_color = "blue"
            mystery = 1
            weather = "rain"
        "#;
        let (config, warnings) = Config::parse(text);
        let defaults = Config::default();
        assert_eq!(config.scene, defaults.scene);
        assert_eq!(config.frames_per_second, defaults.frames_per_second);
        assert_eq!(config.fit, defaults.fit);
        assert_eq!(config.season, Some(Season::Autumn));
        assert_eq!(config.time_of_day, defaults.time_of_day);
        assert_eq!(config.pause_on_battery, defaults.pause_on_battery);
        assert_eq!(config.letterbox_colour, defaults.letterbox_colour);
        assert_eq!(config.weather, Some(Weather::Rain));
        assert_eq!(warnings.len(), 7, "{warnings:?}");
    }

    #[test]
    fn a_file_that_is_not_toml_means_defaults_and_one_warning() {
        let (config, warnings) = Config::parse("scene = \"east\nfps = = 3");
        assert_eq!(config, Config::default());
        assert_eq!(warnings.len(), 1);
    }

    #[test]
    fn an_empty_file_and_a_byte_order_mark_are_fine() {
        assert_eq!(Config::parse(""), (Config::default(), Vec::new()));
        let (config, warnings) = Config::parse("\u{feff}fps = 30\n");
        assert_eq!((config.frames_per_second, warnings.len()), (30, 0));
    }

    #[test]
    fn a_retired_key_is_only_a_warning() {
        let (config, warnings) = Config::parse("attach = \"auto\"\nfps = 12\n");
        assert_eq!(config.frames_per_second, 12);
        assert_eq!(warnings, vec!["unknown key `attach` ignored".to_string()]);
    }

    #[test]
    fn time_of_day_accepts_names_and_hours() {
        let time_of_day = |text: &str| Config::parse(text).0.time_of_day;
        assert_eq!(time_of_day("time = 6"), TimeChoice::FixedHour(6.0));
        assert_eq!(time_of_day("time = 17.5"), TimeChoice::FixedHour(17.5));
        assert_eq!(time_of_day("time = \"golden\""), TimeChoice::GoldenHour);
        assert_eq!(time_of_day("time = \"21.25\""), TimeChoice::FixedHour(21.25));
        assert_eq!(time_of_day("time = 99"), TimeChoice::RealTime);
        assert_eq!(time_of_day("time = -1"), TimeChoice::RealTime);
        assert_eq!(time_of_day("time = true"), TimeChoice::RealTime);
        assert_eq!(time_of_day("time = nan"), TimeChoice::RealTime);
        assert!(TimeChoice::Dawn.pinned_hour(Season::Summer).unwrap() < 7.0);
        assert!(TimeChoice::GoldenHour.pinned_hour(Season::Winter).unwrap() < 17.5);
    }

    #[test]
    fn hour_twenty_four_is_midnight() {
        let time_of_day = |text: &str| Config::parse(text).0.time_of_day;
        assert_eq!(time_of_day("time = 24"), TimeChoice::FixedHour(0.0));
        assert_eq!(time_of_day("time = 24.0"), TimeChoice::FixedHour(0.0));
        assert_eq!(time_of_day("time = \"24\""), TimeChoice::FixedHour(0.0));
        assert_eq!(time_of_day("time = 23.5"), TimeChoice::FixedHour(23.5));
        assert_eq!(time_of_day("time = 24.5"), TimeChoice::RealTime);
    }

    #[test]
    fn numbers_must_be_finite_and_in_range() {
        let frames_per_second = |text: &str| Config::parse(text).0.frames_per_second;
        assert_eq!(frames_per_second("fps = 59.6"), 60);
        assert_eq!(frames_per_second("fps = 60.4"), 20);
        assert_eq!(frames_per_second("fps = inf"), 20);
        assert_eq!(frames_per_second("fps = nan"), 20);
        assert_eq!(frames_per_second("fps = 4"), 20);
        assert_eq!(frames_per_second("fps = [30]"), 20);
        assert_eq!(Config::parse("seed = -3").0.world_seed, 1924);
        assert_eq!(Config::parse("seed = 7").0.world_seed, 7);
    }

    #[test]
    fn colours_are_strictly_three_or_six_hex_digits() {
        assert_eq!(parse_hex_colour("#0b0e14"), Some(0x0B0E14));
        assert_eq!(parse_hex_colour("fff"), Some(0xFFFFFF));
        assert_eq!(parse_hex_colour(" #A1b2C3 "), Some(0xA1B2C3));
        for rejected in ["#12345", "#+12345", "-12", "#ggg", "", "#", "#1234567", "#éé"] {
            assert_eq!(parse_hex_colour(rejected), None, "{rejected:?}");
        }
    }

    #[test]
    fn a_tray_change_starts_from_hand_edits_that_were_never_reloaded() {
        let path = empty_scratch_directory("hand-edits").join(CONFIG_FILE_NAME);
        let config_in_memory = Config::default();

        std::fs::write(&path, "fps = 30\nscene = \"east\"\n").unwrap();
        let starting_point = config_in_file_or_else_in_memory(&path, &config_in_memory);
        assert_eq!((starting_point.frames_per_second, starting_point.scene), (30, SceneChoice::East));

        std::fs::write(&path, "fps = = broken").unwrap();
        assert_eq!(config_in_file_or_else_in_memory(&path, &config_in_memory), config_in_memory);

        std::fs::remove_file(&path).unwrap();
        assert_eq!(config_in_file_or_else_in_memory(&path, &config_in_memory), config_in_memory);
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn saving_over_a_file_that_cannot_be_parsed_keeps_a_backup_of_it() {
        let path = empty_scratch_directory("backup").join(CONFIG_FILE_NAME);
        let backup_path = path.with_extension("toml.bak");
        let broken_toml = b"fps = = 3".to_vec();
        let mut saved_as_utf16_by_an_old_editor = vec![0xFF, 0xFE];
        for utf16_unit in "fps = 30".encode_utf16() {
            saved_as_utf16_by_an_old_editor.extend_from_slice(&utf16_unit.to_le_bytes());
        }
        for original_bytes in [broken_toml, saved_as_utf16_by_an_old_editor] {
            std::fs::write(&path, &original_bytes).unwrap();
            save_keeping_a_backup_of_a_file_that_cannot_be_parsed(&Config::default(), &path).unwrap();
            assert_eq!(std::fs::read(&backup_path).unwrap(), original_bytes);
            let (saved, _warnings) = Config::parse(&std::fs::read_to_string(&path).unwrap());
            assert_eq!(saved, Config::default());
        }

        std::fs::remove_file(&backup_path).unwrap();
        std::fs::write(&path, "fps = 30\n").unwrap();
        save_keeping_a_backup_of_a_file_that_cannot_be_parsed(&Config::default(), &path).unwrap();
        assert!(!backup_path.exists(), "a healthy file needs no backup");
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn the_first_run_creates_a_commented_file() {
        let path = empty_scratch_directory("first-run").join("nested").join(CONFIG_FILE_NAME);
        let first = load_or_create_with_defaults(&path);
        assert!(first.was_just_created && first.warnings.is_empty());
        assert_eq!(first.config, Config::default());
        assert!(std::fs::read_to_string(&path).unwrap().contains("# Frames per second"));

        let second = load_or_create_with_defaults(&path);
        assert!(!second.was_just_created && second.warnings.is_empty());
        let _ = std::fs::remove_dir_all(path.parent().unwrap().parent().unwrap());
    }
}
