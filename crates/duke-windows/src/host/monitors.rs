use windows_sys::core::BOOL;
use windows_sys::Win32::Foundation::{LPARAM, RECT};
use windows_sys::Win32::Graphics::Gdi::{
    EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFO, MONITORINFOEXW,
};
use windows_sys::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};
use windows_sys::Win32::UI::WindowsAndMessaging::MONITORINFOF_PRIMARY;

const DOTS_PER_INCH_AT_100_PERCENT: u32 = 96;
const KEEP_ENUMERATING: BOOL = 1;

#[derive(Clone)]
pub struct Monitor {
    pub handle: isize,
    pub rectangle: RECT,
    pub work_area: RECT,
    pub dots_per_inch: u32,
    pub is_primary: bool,
    pub device_name: String,
}

impl Monitor {
    pub fn width(&self) -> i32 {
        self.rectangle.right - self.rectangle.left
    }

    pub fn height(&self) -> i32 {
        self.rectangle.bottom - self.rectangle.top
    }
}

unsafe extern "system" fn add_monitor_to_list(
    monitor: HMONITOR,
    _device_context: HDC,
    _clip_rectangle: *mut RECT,
    list_address: LPARAM,
) -> BOOL {
    let monitors = unsafe { &mut *(list_address as *mut Vec<Monitor>) };
    let mut information: MONITORINFOEXW = unsafe { std::mem::zeroed() };
    information.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;
    let information_address = &mut information as *mut MONITORINFOEXW as *mut MONITORINFO;
    if unsafe { GetMonitorInfoW(monitor, information_address) } == 0 {
        return KEEP_ENUMERATING;
    }
    let mut horizontal_dots_per_inch = DOTS_PER_INCH_AT_100_PERCENT;
    let mut vertical_dots_per_inch = DOTS_PER_INCH_AT_100_PERCENT;
    let dots_per_inch_known = unsafe {
        GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut horizontal_dots_per_inch, &mut vertical_dots_per_inch)
    } == 0;
    let device_name_length =
        information.szDevice.iter().position(|&character| character == 0).unwrap_or(information.szDevice.len());
    monitors.push(Monitor {
        handle: monitor as isize,
        rectangle: information.monitorInfo.rcMonitor,
        work_area: information.monitorInfo.rcWork,
        dots_per_inch: if dots_per_inch_known { horizontal_dots_per_inch } else { DOTS_PER_INCH_AT_100_PERCENT },
        is_primary: information.monitorInfo.dwFlags & MONITORINFOF_PRIMARY != 0,
        device_name: String::from_utf16_lossy(&information.szDevice[..device_name_length]),
    });
    KEEP_ENUMERATING
}

pub fn all_monitors_primary_first() -> Vec<Monitor> {
    let mut monitors: Vec<Monitor> = Vec::new();
    let list_address = &mut monitors as *mut Vec<Monitor> as LPARAM;
    unsafe {
        EnumDisplayMonitors(std::ptr::null_mut(), std::ptr::null(), Some(add_monitor_to_list), list_address);
    }
    monitors.sort_by_key(|monitor| (!monitor.is_primary, monitor.rectangle.left, monitor.rectangle.top));
    monitors
}
