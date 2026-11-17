use crate::host::win32::{top_down_bitmap_description, wide_null_terminated};
use duke_core::wallpaper::menu::chapel_tower_icon_pixels;
use std::ptr::{null, null_mut};
use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::Graphics::Gdi::{
    CreateBitmap, CreateDIBSection, DeleteObject, GetDC, ReleaseDC, DIB_RGB_COLORS,
};
use windows_sys::Win32::UI::Shell::{
    Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_SHOWTIP, NIF_TIP, NIM_ADD, NIM_DELETE, NIM_MODIFY, NIM_SETVERSION,
    NOTIFYICONDATAW, NOTIFYICON_VERSION_4,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{CreateIconIndirect, HICON, ICONINFO, WM_APP};

pub const MESSAGE_SENT_WHEN_THE_TRAY_ICON_IS_USED: u32 = WM_APP + 1;
const OUR_ONLY_TRAY_ICON: u32 = 1;
const SMALLEST_ICON_SIDE: i32 = 16;
const LARGEST_ICON_SIDE: i32 = 256;
pub fn create_chapel_tower_icon(side: i32) -> HICON {
    let side = side.clamp(SMALLEST_ICON_SIDE, LARGEST_ICON_SIDE);
    let colour_bitmap_description = top_down_bitmap_description(side, side);
    unsafe {
        let screen = GetDC(null_mut());
        let mut colour_bitmap_pixels: *mut core::ffi::c_void = null_mut();
        let colour_bitmap = CreateDIBSection(
            screen,
            &colour_bitmap_description,
            DIB_RGB_COLORS,
            &mut colour_bitmap_pixels,
            null_mut(),
            0,
        );
        ReleaseDC(null_mut(), screen);
        if colour_bitmap.is_null() || colour_bitmap_pixels.is_null() {
            return null_mut();
        }
        let pixel_count = (side * side) as usize;
        std::slice::from_raw_parts_mut(colour_bitmap_pixels as *mut u32, pixel_count)
            .copy_from_slice(&chapel_tower_icon_pixels(side));

        let one_bit_mask_unused_because_the_colour_bitmap_has_alpha = CreateBitmap(side, side, 1, 1, null());
        let icon_parts = ICONINFO {
            fIcon: 1,
            xHotspot: 0,
            yHotspot: 0,
            hbmMask: one_bit_mask_unused_because_the_colour_bitmap_has_alpha,
            hbmColor: colour_bitmap,
        };
        let icon = CreateIconIndirect(&icon_parts);
        DeleteObject(colour_bitmap);
        DeleteObject(one_bit_mask_unused_because_the_colour_bitmap_has_alpha);
        icon
    }
}

fn tray_icon_description(control_window: HWND, icon: HICON, tooltip: &str) -> NOTIFYICONDATAW {
    let mut description: NOTIFYICONDATAW = unsafe { std::mem::zeroed() };
    description.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
    description.hWnd = control_window;
    description.uID = OUR_ONLY_TRAY_ICON;
    description.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP | NIF_SHOWTIP;
    description.uCallbackMessage = MESSAGE_SENT_WHEN_THE_TRAY_ICON_IS_USED;
    description.hIcon = icon;
    let tooltip = wide_null_terminated(tooltip);
    let characters_that_fit = tooltip.len().min(description.szTip.len() - 1);
    description.szTip[..characters_that_fit].copy_from_slice(&tooltip[..characters_that_fit]);
    description.szTip[characters_that_fit] = 0;
    description.Anonymous.uVersion = NOTIFYICON_VERSION_4;
    description
}

pub fn show_tray_icon(control_window: HWND, icon: HICON, tooltip: &str) -> bool {
    let description = tray_icon_description(control_window, icon, tooltip);
    unsafe {
        let remove_a_registration_left_by_a_previous_explorer = NIM_DELETE;
        Shell_NotifyIconW(remove_a_registration_left_by_a_previous_explorer, &description);
        let was_added = Shell_NotifyIconW(NIM_ADD, &description) != 0;
        if was_added {
            Shell_NotifyIconW(NIM_SETVERSION, &description);
        }
        was_added
    }
}

pub fn set_tray_tooltip(control_window: HWND, icon: HICON, tooltip: &str) {
    let description = tray_icon_description(control_window, icon, tooltip);
    unsafe { Shell_NotifyIconW(NIM_MODIFY, &description) };
}

pub fn remove_tray_icon(control_window: HWND) {
    let description = tray_icon_description(control_window, null_mut(), "");
    unsafe { Shell_NotifyIconW(NIM_DELETE, &description) };
}
