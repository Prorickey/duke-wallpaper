pub mod atmosphere;
mod canvas;
pub mod chapel;
mod colour;
pub mod east;
mod environment;
#[cfg(test)]
mod preview;
mod randomness;
mod scene;
pub mod wallpaper;

#[cfg(test)]
mod tests;

pub use canvas::Canvas;
pub use colour::{
    add_colours, blend_colours, colour_channels, interpolate, rgb, scale_brightness, tint_colour, Colour,
};
pub use environment::{apply_lighting, Environment, Lighting, Season, Weather};
pub use randomness::{
    fold_seed_to_32_bits, fractal_noise, hash_fraction, hash_position, value_noise, RandomNumberGenerator,
};
pub use scene::{Scene, SceneInfo};

pub const CANVAS_WIDTH: i32 = 640;
pub const CANVAS_HEIGHT: i32 = 360;
