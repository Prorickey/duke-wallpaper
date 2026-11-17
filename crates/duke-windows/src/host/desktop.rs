use crate::host::win32::{
    child_windows_top_first, class_name_of, describe_rectangle, describe_window_handle, extended_style_of,
    process_id_owning, screen_rectangle_of, wide_null_terminated,
};
use std::fmt::Write as _;
use std::ptr::{null, null_mut};
use std::time::Duration;
use windows_sys::Win32::Foundation::{GetLastError, SetLastError, HWND, POINT, RECT};
use windows_sys::Win32::Graphics::Gdi::{
    MapWindowPoints, RedrawWindow, RDW_ALLCHILDREN, RDW_ERASE, RDW_INVALIDATE, RDW_UPDATENOW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    FindWindowExW, FindWindowW, GetAncestor, GetWindowLongPtrW, IsWindow, IsWindowVisible, SendMessageTimeoutW,
    SetLayeredWindowAttributes, SetParent, SetWindowLongPtrW, SetWindowPos, SystemParametersInfoW, GA_PARENT,
    GWL_EXSTYLE, GWL_STYLE, HWND_BOTTOM, LWA_ALPHA, SMTO_NORMAL, SPI_SETDESKWALLPAPER, SWP_NOACTIVATE, SWP_NOMOVE,
    SWP_NOSIZE, SWP_NOZORDER, SWP_SHOWWINDOW, WS_CHILD, WS_EX_LAYERED, WS_POPUP,
};

const MESSAGE_ASKING_PROGMAN_TO_MOVE_THE_WALLPAPER_INTO_ITS_OWN_WORKERW: u32 = 0x052C;
const ARGUMENTS_CURRENT_WINDOWS_EXPECTS: (usize, isize) = (0xD, 0x1);
const ARGUMENTS_OLDER_WINDOWS_EXPECTS: (usize, isize) = (0, 0);
const MILLISECONDS_TO_WAIT_FOR_PROGMAN_TO_ANSWER: u32 = 1000;
const TIMES_TO_LOOK_FOR_THE_NEW_WORKERW: u32 = 10;
const PAUSE_BETWEEN_LOOKS_FOR_THE_NEW_WORKERW: Duration = Duration::from_millis(30);
const WINDOW_HAS_NO_DRAWING_SURFACE_OF_ITS_OWN: u32 = 0x0020_0000;
const MOST_WORKERW_WINDOWS_TO_EXAMINE: usize = 256;
const MOST_GRANDCHILDREN_TO_DESCRIBE: usize = 8;
const FULLY_OPAQUE: u8 = 255;
const DIRECTLY_BELOW_NOTHING_MEANS_ON_TOP: HWND = null_mut();
const NULL_PATH_MEANS_RELOAD_THE_CURRENT_WALLPAPER: *mut core::ffi::c_void = null_mut();
const WRITE_NOTHING_TO_THE_USER_PROFILE: u32 = 0;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum DesktopLayout {
    IconsAndWallpaperAreChildrenOfProgman,
    IconsHaveTheirOwnWorkerWInFrontOfTheWallpaperWorkerW,
    ProgmanHoldsTheIconsAndThereIsNoWallpaperWorkerW,
}

impl DesktopLayout {
    pub fn description(self) -> &'static str {
        match self {
            DesktopLayout::IconsAndWallpaperAreChildrenOfProgman => {
                "raised desktop (Windows 11 24H2+): icons and wallpaper WorkerW are children of Progman"
            }
            DesktopLayout::IconsHaveTheirOwnWorkerWInFrontOfTheWallpaperWorkerW => {
                "classic (Windows 8 - 11 23H2): icons in a top-level WorkerW, wallpaper WorkerW behind it"
            }
            DesktopLayout::ProgmanHoldsTheIconsAndThereIsNoWallpaperWorkerW => {
                "Progman only: no wallpaper WorkerW (0x052C not sent yet, or not honoured)"
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct Desktop {
    pub progman: HWND,
    pub explorer_process_id: u32,
    pub progman_has_no_drawing_surface: bool,
    pub icon_view: HWND,
    pub icon_view_parent: HWND,
    pub wallpaper_workerw: HWND,
    pub layout: DesktopLayout,
    pub explorer_top_level_workerws: Vec<HWND>,
}

fn find_window_of_class(parent: HWND, after_sibling: HWND, class_name: &str) -> HWND {
    let class_name = wide_null_terminated(class_name);
    unsafe { FindWindowExW(parent, after_sibling, class_name.as_ptr(), null()) }
}

pub fn find_progman() -> HWND {
    let class_name = wide_null_terminated("Progman");
    unsafe { FindWindowW(class_name.as_ptr(), null()) }
}

pub fn icon_view_directly_under(parent: HWND) -> HWND {
    find_window_of_class(parent, null_mut(), "SHELLDLL_DefView")
}

pub fn look_at_desktop() -> Option<Desktop> {
    let progman = find_progman();
    if progman.is_null() {
        return None;
    }
    let explorer_process_id = process_id_owning(progman);
    let progman_has_no_drawing_surface = extended_style_of(progman) & WINDOW_HAS_NO_DRAWING_SURFACE_OF_ITS_OWN != 0;

    let mut explorer_top_level_workerws = Vec::new();
    let mut workerw = find_window_of_class(null_mut(), null_mut(), "WorkerW");
    let mut workerws_examined = 0;
    while !workerw.is_null() && workerws_examined < MOST_WORKERW_WINDOWS_TO_EXAMINE {
        if process_id_owning(workerw) == explorer_process_id {
            explorer_top_level_workerws.push(workerw);
        }
        workerw = find_window_of_class(null_mut(), workerw, "WorkerW");
        workerws_examined += 1;
    }

    let mut icon_view = icon_view_directly_under(progman);
    let mut icon_view_parent = if icon_view.is_null() { null_mut() } else { progman };
    if icon_view.is_null() {
        for &workerw in &explorer_top_level_workerws {
            let icon_view_in_this_workerw = icon_view_directly_under(workerw);
            if !icon_view_in_this_workerw.is_null() {
                icon_view = icon_view_in_this_workerw;
                icon_view_parent = workerw;
                break;
            }
        }
    }

    let icons_have_their_own_workerw = !icon_view_parent.is_null() && icon_view_parent != progman;
    let workerw_inside_progman = find_window_of_class(progman, null_mut(), "WorkerW");
    let layout;
    let wallpaper_workerw;
    if icons_have_their_own_workerw {
        layout = DesktopLayout::IconsHaveTheirOwnWorkerWInFrontOfTheWallpaperWorkerW;
        let mut next_workerw_behind_the_icons = find_window_of_class(null_mut(), icon_view_parent, "WorkerW");
        let mut workerws_examined = 0;
        while !next_workerw_behind_the_icons.is_null()
            && process_id_owning(next_workerw_behind_the_icons) != explorer_process_id
            && workerws_examined < MOST_WORKERW_WINDOWS_TO_EXAMINE
        {
            next_workerw_behind_the_icons = find_window_of_class(null_mut(), next_workerw_behind_the_icons, "WorkerW");
            workerws_examined += 1;
        }
        let belongs_to_explorer = !next_workerw_behind_the_icons.is_null()
            && process_id_owning(next_workerw_behind_the_icons) == explorer_process_id;
        wallpaper_workerw = if belongs_to_explorer { next_workerw_behind_the_icons } else { null_mut() };
    } else if progman_has_no_drawing_surface || !workerw_inside_progman.is_null() {
        layout = DesktopLayout::IconsAndWallpaperAreChildrenOfProgman;
        wallpaper_workerw = workerw_inside_progman;
    } else {
        layout = DesktopLayout::ProgmanHoldsTheIconsAndThereIsNoWallpaperWorkerW;
        wallpaper_workerw = null_mut();
    }

    Some(Desktop {
        progman,
        explorer_process_id,
        progman_has_no_drawing_surface,
        icon_view,
        icon_view_parent,
        wallpaper_workerw,
        layout,
        explorer_top_level_workerws,
    })
}

pub fn look_at_desktop_after_asking_for_a_wallpaper_workerw() -> Option<Desktop> {
    let mut desktop = look_at_desktop()?;
    for (first_argument, second_argument) in [ARGUMENTS_CURRENT_WINDOWS_EXPECTS, ARGUMENTS_OLDER_WINDOWS_EXPECTS] {
        if !desktop.wallpaper_workerw.is_null() {
            break;
        }
        let mut answer_from_progman = 0usize;
        let progman_answered = unsafe {
            SendMessageTimeoutW(
                desktop.progman,
                MESSAGE_ASKING_PROGMAN_TO_MOVE_THE_WALLPAPER_INTO_ITS_OWN_WORKERW,
                first_argument,
                second_argument,
                SMTO_NORMAL,
                MILLISECONDS_TO_WAIT_FOR_PROGMAN_TO_ANSWER,
                &mut answer_from_progman,
            )
        } != 0;
        duke_core::info!(
            "sent 0x052C({first_argument:#x}, {second_argument:#x}) to Progman: {}",
            if progman_answered { "ok" } else { "no answer" }
        );
        for _ in 0..TIMES_TO_LOOK_FOR_THE_NEW_WORKERW {
            desktop = look_at_desktop()?;
            if !desktop.wallpaper_workerw.is_null() {
                break;
            }
            std::thread::sleep(PAUSE_BETWEEN_LOOKS_FOR_THE_NEW_WORKERW);
        }
    }
    Some(desktop)
}

impl Desktop {
    pub fn parent_for_wallpaper_windows(&self) -> HWND {
        let icons_are_in_front_of_a_wallpaper_workerw = self.layout
            == DesktopLayout::IconsHaveTheirOwnWorkerWInFrontOfTheWallpaperWorkerW
            && !self.wallpaper_workerw.is_null();
        if icons_are_in_front_of_a_wallpaper_workerw {
            self.wallpaper_workerw
        } else {
            self.progman
        }
    }

    pub fn only_layered_children_are_shown(&self) -> bool {
        self.progman_has_no_drawing_surface || self.layout == DesktopLayout::IconsAndWallpaperAreChildrenOfProgman
    }

    pub fn how_wallpaper_windows_attach(&self) -> &'static str {
        match self.layout {
            DesktopLayout::IconsAndWallpaperAreChildrenOfProgman => {
                "layered child of Progman, between the icon view and the wallpaper WorkerW"
            }
            DesktopLayout::IconsHaveTheirOwnWorkerWInFrontOfTheWallpaperWorkerW => {
                if self.wallpaper_workerw.is_null() {
                    "child of Progman (no wallpaper WorkerW found behind the icon host)"
                } else {
                    "child of the wallpaper WorkerW behind the icons"
                }
            }
            DesktopLayout::ProgmanHoldsTheIconsAndThereIsNoWallpaperWorkerW => {
                "child of Progman below the icon view (no WorkerW available)"
            }
        }
    }
}

pub fn turn_hidden_window_into_child_of_desktop(
    wallpaper_window: HWND,
    parent: HWND,
    only_layered_children_are_shown: bool,
) -> Result<(), String> {
    unsafe {
        if only_layered_children_are_shown {
            let extended_style_while_still_top_level = extended_style_of(wallpaper_window) | WS_EX_LAYERED;
            SetWindowLongPtrW(wallpaper_window, GWL_EXSTYLE, extended_style_while_still_top_level as isize);
            if SetLayeredWindowAttributes(wallpaper_window, 0, FULLY_OPAQUE, LWA_ALPHA) == 0 {
                return Err(format!("SetLayeredWindowAttributes failed (error {})", GetLastError()));
            }
        }
        let style = GetWindowLongPtrW(wallpaper_window, GWL_STYLE) as u32;
        let child_instead_of_popup = (style & !WS_POPUP) | WS_CHILD;
        SetWindowLongPtrW(wallpaper_window, GWL_STYLE, child_instead_of_popup as isize);

        SetLastError(0);
        let previous_parent = SetParent(wallpaper_window, parent);
        if previous_parent.is_null() && GetLastError() != 0 {
            return Err(format!("SetParent failed (error {})", GetLastError()));
        }
        if GetAncestor(wallpaper_window, GA_PARENT) != parent {
            return Err("SetParent did not take effect".into());
        }
    }
    Ok(())
}

pub fn cover_monitor_and_show(wallpaper_window: HWND, parent: HWND, monitor_rectangle: &RECT) -> bool {
    let mut corners = [
        POINT { x: monitor_rectangle.left, y: monitor_rectangle.top },
        POINT { x: monitor_rectangle.right, y: monitor_rectangle.bottom },
    ];
    let from_screen_coordinates: HWND = null_mut();
    unsafe { MapWindowPoints(from_screen_coordinates, parent, corners.as_mut_ptr(), corners.len() as u32) };
    let [top_left, bottom_right] = corners;
    let depth_is_left_alone: HWND = null_mut();
    unsafe {
        SetWindowPos(
            wallpaper_window,
            depth_is_left_alone,
            top_left.x,
            top_left.y,
            bottom_right.x - top_left.x,
            bottom_right.y - top_left.y,
            SWP_NOZORDER | SWP_NOACTIVATE | SWP_SHOWWINDOW,
        ) != 0
    }
}

pub fn keep_below_icons_and_above_wallpaper(parent: HWND, wallpaper_windows: &[HWND]) -> bool {
    let siblings_top_first = child_windows_top_first(parent);
    let icon_view = icon_view_directly_under(parent);
    let mut icon_view_depth = None;
    let mut shallowest_workerw_depth = None;
    for (depth, &sibling) in siblings_top_first.iter().enumerate() {
        if sibling == icon_view {
            icon_view_depth = Some(depth);
        }
        if shallowest_workerw_depth.is_none() && class_name_of(sibling) == "WorkerW" {
            shallowest_workerw_depth = Some(depth);
        }
    }

    let mut every_window_is_in_place = true;
    for &wallpaper_window in wallpaper_windows {
        let Some(depth) = siblings_top_first.iter().position(|&sibling| sibling == wallpaper_window) else {
            every_window_is_in_place = false;
            continue;
        };
        let is_below_the_icons = icon_view_depth.is_none_or(|icon_view_depth| depth > icon_view_depth);
        let is_above_the_wallpaper = shallowest_workerw_depth.is_none_or(|workerw_depth| depth < workerw_depth);
        if !is_below_the_icons || !is_above_the_wallpaper {
            every_window_is_in_place = false;
        }
    }
    if every_window_is_in_place {
        return false;
    }

    let directly_below = if icon_view.is_null() { DIRECTLY_BELOW_NOTHING_MEANS_ON_TOP } else { icon_view };
    let change_depth_only = SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE;
    unsafe {
        for &wallpaper_window in wallpaper_windows {
            SetWindowPos(wallpaper_window, directly_below, 0, 0, 0, 0, change_depth_only);
        }
        for &sibling in &siblings_top_first {
            if class_name_of(sibling) == "WorkerW" {
                SetWindowPos(sibling, HWND_BOTTOM, 0, 0, 0, 0, change_depth_only);
            }
        }
    }
    true
}

pub fn make_static_wallpaper_reappear(layout: DesktopLayout, parent: HWND, progman: HWND) {
    let our_windows_were_separate_layers_above_the_wallpaper =
        layout == DesktopLayout::IconsAndWallpaperAreChildrenOfProgman;
    if our_windows_were_separate_layers_above_the_wallpaper {
        return;
    }
    unsafe {
        for window_we_drew_inside in [parent, progman] {
            if IsWindow(window_we_drew_inside) != 0 {
                RedrawWindow(
                    window_we_drew_inside,
                    null(),
                    null_mut(),
                    RDW_INVALIDATE | RDW_ERASE | RDW_ALLCHILDREN | RDW_UPDATENOW,
                );
            }
        }
        SystemParametersInfoW(
            SPI_SETDESKWALLPAPER,
            0,
            NULL_PATH_MEANS_RELOAD_THE_CURRENT_WALLPAPER,
            WRITE_NOTHING_TO_THE_USER_PROFILE,
        );
    }
}

fn describe_window(description: &mut String, indent: &str, window: HWND, role: &str) {
    let style = unsafe { GetWindowLongPtrW(window, GWL_STYLE) } as u32;
    let extended_style = extended_style_of(window);
    let mut notable_styles = Vec::new();
    if extended_style & WS_EX_LAYERED != 0 {
        notable_styles.push("LAYERED");
    }
    if extended_style & WINDOW_HAS_NO_DRAWING_SURFACE_OF_ITS_OWN != 0 {
        notable_styles.push("NOREDIRECTIONBITMAP");
    }
    let notable_styles =
        if notable_styles.is_empty() { String::new() } else { format!(" [{}]", notable_styles.join(",")) };
    let visibility = if unsafe { IsWindowVisible(window) } != 0 { "visible" } else { "hidden " };
    let role = if role.is_empty() { String::new() } else { format!("  <- {role}") };
    let _ = writeln!(
        description,
        "{indent}{} {:<18} {:<22} {visibility} style={style:#010x} ex={extended_style:#010x}{notable_styles}{role}",
        describe_window_handle(window),
        class_name_of(window),
        describe_rectangle(&screen_rectangle_of(window)),
    );
}

pub fn describe_hierarchy(desktop: &Desktop, wallpaper_windows: &[HWND]) -> String {
    let role_of = |window: HWND| -> &'static str {
        if window == desktop.icon_view {
            "desktop icons"
        } else if window == desktop.wallpaper_workerw {
            "wallpaper WorkerW"
        } else if wallpaper_windows.contains(&window) {
            "duke-wallpaper"
        } else if window == desktop.icon_view_parent && window != desktop.progman {
            "icon host"
        } else {
            ""
        }
    };

    let mut description = String::new();
    let _ = writeln!(description, "explorer.exe pid {}", desktop.explorer_process_id);
    describe_window(&mut description, "", desktop.progman, "");
    for child in child_windows_top_first(desktop.progman) {
        describe_window(&mut description, "  ", child, role_of(child));
        for grandchild in child_windows_top_first(child).into_iter().take(MOST_GRANDCHILDREN_TO_DESCRIBE) {
            describe_window(&mut description, "    ", grandchild, role_of(grandchild));
        }
    }

    let mut hidden_helper_workerws = 0;
    for &workerw in &desktop.explorer_top_level_workerws {
        let children = child_windows_top_first(workerw);
        let matters_to_the_desktop = workerw == desktop.icon_view_parent
            || workerw == desktop.wallpaper_workerw
            || unsafe { IsWindowVisible(workerw) } != 0
            || !children.is_empty();
        if !matters_to_the_desktop {
            hidden_helper_workerws += 1;
            continue;
        }
        describe_window(&mut description, "", workerw, role_of(workerw));
        for child in children.into_iter().take(MOST_GRANDCHILDREN_TO_DESCRIBE) {
            describe_window(&mut description, "  ", child, role_of(child));
        }
    }
    let _ = writeln!(
        description,
        "({} top-level WorkerW windows owned by Explorer; {hidden_helper_workerws} hidden, childless ones not listed)",
        desktop.explorer_top_level_workerws.len()
    );
    description
}
