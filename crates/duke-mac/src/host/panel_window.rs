use crate::host::desktop_windows::without_animation;
use crate::host::images::show_canvas_in_layer;
use duke_core::wallpaper::control_panel::ControlPanel;
use duke_core::wallpaper::menu::MenuChoice;
use duke_core::Canvas;
use objc2::rc::Retained;
use objc2::{MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{
    NSBackingStoreType, NSEvent, NSPopUpMenuWindowLevel, NSScreen, NSWindow, NSWindowCollectionBehavior,
    NSWindowStyleMask,
};
use objc2_foundation::{NSPoint, NSRect, NSSize};
use objc2_quartz_core::{kCAFilterNearest, kCAGravityResize, CALayer};

const POINTS_BETWEEN_THE_MENU_BAR_AND_THE_PANEL: f64 = 6.0;
const POINTS_PER_PANEL_PIXEL_AT_STANDARD_SCALE: f64 = 2.0;
const LARGEST_SHARE_OF_THE_SCREEN_HEIGHT: f64 = 0.9;

fn rectangle_contains(rectangle: NSRect, point: NSPoint) -> bool {
    point.x >= rectangle.origin.x
        && point.x < rectangle.origin.x + rectangle.size.width
        && point.y >= rectangle.origin.y
        && point.y < rectangle.origin.y + rectangle.size.height
}

pub fn pointer_is_inside(rectangle: NSRect) -> bool {
    rectangle_contains(rectangle, NSEvent::mouseLocation())
}

pub struct PanelWindow {
    window: Retained<NSWindow>,
    picture: Retained<CALayer>,
    points_per_panel_pixel: f64,
    pub panel: ControlPanel,
    pub app_has_been_active_since_opening: bool,
}

impl PanelWindow {
    pub fn open(panel: ControlPanel, status_item_frame: NSRect, mtm: MainThreadMarker) -> Option<PanelWindow> {
        let screens = NSScreen::screens(mtm);
        let screen = screens
            .iter()
            .find(|screen| rectangle_contains(screen.frame(), status_item_frame.origin))
            .or_else(|| screens.firstObject())?;
        let visible = screen.visibleFrame();
        let backing_scale = screen.backingScaleFactor().max(1.0);
        let wanted_device_pixels = (POINTS_PER_PANEL_PIXEL_AT_STANDARD_SCALE * backing_scale).round();
        let fitting_device_pixels =
            (visible.size.height * LARGEST_SHARE_OF_THE_SCREEN_HEIGHT * backing_scale / panel.height() as f64).floor();
        let device_pixels_per_panel_pixel = wanted_device_pixels.min(fitting_device_pixels).max(1.0);
        let points_per_panel_pixel = device_pixels_per_panel_pixel / backing_scale;

        let size =
            NSSize::new(panel.width() as f64 * points_per_panel_pixel, panel.height() as f64 * points_per_panel_pixel);
        let preferred_left = status_item_frame.origin.x + status_item_frame.size.width / 2.0 - size.width / 2.0;
        let preferred_bottom = status_item_frame.origin.y - POINTS_BETWEEN_THE_MENU_BAR_AND_THE_PANEL - size.height;
        let left = preferred_left
            .clamp(visible.origin.x, (visible.origin.x + visible.size.width - size.width).max(visible.origin.x));
        let bottom = preferred_bottom.max(visible.origin.y);
        let frame = NSRect::new(NSPoint::new(left.round(), bottom.round()), size);

        let window = unsafe {
            NSWindow::initWithContentRect_styleMask_backing_defer(
                NSWindow::alloc(mtm),
                frame,
                NSWindowStyleMask::Borderless,
                NSBackingStoreType::Buffered,
                false,
            )
        };
        unsafe { window.setReleasedWhenClosed(false) };
        window.setLevel(NSPopUpMenuWindowLevel);
        window.setCollectionBehavior(
            NSWindowCollectionBehavior::CanJoinAllSpaces | NSWindowCollectionBehavior::FullScreenAuxiliary,
        );
        window.setOpaque(true);
        window.setHasShadow(true);
        let content = window.contentView()?;
        content.setWantsLayer(true);
        let background = content.layer()?;
        let picture = CALayer::new();
        picture.setMagnificationFilter(unsafe { kCAFilterNearest });
        picture.setContentsGravity(unsafe { kCAGravityResize });
        without_animation(|| picture.setFrame(NSRect::new(NSPoint::new(0.0, 0.0), size)));
        background.addSublayer(&picture);
        window.orderFrontRegardless();
        Some(PanelWindow { window, picture, points_per_panel_pixel, panel, app_has_been_active_since_opening: false })
    }

    pub fn frame(&self) -> NSRect {
        self.window.frame()
    }

    fn pointer_in_panel_pixels(&self) -> Option<(i32, i32)> {
        let frame = self.frame();
        let pointer = NSEvent::mouseLocation();
        if !rectangle_contains(frame, pointer) {
            return None;
        }
        let from_left = (pointer.x - frame.origin.x) / self.points_per_panel_pixel;
        let from_top = (frame.origin.y + frame.size.height - pointer.y) / self.points_per_panel_pixel;
        Some((from_left as i32, from_top as i32))
    }

    pub fn follow_the_pointer(&mut self) -> bool {
        let (x, y) = self.pointer_in_panel_pixels().unwrap_or((-1, -1));
        self.panel.point_at(x, y)
    }

    pub fn choice_under_the_pointer(&self) -> Option<MenuChoice> {
        let (x, y) = self.pointer_in_panel_pixels()?;
        self.panel.choice_at(x, y)
    }

    pub fn replace_panel(&mut self, panel: ControlPanel) {
        let height_changed = panel.height() != self.panel.height();
        self.panel = panel;
        self.follow_the_pointer();
        if height_changed {
            let old_frame = self.frame();
            let size = NSSize::new(
                self.panel.width() as f64 * self.points_per_panel_pixel,
                self.panel.height() as f64 * self.points_per_panel_pixel,
            );
            let top = old_frame.origin.y + old_frame.size.height;
            self.window.setFrame_display(NSRect::new(NSPoint::new(old_frame.origin.x, top - size.height), size), true);
            without_animation(|| self.picture.setFrame(NSRect::new(NSPoint::new(0.0, 0.0), size)));
        }
    }

    pub fn redraw(&mut self, wallpaper_frame: &Canvas) {
        let canvas = self.panel.draw(wallpaper_frame);
        without_animation(|| show_canvas_in_layer(canvas, &self.picture));
    }
}

impl Drop for PanelWindow {
    fn drop(&mut self) {
        self.window.orderOut(None);
        self.window.close();
    }
}
