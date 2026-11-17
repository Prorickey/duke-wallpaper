use crate::atmosphere::{compose_frame, Atmosphere, Controls, WallClock};
use crate::wallpaper::clock::season_of_month;
use crate::wallpaper::config::{Config, SceneChoice};
use crate::{Canvas, Environment, Scene};

pub const LONGEST_SIMULATION_STEP_SECONDS: f32 = 0.25;
const SHORTEST_DISSOLVE_SECONDS: f32 = 0.05;
const SECONDS_PER_MINUTE: f64 = 60.0;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum CampusScene {
    Chapel,
    EastCampus,
}

impl CampusScene {
    pub const ALL: [CampusScene; 2] = [CampusScene::Chapel, CampusScene::EastCampus];

    pub fn name(self) -> &'static str {
        match self {
            CampusScene::Chapel => "chapel",
            CampusScene::EastCampus => "east",
        }
    }

    pub fn named(name: &str) -> Option<CampusScene> {
        match SceneChoice::parse(name)? {
            SceneChoice::Chapel => Some(CampusScene::Chapel),
            SceneChoice::East => Some(CampusScene::EastCampus),
            SceneChoice::Auto => None,
        }
    }

    pub fn build(self) -> Box<dyn Scene> {
        match self {
            CampusScene::Chapel => Box::new(crate::chapel::ChapelScene::new()),
            CampusScene::EastCampus => Box::new(crate::east::EastCampusScene::new()),
        }
    }
}

fn scene_pinned_by(config: &Config) -> Option<CampusScene> {
    match config.scene {
        SceneChoice::Chapel => Some(CampusScene::Chapel),
        SceneChoice::East => Some(CampusScene::EastCampus),
        SceneChoice::Auto => None,
    }
}

pub struct Engine {
    config: Config,
    chapel_scene: Option<Box<dyn Scene>>,
    east_campus_scene: Option<Box<dyn Scene>>,
    atmosphere: Atmosphere,
    pub scene_on_screen: CampusScene,
    pub frame: Canvas,
    incoming_scene_frame: Canvas,
    outgoing_scene_snapshot: Canvas,
    dissolve_progress: Option<f32>,
    seconds_this_scene_has_been_showing: f64,
    pub environment: Environment,
    frames_rendered: u64,
}

impl Engine {
    pub fn new(config: &Config) -> Self {
        Engine {
            config: config.clone(),
            chapel_scene: None,
            east_campus_scene: None,
            atmosphere: Atmosphere::new(config.world_seed),
            scene_on_screen: scene_pinned_by(config).unwrap_or(CampusScene::Chapel),
            frame: Canvas::full_size(),
            incoming_scene_frame: Canvas::full_size(),
            outgoing_scene_snapshot: Canvas::full_size(),
            dissolve_progress: None,
            seconds_this_scene_has_been_showing: 0.0,
            environment: Environment::default(),
            frames_rendered: 0,
        }
    }

    pub fn use_config(&mut self, new_config: &Config) {
        if new_config.world_seed != self.config.world_seed {
            self.atmosphere = Atmosphere::new(new_config.world_seed);
        }
        if new_config.scene != self.config.scene {
            self.seconds_this_scene_has_been_showing = 0.0;
        }
        self.config = new_config.clone();
        if let Some(pinned_scene) = scene_pinned_by(new_config) {
            self.dissolve_to(pinned_scene);
        }
    }

    pub fn dissolve_to(&mut self, scene: CampusScene) {
        if scene == self.scene_on_screen {
            return;
        }
        let there_is_a_picture_to_dissolve_from = self.frames_rendered > 0;
        if there_is_a_picture_to_dissolve_from && self.config.transition_seconds > 0.0 {
            self.outgoing_scene_snapshot.pixels.copy_from_slice(&self.frame.pixels);
            self.dissolve_progress = Some(0.0);
        } else {
            self.dissolve_progress = None;
        }
        self.scene_on_screen = scene;
        self.seconds_this_scene_has_been_showing = 0.0;
    }

    pub fn has_rendered_a_frame(&self) -> bool {
        self.frames_rendered > 0
    }

    #[cfg(test)]
    fn is_dissolving(&self) -> bool {
        self.dissolve_progress.is_some()
    }

    pub fn render_next_frame(&mut self, seconds_since_last_frame: f32, clock: &WallClock) -> &Canvas {
        let seconds_to_simulate = if seconds_since_last_frame.is_finite() {
            seconds_since_last_frame.clamp(0.0, LONGEST_SIMULATION_STEP_SECONDS)
        } else {
            0.0
        };

        let cycling_between_scenes = self.config.scene == SceneChoice::Auto;
        if cycling_between_scenes && self.dissolve_progress.is_none() {
            self.seconds_this_scene_has_been_showing += seconds_to_simulate as f64;
            let seconds_each_scene_is_shown = self.config.cycle_minutes as f64 * SECONDS_PER_MINUTE;
            if self.seconds_this_scene_has_been_showing >= seconds_each_scene_is_shown {
                let the_other_scene = match self.scene_on_screen {
                    CampusScene::Chapel => CampusScene::EastCampus,
                    CampusScene::EastCampus => CampusScene::Chapel,
                };
                self.dissolve_to(the_other_scene);
            }
        }

        let season_that_decides_dawn_and_dusk = match self.config.season {
            Some(pinned_season) => pinned_season,
            None if self.frames_rendered > 0 => self.environment.season,
            None => season_of_month(clock.month),
        };
        let controls = Controls {
            season: self.config.season,
            weather: self.config.weather,
            hour: self.config.time_of_day.pinned_hour(season_that_decides_dawn_and_dusk),
            weather_spell_minutes: self.config.weather_minutes,
        };
        self.environment = self.atmosphere.step(seconds_to_simulate, clock, &controls);

        let scene_on_screen = self.scene_on_screen;
        let built_scene = match scene_on_screen {
            CampusScene::Chapel => &mut self.chapel_scene,
            CampusScene::EastCampus => &mut self.east_campus_scene,
        };
        let scene = built_scene.get_or_insert_with(|| scene_on_screen.build());

        match self.dissolve_progress {
            None => compose_frame(scene.as_mut(), &mut self.atmosphere, &self.environment, &mut self.frame),
            Some(progress) => {
                compose_frame(scene.as_mut(), &mut self.atmosphere, &self.environment, &mut self.incoming_scene_frame);
                dither_dissolve(&self.outgoing_scene_snapshot, &self.incoming_scene_frame, progress, &mut self.frame);
                let dissolve_seconds = self.config.transition_seconds.max(SHORTEST_DISSOLVE_SECONDS);
                let progress_after_this_frame = progress + seconds_to_simulate / dissolve_seconds;
                self.dissolve_progress =
                    if progress_after_this_frame < 1.0 { Some(progress_after_this_frame) } else { None };
            }
        }
        self.frames_rendered += 1;
        &self.frame
    }
}

const BAYER_FOUR_BY_FOUR_THRESHOLDS: [[u8; 4]; 4] = [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]];
const THRESHOLDS_IN_THE_BAYER_MATRIX: f32 = 16.0;
const SWEEPING_BAND_WIDTH_AS_FRACTION_OF_CANVAS: f32 = 0.5;

pub fn dither_dissolve(outgoing: &Canvas, incoming: &Canvas, progress: f32, output: &mut Canvas) {
    let width = output.width as usize;
    let height = output.height as usize;
    debug_assert!(outgoing.pixels.len() == width * height && incoming.pixels.len() == width * height);

    let progress = progress.clamp(0.0, 1.0);
    let eased_progress = progress * progress * (3.0 - 2.0 * progress);
    let band_leading_edge = eased_progress * (1.0 + SWEEPING_BAND_WIDTH_AS_FRACTION_OF_CANVAS);

    let mut thresholds_showing_incoming_per_column: Vec<u8> = Vec::with_capacity(width);
    for column in 0..width {
        let column_position = column as f32 / width as f32;
        let how_far_the_band_has_passed =
            (band_leading_edge - column_position) / SWEEPING_BAND_WIDTH_AS_FRACTION_OF_CANVAS;
        let thresholds_passed = (how_far_the_band_has_passed.clamp(0.0, 1.0) * THRESHOLDS_IN_THE_BAYER_MATRIX).round();
        thresholds_showing_incoming_per_column.push(thresholds_passed as u8);
    }

    for row in 0..height {
        let thresholds_for_this_row = &BAYER_FOUR_BY_FOUR_THRESHOLDS[row % 4];
        let first_pixel_of_row = row * width;
        for column in 0..width {
            let pixel = first_pixel_of_row + column;
            let incoming_shows_here =
                thresholds_showing_incoming_per_column[column] > thresholds_for_this_row[column % 4];
            output.pixels[pixel] = if incoming_shows_here { incoming.pixels[pixel] } else { outgoing.pixels[pixel] };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DARK: u32 = 0x111111;
    const LIGHT: u32 = 0xEEEEEE;

    fn pixels_showing_incoming(progress: f32) -> usize {
        let mut outgoing = Canvas::full_size();
        let mut incoming = Canvas::full_size();
        outgoing.clear(DARK);
        incoming.clear(LIGHT);
        let mut output = Canvas::full_size();
        dither_dissolve(&outgoing, &incoming, progress, &mut output);
        assert!(output.pixels.iter().all(|&pixel| pixel == DARK || pixel == LIGHT), "no blended colours");
        output.pixels.iter().filter(|&&pixel| pixel == LIGHT).count()
    }

    #[test]
    fn dissolve_goes_from_all_outgoing_to_all_incoming_without_going_back() {
        let pixel_count = (crate::CANVAS_WIDTH * crate::CANVAS_HEIGHT) as usize;
        assert_eq!(pixels_showing_incoming(0.0), 0);
        assert_eq!(pixels_showing_incoming(1.0), pixel_count);
        let halfway = pixels_showing_incoming(0.5);
        assert!(halfway > pixel_count / 4 && halfway < pixel_count * 3 / 4);
        let mut showing_at_previous_step = 0;
        for step in 0..=20 {
            let showing = pixels_showing_incoming(step as f32 / 20.0);
            assert!(showing >= showing_at_previous_step);
            showing_at_previous_step = showing;
        }
    }

    #[test]
    fn auto_cycle_dissolves_to_the_other_scene_and_pinning_switches_back() {
        let just_under_a_second_per_scene = Config {
            scene: SceneChoice::Auto,
            cycle_minutes: 0.9 / 60.0,
            transition_seconds: 1.0,
            ..Config::default()
        };
        let mut engine = Engine::new(&just_under_a_second_per_scene);
        let clock = WallClock { month: 10, day: 15, hour: 13.0 };
        assert_eq!(engine.scene_on_screen, CampusScene::Chapel);
        for _ in 0..5 {
            engine.render_next_frame(0.25, &clock);
        }
        assert_eq!(engine.scene_on_screen, CampusScene::EastCampus);
        assert!(engine.is_dissolving());
        for _ in 0..3 {
            engine.render_next_frame(0.25, &clock);
        }
        assert!(!engine.is_dissolving());

        let chapel_pinned = Config { scene: SceneChoice::Chapel, ..just_under_a_second_per_scene };
        engine.use_config(&chapel_pinned);
        assert_eq!(engine.scene_on_screen, CampusScene::Chapel);
        assert!(engine.is_dissolving());
    }

    #[test]
    fn a_zero_second_transition_is_a_hard_cut() {
        let hard_cut = Config { scene: SceneChoice::Chapel, transition_seconds: 0.0, ..Config::default() };
        let mut engine = Engine::new(&hard_cut);
        let clock = WallClock { month: 6, day: 1, hour: 12.0 };
        engine.render_next_frame(0.05, &clock);
        engine.dissolve_to(CampusScene::EastCampus);
        assert_eq!(engine.scene_on_screen, CampusScene::EastCampus);
        assert!(!engine.is_dissolving());
    }

    #[test]
    fn absurd_elapsed_times_are_harmless() {
        let mut engine = Engine::new(&Config::default());
        let clock = WallClock { month: 1, day: 1, hour: 0.0 };
        assert!(!engine.has_rendered_a_frame());
        for seconds_since_last_frame in [1.0e6, f32::NAN, f32::INFINITY, -5.0] {
            engine.render_next_frame(seconds_since_last_frame, &clock);
        }
        assert!(engine.has_rendered_a_frame());
        assert_eq!(engine.frame.pixels.len(), (crate::CANVAS_WIDTH * crate::CANVAS_HEIGHT) as usize);
    }
}
