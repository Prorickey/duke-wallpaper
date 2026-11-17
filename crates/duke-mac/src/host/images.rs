use duke_core::wallpaper::menu::chapel_tower_icon_pixels;
use duke_core::Canvas;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::AllocAnyThread;
use objc2_app_kit::NSImage;
use objc2_core_foundation::{CFData, CFRetained};
use objc2_core_graphics::{
    CGBitmapInfo, CGColorRenderingIntent, CGColorSpace, CGDataProvider, CGImage, CGImageAlphaInfo, CGImageByteOrderInfo,
};
use objc2_foundation::NSSize;
use objc2_quartz_core::CALayer;

const BYTES_PER_PIXEL: usize = 4;
const MENU_BAR_ICON_POINTS: f64 = 16.0;
const MENU_BAR_ICON_PIXELS: i32 = 32;

fn image_of_pixels(pixels: &[u32], width: i32, height: i32, alpha: CGImageAlphaInfo) -> Option<CFRetained<CGImage>> {
    let (width, height) = (width as usize, height as usize);
    let byte_count = width * height * BYTES_PER_PIXEL;
    let bytes = unsafe { CFData::new(None, pixels.as_ptr().cast(), byte_count as isize)? };
    let provider = CGDataProvider::with_cf_data(Some(&bytes))?;
    let colour_space = CGColorSpace::new_device_rgb()?;
    let little_endian_pixels_read_as_blue_green_red_then_alpha =
        CGBitmapInfo(CGImageByteOrderInfo::Order32Little.0 | alpha.0);
    unsafe {
        CGImage::new(
            width,
            height,
            8,
            8 * BYTES_PER_PIXEL,
            width * BYTES_PER_PIXEL,
            Some(&colour_space),
            little_endian_pixels_read_as_blue_green_red_then_alpha,
            Some(&provider),
            std::ptr::null(),
            false,
            CGColorRenderingIntent::RenderingIntentDefault,
        )
    }
}

pub fn show_canvas_in_layer(canvas: &Canvas, layer: &CALayer) {
    let Some(image) = image_of_pixels(&canvas.pixels, canvas.width, canvas.height, CGImageAlphaInfo::NoneSkipFirst)
    else {
        return;
    };
    let image_as_an_object: &AnyObject = unsafe { &*(CFRetained::as_ptr(&image).as_ptr() as *const AnyObject) };
    unsafe { layer.setContents(Some(image_as_an_object)) };
}

pub fn chapel_tower_menu_bar_icon() -> Option<Retained<NSImage>> {
    let pixels = chapel_tower_icon_pixels(MENU_BAR_ICON_PIXELS);
    let image =
        image_of_pixels(&pixels, MENU_BAR_ICON_PIXELS, MENU_BAR_ICON_PIXELS, CGImageAlphaInfo::PremultipliedFirst)?;
    let size = NSSize::new(MENU_BAR_ICON_POINTS, MENU_BAR_ICON_POINTS);
    Some(NSImage::initWithCGImage_size(NSImage::alloc(), &image, size))
}
