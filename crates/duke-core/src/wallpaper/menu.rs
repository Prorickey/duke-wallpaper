use crate::wallpaper::config::{self, Config, SceneChoice, TimeChoice};
use crate::{info, warn, Season, Weather};
use std::path::Path;

pub const PROGRAM_TITLE: &str = "Duke Pixel Art Background";
pub const START_AT_LOGIN_LABEL: &str = if cfg!(target_os = "macos") { "Open at login" } else { "Start with Windows" };
const ICON_ART_SIDE: i32 = 16;

const CHAPEL_TOWER_ICON_ART: [&str; ICON_ART_SIDE as usize] = [
    " .............. ",
    "....#..##..#....",
    "....#.####.#....",
    "....########....",
    "....#o#oo#o#+...",
    "....#o#oo#o#+...",
    "....########+...",
    "....###oo###+...",
    "....###oo###+...",
    "....###oo###+...",
    "....########+...",
    "...##########+..",
    "...####oo####+..",
    "...####oo####+..",
    "..############+.",
    " .............. ",
];
const STONE: u32 = 0xFFE4_D6BC;
const STONE_SHADOW_ON_THE_TILE: u32 = 0xFF0A_1F5C;
const DARK_OPENING: u32 = 0xFF1B_2A55;
const DUKE_BLUE_TILE: u32 = 0xFF01_2169;
const TRANSPARENT: u32 = 0x0000_0000;

pub fn chapel_tower_icon_pixels(side: i32) -> Vec<u32> {
    let mut pixels = Vec::with_capacity((side * side) as usize);
    for row in 0..side {
        let art_row = CHAPEL_TOWER_ICON_ART[(row * ICON_ART_SIDE / side) as usize].as_bytes();
        for column in 0..side {
            let art_character = art_row[(column * ICON_ART_SIDE / side) as usize];
            let colour_with_alpha = match art_character {
                b'#' => STONE,
                b'+' => STONE_SHADOW_ON_THE_TILE,
                b'o' => DARK_OPENING,
                b'.' => DUKE_BLUE_TILE,
                _ => TRANSPARENT,
            };
            pixels.push(colour_with_alpha);
        }
    }
    pixels
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum MenuChoice {
    Scene(SceneChoice),
    Season(Option<Season>),
    Weather(Option<Weather>),
    TimeOfDay(TimeChoice),
    PauseOrResume,
    StartAtLogin,
    OpenConfigFile,
    ReloadConfig,
    Quit,
}

pub fn config_after_choice(config: &Config, choice: MenuChoice) -> Config {
    let mut changed = config.clone();
    match choice {
        MenuChoice::Scene(scene) => changed.scene = scene,
        MenuChoice::Season(season) => changed.season = season,
        MenuChoice::Weather(weather) => changed.weather = weather,
        MenuChoice::TimeOfDay(time_of_day) => changed.time_of_day = time_of_day,
        MenuChoice::PauseOrResume
        | MenuChoice::StartAtLogin
        | MenuChoice::OpenConfigFile
        | MenuChoice::ReloadConfig
        | MenuChoice::Quit => {}
    }
    changed
}

pub fn config_saved_after_choice(config_path: &Path, config: &Config, choice: MenuChoice) -> Option<Config> {
    let config_with_hand_edits_not_yet_reloaded = config::config_in_file_or_else_in_memory(config_path, config);
    let changed_config = config_after_choice(&config_with_hand_edits_not_yet_reloaded, choice);
    if changed_config == *config {
        return None;
    }
    if let Err(error) = config::save_keeping_a_backup_of_a_file_that_cannot_be_parsed(&changed_config, config_path) {
        warn!("could not save {}: {error}", config_path.display());
    }
    Some(changed_config)
}

pub fn config_reloaded_from(config_path: &Path) -> Option<Config> {
    let text = match std::fs::read_to_string(config_path) {
        Ok(text) => text,
        Err(error) => {
            warn!("could not reload {}: {error}", config_path.display());
            return None;
        }
    };
    let (reloaded_config, warnings) = Config::parse(&text);
    info!("config reloaded from {}", config_path.display());
    for warning in warnings {
        warn!("config: {warning}");
    }
    Some(reloaded_config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_choice_changes_exactly_one_setting() {
        let defaults = Config::default();
        let east = config_after_choice(&defaults, MenuChoice::Scene(SceneChoice::East));
        assert_eq!(east, Config { scene: SceneChoice::East, ..defaults.clone() });
        let winter = config_after_choice(&defaults, MenuChoice::Season(Some(Season::Winter)));
        assert_eq!(winter, Config { season: Some(Season::Winter), ..defaults.clone() });
        let snow = config_after_choice(&defaults, MenuChoice::Weather(Some(Weather::Snow)));
        assert_eq!(snow, Config { weather: Some(Weather::Snow), ..defaults.clone() });
        let night = config_after_choice(&defaults, MenuChoice::TimeOfDay(TimeChoice::Night));
        assert_eq!(night, Config { time_of_day: TimeChoice::Night, ..defaults.clone() });
        for choice in [MenuChoice::PauseOrResume, MenuChoice::StartAtLogin, MenuChoice::ReloadConfig, MenuChoice::Quit]
        {
            assert_eq!(config_after_choice(&defaults, choice), defaults);
        }
    }

    #[test]
    fn the_icon_is_a_tower_on_a_blue_tile_at_any_size() {
        for side in [16, 20, 24, 32, 64] {
            let pixels = chapel_tower_icon_pixels(side);
            assert_eq!(pixels.len(), (side * side) as usize);
            let corner = pixels[0];
            let centre = pixels[(side / 2 * side + side / 2) as usize];
            assert_eq!(corner, TRANSPARENT);
            assert!(centre == STONE || centre == DARK_OPENING);
            assert!(pixels.contains(&DUKE_BLUE_TILE) && pixels.contains(&STONE));
        }
    }
}
