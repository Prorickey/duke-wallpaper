use crate::host::monitors::Monitor;
use crate::host::win32::{
    class_name_of, extended_style_of, process_id_owning, rectangle_contains, screen_rectangle_of,
};
use duke_core::wallpaper::pausing::PowerEvent;
use windows_sys::core::BOOL;
use windows_sys::Win32::Foundation::{HWND, LPARAM, RECT, WPARAM};
use windows_sys::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED};
use windows_sys::Win32::System::Power::{
    GetSystemPowerStatus, RegisterPowerSettingNotification, UnregisterPowerSettingNotification, HPOWERNOTIFY,
    POWERBROADCAST_SETTING, SYSTEM_POWER_STATUS,
};
use windows_sys::Win32::System::RemoteDesktop::{
    WTSFreeMemory, WTSQuerySessionInformationW, WTSRegisterSessionNotification, WTSSessionInfoEx,
    WTSUnRegisterSessionNotification, NOTIFY_FOR_THIS_SESSION, WTSINFOEXW, WTS_CURRENT_SERVER_HANDLE,
    WTS_CURRENT_SESSION, WTS_SESSIONSTATE_LOCK, WTS_SESSIONSTATE_UNLOCK,
};
use windows_sys::Win32::System::SystemServices::GUID_CONSOLE_DISPLAY_STATE;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    EnumWindows, IsIconic, IsWindowVisible, IsZoomed, DEVICE_NOTIFY_WINDOW_HANDLE, PBT_APMRESUMEAUTOMATIC,
    PBT_APMRESUMESUSPEND, PBT_APMSUSPEND, PBT_POWERSETTINGCHANGE, WS_EX_TOOLWINDOW, WS_EX_TRANSPARENT,
    WTS_SESSION_LOCK, WTS_SESSION_UNLOCK,
};

const DISPLAY_STATE_OFF: u32 = 0;
const RUNNING_ON_BATTERY: u8 = 0;
const SESSION_INFORMATION_LEVEL_WITH_LOCK_STATE: u32 = 1;
const KEEP_ENUMERATING: BOOL = 1;
const STOP_ENUMERATING: BOOL = 0;
const WINDOWS_THAT_ARE_THE_DESKTOP_ITSELF: [&str; 4] =
    ["Progman", "WorkerW", "Shell_TrayWnd", "Shell_SecondaryTrayWnd"];

pub struct DisplayAndSessionNotifications {
    control_window: HWND,
    display_state_registration: HPOWERNOTIFY,
    pub display_state_is_reported: bool,
    pub session_changes_are_reported: bool,
}

pub fn ask_for_display_and_session_notifications(control_window: HWND) -> DisplayAndSessionNotifications {
    let display_state_registration = unsafe {
        RegisterPowerSettingNotification(control_window, &GUID_CONSOLE_DISPLAY_STATE, DEVICE_NOTIFY_WINDOW_HANDLE)
    };
    let session_changes_are_reported =
        unsafe { WTSRegisterSessionNotification(control_window, NOTIFY_FOR_THIS_SESSION) } != 0;
    DisplayAndSessionNotifications {
        control_window,
        display_state_registration,
        display_state_is_reported: display_state_registration != 0,
        session_changes_are_reported,
    }
}

pub fn stop_display_and_session_notifications(notifications: DisplayAndSessionNotifications) {
    unsafe {
        if notifications.display_state_is_reported {
            UnregisterPowerSettingNotification(notifications.display_state_registration);
        }
        if notifications.session_changes_are_reported {
            WTSUnRegisterSessionNotification(notifications.control_window);
        }
    }
}

pub unsafe fn power_event_in_power_broadcast(broadcast_kind: WPARAM, broadcast_data: LPARAM) -> Option<PowerEvent> {
    match broadcast_kind as u32 {
        PBT_APMSUSPEND => Some(PowerEvent::SystemSuspending),
        PBT_APMRESUMEAUTOMATIC | PBT_APMRESUMESUSPEND => Some(PowerEvent::SystemResumed),
        PBT_POWERSETTINGCHANGE => {
            let changed_setting = broadcast_data as *const POWERBROADCAST_SETTING;
            if changed_setting.is_null() {
                return None;
            }
            let changed_setting = unsafe { &*changed_setting };
            let setting = &changed_setting.PowerSetting;
            let display_state = &GUID_CONSOLE_DISPLAY_STATE;
            let is_the_display_state = setting.data1 == display_state.data1
                && setting.data2 == display_state.data2
                && setting.data3 == display_state.data3
                && setting.data4 == display_state.data4;
            if !is_the_display_state || (changed_setting.DataLength as usize) < std::mem::size_of::<u32>() {
                return None;
            }
            let new_display_state = unsafe { std::ptr::read_unaligned(changed_setting.Data.as_ptr() as *const u32) };
            if new_display_state == DISPLAY_STATE_OFF {
                Some(PowerEvent::DisplayTurnedOff)
            } else {
                Some(PowerEvent::DisplayTurnedOn)
            }
        }
        _ => None,
    }
}

pub fn power_event_in_session_change(session_change_kind: WPARAM) -> Option<PowerEvent> {
    match session_change_kind as u32 {
        WTS_SESSION_LOCK => Some(PowerEvent::SessionLocked),
        WTS_SESSION_UNLOCK => Some(PowerEvent::SessionUnlocked),
        _ => None,
    }
}

pub fn session_is_locked() -> Option<bool> {
    let mut session_information: *mut u16 = std::ptr::null_mut();
    let mut bytes_returned = 0u32;
    let answered = unsafe {
        WTSQuerySessionInformationW(
            WTS_CURRENT_SERVER_HANDLE,
            WTS_CURRENT_SESSION,
            WTSSessionInfoEx,
            &mut session_information,
            &mut bytes_returned,
        )
    } != 0;
    if !answered || session_information.is_null() {
        return None;
    }
    let mut is_locked = None;
    if bytes_returned as usize >= std::mem::size_of::<WTSINFOEXW>() {
        let information = unsafe { std::ptr::read_unaligned(session_information as *const WTSINFOEXW) };
        if information.Level == SESSION_INFORMATION_LEVEL_WITH_LOCK_STATE {
            let lock_state = unsafe { information.Data.WTSInfoExLevel1.SessionFlags } as u32;
            is_locked = match lock_state {
                WTS_SESSIONSTATE_LOCK => Some(true),
                WTS_SESSIONSTATE_UNLOCK => Some(false),
                _ => None,
            };
        }
    }
    unsafe { WTSFreeMemory(session_information.cast()) };
    is_locked
}

pub fn running_on_battery() -> bool {
    let mut power_status: SYSTEM_POWER_STATUS = unsafe { std::mem::zeroed() };
    let answered = unsafe { GetSystemPowerStatus(&mut power_status) } != 0;
    answered && power_status.ACLineStatus == RUNNING_ON_BATTERY
}

fn window_hides_monitor(window_rectangle: &RECT, window_is_maximized: bool, monitor: &Monitor) -> bool {
    let fills_the_whole_monitor = rectangle_contains(window_rectangle, &monitor.rectangle);
    let is_maximized_over_the_work_area =
        window_is_maximized && rectangle_contains(window_rectangle, &monitor.work_area);
    fills_the_whole_monitor || is_maximized_over_the_work_area
}

struct SearchForHiddenMonitors<'monitors> {
    monitors: &'monitors [Monitor],
    is_hidden: Vec<bool>,
    our_process_id: u32,
}

unsafe extern "system" fn note_monitors_this_window_hides(window: HWND, search_address: LPARAM) -> BOOL {
    let search = unsafe { &mut *(search_address as *mut SearchForHiddenMonitors) };
    let is_on_screen = unsafe { IsWindowVisible(window) != 0 && IsIconic(window) == 0 };
    if !is_on_screen {
        return KEEP_ENUMERATING;
    }
    let is_an_overlay_not_an_application = extended_style_of(window) & (WS_EX_TRANSPARENT | WS_EX_TOOLWINDOW) != 0;
    if is_an_overlay_not_an_application {
        return KEEP_ENUMERATING;
    }
    let window_rectangle = screen_rectangle_of(window);
    let window_is_maximized = unsafe { IsZoomed(window) } != 0;
    let mut monitors_this_window_hides = Vec::new();
    for (monitor_index, monitor) in search.monitors.iter().enumerate() {
        let already_known_hidden = search.is_hidden[monitor_index];
        if !already_known_hidden && window_hides_monitor(&window_rectangle, window_is_maximized, monitor) {
            monitors_this_window_hides.push(monitor_index);
        }
    }
    if monitors_this_window_hides.is_empty() {
        return KEEP_ENUMERATING;
    }

    let mut cloaked_by_the_window_manager = 0u32;
    unsafe {
        DwmGetWindowAttribute(
            window,
            DWMWA_CLOAKED as u32,
            (&mut cloaked_by_the_window_manager as *mut u32).cast(),
            std::mem::size_of::<u32>() as u32,
        )
    };
    if cloaked_by_the_window_manager != 0 || process_id_owning(window) == search.our_process_id {
        return KEEP_ENUMERATING;
    }
    if WINDOWS_THAT_ARE_THE_DESKTOP_ITSELF.contains(&class_name_of(window).as_str()) {
        return KEEP_ENUMERATING;
    }

    for monitor_index in monitors_this_window_hides {
        search.is_hidden[monitor_index] = true;
    }
    let every_monitor_is_hidden = search.is_hidden.iter().all(|&is_hidden| is_hidden);
    if every_monitor_is_hidden {
        STOP_ENUMERATING
    } else {
        KEEP_ENUMERATING
    }
}

pub fn monitors_hidden_behind_application_windows(monitors: &[Monitor]) -> Vec<bool> {
    let mut search = SearchForHiddenMonitors {
        monitors,
        is_hidden: vec![false; monitors.len()],
        our_process_id: std::process::id(),
    };
    if !monitors.is_empty() {
        let search_address = &mut search as *mut SearchForHiddenMonitors as LPARAM;
        unsafe { EnumWindows(Some(note_monitors_this_window_hides), search_address) };
    }
    search.is_hidden
}

#[cfg(test)]
mod tests {
    use super::*;
    use duke_core::wallpaper::pausing::PowerEvent::*;

    #[test]
    fn only_fullscreen_and_maximized_windows_hide_a_monitor() {
        let monitor = Monitor {
            handle: 1,
            rectangle: RECT { left: 0, top: 0, right: 1920, bottom: 1200 },
            work_area: RECT { left: 0, top: 0, right: 1920, bottom: 1140 },
            dots_per_inch: 96,
            is_primary: true,
            device_name: String::new(),
        };
        let fullscreen = RECT { left: 0, top: 0, right: 1920, bottom: 1200 };
        let maximized_with_its_border_overhanging = RECT { left: -9, top: -9, right: 1929, bottom: 1149 };
        let snapped_to_the_left_half = RECT { left: 0, top: 0, right: 960, bottom: 1140 };
        let on_another_monitor = RECT { left: 1920, top: 0, right: 3840, bottom: 1200 };
        assert!(window_hides_monitor(&fullscreen, false, &monitor));
        assert!(window_hides_monitor(&maximized_with_its_border_overhanging, true, &monitor));
        assert!(!window_hides_monitor(&maximized_with_its_border_overhanging, false, &monitor));
        assert!(!window_hides_monitor(&snapped_to_the_left_half, true, &monitor));
        assert!(!window_hides_monitor(&on_another_monitor, true, &monitor));
    }

    #[test]
    fn session_changes_map_to_lock_and_unlock_only() {
        assert_eq!(power_event_in_session_change(WTS_SESSION_LOCK as WPARAM), Some(SessionLocked));
        assert_eq!(power_event_in_session_change(WTS_SESSION_UNLOCK as WPARAM), Some(SessionUnlocked));
        assert_eq!(power_event_in_session_change(1), None);
        let no_data: LPARAM = 0;
        unsafe {
            assert_eq!(power_event_in_power_broadcast(PBT_APMSUSPEND as WPARAM, no_data), Some(SystemSuspending));
            assert_eq!(power_event_in_power_broadcast(PBT_APMRESUMEAUTOMATIC as WPARAM, no_data), Some(SystemResumed));
            assert_eq!(power_event_in_power_broadcast(PBT_POWERSETTINGCHANGE as WPARAM, no_data), None);
        }
    }
}
