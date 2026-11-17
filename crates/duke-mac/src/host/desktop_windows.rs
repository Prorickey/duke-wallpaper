use crate::host::images::show_canvas_in_layer;
use duke_core::wallpaper::config::Fit;
use duke_core::wallpaper::fit::place_canvas;
use duke_core::Canvas;
use objc2::rc::Retained;
use objc2::{MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{
    NSBackingStoreType, NSScreen, NSWindow, NSWindowCollectionBehavior, NSWindowOcclusionState, NSWindowStyleMask,
};
use objc2_core_graphics::{CGColor, CGWindowLevelForKey, CGWindowLevelKey};
use objc2_foundation::{NSPoint, NSRect, NSSize};
use objc2_quartz_core::{kCAFilterNearest, kCAGravityResize, CALayer, CATransaction};

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Screen {
    pub frame: NSRect,
    pub backing_scale: f64,
}

pub fn screens_now(mtm: MainThreadMarker) -> Vec<Screen> {
    NSScreen::screens(mtm)
        .iter()
        .map(|screen| Screen { frame: screen.frame(), backing_scale: screen.backingScaleFactor() })
        .collect()
}

fn colour_from(red_green_blue: u32) -> objc2_core_foundation::CFRetained<CGColor> {
    let channel = |shift: u32| ((red_green_blue >> shift) & 0xFF) as f64 / 255.0;
    CGColor::new_srgb(channel(16), channel(8), channel(0), 1.0)
}

pub fn without_animation(change: impl FnOnce()) {
    CATransaction::begin();
    CATransaction::setDisableActions(true);
    change();
    CATransaction::commit();
}

pub struct DesktopWindow {
    window: Retained<NSWindow>,
    background: Retained<CALayer>,
    picture: Retained<CALayer>,
    screen: Screen,
    pub is_covered: bool,
}

impl DesktopWindow {
    pub fn cover(screen: Screen, fit: Fit, letterbox_colour: u32, mtm: MainThreadMarker) -> Option<DesktopWindow> {
        let window = unsafe {
            NSWindow::initWithContentRect_styleMask_backing_defer(
                NSWindow::alloc(mtm),
                screen.frame,
                NSWindowStyleMask::Borderless,
                NSBackingStoreType::Buffered,
                false,
            )
        };
        unsafe { window.setReleasedWhenClosed(false) };
        window.setLevel(CGWindowLevelForKey(CGWindowLevelKey::DesktopWindowLevelKey) as isize);
        window.setCollectionBehavior(
            NSWindowCollectionBehavior::CanJoinAllSpaces
                | NSWindowCollectionBehavior::Stationary
                | NSWindowCollectionBehavior::IgnoresCycle,
        );
        window.setIgnoresMouseEvents(true);
        window.setOpaque(true);
        window.setHasShadow(false);
        let content = window.contentView()?;
        content.setWantsLayer(true);
        let background = content.layer()?;
        let picture = CALayer::new();
        picture.setMagnificationFilter(unsafe { kCAFilterNearest });
        picture.setContentsGravity(unsafe { kCAGravityResize });
        background.addSublayer(&picture);
        let desktop_window = DesktopWindow { window, background, picture, screen, is_covered: false };
        desktop_window.arrange(fit, letterbox_colour);
        desktop_window.window.orderFrontRegardless();
        Some(desktop_window)
    }

    pub fn arrange(&self, fit: Fit, letterbox_colour: u32) {
        let scale = self.screen.backing_scale.max(1.0);
        let device_width = (self.screen.frame.size.width * scale).round() as i32;
        let device_height = (self.screen.frame.size.height * scale).round() as i32;
        let placement = place_canvas(fit, device_width, device_height);
        let bottom_in_device_pixels = device_height - placement.top - placement.height;
        let picture_frame = NSRect::new(
            NSPoint::new(placement.left as f64 / scale, bottom_in_device_pixels as f64 / scale),
            NSSize::new(placement.width as f64 / scale, placement.height as f64 / scale),
        );
        without_animation(|| {
            self.background.setBackgroundColor(Some(&colour_from(letterbox_colour)));
            self.picture.setFrame(picture_frame);
        });
    }

    pub fn show(&self, frame: &Canvas) {
        if !self.is_covered {
            without_animation(|| show_canvas_in_layer(frame, &self.picture));
        }
    }

    pub fn can_be_seen(&self) -> bool {
        self.window.occlusionState().contains(NSWindowOcclusionState::Visible)
    }
}

impl Drop for DesktopWindow {
    fn drop(&mut self) {
        self.window.orderOut(None);
        self.window.close();
    }
}
