pub type Colour = u32;

const BLEND_STEPS: u32 = 256;
const BLEND_SHIFT: u32 = 8;
const BRIGHTEST_CHANNEL: f32 = 255.0;

#[inline]
pub const fn rgb(red: u8, green: u8, blue: u8) -> Colour {
    ((red as u32) << 16) | ((green as u32) << 8) | blue as u32
}

#[inline]
pub const fn colour_channels(colour: Colour) -> (u8, u8, u8) {
    ((colour >> 16) as u8, (colour >> 8) as u8, colour as u8)
}

#[inline]
pub fn interpolate(start: f32, end: f32, fraction: f32) -> f32 {
    start + (end - start) * fraction
}

#[inline]
pub fn blend_colours(from: Colour, to: Colour, fraction: f32) -> Colour {
    let weight_of_to = (fraction.clamp(0.0, 1.0) * BLEND_STEPS as f32) as u32;
    let weight_of_from = BLEND_STEPS - weight_of_to;
    let (from_red, from_green, from_blue) = colour_channels(from);
    let (to_red, to_green, to_blue) = colour_channels(to);
    let blend_channel = |from_channel: u8, to_channel: u8| {
        ((from_channel as u32 * weight_of_from + to_channel as u32 * weight_of_to) >> BLEND_SHIFT) as u8
    };
    rgb(blend_channel(from_red, to_red), blend_channel(from_green, to_green), blend_channel(from_blue, to_blue))
}

#[inline]
pub fn scale_brightness(colour: Colour, factor: f32) -> Colour {
    tint_colour(colour, [factor, factor, factor])
}

#[inline]
pub fn tint_colour(colour: Colour, tint: [f32; 3]) -> Colour {
    let (red, green, blue) = colour_channels(colour);
    let tint_channel = |channel: u8, multiplier: f32| (channel as f32 * multiplier).clamp(0.0, BRIGHTEST_CHANNEL) as u8;
    rgb(tint_channel(red, tint[0]), tint_channel(green, tint[1]), tint_channel(blue, tint[2]))
}

#[inline]
pub fn add_colours(base: Colour, light: Colour, amount: f32) -> Colour {
    let (base_red, base_green, base_blue) = colour_channels(base);
    let (light_red, light_green, light_blue) = colour_channels(light);
    let add_channel = |base_channel: u8, light_channel: u8| {
        (base_channel as f32 + light_channel as f32 * amount).clamp(0.0, BRIGHTEST_CHANNEL) as u8
    };
    rgb(add_channel(base_red, light_red), add_channel(base_green, light_green), add_channel(base_blue, light_blue))
}
