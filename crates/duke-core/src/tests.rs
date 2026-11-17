use crate::preview::standard_shots;
use crate::*;

const RED: Colour = rgb(255, 0, 0);
const BLUE: Colour = rgb(0, 0, 255);

fn painted_pixels(canvas: &Canvas, colour: Colour) -> usize {
    canvas.pixels.iter().filter(|&&pixel| pixel == colour).count()
}

#[test]
fn a_canvas_with_a_negative_size_is_empty_instead_of_huge() {
    let canvas = Canvas::new(-5, 10);
    assert_eq!((canvas.width, canvas.height, canvas.pixels.len()), (0, 10, 0));
    let full_size = Canvas::full_size();
    assert_eq!(full_size.pixels.len(), (CANVAS_WIDTH * CANVAS_HEIGHT) as usize);
}

#[test]
fn pixels_outside_the_canvas_are_ignored_and_read_as_black() {
    let mut canvas = Canvas::new(4, 3);
    canvas.clear(BLUE);
    canvas.set_pixel(-1, 0, RED);
    canvas.set_pixel(4, 0, RED);
    canvas.blend_pixel(0, 3, RED, 1.0);
    canvas.add_glow(i32::MIN, i32::MAX, RED, 1.0);
    assert_eq!(painted_pixels(&canvas, BLUE), 12);
    assert_eq!(canvas.pixel_at(-1, -1), 0);
    canvas.set_pixel(3, 2, RED);
    assert_eq!(canvas.pixel_at(3, 2), RED);
}

#[test]
fn horizontal_lines_clip_and_accept_their_ends_in_either_order() {
    let mut canvas = Canvas::new(10, 2);
    canvas.horizontal_line(7, -20, 1, RED);
    assert_eq!(painted_pixels(&canvas, RED), 8);
    assert_eq!(canvas.pixel_at(7, 1), RED);
    assert_eq!(canvas.pixel_at(8, 1), 0);
    canvas.horizontal_line(20, 30, 0, BLUE);
    canvas.horizontal_line(0, 9, 2, BLUE);
    assert_eq!(painted_pixels(&canvas, BLUE), 0);
}

#[test]
fn rectangles_with_extreme_sizes_clip_without_overflowing() {
    let mut canvas = Canvas::new(6, 4);
    canvas.fill_rectangle(-5, -5, i32::MAX, i32::MAX, RED);
    assert_eq!(painted_pixels(&canvas, RED), 24);
    canvas.fill_rectangle(i32::MAX - 1, i32::MAX - 1, 10, 10, BLUE);
    canvas.fill_rectangle(1, 1, 2, 2, BLUE);
    assert_eq!(painted_pixels(&canvas, BLUE), 4);
    assert_eq!(canvas.pixel_at(2, 2), BLUE);
    assert_eq!(canvas.pixel_at(3, 3), RED);
}

#[test]
fn lines_reach_both_ends_in_every_direction() {
    for (start, end) in [((0, 0), (9, 3)), ((9, 3), (0, 0)), ((2, 9), (4, 0)), ((5, 5), (5, 5))] {
        let mut canvas = Canvas::new(10, 10);
        canvas.draw_line(start.0, start.1, end.0, end.1, RED);
        assert_eq!(canvas.pixel_at(start.0, start.1), RED);
        assert_eq!(canvas.pixel_at(end.0, end.1), RED);
        let longest_side = (end.0 - start.0).abs().max((end.1 - start.1).abs()) as usize;
        assert_eq!(painted_pixels(&canvas, RED), longest_side + 1);
    }
}

#[test]
fn ellipses_are_symmetric_and_a_zero_radius_marks_the_centre() {
    let mut canvas = Canvas::new(21, 21);
    canvas.fill_ellipse(10, 10, 6, 3, RED);
    for y in 0..21 {
        for x in 0..21 {
            assert_eq!(canvas.pixel_at(x, y), canvas.pixel_at(20 - x, 20 - y));
        }
    }
    assert_eq!(canvas.pixel_at(16, 10), RED);
    assert_eq!(canvas.pixel_at(10, 14), 0);
    let mut dot = Canvas::new(5, 5);
    dot.fill_ellipse(2, 2, 0, 4, RED);
    assert_eq!(painted_pixels(&dot, RED), 1);
}

#[test]
fn polygons_fill_their_inside_and_ignore_degenerate_corner_lists() {
    let mut canvas = Canvas::new(12, 12);
    canvas.fill_polygon(&[(2, 2), (8, 2), (8, 8), (2, 8)], RED);
    assert_eq!(canvas.pixel_at(5, 5), RED);
    assert_eq!(canvas.pixel_at(5, 1), 0);
    assert_eq!(canvas.pixel_at(5, 9), 0);
    let mut untouched = Canvas::new(12, 12);
    untouched.fill_polygon(&[(0, 0), (11, 11)], RED);
    untouched.fill_polygon(&[(-50, -50), (-40, -50), (-40, -40)], RED);
    assert_eq!(painted_pixels(&untouched, RED), 0);
    let mut notched = Canvas::new(12, 12);
    notched.fill_polygon(&[(0, 0), (10, 0), (10, 10), (5, 4), (0, 10)], RED);
    assert_eq!(notched.pixel_at(5, 8), 0);
    assert_eq!(notched.pixel_at(1, 8), RED);
}

#[test]
fn drawing_a_canvas_skips_its_transparent_colour() {
    let mut sprite = Canvas::new(2, 2);
    sprite.clear(BLUE);
    sprite.set_pixel(1, 1, RED);
    let mut canvas = Canvas::new(4, 4);
    canvas.draw_canvas(&sprite, 3, 3, Some(BLUE));
    canvas.draw_canvas(&sprite, -1, -1, None);
    assert_eq!(canvas.pixel_at(0, 0), RED);
    assert_eq!(canvas.pixel_at(3, 3), 0);
    let mut keyed = Canvas::new(4, 4);
    keyed.draw_canvas(&sprite, 2, 2, Some(BLUE));
    assert_eq!(painted_pixels(&keyed, RED), 1);
    assert_eq!(keyed.pixel_at(3, 3), RED);
}

#[test]
fn colour_helpers_blend_tint_and_saturate() {
    let colour = rgb(12, 34, 56);
    assert_eq!(colour_channels(colour), (12, 34, 56));
    assert_eq!(blend_colours(RED, BLUE, 0.0), RED);
    assert_eq!(blend_colours(RED, BLUE, 1.0), BLUE);
    assert_eq!(blend_colours(RED, BLUE, 7.0), BLUE);
    assert_eq!(blend_colours(RED, BLUE, f32::NAN), RED);
    assert_eq!(blend_colours(rgb(0, 0, 0), rgb(200, 100, 0), 0.5), rgb(100, 50, 0));
    assert_eq!(add_colours(rgb(200, 10, 0), rgb(100, 100, 100), 1.0), rgb(255, 110, 100));
    assert_eq!(tint_colour(rgb(100, 100, 100), [3.0, 0.5, -1.0]), rgb(255, 50, 0));
    assert_eq!(scale_brightness(rgb(100, 50, 10), 2.0), rgb(200, 100, 20));
    assert_eq!(interpolate(2.0, 4.0, 0.25), 2.5);
}

#[test]
fn random_numbers_repeat_for_a_seed_and_stay_in_range() {
    let mut first = RandomNumberGenerator::new(7);
    let mut second = RandomNumberGenerator::new(7);
    let mut extremes = RandomNumberGenerator::new(8);
    for _ in 0..1000 {
        assert_eq!(first.next_u32(), second.next_u32());
        let fraction = first.next_fraction();
        assert!((0.0..1.0).contains(&fraction));
        second.next_fraction();
        let whole_range = extremes.integer_between(i32::MIN, i32::MAX);
        assert!((i32::MIN..=i32::MAX).contains(&whole_range));
        assert!((-3..=3).contains(&extremes.integer_between(-3, 3)));
        assert!((1.5..2.5).contains(&extremes.float_between(1.5, 2.5)));
    }
    assert_eq!(extremes.integer_between(5, -5), 5);
    assert!(!extremes.succeeds_with_probability(0.0));
    assert!(extremes.succeeds_with_probability(1.0));
    assert_eq!(*extremes.choose_from(&["only"]), "only");
}

#[test]
fn seeds_fold_both_halves_into_32_bits() {
    assert_eq!(fold_seed_to_32_bits(0x0000_0001_0000_0002), 3);
    assert_eq!(fold_seed_to_32_bits(1924), 1924);
}

#[test]
fn noise_stays_in_range_at_extreme_coordinates_and_without_octaves() {
    for (x, y) in [(0.25, 0.75), (3.0e9, -3.0e9), (f32::MAX, f32::MIN), (f32::NAN, f32::INFINITY), (-0.5, 123.4)] {
        let value = value_noise(x, y, 9);
        assert!((0.0..=1.0).contains(&value), "{value} at {x}, {y}");
        let fractal = fractal_noise(x, y, 4, 9);
        assert!((0.0..=1.0).contains(&fractal), "{fractal} at {x}, {y}");
    }
    assert_eq!(fractal_noise(1.0, 2.0, 0, 3), 0.5);
    assert_eq!(hash_position(1, 2, 3), hash_position(1, 2, 3));
    assert_ne!(hash_position(1, 2, 3), hash_position(2, 1, 3));
    assert!((0.0..1.0).contains(&hash_fraction(-7, 7, 1)));
}

#[test]
fn daylight_wraps_hours_and_treats_unusable_hours_as_noon() {
    let at = |hour: f32| Environment { hour, ..Environment::default() };
    assert_eq!(at(26.0).daylight(), at(2.0).daylight());
    assert_eq!(at(-5.0).golden_hour_strength(), at(19.0).golden_hour_strength());
    assert_eq!(at(f32::NAN).daylight(), at(12.0).daylight());
    assert_eq!(at(f32::INFINITY).golden_hour_strength(), at(12.0).golden_hour_strength());
    assert_eq!(at(12.0).daylight(), 1.0);
    assert_eq!(at(2.0).daylight(), 0.0);
    assert_eq!(at(2.0).darkness(), 1.0);
    let (sunrise, _) = Season::Summer.sunrise_and_sunset_hours();
    assert_eq!(at(sunrise + 0.4).golden_hour_strength(), 1.0);
    assert_eq!(at(12.0).golden_hour_strength(), 0.0);
    assert!(at(f32::NAN).lighting().tint.iter().all(|channel| channel.is_finite()));
}

#[test]
fn shadows_swing_from_left_to_right_and_fade_under_cloud() {
    let at = |hour: f32, weather: Weather| Environment { hour, weather, ..Environment::default() };
    let (morning_across, morning_down) = at(7.0, Weather::Clear).shadow_direction();
    let (evening_across, _) = at(19.5, Weather::Clear).shadow_direction();
    let (noon_across, noon_down) = at(13.25, Weather::Clear).shadow_direction();
    assert!(morning_across < 0.0 && evening_across > 0.0);
    assert!(noon_across.abs() < morning_across.abs() && noon_down < morning_down);
    assert!(at(13.0, Weather::Storm).shadow_strength() < at(13.0, Weather::Clear).shadow_strength());
    assert_eq!(at(13.0, Weather::Clear).overcast(), 0.0);
}

#[test]
fn lighting_leaves_neutral_frames_alone_and_lightning_brightens() {
    let mut canvas = Canvas::new(2, 1);
    canvas.set_pixel(0, 0, rgb(10, 20, 30));
    canvas.set_pixel(1, 0, rgb(200, 100, 50));
    let original = canvas.pixels.clone();
    apply_lighting(&mut canvas, &Lighting { tint: [1.0, 1.0, 1.0], desaturation: 0.0, lightning_flash: 0.0 });
    assert_eq!(canvas.pixels, original);
    apply_lighting(&mut canvas, &Lighting { tint: [1.0, 1.0, 1.0], desaturation: 1.0, lightning_flash: 0.0 });
    let (red, green, blue) = colour_channels(canvas.pixel_at(1, 0));
    assert!(red == green && green == blue);
    let mut flashed = Canvas::new(1, 1);
    apply_lighting(&mut flashed, &Lighting { tint: [1.0, 1.0, 1.0], desaturation: 0.0, lightning_flash: 1.0 });
    let (red, _, blue) = colour_channels(flashed.pixel_at(0, 0));
    assert!(red > 0 && blue >= red);
}

#[test]
fn the_standard_shots_have_unique_names() {
    let shots = standard_shots();
    let mut names: Vec<&str> = shots.iter().map(|(name, _)| *name).collect();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), 14);
    assert_eq!(shots.len(), 14);
}
