use std::ptr::{null, null_mut};
use windows_sys::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{BITMAPINFO, BITMAPINFOHEADER, BI_RGB, HBRUSH};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, GetClassNameW, GetWindow, GetWindowLongPtrW, GetWindowRect, GetWindowThreadProcessId,
    LoadCursorW, PeekMessageW, RegisterClassExW, TranslateMessage, GWL_EXSTYLE, GW_CHILD, GW_HWNDNEXT, IDC_ARROW, MSG,
    PM_REMOVE, WM_QUIT, WNDCLASSEXW,
};

pub type WindowProcedure = unsafe extern "system" fn(HWND, u32, WPARAM, LPARAM) -> LRESULT;

pub const EMPTY_RECTANGLE: RECT = RECT { left: 0, top: 0, right: 0, bottom: 0 };
const LONGEST_CLASS_NAME: usize = 128;
const MOST_CHILD_WINDOWS_TO_LIST: usize = 4096;
const BITS_PER_PIXEL: u16 = 32;

pub fn wide_null_terminated(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

pub fn this_program() -> HINSTANCE {
    unsafe { GetModuleHandleW(null()) }
}

pub fn class_name_of(window: HWND) -> String {
    let mut class_name = [0u16; LONGEST_CLASS_NAME];
    let length = unsafe { GetClassNameW(window, class_name.as_mut_ptr(), LONGEST_CLASS_NAME as i32) };
    String::from_utf16_lossy(&class_name[..length.max(0) as usize])
}

pub fn screen_rectangle_of(window: HWND) -> RECT {
    let mut rectangle = EMPTY_RECTANGLE;
    unsafe { GetWindowRect(window, &mut rectangle) };
    rectangle
}

pub fn extended_style_of(window: HWND) -> u32 {
    unsafe { GetWindowLongPtrW(window, GWL_EXSTYLE) as u32 }
}

pub fn process_id_owning(window: HWND) -> u32 {
    let mut process_id = 0u32;
    unsafe { GetWindowThreadProcessId(window, &mut process_id) };
    process_id
}

pub fn child_windows_top_first(parent: HWND) -> Vec<HWND> {
    let mut children = Vec::new();
    let mut child = unsafe { GetWindow(parent, GW_CHILD) };
    while !child.is_null() && children.len() < MOST_CHILD_WINDOWS_TO_LIST {
        children.push(child);
        child = unsafe { GetWindow(child, GW_HWNDNEXT) };
    }
    children
}

pub fn describe_window_handle(window: HWND) -> String {
    format!("0x{:08X}", window as usize)
}

pub fn describe_rectangle(rectangle: &RECT) -> String {
    let width = rectangle.right - rectangle.left;
    let height = rectangle.bottom - rectangle.top;
    format!("({},{}) {width}x{height}", rectangle.left, rectangle.top)
}

pub fn same_rectangle(one: &RECT, other: &RECT) -> bool {
    one.left == other.left && one.top == other.top && one.right == other.right && one.bottom == other.bottom
}

pub fn rectangle_contains(outer: &RECT, inner: &RECT) -> bool {
    outer.left <= inner.left && outer.top <= inner.top && outer.right >= inner.right && outer.bottom >= inner.bottom
}

pub fn top_down_bitmap_description(width: i32, height: i32) -> BITMAPINFO {
    let negative_height_means_top_row_first = -height;
    BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width,
            biHeight: negative_height_means_top_row_first,
            biPlanes: 1,
            biBitCount: BITS_PER_PIXEL,
            biCompression: BI_RGB,
            biSizeImage: 0,
            biXPelsPerMeter: 0,
            biYPelsPerMeter: 0,
            biClrUsed: 0,
            biClrImportant: 0,
        },
        bmiColors: unsafe { std::mem::zeroed() },
    }
}

pub fn register_window_class(class_name: &str, window_procedure: WindowProcedure, background_brush: HBRUSH) {
    let class_name = wide_null_terminated(class_name);
    let class = WNDCLASSEXW {
        cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
        style: 0,
        lpfnWndProc: Some(window_procedure),
        cbClsExtra: 0,
        cbWndExtra: 0,
        hInstance: this_program(),
        hIcon: null_mut(),
        hCursor: unsafe { LoadCursorW(null_mut(), IDC_ARROW) },
        hbrBackground: background_brush,
        lpszMenuName: null(),
        lpszClassName: class_name.as_ptr(),
        hIconSm: null_mut(),
    };
    unsafe { RegisterClassExW(&class) };
}

pub fn dispatch_waiting_window_messages() -> bool {
    let mut quit_message_seen = false;
    let mut message: MSG = unsafe { std::mem::zeroed() };
    while unsafe { PeekMessageW(&mut message, null_mut(), 0, 0, PM_REMOVE) } != 0 {
        if message.message == WM_QUIT {
            quit_message_seen = true;
            continue;
        }
        unsafe {
            TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
    quit_message_seen
}
