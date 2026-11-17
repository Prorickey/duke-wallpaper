use crate::host::monitors::{self, Monitor};
use crate::host::wallpaper::{queue_for_the_main_loop, WindowEvent};
use crate::host::win32::{self, top_down_bitmap_description, wide_null_terminated};
use duke_core::wallpaper::control_panel::ControlPanel;
use duke_core::wallpaper::menu::{MenuChoice, PROGRAM_TITLE};
use duke_core::Canvas;
use std::cell::Cell;
use std::ptr::{null, null_mut};
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows_sys::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUNDSMALL};
use windows_sys::Win32::Graphics::Gdi::{
    BeginPaint, EndPaint, GetDC, ReleaseDC, SetStretchBltMode, StretchDIBits, COLORONCOLOR, DIB_RGB_COLORS,
    PAINTSTRUCT, SRCCOPY,
};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{TrackMouseEvent, TME_LEAVE, TRACKMOUSEEVENT, VK_ESCAPE};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, GetCursorPos, LoadCursorW, RegisterClassExW, SetCursor,
    SetForegroundWindow, SetWindowPos, ShowWindow, CS_DROPSHADOW, HTCLIENT, IDC_ARROW, IDC_HAND, SWP_NOACTIVATE,
    SWP_NOZORDER, SW_SHOW, WA_INACTIVE, WM_ACTIVATE, WM_ERASEBKGND, WM_KEYDOWN, WM_LBUTTONUP, WM_MOUSEMOVE, WM_PAINT,
    WM_SETCURSOR, WNDCLASSEXW, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
};

const PANEL_WINDOW_CLASS: &str = "DukePixelArtControlPanel";
const DEVICE_PIXELS_PER_PANEL_PIXEL_AT_100_PERCENT: f32 = 2.0;
const DOTS_PER_INCH_AT_100_PERCENT: f32 = 96.0;
const SPACE_BETWEEN_THE_PANEL_AND_THE_TASKBAR: i32 = 12;
const WM_MOUSELEAVE: u32 = 0x02A3;
const MESSAGE_WAS_HANDLED: LRESULT = 0;
const BACKGROUND_NEEDS_NO_ERASING: LRESULT = 1;
const CURSOR_WAS_SET: LRESULT = 1;

thread_local! {
    static POINTER_IS_ON_A_CONTROL: Cell<bool> = const { Cell::new(false) };
}

fn signed_coordinates_in(packed: LPARAM) -> (i32, i32) {
    let x = (packed & 0xFFFF) as u16 as i16 as i32;
    let y = ((packed >> 16) & 0xFFFF) as u16 as i16 as i32;
    (x, y)
}

unsafe extern "system" fn turn_panel_messages_into_events(
    window: HWND,
    message: u32,
    first_argument: WPARAM,
    second_argument: LPARAM,
) -> LRESULT {
    match message {
        WM_MOUSEMOVE => {
            let mut tell_us_when_the_pointer_leaves = TRACKMOUSEEVENT {
                cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32,
                dwFlags: TME_LEAVE,
                hwndTrack: window,
                dwHoverTime: 0,
            };
            unsafe { TrackMouseEvent(&mut tell_us_when_the_pointer_leaves) };
            let (x, y) = signed_coordinates_in(second_argument);
            queue_for_the_main_loop(WindowEvent::PanelPointerMoved(x, y));
            MESSAGE_WAS_HANDLED
        }
        WM_MOUSELEAVE => {
            queue_for_the_main_loop(WindowEvent::PanelPointerLeft);
            MESSAGE_WAS_HANDLED
        }
        WM_LBUTTONUP => {
            let (x, y) = signed_coordinates_in(second_argument);
            queue_for_the_main_loop(WindowEvent::PanelClicked(x, y));
            MESSAGE_WAS_HANDLED
        }
        WM_ACTIVATE => {
            let another_window_was_activated = (first_argument & 0xFFFF) as u32 == WA_INACTIVE;
            if another_window_was_activated {
                queue_for_the_main_loop(WindowEvent::PanelDismissed);
            }
            MESSAGE_WAS_HANDLED
        }
        WM_KEYDOWN => {
            if first_argument as u16 == VK_ESCAPE {
                queue_for_the_main_loop(WindowEvent::PanelDismissed);
            }
            MESSAGE_WAS_HANDLED
        }
        WM_SETCURSOR if (second_argument & 0xFFFF) as u32 == HTCLIENT => {
            let cursor = if POINTER_IS_ON_A_CONTROL.get() { IDC_HAND } else { IDC_ARROW };
            unsafe { SetCursor(LoadCursorW(null_mut(), cursor)) };
            CURSOR_WAS_SET
        }
        WM_PAINT => {
            unsafe {
                let mut paint: PAINTSTRUCT = std::mem::zeroed();
                BeginPaint(window, &mut paint);
                EndPaint(window, &paint);
            }
            queue_for_the_main_loop(WindowEvent::PanelNeedsPainting);
            MESSAGE_WAS_HANDLED
        }
        WM_ERASEBKGND => BACKGROUND_NEEDS_NO_ERASING,
        _ => unsafe { DefWindowProcW(window, message, first_argument, second_argument) },
    }
}

fn whole_pixel_scale(dots_per_inch: u32, work_area: &RECT, panel_width: i32, panel_height: i32) -> i32 {
    let wanted = (dots_per_inch as f32 / DOTS_PER_INCH_AT_100_PERCENT * DEVICE_PIXELS_PER_PANEL_PIXEL_AT_100_PERCENT)
        .round() as i32;
    let fits_nine_tenths_of_the_height = (work_area.bottom - work_area.top) * 9 / 10 / panel_height.max(1);
    let fits_half_the_width = (work_area.right - work_area.left) / 2 / panel_width.max(1);
    wanted.min(fits_nine_tenths_of_the_height).min(fits_half_the_width).max(1)
}

fn top_left_beside_the_tray(pointer: POINT, monitor: &Monitor, width: i32, height: i32) -> (i32, i32) {
    let work = monitor.work_area;
    let screen = monitor.rectangle;
    let taskbar_is_at_the_bottom = work.bottom < screen.bottom;
    let taskbar_is_at_the_top = work.top > screen.top;
    let preferred_top = if taskbar_is_at_the_bottom {
        work.bottom - height - SPACE_BETWEEN_THE_PANEL_AND_THE_TASKBAR
    } else if taskbar_is_at_the_top {
        work.top + SPACE_BETWEEN_THE_PANEL_AND_THE_TASKBAR
    } else {
        pointer.y - height / 2
    };
    let preferred_left = pointer.x - width / 2;
    let left = preferred_left.clamp(work.left, (work.right - width).max(work.left));
    let top = preferred_top.clamp(work.top, (work.bottom - height).max(work.top));
    (left, top)
}

fn monitor_under(pointer: POINT) -> Option<Monitor> {
    let all_monitors = monitors::all_monitors_primary_first();
    let is_under_the_pointer = |monitor: &&Monitor| {
        let area = monitor.rectangle;
        pointer.x >= area.left && pointer.x < area.right && pointer.y >= area.top && pointer.y < area.bottom
    };
    all_monitors.iter().find(is_under_the_pointer).or(all_monitors.first()).cloned()
}

pub struct ControlPanelWindow {
    window: HWND,
    pub panel: ControlPanel,
    scale: i32,
    pointer: Option<(i32, i32)>,
}

impl ControlPanelWindow {
    pub fn open(panel: ControlPanel) -> Option<ControlPanelWindow> {
        let class_name = wide_null_terminated(PANEL_WINDOW_CLASS);
        let class = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: CS_DROPSHADOW,
            lpfnWndProc: Some(turn_panel_messages_into_events),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: win32::this_program(),
            hIcon: null_mut(),
            hCursor: unsafe { LoadCursorW(null_mut(), IDC_ARROW) },
            hbrBackground: null_mut(),
            lpszMenuName: null(),
            lpszClassName: class_name.as_ptr(),
            hIconSm: null_mut(),
        };
        unsafe { RegisterClassExW(&class) };

        let mut pointer = POINT { x: 0, y: 0 };
        unsafe { GetCursorPos(&mut pointer) };
        let monitor = monitor_under(pointer)?;
        let scale = whole_pixel_scale(monitor.dots_per_inch, &monitor.work_area, panel.width(), panel.height());
        let (width, height) = (panel.width() * scale, panel.height() * scale);
        let (left, top) = top_left_beside_the_tray(pointer, &monitor, width, height);
        let title = wide_null_terminated(PROGRAM_TITLE);
        let window = unsafe {
            CreateWindowExW(
                WS_EX_TOOLWINDOW | WS_EX_TOPMOST,
                class_name.as_ptr(),
                title.as_ptr(),
                WS_POPUP,
                left,
                top,
                width,
                height,
                null_mut(),
                null_mut(),
                win32::this_program(),
                null(),
            )
        };
        if window.is_null() {
            return None;
        }
        let small_rounded_corners = DWMWCP_ROUNDSMALL;
        unsafe {
            DwmSetWindowAttribute(
                window,
                DWMWA_WINDOW_CORNER_PREFERENCE as u32,
                (&small_rounded_corners as *const i32).cast(),
                std::mem::size_of::<i32>() as u32,
            );
            ShowWindow(window, SW_SHOW);
            SetForegroundWindow(window);
        }
        POINTER_IS_ON_A_CONTROL.set(false);
        Some(ControlPanelWindow { window, panel, scale, pointer: None })
    }

    pub fn point_at(&mut self, device_x: i32, device_y: i32) -> bool {
        self.pointer = Some((device_x, device_y));
        let hover_changed = self.panel.point_at(device_x / self.scale, device_y / self.scale);
        POINTER_IS_ON_A_CONTROL.set(self.panel.is_pointing_at_a_control());
        hover_changed
    }

    pub fn forget_the_pointer(&mut self) -> bool {
        self.pointer = None;
        POINTER_IS_ON_A_CONTROL.set(false);
        self.panel.point_at(-1, -1)
    }

    pub fn choice_at(&self, device_x: i32, device_y: i32) -> Option<MenuChoice> {
        self.panel.choice_at(device_x / self.scale, device_y / self.scale)
    }

    pub fn replace_panel(&mut self, panel: ControlPanel) {
        let height_changed = panel.height() != self.panel.height();
        self.panel = panel;
        if let Some((x, y)) = self.pointer {
            self.point_at(x, y);
        }
        if height_changed {
            let mut pointer = POINT { x: 0, y: 0 };
            unsafe { GetCursorPos(&mut pointer) };
            if let Some(monitor) = monitor_under(pointer) {
                let (width, height) = (self.panel.width() * self.scale, self.panel.height() * self.scale);
                let (left, top) = top_left_beside_the_tray(pointer, &monitor, width, height);
                unsafe {
                    SetWindowPos(self.window, null_mut(), left, top, width, height, SWP_NOZORDER | SWP_NOACTIVATE)
                };
            }
        }
    }

    pub fn redraw(&mut self, wallpaper_frame: &Canvas) {
        self.panel.draw(wallpaper_frame);
        self.paint();
    }

    pub fn paint(&self) {
        let canvas = &self.panel.canvas;
        let bitmap = top_down_bitmap_description(canvas.width, canvas.height);
        unsafe {
            let device_context = GetDC(self.window);
            if device_context.is_null() {
                return;
            }
            SetStretchBltMode(device_context, COLORONCOLOR);
            StretchDIBits(
                device_context,
                0,
                0,
                canvas.width * self.scale,
                canvas.height * self.scale,
                0,
                0,
                canvas.width,
                canvas.height,
                canvas.pixels.as_ptr().cast(),
                &bitmap,
                DIB_RGB_COLORS,
                SRCCOPY,
            );
            ReleaseDC(self.window, device_context);
        }
    }
}

impl Drop for ControlPanelWindow {
    fn drop(&mut self) {
        POINTER_IS_ON_A_CONTROL.set(false);
        unsafe { DestroyWindow(self.window) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn monitor(screen: RECT, work_area: RECT) -> Monitor {
        Monitor {
            handle: 1,
            rectangle: screen,
            work_area,
            dots_per_inch: 96,
            is_primary: true,
            device_name: "test".to_string(),
        }
    }

    #[test]
    fn the_panel_grows_with_the_display_scale_but_always_fits_the_screen() {
        let full_hd_work_area = RECT { left: 0, top: 0, right: 1920, bottom: 1032 };
        assert_eq!(whole_pixel_scale(96, &full_hd_work_area, 248, 362), 2);
        assert_eq!(whole_pixel_scale(144, &full_hd_work_area, 248, 362), 2);
        let four_k_work_area = RECT { left: 0, top: 0, right: 3840, bottom: 2064 };
        assert_eq!(whole_pixel_scale(144, &four_k_work_area, 248, 362), 3);
        assert_eq!(whole_pixel_scale(192, &four_k_work_area, 248, 362), 4);
        let tiny_work_area = RECT { left: 0, top: 0, right: 300, bottom: 300 };
        assert_eq!(whole_pixel_scale(96, &tiny_work_area, 248, 362), 1);
    }

    #[test]
    fn the_panel_sits_above_a_bottom_taskbar_under_the_pointer_and_inside_the_work_area() {
        let screen = RECT { left: 0, top: 0, right: 1920, bottom: 1080 };
        let bottom_taskbar = monitor(screen, RECT { left: 0, top: 0, right: 1920, bottom: 1032 });
        let near_the_clock = POINT { x: 1800, y: 1050 };
        assert_eq!(top_left_beside_the_tray(near_the_clock, &bottom_taskbar, 496, 724), (1920 - 496, 1032 - 724 - 12));
        let in_the_middle = POINT { x: 960, y: 1050 };
        assert_eq!(top_left_beside_the_tray(in_the_middle, &bottom_taskbar, 496, 724), (960 - 248, 1032 - 724 - 12));

        let top_taskbar = monitor(screen, RECT { left: 0, top: 48, right: 1920, bottom: 1080 });
        assert_eq!(top_left_beside_the_tray(POINT { x: 1800, y: 20 }, &top_taskbar, 496, 724), (1920 - 496, 60));
    }
}
