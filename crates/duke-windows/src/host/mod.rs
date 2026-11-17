mod blit;
mod control_panel_window;
mod desktop;
mod monitors;
mod pacing;
mod power;
mod single_instance;
mod start_with_windows;
mod tray;
mod wallpaper;
mod win32;

use duke_core::wallpaper::clock::LocalTime;
use windows_sys::Win32::System::SystemInformation::GetLocalTime;
use windows_sys::Win32::UI::HiDpi::{SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2};

pub fn run() -> i32 {
    let address_physical_pixels_on_every_monitor = DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2;
    unsafe { SetProcessDpiAwarenessContext(address_physical_pixels_on_every_monitor) };
    wallpaper::run()
}

pub fn local_time_now() -> LocalTime {
    let system_time = unsafe {
        let mut system_time = std::mem::zeroed();
        GetLocalTime(&mut system_time);
        system_time
    };
    LocalTime {
        year: system_time.wYear as u32,
        month: system_time.wMonth as u32,
        day: system_time.wDay as u32,
        hour: system_time.wHour as u32,
        minute: system_time.wMinute as u32,
        second: system_time.wSecond as u32,
        millisecond: system_time.wMilliseconds as u32,
    }
}
