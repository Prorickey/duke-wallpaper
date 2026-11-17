use crate::wallpaper::config::{Config, SceneChoice, TimeChoice};
use crate::wallpaper::engine::CampusScene;
use crate::wallpaper::menu::{chapel_tower_icon_pixels, MenuChoice, START_AT_LOGIN_LABEL};
use crate::wallpaper::pixel_font::{draw_text, text_width, GLYPH_HEIGHT};
use crate::{blend_colours, Canvas, Colour, Season, Weather, CANVAS_HEIGHT, CANVAS_WIDTH};

pub const PANEL_WIDTH: i32 = 248;
const MARGIN: i32 = 8;
const CONTENT_WIDTH: i32 = PANEL_WIDTH - 2 * MARGIN;
const GAP_BETWEEN_CONTROLS: i32 = 3;
const CHIP_HEIGHT: i32 = 13;
const TOGGLE_HEIGHT: i32 = 11;
const TOGGLE_BOX_SIDE: i32 = 9;
const ICON_SIDE: i32 = 16;
const HEADER_HEIGHT: i32 = 24;
const PREVIEW_WIDTH: i32 = CONTENT_WIDTH;
const PREVIEW_HEIGHT: i32 = PREVIEW_WIDTH * CANVAS_HEIGHT / CANVAS_WIDTH;
const SPACE_BELOW_A_SECTION_LABEL: i32 = 3;
const SPACE_AFTER_A_SECTION: i32 = 7;
const BAYER_ORDER_FOUR: [[f32; 4]; 4] =
    [[0.0, 8.0, 2.0, 10.0], [12.0, 4.0, 14.0, 6.0], [3.0, 11.0, 1.0, 9.0], [15.0, 7.0, 13.0, 5.0]];
const BACKGROUND_SHADES: f32 = 6.0;

const BACKGROUND_TOP: Colour = 0x13_24_52;
const BACKGROUND_BOTTOM: Colour = 0x07_10_2A;
const PANEL_BORDER: Colour = 0x2D_4C_94;
const PANEL_BORDER_HIGHLIGHT: Colour = 0x4A_6F_C4;
const TITLE_TEXT: Colour = 0xF1_E8_D2;
const SUBTITLE_TEXT: Colour = 0x93_A7_D4;
const SECTION_LABEL_TEXT: Colour = 0x79_95_D2;
const SECTION_RULE: Colour = 0x1F_33_6B;
const PREVIEW_FRAME: Colour = 0x3A_5C_AA;
const PREVIEW_FRAME_SHADOW: Colour = 0x05_0B_1E;

#[derive(Copy, Clone)]
struct ChipColours {
    fill: Colour,
    top_edge: Colour,
    bottom_edge: Colour,
    text: Colour,
}

const RESTING_CHIP: ChipColours =
    ChipColours { fill: 0x17_2C_60, top_edge: 0x27_43_86, bottom_edge: 0x0A_15_36, text: 0xC2_CE_EB };
const HOVERED_CHIP: ChipColours =
    ChipColours { fill: 0x24_3E_84, top_edge: 0x3A_5C_AE, bottom_edge: 0x10_1F_4C, text: 0xFF_FF_FF };
const SELECTED_CHIP: ChipColours =
    ChipColours { fill: 0xE4_D6_BC, top_edge: 0xF8_F0_DE, bottom_edge: 0xA8_95_70, text: 0x01_21_69 };
const HOVERED_SELECTED_CHIP: ChipColours =
    ChipColours { fill: 0xF0_E5_CF, top_edge: 0xFF_FA_EE, bottom_edge: 0xB8_A5_80, text: 0x01_21_69 };
const BUTTON: ChipColours =
    ChipColours { fill: 0x00_53_9B, top_edge: 0x2C_7B_C8, bottom_edge: 0x00_36_6A, text: 0xFF_FF_FF };
const HOVERED_BUTTON: ChipColours =
    ChipColours { fill: 0x16_6A_B8, top_edge: 0x4A_94_DC, bottom_edge: 0x00_45_84, text: 0xFF_FF_FF };
const QUIT_BUTTON: ChipColours =
    ChipColours { fill: 0x74_23_33, top_edge: 0x9C_36_4A, bottom_edge: 0x4C_14_20, text: 0xF6_DD_E2 };
const HOVERED_QUIT_BUTTON: ChipColours =
    ChipColours { fill: 0x93_2E_42, top_edge: 0xBA_47_5D, bottom_edge: 0x5E_19_28, text: 0xFF_FF_FF };
const TOGGLE_OFF_BOX: Colour = 0x0A_15_36;
const TOGGLE_ON_BOX: Colour = 0xE4_D6_BC;
const TOGGLE_TICK: Colour = 0x01_21_69;
const TOGGLE_TEXT: Colour = 0xC2_CE_EB;
const HOVERED_TOGGLE_TEXT: Colour = 0xFF_FF_FF;
const TICK_ART: [&str; 7] = ["......#", ".....##", "#...##.", "##.##..", ".###...", "..#....", "......."];

#[derive(Copy, Clone, Debug, PartialEq)]
enum Look {
    Chip { is_selected: bool },
    Toggle { is_on: bool },
    Button,
    QuitButton,
}

#[derive(Clone, Debug)]
struct Control {
    left: i32,
    top: i32,
    width: i32,
    height: i32,
    label: String,
    look: Look,
    choice: MenuChoice,
}

impl Control {
    fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.left && x < self.left + self.width && y >= self.top && y < self.top + self.height
    }
}

pub struct PanelState<'a> {
    pub config: &'a Config,
    pub scene_on_screen: CampusScene,
    pub weather_on_screen: Weather,
    pub activity: &'a str,
    pub is_paused_by_user: bool,
    pub starts_at_login: bool,
}

pub struct ControlPanel {
    controls: Vec<Control>,
    section_labels: Vec<(i32, &'static str)>,
    preview_top: i32,
    subtitle: String,
    hovered_control: Option<usize>,
    pub canvas: Canvas,
}

fn scene_title(scene: CampusScene) -> &'static str {
    match scene {
        CampusScene::Chapel => "Duke Chapel",
        CampusScene::EastCampus => "East Campus",
    }
}

fn time_label(time_of_day: TimeChoice) -> String {
    match time_of_day {
        TimeChoice::RealTime => "Real".to_string(),
        TimeChoice::Dawn => "Dawn".to_string(),
        TimeChoice::Midday => "Midday".to_string(),
        TimeChoice::GoldenHour => "Golden".to_string(),
        TimeChoice::Night => "Night".to_string(),
        TimeChoice::FixedHour(hour) => {
            let whole_hour = hour as u32;
            let minute = ((hour.fract() * 60.0) as u32).min(59);
            format!("{whole_hour:02}:{minute:02}")
        }
    }
}

struct LayoutCursor {
    top: i32,
    controls: Vec<Control>,
    section_labels: Vec<(i32, &'static str)>,
}

impl LayoutCursor {
    fn add_section_of_chips(&mut self, label: &'static str, columns: i32, chips: Vec<(String, bool, MenuChoice)>) {
        self.section_labels.push((self.top, label));
        self.top += GLYPH_HEIGHT + SPACE_BELOW_A_SECTION_LABEL;
        let chip_width = (CONTENT_WIDTH - (columns - 1) * GAP_BETWEEN_CONTROLS) / columns;
        let rows = (chips.len() as i32 + columns - 1) / columns;
        for (position, (label, is_selected, choice)) in chips.into_iter().enumerate() {
            let column = position as i32 % columns;
            let row = position as i32 / columns;
            self.controls.push(Control {
                left: MARGIN + column * (chip_width + GAP_BETWEEN_CONTROLS),
                top: self.top + row * (CHIP_HEIGHT + GAP_BETWEEN_CONTROLS),
                width: chip_width,
                height: CHIP_HEIGHT,
                label,
                look: Look::Chip { is_selected },
                choice,
            });
        }
        self.top += rows * CHIP_HEIGHT + (rows - 1) * GAP_BETWEEN_CONTROLS + SPACE_AFTER_A_SECTION;
    }

    fn add_toggle(&mut self, label: &str, is_on: bool, choice: MenuChoice) {
        self.controls.push(Control {
            left: MARGIN,
            top: self.top,
            width: CONTENT_WIDTH,
            height: TOGGLE_HEIGHT,
            label: label.to_string(),
            look: Look::Toggle { is_on },
            choice,
        });
        self.top += TOGGLE_HEIGHT + GAP_BETWEEN_CONTROLS;
    }

    fn add_row_of_buttons(&mut self, buttons: [(&str, Look, MenuChoice); 3]) {
        let button_width = (CONTENT_WIDTH - 2 * GAP_BETWEEN_CONTROLS) / 3;
        for (position, (label, look, choice)) in buttons.into_iter().enumerate() {
            self.controls.push(Control {
                left: MARGIN + position as i32 * (button_width + GAP_BETWEEN_CONTROLS),
                top: self.top,
                width: button_width,
                height: CHIP_HEIGHT,
                label: label.to_string(),
                look,
                choice,
            });
        }
        self.top += CHIP_HEIGHT;
    }
}

impl ControlPanel {
    pub fn new(state: &PanelState) -> ControlPanel {
        let config = state.config;
        let preview_top = MARGIN + HEADER_HEIGHT;
        let mut cursor = LayoutCursor {
            top: preview_top + PREVIEW_HEIGHT + 2 + SPACE_AFTER_A_SECTION + 1,
            controls: Vec::new(),
            section_labels: Vec::new(),
        };

        let scene =
            |label: &str, choice: SceneChoice| (label.to_string(), config.scene == choice, MenuChoice::Scene(choice));
        cursor.add_section_of_chips(
            "Scene",
            3,
            vec![
                scene("Chapel", SceneChoice::Chapel),
                scene("East Campus", SceneChoice::East),
                scene("Auto", SceneChoice::Auto),
            ],
        );

        let mut seasons = vec![("Auto".to_string(), config.season.is_none(), MenuChoice::Season(None))];
        for season in Season::ALL {
            seasons.push((season.name().to_string(), config.season == Some(season), MenuChoice::Season(Some(season))));
        }
        cursor.add_section_of_chips("Season", 5, seasons);

        let mut weathers = vec![("Auto".to_string(), config.weather.is_none(), MenuChoice::Weather(None))];
        for weather in Weather::ALL {
            weathers.push((
                weather.name().to_string(),
                config.weather == Some(weather),
                MenuChoice::Weather(Some(weather)),
            ));
        }
        cursor.add_section_of_chips("Weather", 4, weathers);

        let mut times_of_day =
            vec![TimeChoice::RealTime, TimeChoice::Dawn, TimeChoice::Midday, TimeChoice::GoldenHour, TimeChoice::Night];
        if matches!(config.time_of_day, TimeChoice::FixedHour(_)) {
            times_of_day.push(config.time_of_day);
        }
        let times = times_of_day
            .into_iter()
            .map(|time_of_day| {
                (time_label(time_of_day), config.time_of_day == time_of_day, MenuChoice::TimeOfDay(time_of_day))
            })
            .collect();
        cursor.add_section_of_chips("Time of day", 5, times);

        cursor.top += 2;
        cursor.add_toggle("Pause animation", state.is_paused_by_user, MenuChoice::PauseOrResume);
        cursor.add_toggle(START_AT_LOGIN_LABEL, state.starts_at_login, MenuChoice::StartAtLogin);
        cursor.top += GAP_BETWEEN_CONTROLS;
        cursor.add_row_of_buttons([
            ("Edit config", Look::Button, MenuChoice::OpenConfigFile),
            ("Reload", Look::Button, MenuChoice::ReloadConfig),
            ("Quit", Look::QuitButton, MenuChoice::Quit),
        ]);

        let panel_height = cursor.top + MARGIN;
        let weather_shown = match config.weather {
            Some(weather) => weather.name(),
            None => state.weather_on_screen.name(),
        };
        ControlPanel {
            controls: cursor.controls,
            section_labels: cursor.section_labels,
            preview_top,
            subtitle: format!("{} · {} · {}", scene_title(state.scene_on_screen), weather_shown, state.activity),
            hovered_control: None,
            canvas: Canvas::new(PANEL_WIDTH, panel_height),
        }
    }

    pub fn width(&self) -> i32 {
        self.canvas.width
    }

    pub fn height(&self) -> i32 {
        self.canvas.height
    }

    pub fn point_at(&mut self, x: i32, y: i32) -> bool {
        let now_hovered = self.controls.iter().position(|control| control.contains(x, y));
        let hover_changed = now_hovered != self.hovered_control;
        self.hovered_control = now_hovered;
        hover_changed
    }

    pub fn is_pointing_at_a_control(&self) -> bool {
        self.hovered_control.is_some()
    }

    pub fn choice_at(&self, x: i32, y: i32) -> Option<MenuChoice> {
        self.controls.iter().find(|control| control.contains(x, y)).map(|control| control.choice)
    }

    pub fn draw(&mut self, wallpaper_frame: &Canvas) -> &Canvas {
        let canvas = &mut self.canvas;
        let (width, height) = (canvas.width, canvas.height);
        for y in 0..height {
            for x in 0..width {
                let progress_down = y as f32 / (height - 1).max(1) as f32;
                let dither = BAYER_ORDER_FOUR[(y & 3) as usize][(x & 3) as usize] / 16.0;
                let shade = ((progress_down * BACKGROUND_SHADES + dither).floor() / BACKGROUND_SHADES).min(1.0);
                canvas.set_pixel(x, y, blend_colours(BACKGROUND_TOP, BACKGROUND_BOTTOM, shade));
            }
        }
        canvas.horizontal_line(1, width - 2, 0, PANEL_BORDER);
        canvas.horizontal_line(1, width - 2, height - 1, PANEL_BORDER);
        canvas.fill_rectangle(0, 1, 1, height - 2, PANEL_BORDER);
        canvas.fill_rectangle(width - 1, 1, 1, height - 2, PANEL_BORDER);
        canvas.horizontal_line(2, width - 3, 1, PANEL_BORDER_HIGHLIGHT);

        let icon_pixels = chapel_tower_icon_pixels(ICON_SIDE);
        for (index, pixel_with_alpha) in icon_pixels.iter().enumerate() {
            let is_opaque = pixel_with_alpha >> 24 != 0;
            if is_opaque {
                canvas.set_pixel(
                    MARGIN + index as i32 % ICON_SIDE,
                    MARGIN + index as i32 / ICON_SIDE,
                    pixel_with_alpha & 0xFF_FF_FF,
                );
            }
        }
        let text_left = MARGIN + ICON_SIDE + 6;
        draw_text(canvas, text_left, MARGIN, "Duke Pixel Art", TITLE_TEXT);
        draw_text(canvas, text_left, MARGIN + GLYPH_HEIGHT + 3, &self.subtitle, SUBTITLE_TEXT);

        let preview_left = MARGIN;
        let preview_top = self.preview_top;
        canvas.fill_rectangle(preview_left - 1, preview_top - 1, PREVIEW_WIDTH + 2, PREVIEW_HEIGHT + 2, PREVIEW_FRAME);
        canvas.horizontal_line(
            preview_left,
            preview_left + PREVIEW_WIDTH,
            preview_top + PREVIEW_HEIGHT + 1,
            PREVIEW_FRAME_SHADOW,
        );
        for y in 0..PREVIEW_HEIGHT {
            let source_y = y * wallpaper_frame.height / PREVIEW_HEIGHT;
            for x in 0..PREVIEW_WIDTH {
                let source_x = x * wallpaper_frame.width / PREVIEW_WIDTH;
                canvas.set_pixel(preview_left + x, preview_top + y, wallpaper_frame.pixel_at(source_x, source_y));
            }
        }

        for &(top, label) in &self.section_labels {
            draw_text(canvas, MARGIN, top, label, SECTION_LABEL_TEXT);
            let rule_starts_at = MARGIN + text_width(label) + 4;
            canvas.horizontal_line(rule_starts_at, MARGIN + CONTENT_WIDTH - 1, top + GLYPH_HEIGHT / 2, SECTION_RULE);
        }

        for (index, control) in self.controls.iter().enumerate() {
            let is_hovered = self.hovered_control == Some(index);
            match control.look {
                Look::Chip { is_selected } => {
                    let colours = match (is_selected, is_hovered) {
                        (true, true) => HOVERED_SELECTED_CHIP,
                        (true, false) => SELECTED_CHIP,
                        (false, true) => HOVERED_CHIP,
                        (false, false) => RESTING_CHIP,
                    };
                    draw_chip(canvas, control, colours);
                }
                Look::Button => draw_chip(canvas, control, if is_hovered { HOVERED_BUTTON } else { BUTTON }),
                Look::QuitButton => {
                    draw_chip(canvas, control, if is_hovered { HOVERED_QUIT_BUTTON } else { QUIT_BUTTON })
                }
                Look::Toggle { is_on } => draw_toggle(canvas, control, is_on, is_hovered),
            }
        }
        &self.canvas
    }
}

fn draw_chip(canvas: &mut Canvas, control: &Control, colours: ChipColours) {
    let (left, top, width, height) = (control.left, control.top, control.width, control.height);
    let right = left + width - 1;
    let bottom = top + height - 1;
    canvas.fill_rectangle(left, top + 1, width, height - 2, colours.fill);
    canvas.horizontal_line(left + 1, right - 1, top, colours.top_edge);
    canvas.horizontal_line(left + 1, right - 1, bottom, colours.bottom_edge);
    canvas.set_pixel(left, top + 1, colours.top_edge);
    canvas.set_pixel(right, top + 1, colours.top_edge);
    canvas.set_pixel(left, bottom - 1, colours.bottom_edge);
    canvas.set_pixel(right, bottom - 1, colours.bottom_edge);
    let text_left = left + (width - text_width(&control.label)) / 2;
    let text_top = top + (height - GLYPH_HEIGHT) / 2;
    draw_text(canvas, text_left, text_top, &control.label, colours.text);
}

fn draw_toggle(canvas: &mut Canvas, control: &Control, is_on: bool, is_hovered: bool) {
    let box_left = control.left;
    let box_top = control.top + (control.height - TOGGLE_BOX_SIDE) / 2;
    let box_colour = if is_on { TOGGLE_ON_BOX } else { TOGGLE_OFF_BOX };
    let edge_colour = if is_hovered { HOVERED_CHIP.top_edge } else { RESTING_CHIP.top_edge };
    canvas.fill_rectangle(box_left, box_top, TOGGLE_BOX_SIDE, TOGGLE_BOX_SIDE, edge_colour);
    canvas.fill_rectangle(box_left + 1, box_top + 1, TOGGLE_BOX_SIDE - 2, TOGGLE_BOX_SIDE - 2, box_colour);
    if is_on {
        for (row, cells) in TICK_ART.iter().enumerate() {
            for (column, cell) in cells.bytes().enumerate() {
                if cell == b'#' {
                    canvas.set_pixel(box_left + 1 + column as i32, box_top + 1 + row as i32, TOGGLE_TICK);
                }
            }
        }
    }
    let text_colour = if is_hovered { HOVERED_TOGGLE_TEXT } else { TOGGLE_TEXT };
    draw_text(
        canvas,
        box_left + TOGGLE_BOX_SIDE + 5,
        control.top + (control.height - GLYPH_HEIGHT) / 2,
        &control.label,
        text_colour,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn panel_for(config: &Config) -> ControlPanel {
        ControlPanel::new(&PanelState {
            config,
            scene_on_screen: CampusScene::Chapel,
            weather_on_screen: Weather::Clear,
            activity: "20 fps",
            is_paused_by_user: false,
            starts_at_login: true,
        })
    }

    fn centre_of(control: &Control) -> (i32, i32) {
        (control.left + control.width / 2, control.top + control.height / 2)
    }

    #[test]
    fn every_control_fits_inside_the_panel_without_overlapping_another() {
        let config = Config { time_of_day: TimeChoice::FixedHour(14.5), ..Config::default() };
        let panel = panel_for(&config);
        for (index, control) in panel.controls.iter().enumerate() {
            assert!(
                control.left >= MARGIN && control.left + control.width <= PANEL_WIDTH - MARGIN,
                "{}",
                control.label
            );
            assert!(
                control.top >= MARGIN && control.top + control.height <= panel.height() - MARGIN,
                "{}",
                control.label
            );
            assert!(text_width(&control.label) <= control.width - 2, "{} is too wide for its control", control.label);
            for other in &panel.controls[index + 1..] {
                let overlap = control.left < other.left + other.width
                    && other.left < control.left + control.width
                    && control.top < other.top + other.height
                    && other.top < control.top + control.height;
                assert!(!overlap, "{} overlaps {}", control.label, other.label);
            }
        }
    }

    #[test]
    fn clicking_a_control_picks_its_choice_and_every_choice_is_offered() {
        let panel = panel_for(&Config::default());
        for control in &panel.controls {
            let (x, y) = centre_of(control);
            assert_eq!(panel.choice_at(x, y), Some(control.choice));
        }
        let offered: Vec<MenuChoice> = panel.controls.iter().map(|control| control.choice).collect();
        for season in Season::ALL {
            assert!(offered.contains(&MenuChoice::Season(Some(season))));
        }
        for weather in Weather::ALL {
            assert!(offered.contains(&MenuChoice::Weather(Some(weather))));
        }
        for choice in [
            MenuChoice::Scene(SceneChoice::East),
            MenuChoice::TimeOfDay(TimeChoice::Night),
            MenuChoice::PauseOrResume,
            MenuChoice::StartAtLogin,
            MenuChoice::OpenConfigFile,
            MenuChoice::ReloadConfig,
            MenuChoice::Quit,
        ] {
            assert!(offered.contains(&choice), "{choice:?} is missing");
        }
        assert_eq!(panel.choice_at(1, 1), None);
    }

    #[test]
    fn the_current_settings_are_the_selected_chips() {
        let config = Config { scene: SceneChoice::East, weather: Some(Weather::Snow), ..Config::default() };
        let panel = panel_for(&config);
        let selected: Vec<MenuChoice> = panel
            .controls
            .iter()
            .filter(|control| matches!(control.look, Look::Chip { is_selected: true }))
            .map(|control| control.choice)
            .collect();
        assert_eq!(
            selected,
            vec![
                MenuChoice::Scene(SceneChoice::East),
                MenuChoice::Season(None),
                MenuChoice::Weather(Some(Weather::Snow)),
                MenuChoice::TimeOfDay(TimeChoice::RealTime)
            ]
        );
    }

    #[test]
    fn hovering_reports_changes_only_and_lights_the_control_under_the_pointer() {
        let mut panel = panel_for(&Config::default());
        let quit = panel.controls.iter().find(|control| control.choice == MenuChoice::Quit).unwrap().clone();
        let (x, y) = centre_of(&quit);
        assert!(panel.point_at(x, y));
        assert!(!panel.point_at(x, y));
        assert!(panel.is_pointing_at_a_control());
        let frame = Canvas::full_size();
        let canvas = panel.draw(&frame);
        assert_eq!(canvas.pixel_at(quit.left + 1, quit.top + 2), HOVERED_QUIT_BUTTON.fill);
        assert!(panel.point_at(0, 0));
        assert!(!panel.is_pointing_at_a_control());
    }

    #[test]
    fn the_preview_shows_the_wallpaper_frame() {
        let mut panel = panel_for(&Config::default());
        let mut frame = Canvas::full_size();
        frame.clear(0x12_34_56);
        let preview_top = panel.preview_top;
        let canvas = panel.draw(&frame);
        assert_eq!(canvas.pixel_at(MARGIN + 10, preview_top + 10), 0x12_34_56);
    }
}
