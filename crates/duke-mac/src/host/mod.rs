mod desktop_windows;
mod images;
mod panel_window;
mod system;

use desktop_windows::{screens_now, DesktopWindow, Screen};
use duke_core::wallpaper::config::{self, Config, LOG_FILE_NAME};
use duke_core::wallpaper::control_panel::{ControlPanel, PanelState};
use duke_core::wallpaper::engine::Engine;
use duke_core::wallpaper::logging;
use duke_core::wallpaper::menu::{self, MenuChoice, PROGRAM_TITLE};
use duke_core::wallpaper::pausing::PauseReasons;
use duke_core::{info, warn};
use objc2::rc::Retained;
use objc2::MainThreadMarker;
use objc2_app_kit::{
    NSApplication, NSApplicationActivationPolicy, NSEvent, NSEventMask, NSEventType, NSStatusBar, NSStatusItem,
    NSVariableStatusItemLength,
};
use objc2_foundation::{NSDate, NSDefaultRunLoopMode, NSRect, NSString};
use panel_window::{pointer_is_inside, PanelWindow};
use std::path::PathBuf;
use std::time::{Duration, Instant};

const HOUSEKEEPING_INTERVAL: Duration = Duration::from_millis(500);
const INTERVAL_BETWEEN_ASKING_ABOUT_LOCK_SLEEP_AND_POWER: Duration = Duration::from_secs(5);
const LOCK_FILE_NAME: &str = "duke-wallpaper.lock";
const EXIT_CODE_SUCCESS: i32 = 0;
const EXIT_CODE_FAILURE: i32 = 1;

struct Wallpaper {
    mtm: MainThreadMarker,
    config: Config,
    config_path: PathBuf,
    engine: Engine,
    screens: Vec<Screen>,
    desktop_windows: Vec<DesktopWindow>,
    status_item: Retained<NSStatusItem>,
    panel_window: Option<PanelWindow>,
    pause_reasons: PauseReasons,
    frame_interval: Duration,
    next_frame_at: Instant,
    previous_frame_at: Instant,
    next_housekeeping_at: Instant,
    ask_about_lock_sleep_and_power_at: Instant,
    was_rendering: bool,
    quit_requested: bool,
}

impl Wallpaper {
    fn status_item_frame(&self) -> Option<NSRect> {
        Some(self.status_item.button(self.mtm)?.window()?.frame())
    }

    fn control_panel_showing_the_current_settings(&self) -> ControlPanel {
        let activity = if self.pause_reasons.paused_by_user {
            "paused".to_string()
        } else {
            format!("{} fps", self.config.frames_per_second)
        };
        ControlPanel::new(&PanelState {
            config: &self.config,
            scene_on_screen: self.engine.scene_on_screen,
            weather_on_screen: self.engine.environment.weather,
            activity: &activity,
            is_paused_by_user: self.pause_reasons.paused_by_user,
            starts_at_login: system::opens_at_login(),
        })
    }

    fn open_or_close_the_control_panel(&mut self) {
        if self.panel_window.take().is_some() {
            return;
        }
        let Some(status_item_frame) = self.status_item_frame() else {
            return;
        };
        let panel = self.control_panel_showing_the_current_settings();
        let Some(mut panel_window) = PanelWindow::open(panel, status_item_frame, self.mtm) else {
            warn!("could not open the control panel");
            return;
        };
        #[allow(deprecated)]
        NSApplication::sharedApplication(self.mtm).activateIgnoringOtherApps(true);
        panel_window.redraw(&self.engine.frame);
        self.panel_window = Some(panel_window);
    }

    fn refresh_the_control_panel(&mut self) {
        let panel = self.control_panel_showing_the_current_settings();
        if let Some(panel_window) = self.panel_window.as_mut() {
            panel_window.replace_panel(panel);
            panel_window.redraw(&self.engine.frame);
        }
    }

    fn use_config(&mut self, new_config: Config) {
        self.engine.use_config(&new_config);
        self.frame_interval = Duration::from_secs_f64(1.0 / new_config.frames_per_second.max(1) as f64);
        self.config = new_config;
        for desktop_window in &mut self.desktop_windows {
            desktop_window.arrange(self.config.fit, self.config.letterbox_colour);
            if !self.config.pause_on_fullscreen {
                desktop_window.is_covered = false;
            }
        }
    }

    fn apply_menu_choice(&mut self, choice: MenuChoice) {
        info!("control panel: {choice:?}");
        match choice {
            MenuChoice::PauseOrResume => self.pause_reasons.paused_by_user = !self.pause_reasons.paused_by_user,
            MenuChoice::Quit => self.quit_requested = true,
            MenuChoice::StartAtLogin => {
                let enable = !system::opens_at_login();
                match system::set_opens_at_login(enable) {
                    Ok(()) => info!("open at login {}", if enable { "enabled" } else { "disabled" }),
                    Err(problem) => warn!("open at login: {problem}"),
                }
            }
            MenuChoice::OpenConfigFile => system::open_in_text_editor(&self.config_path),
            MenuChoice::ReloadConfig => {
                if let Some(reloaded_config) = menu::config_reloaded_from(&self.config_path) {
                    self.use_config(reloaded_config);
                }
            }
            MenuChoice::Scene(_) | MenuChoice::Season(_) | MenuChoice::Weather(_) | MenuChoice::TimeOfDay(_) => {
                if let Some(changed_config) = menu::config_saved_after_choice(&self.config_path, &self.config, choice) {
                    self.use_config(changed_config);
                }
            }
        }
    }

    fn handle_event_before_appkit_sees_it(&mut self, event: &NSEvent) -> bool {
        let event_type = event.r#type();
        if event_type == NSEventType::LeftMouseDown || event_type == NSEventType::RightMouseDown {
            let clicked_the_status_item = self.status_item_frame().is_some_and(pointer_is_inside);
            if clicked_the_status_item {
                self.open_or_close_the_control_panel();
                return true;
            }
            let clicked_inside_the_panel =
                self.panel_window.as_ref().is_some_and(|panel_window| pointer_is_inside(panel_window.frame()));
            if self.panel_window.is_some() && !clicked_inside_the_panel {
                self.panel_window = None;
            }
            return clicked_inside_the_panel;
        }
        if event_type == NSEventType::LeftMouseUp {
            let Some(choice) =
                self.panel_window.as_ref().and_then(|panel_window| panel_window.choice_under_the_pointer())
            else {
                return false;
            };
            if matches!(choice, MenuChoice::Quit | MenuChoice::OpenConfigFile) {
                self.panel_window = None;
            }
            self.apply_menu_choice(choice);
            self.refresh_the_control_panel();
            return true;
        }
        if event_type == NSEventType::KeyDown && self.panel_window.is_some() {
            const ESCAPE_KEY: u16 = 53;
            if event.keyCode() == ESCAPE_KEY {
                self.panel_window = None;
                return true;
            }
        }
        false
    }

    fn keep_a_window_on_every_screen(&mut self) {
        let screens = screens_now(self.mtm);
        if screens == self.screens && !self.desktop_windows.is_empty() {
            return;
        }
        info!("covering {} screen(s)", screens.len());
        self.desktop_windows.clear();
        for screen in &screens {
            match DesktopWindow::cover(*screen, self.config.fit, self.config.letterbox_colour, self.mtm) {
                Some(desktop_window) => self.desktop_windows.push(desktop_window),
                None => warn!("could not cover the screen at {:?}", screen.frame),
            }
        }
        self.screens = screens;
        for desktop_window in &self.desktop_windows {
            desktop_window.show(&self.engine.frame);
        }
    }

    fn take_one_turn(&mut self, now: Instant) -> Instant {
        if now >= self.ask_about_lock_sleep_and_power_at {
            self.ask_about_lock_sleep_and_power_at = now + INTERVAL_BETWEEN_ASKING_ABOUT_LOCK_SLEEP_AND_POWER;
            self.pause_reasons.session_is_locked = system::screen_is_locked();
            self.pause_reasons.display_is_off = system::main_display_is_asleep();
            self.pause_reasons.running_on_battery = system::running_on_battery();
        }
        if now >= self.next_housekeeping_at {
            self.next_housekeeping_at = now + HOUSEKEEPING_INTERVAL;
            self.keep_a_window_on_every_screen();
            for desktop_window in &mut self.desktop_windows {
                desktop_window.is_covered = self.config.pause_on_fullscreen && !desktop_window.can_be_seen();
            }
        }

        let app_is_active = NSApplication::sharedApplication(self.mtm).isActive();
        if let Some(panel_window) = self.panel_window.as_mut() {
            if app_is_active {
                panel_window.app_has_been_active_since_opening = true;
            }
            let another_app_took_over = panel_window.app_has_been_active_since_opening && !app_is_active;
            if another_app_took_over {
                self.panel_window = None;
            } else if panel_window.follow_the_pointer() {
                panel_window.redraw(&self.engine.frame);
            }
        }

        let reason_not_to_render = self.pause_reasons.reason_not_to_render(self.config.pause_on_battery);
        let some_window_can_be_seen = self.desktop_windows.iter().any(|desktop_window| !desktop_window.is_covered);
        let should_render = reason_not_to_render.is_none() && (some_window_can_be_seen || self.panel_window.is_some());
        if should_render && !self.was_rendering {
            self.previous_frame_at = now;
            self.next_frame_at = now;
        }
        if should_render != self.was_rendering {
            info!(
                "{PROGRAM_TITLE} - {}",
                reason_not_to_render.unwrap_or(if should_render { "running" } else { "idle (screens covered)" })
            );
        }
        self.was_rendering = should_render;

        if should_render && now >= self.next_frame_at {
            let seconds_since_last_frame = now.saturating_duration_since(self.previous_frame_at).as_secs_f32();
            self.previous_frame_at = now;
            self.engine.render_next_frame(seconds_since_last_frame, &system::local_time_now().as_atmosphere_clock());
            for desktop_window in &self.desktop_windows {
                desktop_window.show(&self.engine.frame);
            }
            if let Some(panel_window) = self.panel_window.as_mut() {
                panel_window.redraw(&self.engine.frame);
            }
            self.next_frame_at += self.frame_interval;
            if self.next_frame_at <= Instant::now() {
                self.next_frame_at = Instant::now() + self.frame_interval;
            }
        }

        let mut wake_at = self.ask_about_lock_sleep_and_power_at.min(self.next_housekeeping_at);
        if should_render {
            wake_at = wake_at.min(self.next_frame_at);
        }
        if self.panel_window.is_some() {
            wake_at = wake_at.min(now + self.frame_interval);
        }
        wake_at
    }
}

fn panicked_while_running(work: impl FnOnce()) -> bool {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(work)).is_err()
}

pub fn run() -> i32 {
    let Some(mtm) = MainThreadMarker::new() else {
        return EXIT_CODE_FAILURE;
    };
    let config_path = config::default_config_path();
    logging::use_clock(system::local_time_now);
    logging::start(Some(&config_path.with_file_name(LOG_FILE_NAME)), false);
    logging::record_panics_in_the_log();
    let Some(_single_instance) = system::claim_single_instance(&config_path.with_file_name(LOCK_FILE_NAME)) else {
        info!("another copy is already running; this one is exiting");
        return EXIT_CODE_SUCCESS;
    };
    info!("duke-wallpaper {} starting on macOS", env!("CARGO_PKG_VERSION"));

    let app = NSApplication::sharedApplication(mtm);
    app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);
    app.finishLaunching();

    let from_disk = config::load_or_create_with_defaults(&config_path);
    info!("config: {}", config_path.display());
    for warning in &from_disk.warnings {
        warn!("config: {warning}");
    }
    let config = from_disk.config;

    let status_item = NSStatusBar::systemStatusBar().statusItemWithLength(NSVariableStatusItemLength);
    if let Some(button) = status_item.button(mtm) {
        match images::chapel_tower_menu_bar_icon() {
            Some(icon) => button.setImage(Some(&icon)),
            None => button.setTitle(&NSString::from_str("Duke")),
        }
        button.setToolTip(Some(&NSString::from_str(PROGRAM_TITLE)));
    }

    let now = Instant::now();
    let mut wallpaper = Wallpaper {
        mtm,
        engine: Engine::new(&config),
        frame_interval: Duration::from_secs_f64(1.0 / config.frames_per_second.max(1) as f64),
        config,
        config_path,
        screens: Vec::new(),
        desktop_windows: Vec::new(),
        status_item,
        panel_window: None,
        pause_reasons: PauseReasons::default(),
        next_frame_at: now,
        previous_frame_at: now,
        next_housekeeping_at: now,
        ask_about_lock_sleep_and_power_at: now,
        was_rendering: false,
        quit_requested: false,
    };
    wallpaper.engine.render_next_frame(0.0, &system::local_time_now().as_atmosphere_clock());

    let stopped_by_a_panic = panicked_while_running(|| {
        while !wallpaper.quit_requested {
            let wake_at = wallpaper.take_one_turn(Instant::now());
            let seconds_to_wait = wake_at.saturating_duration_since(Instant::now()).as_secs_f64();
            let mut wait_until = NSDate::dateWithTimeIntervalSinceNow(seconds_to_wait);
            while let Some(event) = app.nextEventMatchingMask_untilDate_inMode_dequeue(
                NSEventMask::Any,
                Some(&wait_until),
                unsafe { NSDefaultRunLoopMode },
                true,
            ) {
                if !wallpaper.handle_event_before_appkit_sees_it(&event) {
                    app.sendEvent(&event);
                }
                wait_until = NSDate::dateWithTimeIntervalSinceNow(0.0);
            }
        }
    });

    info!("shutting down");
    wallpaper.panel_window = None;
    wallpaper.desktop_windows.clear();
    NSStatusBar::systemStatusBar().removeStatusItem(&wallpaper.status_item);
    info!("exited cleanly");
    if stopped_by_a_panic {
        warn!("stopped after an internal error (see the PANIC entry above)");
        return EXIT_CODE_FAILURE;
    }
    EXIT_CODE_SUCCESS
}
