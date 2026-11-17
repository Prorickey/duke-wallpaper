use crate::host::control_panel_window::ControlPanelWindow;
use crate::host::desktop::{self, DesktopLayout};
use crate::host::monitors::{self, Monitor};
use crate::host::pacing::{FrameStatistics, FrameTimer};
use crate::host::power;
use crate::host::tray;
use crate::host::win32::{
    self, describe_rectangle, describe_window_handle, same_rectangle, screen_rectangle_of, wide_null_terminated,
};
use crate::host::{blit, single_instance, start_with_windows};
use duke_core::wallpaper::config::{self, Config};
use duke_core::wallpaper::control_panel::{ControlPanel, PanelState};
use duke_core::wallpaper::engine::Engine;
use duke_core::wallpaper::fit::{self, Placement};
use duke_core::wallpaper::logging;
use duke_core::wallpaper::menu::{self, MenuChoice, PROGRAM_TITLE};
use duke_core::wallpaper::pausing::{PauseReasons, PowerEvent};
use duke_core::{info, warn};
use std::cell::RefCell;
use std::path::PathBuf;
use std::ptr::{null, null_mut};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{BeginPaint, EndPaint, GetDC, ReleaseDC, PAINTSTRUCT};
use windows_sys::Win32::UI::Shell::ShellExecuteW;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyIcon, DestroyWindow, GetAncestor, GetSystemMetrics, IsWindow,
    IsWindowVisible, RegisterWindowMessageW, GA_PARENT, HICON, MA_NOACTIVATE, PBT_APMPOWERSTATUSCHANGE, SM_CXSMICON,
    SPI_SETWORKAREA, SW_SHOWNORMAL, WM_CLOSE, WM_CONTEXTMENU, WM_DESTROY, WM_DISPLAYCHANGE, WM_DPICHANGED,
    WM_ENDSESSION, WM_ERASEBKGND, WM_MOUSEACTIVATE, WM_PAINT, WM_POWERBROADCAST, WM_QUERYENDSESSION, WM_SETTINGCHANGE,
    WM_USER, WM_WTSSESSION_CHANGE, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_POPUP,
};

const HOUSEKEEPING_INTERVAL: Duration = Duration::from_millis(500);
const HOUSEKEEPING_TURNS_BETWEEN_DESKTOP_CHECKS: u64 = 4;
const INTERVAL_BETWEEN_ASKING_WINDOWS_ABOUT_LOCK_AND_POWER: Duration = Duration::from_secs(5);
const INTERVAL_BETWEEN_STATISTICS_IN_THE_LOG: Duration = Duration::from_secs(600);
const TIME_EXPLORER_NEEDS_TO_REARRANGE_AFTER_A_DISPLAY_CHANGE: Duration = Duration::from_millis(400);
const TIME_A_NEW_EXPLORER_NEEDS_TO_BUILD_ITS_DESKTOP: Duration = Duration::from_millis(1500);
const TIME_THE_DESKTOP_NEEDS_TO_SETTLE_AFTER_WAKING: Duration = Duration::from_millis(1500);
const DELAY_BEFORE_REPLACING_A_DESTROYED_WINDOW: Duration = Duration::from_millis(1000);
const DELAY_BEFORE_TRYING_TO_ATTACH_AGAIN: Duration = Duration::from_secs(3);
const TRAY_ICON_CHOSEN_WITH_THE_MOUSE: u32 = WM_USER;
const TRAY_ICON_CHOSEN_WITH_THE_KEYBOARD: u32 = WM_USER + 1;
const LOW_WORD: u32 = 0xFFFF;
const SHELL_EXECUTE_RESULTS_UP_TO_THIS_ARE_ERRORS: usize = 32;
const EXIT_CODE_SUCCESS: i32 = 0;
const EXIT_CODE_FAILURE: i32 = 1;
const MESSAGE_WAS_HANDLED: LRESULT = 0;
const TIME_A_CLICK_ON_THE_TRAY_ICON_TAKES_TO_ARRIVE_AFTER_CLOSING_THE_PANEL: Duration = Duration::from_millis(400);
const BACKGROUND_NEEDS_NO_ERASING: LRESULT = 1;
const SESSION_MAY_END: LRESULT = 1;
const POWER_BROADCAST_ACCEPTED: LRESULT = 1;

#[derive(Copy, Clone, Debug)]
pub(in crate::host) enum WindowEvent {
    QuitRequested,
    ExplorerAnnouncedANewTaskbar,
    DisplayLayoutChanged,
    WallpaperWindowNeedsRepainting,
    WindowDestroyed(isize),
    Power(PowerEvent),
    PowerSourceChanged,
    TrayIconClicked,
    PanelPointerMoved(i32, i32),
    PanelPointerLeft,
    PanelClicked(i32, i32),
    PanelDismissed,
    PanelNeedsPainting,
}

thread_local! {
    static WINDOW_EVENTS_FOR_THE_MAIN_LOOP: RefCell<Vec<WindowEvent>> = const { RefCell::new(Vec::new()) };
}

static MESSAGE_EXPLORER_BROADCASTS_WHEN_IT_CREATES_A_TASKBAR: AtomicU32 = AtomicU32::new(0);

pub(in crate::host) fn queue_for_the_main_loop(event: WindowEvent) {
    WINDOW_EVENTS_FOR_THE_MAIN_LOOP.with(|events| events.borrow_mut().push(event));
}

unsafe extern "system" fn turn_window_messages_into_events(
    window: HWND,
    message: u32,
    first_argument: WPARAM,
    second_argument: LPARAM,
) -> LRESULT {
    match message {
        WM_CLOSE => {
            queue_for_the_main_loop(WindowEvent::QuitRequested);
            MESSAGE_WAS_HANDLED
        }
        WM_QUERYENDSESSION => SESSION_MAY_END,
        WM_ENDSESSION => {
            let session_is_really_ending = first_argument != 0;
            if session_is_really_ending {
                queue_for_the_main_loop(WindowEvent::QuitRequested);
            }
            MESSAGE_WAS_HANDLED
        }
        WM_DISPLAYCHANGE | WM_DPICHANGED => {
            queue_for_the_main_loop(WindowEvent::DisplayLayoutChanged);
            MESSAGE_WAS_HANDLED
        }
        WM_SETTINGCHANGE => {
            let work_area_changed = first_argument as u32 == SPI_SETWORKAREA;
            if work_area_changed {
                queue_for_the_main_loop(WindowEvent::DisplayLayoutChanged);
            }
            MESSAGE_WAS_HANDLED
        }
        WM_POWERBROADCAST => {
            if first_argument as u32 == PBT_APMPOWERSTATUSCHANGE {
                queue_for_the_main_loop(WindowEvent::PowerSourceChanged);
            } else if let Some(event) =
                unsafe { power::power_event_in_power_broadcast(first_argument, second_argument) }
            {
                queue_for_the_main_loop(WindowEvent::Power(event));
            }
            POWER_BROADCAST_ACCEPTED
        }
        WM_WTSSESSION_CHANGE => {
            if let Some(event) = power::power_event_in_session_change(first_argument) {
                queue_for_the_main_loop(WindowEvent::Power(event));
            }
            MESSAGE_WAS_HANDLED
        }
        tray::MESSAGE_SENT_WHEN_THE_TRAY_ICON_IS_USED => {
            let what_the_user_did = second_argument as u32 & LOW_WORD;
            let wants_the_menu = what_the_user_did == WM_CONTEXTMENU
                || what_the_user_did == TRAY_ICON_CHOSEN_WITH_THE_MOUSE
                || what_the_user_did == TRAY_ICON_CHOSEN_WITH_THE_KEYBOARD;
            if wants_the_menu {
                queue_for_the_main_loop(WindowEvent::TrayIconClicked);
            }
            MESSAGE_WAS_HANDLED
        }
        WM_PAINT => {
            unsafe {
                let mut paint: PAINTSTRUCT = std::mem::zeroed();
                BeginPaint(window, &mut paint);
                EndPaint(window, &paint);
            }
            queue_for_the_main_loop(WindowEvent::WallpaperWindowNeedsRepainting);
            MESSAGE_WAS_HANDLED
        }
        WM_ERASEBKGND => BACKGROUND_NEEDS_NO_ERASING,
        WM_MOUSEACTIVATE => MA_NOACTIVATE as LRESULT,
        WM_DESTROY => {
            queue_for_the_main_loop(WindowEvent::WindowDestroyed(window as isize));
            MESSAGE_WAS_HANDLED
        }
        _ => {
            let taskbar_created = MESSAGE_EXPLORER_BROADCASTS_WHEN_IT_CREATES_A_TASKBAR.load(Ordering::Relaxed);
            if taskbar_created != 0 && message == taskbar_created {
                queue_for_the_main_loop(WindowEvent::ExplorerAnnouncedANewTaskbar);
                return MESSAGE_WAS_HANDLED;
            }
            unsafe { DefWindowProcW(window, message, first_argument, second_argument) }
        }
    }
}

fn create_hidden_top_level_window() -> HWND {
    let class_name = wide_null_terminated(single_instance::WINDOW_CLASS_OF_A_RUNNING_INSTANCE);
    let title = wide_null_terminated(PROGRAM_TITLE);
    unsafe {
        CreateWindowExW(
            WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
            class_name.as_ptr(),
            title.as_ptr(),
            WS_POPUP,
            0,
            0,
            0,
            0,
            null_mut(),
            null_mut(),
            win32::this_program(),
            null(),
        )
    }
}

pub(in crate::host) struct WallpaperWindow {
    pub window: HWND,
    pub monitor: Monitor,
    pub placement: Placement,
    pub is_hidden_behind_an_application: bool,
}

pub(in crate::host) struct DesktopAttachment {
    pub layout: DesktopLayout,
    pub progman: HWND,
    pub parent: HWND,
}

enum AfterTurn {
    Quit,
    SleepUntil(Option<Instant>),
}

pub(in crate::host) struct Wallpaper {
    pub config: Config,
    pub config_path: PathBuf,
    pub engine: Engine,
    control_window: HWND,
    pub wallpaper_windows: Vec<WallpaperWindow>,
    pub attachment: Option<DesktopAttachment>,
    tray_icon: HICON,
    pub tray_icon_is_shown: bool,
    pub pause_reasons: PauseReasons,
    was_rendering: bool,
    status_line_shown: String,
    frame_interval: Duration,
    next_frame_at: Instant,
    previous_frame_at: Instant,
    next_housekeeping_at: Instant,
    housekeeping_turns: u64,
    ask_windows_about_lock_and_power_at: Instant,
    check_desktop_at: Option<Instant>,
    pub statistics: FrameStatistics,
    quit_requested: bool,
    control_panel: Option<ControlPanelWindow>,
    control_panel_closed_at: Option<Instant>,
}

impl Wallpaper {
    pub fn status_line(&self) -> String {
        let every_window_is_hidden =
            self.wallpaper_windows.iter().all(|wallpaper_window| wallpaper_window.is_hidden_behind_an_application);
        let state = match self.pause_reasons.reason_not_to_render(self.config.pause_on_battery) {
            Some(reason) => reason.to_string(),
            None if self.wallpaper_windows.is_empty() => "waiting for the desktop".to_string(),
            None if every_window_is_hidden => "idle (desktop covered)".to_string(),
            None => format!("{} fps", self.config.frames_per_second),
        };
        format!("{PROGRAM_TITLE} - {state}")
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
            starts_at_login: start_with_windows::is_enabled(),
        })
    }

    fn open_or_close_the_control_panel(&mut self, now: Instant) {
        if self.control_panel.take().is_some() {
            self.control_panel_closed_at = Some(now);
            return;
        }
        let this_click_is_the_one_that_just_closed_it = self.control_panel_closed_at.is_some_and(|closed_at| {
            now.saturating_duration_since(closed_at)
                < TIME_A_CLICK_ON_THE_TRAY_ICON_TAKES_TO_ARRIVE_AFTER_CLOSING_THE_PANEL
        });
        if this_click_is_the_one_that_just_closed_it {
            return;
        }
        let Some(mut panel_window) = ControlPanelWindow::open(self.control_panel_showing_the_current_settings()) else {
            warn!("could not open the control panel");
            return;
        };
        panel_window.redraw(&self.engine.frame);
        self.control_panel = Some(panel_window);
    }

    fn refresh_the_control_panel(&mut self) {
        let panel = self.control_panel_showing_the_current_settings();
        if let Some(panel_window) = self.control_panel.as_mut() {
            panel_window.replace_panel(panel);
            panel_window.redraw(&self.engine.frame);
        }
    }

    pub fn window_handles(&self) -> Vec<HWND> {
        self.wallpaper_windows.iter().map(|wallpaper_window| wallpaper_window.window).collect()
    }

    fn destroy_wallpaper_windows(&mut self) {
        let windows_no_longer_ours_when_their_destroy_events_arrive = std::mem::take(&mut self.wallpaper_windows);
        for wallpaper_window in windows_no_longer_ours_when_their_destroy_events_arrive {
            unsafe { DestroyWindow(wallpaper_window.window) };
        }
    }

    fn keep_windows_attached_and_over_their_monitors(&mut self, now: Instant) {
        let monitors = monitors::all_monitors_primary_first();

        let mut still_attached = false;
        if let Some(attachment) = &self.attachment {
            let desktop_is_the_same = desktop::find_progman() == attachment.progman;
            let parent_still_exists = unsafe { IsWindow(attachment.parent) } != 0;
            let mut every_monitor_still_has_its_window = self.wallpaper_windows.len() == monitors.len();
            for (wallpaper_window, monitor) in self.wallpaper_windows.iter().zip(&monitors) {
                let is_for_this_monitor = wallpaper_window.monitor.handle == monitor.handle;
                let is_still_a_child = unsafe { GetAncestor(wallpaper_window.window, GA_PARENT) } == attachment.parent;
                if !is_for_this_monitor || !is_still_a_child {
                    every_monitor_still_has_its_window = false;
                }
            }
            still_attached = desktop_is_the_same
                && parent_still_exists
                && every_monitor_still_has_its_window
                && !monitors.is_empty();
        }

        let mut something_changed = false;
        if !still_attached {
            if self.attachment.is_some() {
                warn!("the desktop or the monitors changed under the wallpaper windows; attaching again");
            }
            self.destroy_wallpaper_windows();
            self.attachment = None;
            let Some(desktop) = desktop::look_at_desktop_after_asking_for_a_wallpaper_workerw() else {
                warn!("attach: Progman not found (is Explorer running?); trying again shortly");
                self.check_desktop_at = Some(now + DELAY_BEFORE_TRYING_TO_ATTACH_AGAIN);
                return;
            };
            let parent = desktop.parent_for_wallpaper_windows();
            let only_layered_children_are_shown = desktop.only_layered_children_are_shown();
            info!(
                "attach: layout = {}; plan = {} (parent {}, layered {only_layered_children_are_shown})",
                desktop.layout.description(),
                desktop.how_wallpaper_windows_attach(),
                describe_window_handle(parent),
            );
            for monitor in &monitors {
                let window = create_hidden_top_level_window();
                if window.is_null() {
                    warn!("attach: could not create a window for {}", monitor.device_name);
                    continue;
                }
                if let Err(problem) =
                    desktop::turn_hidden_window_into_child_of_desktop(window, parent, only_layered_children_are_shown)
                {
                    warn!("attach: {} failed: {problem}", monitor.device_name);
                    unsafe { DestroyWindow(window) };
                    continue;
                }
                self.wallpaper_windows.push(WallpaperWindow {
                    window,
                    monitor: monitor.clone(),
                    placement: fit::place_canvas(self.config.fit, monitor.width(), monitor.height()),
                    is_hidden_behind_an_application: false,
                });
            }
            let every_monitor_got_a_window = !monitors.is_empty() && self.wallpaper_windows.len() == monitors.len();
            if !every_monitor_got_a_window {
                warn!("attach: could not put a window on every monitor; trying again shortly");
                self.destroy_wallpaper_windows();
                self.check_desktop_at = Some(now + DELAY_BEFORE_TRYING_TO_ATTACH_AGAIN);
                return;
            }
            self.attachment = Some(DesktopAttachment { layout: desktop.layout, progman: desktop.progman, parent });
            something_changed = true;
        }
        let Some(attachment) = &self.attachment else {
            return;
        };
        let parent = attachment.parent;

        let window_handles = self.window_handles();
        let depth_was_wrong = desktop::keep_below_icons_and_above_wallpaper(parent, &window_handles);
        if depth_was_wrong && still_attached {
            info!("the wallpaper windows were no longer between the icons and the wallpaper; depth restored");
        }

        for (wallpaper_window, monitor) in self.wallpaper_windows.iter_mut().zip(monitors) {
            let window = wallpaper_window.window;
            let covers_its_monitor_exactly = same_rectangle(&screen_rectangle_of(window), &monitor.rectangle)
                && unsafe { IsWindowVisible(window) } != 0;
            if !covers_its_monitor_exactly {
                desktop::cover_monitor_and_show(window, parent, &monitor.rectangle);
                info!(
                    "attach: {} covers {} {} at {} dpi",
                    describe_window_handle(window),
                    monitor.device_name,
                    describe_rectangle(&monitor.rectangle),
                    monitor.dots_per_inch,
                );
                wallpaper_window.is_hidden_behind_an_application = false;
                something_changed = true;
            }
            wallpaper_window.placement = fit::place_canvas(self.config.fit, monitor.width(), monitor.height());
            wallpaper_window.monitor = monitor;
        }

        if !still_attached {
            if let Some(desktop_now) = desktop::look_at_desktop() {
                for line in desktop::describe_hierarchy(&desktop_now, &window_handles).lines() {
                    info!("  {line}");
                }
            }
            info!("attach: {} window(s) attached", window_handles.len());
        }
        if something_changed {
            if !self.engine.has_rendered_a_frame() {
                self.engine.render_next_frame(0.0, &crate::host::local_time_now().as_atmosphere_clock());
            }
            self.show_frame();
        }
    }

    pub fn show_frame(&mut self) {
        let frame = &self.engine.frame;
        for wallpaper_window in &self.wallpaper_windows {
            if wallpaper_window.is_hidden_behind_an_application {
                continue;
            }
            let device_context = unsafe { GetDC(wallpaper_window.window) };
            if device_context.is_null() {
                continue;
            }
            blit::draw_canvas_scaled_by_whole_pixels(
                device_context,
                frame,
                &wallpaper_window.placement,
                wallpaper_window.monitor.width(),
                wallpaper_window.monitor.height(),
                self.config.letterbox_colour,
            );
            unsafe { ReleaseDC(wallpaper_window.window, device_context) };
        }
    }

    fn render_and_show_next_frame(&mut self, now: Instant) {
        let seconds_since_last_frame = now.saturating_duration_since(self.previous_frame_at).as_secs_f32();
        self.previous_frame_at = now;

        let rendering_started = Instant::now();
        self.engine.render_next_frame(seconds_since_last_frame, &crate::host::local_time_now().as_atmosphere_clock());
        let blitting_started = Instant::now();
        self.show_frame();
        if let Some(panel_window) = self.control_panel.as_mut() {
            panel_window.redraw(&self.engine.frame);
        }
        let finished = Instant::now();
        self.statistics.record_frame(blitting_started - rendering_started, finished - blitting_started);

        self.next_frame_at += self.frame_interval;
        let fell_behind_schedule = self.next_frame_at <= finished;
        if fell_behind_schedule {
            self.next_frame_at = finished + self.frame_interval;
        }
    }

    pub fn use_config(&mut self, new_config: Config) {
        self.engine.use_config(&new_config);
        self.frame_interval = Duration::from_secs_f64(1.0 / new_config.frames_per_second.max(1) as f64);
        self.config = new_config;
        for wallpaper_window in &mut self.wallpaper_windows {
            wallpaper_window.placement =
                fit::place_canvas(self.config.fit, wallpaper_window.monitor.width(), wallpaper_window.monitor.height());
            if !self.config.pause_on_fullscreen {
                wallpaper_window.is_hidden_behind_an_application = false;
            }
        }
        if self.engine.has_rendered_a_frame() {
            self.show_frame();
        }
    }

    pub fn apply_menu_choice(&mut self, choice: MenuChoice) {
        match choice {
            MenuChoice::PauseOrResume => self.pause_reasons.paused_by_user = !self.pause_reasons.paused_by_user,
            MenuChoice::Quit => self.quit_requested = true,
            MenuChoice::StartAtLogin => {
                let enable = !start_with_windows::is_enabled();
                match start_with_windows::set_enabled_because_the_user_clicked_the_menu_item(enable) {
                    Ok(()) => info!("start with Windows {}", if enable { "enabled" } else { "disabled" }),
                    Err(problem) => warn!("start with Windows: {problem}"),
                }
            }
            MenuChoice::OpenConfigFile => {
                let open = wide_null_terminated("open");
                let config_file = wide_null_terminated(&self.config_path.display().to_string());
                let notepad = wide_null_terminated("notepad.exe");
                unsafe {
                    let result_of_opening_with_the_associated_program =
                        ShellExecuteW(null_mut(), open.as_ptr(), config_file.as_ptr(), null(), null(), SW_SHOWNORMAL);
                    let nothing_is_associated_with_toml_files = result_of_opening_with_the_associated_program as usize
                        <= SHELL_EXECUTE_RESULTS_UP_TO_THIS_ARE_ERRORS;
                    if nothing_is_associated_with_toml_files {
                        ShellExecuteW(
                            null_mut(),
                            open.as_ptr(),
                            notepad.as_ptr(),
                            config_file.as_ptr(),
                            null(),
                            SW_SHOWNORMAL,
                        );
                    }
                }
            }
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

    fn handle_window_event(&mut self, event: WindowEvent, now: Instant) {
        match event {
            WindowEvent::QuitRequested => self.quit_requested = true,
            WindowEvent::ExplorerAnnouncedANewTaskbar => {
                info!("TaskbarCreated: Explorer announced a new taskbar");
                self.tray_icon_is_shown =
                    tray::show_tray_icon(self.control_window, self.tray_icon, &self.status_line());
                self.check_desktop_at = Some(now + TIME_A_NEW_EXPLORER_NEEDS_TO_BUILD_ITS_DESKTOP);
            }
            WindowEvent::DisplayLayoutChanged => {
                self.check_desktop_at = Some(now + TIME_EXPLORER_NEEDS_TO_REARRANGE_AFTER_A_DISPLAY_CHANGE);
            }
            WindowEvent::WallpaperWindowNeedsRepainting => {
                if self.engine.has_rendered_a_frame() {
                    self.show_frame();
                }
            }
            WindowEvent::WindowDestroyed(destroyed_window) => {
                let was_a_wallpaper_window = self
                    .wallpaper_windows
                    .iter()
                    .any(|wallpaper_window| wallpaper_window.window as isize == destroyed_window);
                if was_a_wallpaper_window {
                    warn!("wallpaper window {destroyed_window:#x} was destroyed by the shell; attaching again");
                    self.wallpaper_windows
                        .retain(|wallpaper_window| wallpaper_window.window as isize != destroyed_window);
                    self.check_desktop_at = Some(now + DELAY_BEFORE_REPLACING_A_DESTROYED_WINDOW);
                }
            }
            WindowEvent::Power(power_event) => {
                info!("power: {power_event:?}");
                self.pause_reasons.note(power_event);
                if power_event == PowerEvent::SystemResumed {
                    self.check_desktop_at = Some(now + TIME_THE_DESKTOP_NEEDS_TO_SETTLE_AFTER_WAKING);
                }
                self.ask_windows_about_lock_and_power_at = now;
            }
            WindowEvent::PowerSourceChanged => self.ask_windows_about_lock_and_power_at = now,
            WindowEvent::TrayIconClicked => self.open_or_close_the_control_panel(now),
            WindowEvent::PanelPointerMoved(x, y) => {
                let hover_changed = self.control_panel.as_mut().is_some_and(|panel_window| panel_window.point_at(x, y));
                if hover_changed {
                    self.refresh_the_control_panel();
                }
            }
            WindowEvent::PanelPointerLeft => {
                let hover_changed =
                    self.control_panel.as_mut().is_some_and(|panel_window| panel_window.forget_the_pointer());
                if hover_changed {
                    self.refresh_the_control_panel();
                }
            }
            WindowEvent::PanelClicked(x, y) => {
                let Some(choice) = self.control_panel.as_ref().and_then(|panel_window| panel_window.choice_at(x, y))
                else {
                    return;
                };
                info!("control panel: {choice:?}");
                let the_panel_has_done_its_job = matches!(choice, MenuChoice::Quit | MenuChoice::OpenConfigFile);
                if the_panel_has_done_its_job {
                    self.control_panel = None;
                    self.control_panel_closed_at = Some(now);
                }
                self.apply_menu_choice(choice);
                self.refresh_the_control_panel();
            }
            WindowEvent::PanelDismissed => {
                if self.control_panel.take().is_some() {
                    self.control_panel_closed_at = Some(now);
                }
            }
            WindowEvent::PanelNeedsPainting => {
                if let Some(panel_window) = &self.control_panel {
                    panel_window.paint();
                }
            }
        }
    }

    fn take_one_turn(&mut self) -> AfterTurn {
        self.statistics.main_loop_turns += 1;
        let now = Instant::now();

        loop {
            let events = WINDOW_EVENTS_FOR_THE_MAIN_LOOP.with(|events| std::mem::take(&mut *events.borrow_mut()));
            if events.is_empty() {
                break;
            }
            for event in events {
                self.handle_window_event(event, now);
            }
        }
        if self.quit_requested {
            return AfterTurn::Quit;
        }

        if self.check_desktop_at.is_some_and(|due| now >= due) {
            self.check_desktop_at = None;
            self.keep_windows_attached_and_over_their_monitors(now);
        }

        if now >= self.ask_windows_about_lock_and_power_at {
            self.ask_windows_about_lock_and_power_at = now + INTERVAL_BETWEEN_ASKING_WINDOWS_ABOUT_LOCK_AND_POWER;
            if let Some(is_locked) = power::session_is_locked() {
                self.pause_reasons.session_is_locked = is_locked;
            }
            self.pause_reasons.running_on_battery = power::running_on_battery();
        }

        let reason_not_to_render = self.pause_reasons.reason_not_to_render(self.config.pause_on_battery);
        if reason_not_to_render.is_none() && now >= self.next_housekeeping_at {
            self.next_housekeeping_at = now + HOUSEKEEPING_INTERVAL;
            self.housekeeping_turns += 1;

            if self.config.pause_on_fullscreen {
                let monitors: Vec<Monitor> =
                    self.wallpaper_windows.iter().map(|wallpaper_window| wallpaper_window.monitor.clone()).collect();
                let hidden_monitors = power::monitors_hidden_behind_application_windows(&monitors);
                for (wallpaper_window, is_hidden) in self.wallpaper_windows.iter_mut().zip(hidden_monitors) {
                    wallpaper_window.is_hidden_behind_an_application = is_hidden;
                }
            }

            let time_to_check_the_desktop =
                self.housekeeping_turns.is_multiple_of(HOUSEKEEPING_TURNS_BETWEEN_DESKTOP_CHECKS);
            if time_to_check_the_desktop && self.check_desktop_at.is_none() {
                if !self.tray_icon_is_shown {
                    self.tray_icon_is_shown =
                        tray::show_tray_icon(self.control_window, self.tray_icon, &self.status_line());
                    if self.tray_icon_is_shown {
                        info!("tray icon added");
                    }
                }
                self.keep_windows_attached_and_over_their_monitors(now);
            }

            let statistics_are_due =
                self.statistics.measuring_since.elapsed() >= INTERVAL_BETWEEN_STATISTICS_IN_THE_LOG;
            if statistics_are_due {
                info!("stats: {}", self.statistics.summary());
                self.statistics = FrameStatistics::start();
            }
        }

        let some_window_can_be_seen =
            self.wallpaper_windows.iter().any(|wallpaper_window| !wallpaper_window.is_hidden_behind_an_application);
        let the_control_panel_shows_a_live_preview = self.control_panel.is_some();
        let should_render =
            reason_not_to_render.is_none() && (some_window_can_be_seen || the_control_panel_shows_a_live_preview);
        let resuming_after_a_pause = should_render && !self.was_rendering;
        if resuming_after_a_pause {
            self.previous_frame_at = now;
            self.next_frame_at = now;
        }
        self.was_rendering = should_render;

        let status_line = self.status_line();
        if status_line != self.status_line_shown {
            info!("{status_line}");
            if self.tray_icon_is_shown {
                tray::set_tray_tooltip(self.control_window, self.tray_icon, &status_line);
            }
            self.status_line_shown = status_line;
        }

        if should_render && now >= self.next_frame_at {
            self.render_and_show_next_frame(now);
        }

        let mut wake_at: Option<Instant> = None;
        let mut wake_no_later_than = |deadline: Instant| {
            wake_at = match wake_at {
                Some(earlier_deadline) if earlier_deadline <= deadline => Some(earlier_deadline),
                _ => Some(deadline),
            };
        };
        if should_render {
            wake_no_later_than(self.next_frame_at);
        }
        if reason_not_to_render.is_none() {
            wake_no_later_than(self.next_housekeeping_at);
        }
        let pause_may_end_without_a_notification =
            self.pause_reasons.system_can_be_asked_whether_the_reason_still_holds(self.config.pause_on_battery);
        if reason_not_to_render.is_none() || pause_may_end_without_a_notification {
            wake_no_later_than(self.ask_windows_about_lock_and_power_at);
        }
        if let Some(check_desktop_at) = self.check_desktop_at {
            wake_no_later_than(check_desktop_at);
        }
        AfterTurn::SleepUntil(wake_at)
    }
}

fn panicked_while_running(work: impl FnOnce()) -> bool {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(work)).is_err()
}

pub fn run() -> i32 {
    let config_path = config::default_config_path();
    logging::use_clock(crate::host::local_time_now);
    logging::start(Some(&config_path.with_file_name(config::LOG_FILE_NAME)), false);
    logging::record_panics_in_the_log();

    if !single_instance::claim_for_the_life_of_this_process() {
        info!("another copy is already running; this one is exiting");
        return EXIT_CODE_FAILURE;
    }

    let taskbar_created = wide_null_terminated("TaskbarCreated");
    MESSAGE_EXPLORER_BROADCASTS_WHEN_IT_CREATES_A_TASKBAR
        .store(unsafe { RegisterWindowMessageW(taskbar_created.as_ptr()) }, Ordering::Relaxed);
    let no_background_brush_so_nothing_flickers = null_mut();
    win32::register_window_class(
        single_instance::WINDOW_CLASS_OF_A_RUNNING_INSTANCE,
        turn_window_messages_into_events,
        no_background_brush_so_nothing_flickers,
    );
    let control_window = create_hidden_top_level_window();
    if control_window.is_null() {
        warn!("could not create the control window");
        return EXIT_CODE_FAILURE;
    }

    info!("duke-wallpaper {} starting", env!("CARGO_PKG_VERSION"));
    let from_disk = config::load_or_create_with_defaults(&config_path);
    if from_disk.was_just_created {
        info!("created {} with defaults", config_path.display());
    } else {
        info!("config: {}", config_path.display());
    }
    for warning in &from_disk.warnings {
        warn!("config: {warning}");
    }
    let config = from_disk.config;

    let notifications = power::ask_for_display_and_session_notifications(control_window);
    info!(
        "display-state notifications: {}, session notifications: {}",
        if notifications.display_state_is_reported { "on" } else { "unavailable" },
        if notifications.session_changes_are_reported { "on" } else { "unavailable" },
    );
    let tray_icon = tray::create_chapel_tower_icon(unsafe { GetSystemMetrics(SM_CXSMICON) });
    let tray_icon_is_shown = tray::show_tray_icon(control_window, tray_icon, PROGRAM_TITLE);
    if !tray_icon_is_shown {
        warn!("could not add the tray icon (no taskbar yet?); trying again every two seconds");
    }
    let frame_timer = FrameTimer::new();
    info!(
        "frame pacing: {} fps, {} waitable timer",
        config.frames_per_second,
        if frame_timer.is_high_resolution { "high-resolution" } else { "standard" }
    );

    let pause_reasons = PauseReasons {
        session_is_locked: power::session_is_locked().unwrap_or(false),
        running_on_battery: power::running_on_battery(),
        ..PauseReasons::default()
    };
    if pause_reasons.session_is_locked {
        info!("the session is locked right now");
    }

    let now = Instant::now();
    let mut wallpaper = Wallpaper {
        engine: Engine::new(&config),
        frame_interval: Duration::from_secs_f64(1.0 / config.frames_per_second.max(1) as f64),
        config,
        config_path,
        control_window,
        wallpaper_windows: Vec::new(),
        attachment: None,
        tray_icon,
        tray_icon_is_shown,
        pause_reasons,
        was_rendering: false,
        status_line_shown: String::new(),
        next_frame_at: now,
        previous_frame_at: now,
        next_housekeeping_at: now,
        housekeeping_turns: 0,
        ask_windows_about_lock_and_power_at: now + INTERVAL_BETWEEN_ASKING_WINDOWS_ABOUT_LOCK_AND_POWER,
        check_desktop_at: Some(now),
        statistics: FrameStatistics::start(),
        quit_requested: false,
        control_panel: None,
        control_panel_closed_at: None,
    };

    let stopped_by_a_panic = panicked_while_running(|| loop {
        let quit_message_seen = win32::dispatch_waiting_window_messages();
        let AfterTurn::SleepUntil(wake_at) = wallpaper.take_one_turn() else {
            break;
        };
        if quit_message_seen {
            break;
        }
        let an_event_arrived_during_the_turn =
            WINDOW_EVENTS_FOR_THE_MAIN_LOOP.with(|events| !events.borrow().is_empty());
        if !an_event_arrived_during_the_turn {
            frame_timer.sleep_until_deadline_or_window_message(wake_at);
        }
    });

    info!("shutting down; {}", wallpaper.statistics.summary());
    wallpaper.control_panel = None;
    wallpaper.destroy_wallpaper_windows();
    if let Some(attachment) = wallpaper.attachment.take() {
        desktop::make_static_wallpaper_reappear(attachment.layout, attachment.parent, attachment.progman);
    }
    if wallpaper.tray_icon_is_shown {
        tray::remove_tray_icon(control_window);
    }
    power::stop_display_and_session_notifications(notifications);
    unsafe {
        DestroyIcon(tray_icon);
        DestroyWindow(control_window);
    }
    win32::dispatch_waiting_window_messages();
    info!("exited cleanly");

    if stopped_by_a_panic {
        warn!("stopped after an internal error (see the PANIC entry above)");
        return EXIT_CODE_FAILURE;
    }
    EXIT_CODE_SUCCESS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_panic_in_the_main_loop_still_reaches_the_cleanup_that_follows_it() {
        let print_panics_as_usual = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));
        let mut turns_taken = 0;
        let panicked = panicked_while_running(|| loop {
            turns_taken += 1;
            if turns_taken == 3 {
                panic!("a bug in a scene");
            }
        });
        std::panic::set_hook(print_panics_as_usual);
        let cleanup_was_reached = true;
        assert!(panicked && cleanup_was_reached);
        assert_eq!(turns_taken, 3);
        assert!(!panicked_while_running(|| {}));
    }

    #[test]
    fn window_messages_become_events_for_the_main_loop() {
        let no_window: HWND = null_mut();
        let events_queued_by = |message: u32, first_argument: WPARAM, second_argument: LPARAM| {
            WINDOW_EVENTS_FOR_THE_MAIN_LOOP.with(|events| events.borrow_mut().clear());
            unsafe { turn_window_messages_into_events(no_window, message, first_argument, second_argument) };
            WINDOW_EVENTS_FOR_THE_MAIN_LOOP.with(|events| std::mem::take(&mut *events.borrow_mut()))
        };
        assert!(matches!(events_queued_by(WM_CLOSE, 0, 0)[..], [WindowEvent::QuitRequested]));
        assert!(matches!(events_queued_by(WM_DISPLAYCHANGE, 0, 0)[..], [WindowEvent::DisplayLayoutChanged]));
        assert!(matches!(
            events_queued_by(WM_SETTINGCHANGE, SPI_SETWORKAREA as WPARAM, 0)[..],
            [WindowEvent::DisplayLayoutChanged]
        ));
        assert!(events_queued_by(WM_SETTINGCHANGE, 0, 0).is_empty());
        assert!(matches!(
            events_queued_by(WM_POWERBROADCAST, PBT_APMPOWERSTATUSCHANGE as WPARAM, 0)[..],
            [WindowEvent::PowerSourceChanged]
        ));
        let session_was_unlocked = windows_sys::Win32::UI::WindowsAndMessaging::WTS_SESSION_UNLOCK as WPARAM;
        assert!(matches!(
            events_queued_by(WM_WTSSESSION_CHANGE, session_was_unlocked, 0)[..],
            [WindowEvent::Power(PowerEvent::SessionUnlocked)]
        ));
        let right_click_on_the_tray_icon = WM_CONTEXTMENU as LPARAM;
        assert!(matches!(
            events_queued_by(tray::MESSAGE_SENT_WHEN_THE_TRAY_ICON_IS_USED, 0, right_click_on_the_tray_icon)[..],
            [WindowEvent::TrayIconClicked]
        ));
        let mouse_moved_over_the_tray_icon = windows_sys::Win32::UI::WindowsAndMessaging::WM_MOUSEMOVE as LPARAM;
        assert!(events_queued_by(tray::MESSAGE_SENT_WHEN_THE_TRAY_ICON_IS_USED, 0, mouse_moved_over_the_tray_icon)
            .is_empty());
        assert!(matches!(events_queued_by(WM_DESTROY, 0, 0)[..], [WindowEvent::WindowDestroyed(0)]));
        assert!(matches!(events_queued_by(WM_ENDSESSION, 1, 0)[..], [WindowEvent::QuitRequested]));
        assert!(events_queued_by(WM_ENDSESSION, 0, 0).is_empty());
    }
}
