use crate::atmosphere::drawing::{
    add_to_pixel, blend_onto_canvas, blend_pixels, blend_weight, glow_onto_canvas, hash_to_unit, EffectState,
    COARSE_DITHER,
};
use crate::atmosphere::simulation::LightningStrike;
use crate::{rgb, Canvas, Colour, RandomNumberGenerator, SceneInfo};

#[derive(Copy, Clone, PartialEq)]
enum BoltPart {
    MainChannel,
    Branch,
    Twig,
}

pub(crate) struct BoltSegment {
    start_x: f32,
    start_y: f32,
    end_x: f32,
    end_y: f32,
    half_width: f32,
    part: BoltPart,
}

const BOLT_SEED_MIX: u64 = 0xB017;
const DIMMEST_VISIBLE_BOLT: f32 = 0.02;
const AFTERIMAGE_SECONDS: f32 = 0.55;
const BRANCH_LINGER_SECONDS: f32 = 0.02;
const BOLT_STARTS_ABOVE_CANVAS_PIXELS: f32 = -4.0;
const DISTANCE_AT_WHICH_BOLT_CONNECTS: f32 = 8.0;
const HEIGHT_ABOVE_TARGET_AT_WHICH_BOLT_CONNECTS: f32 = 2.0;
const STEEPEST_TURN_RADIANS: f32 = 1.45;
const MOST_SEGMENTS_IN_MAIN_CHANNEL: usize = 600;
const HALF_WIDTH_AT_TOP: f32 = 1.7;
const HALF_WIDTH_LOST_BY_BOTTOM: f32 = 1.1;
const HALF_WIDTH_FOR_THREE_PIXELS: f32 = 1.35;
const HALF_WIDTH_FOR_TWO_PIXELS: f32 = 0.95;
const LONGEST_BRANCH_PIXELS: f32 = 110.0;
const SHORTEST_BRANCH_REMAINDER_PIXELS: f32 = 2.0;
const BRANCH_LENGTH_NEEDED_TO_SPROUT_TWIGS: f32 = 12.0;
const SAMPLES_PER_PIXEL_ALONG_SEGMENT: f32 = 1.5;
const MASK_MARGIN_PIXELS: i32 = 5;
const OUTER_HALO: u8 = 1;
const INNER_HALO: u8 = 2;
const CHANNEL_EDGE: u8 = 3;
const CHANNEL_CORE: u8 = 4;
const NEAREST_NEIGHBOURS: [(i32, i32); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];
const SECOND_NEIGHBOURS: [(i32, i32); 8] = [(-2, 0), (2, 0), (0, -2), (0, 2), (-1, -1), (1, -1), (-1, 1), (1, 1)];
const CORE_COLOUR: Colour = rgb(255, 255, 255);
const EDGE_COLOUR: Colour = rgb(214, 226, 255);
const INNER_HALO_COLOUR: Colour = rgb(120, 140, 255);
const OUTER_HALO_COLOUR: Colour = rgb(84, 92, 220);
const AFTERIMAGE_COLOUR: Colour = rgb(170, 140, 255);
const IMPACT_INNER_GLOW_COLOUR: Colour = rgb(200, 214, 255);
const IMPACT_MIDDLE_GLOW_COLOUR: Colour = rgb(150, 165, 255);
const IMPACT_OUTER_GLOW_COLOUR: Colour = rgb(110, 120, 240);
const IMPACT_GLOW_WIDTH_PIXELS: f32 = 20.0;
const IMPACT_GLOW_HEIGHT_PIXELS: f32 = 9.0;
const SPARK_SECONDS: f32 = 0.45;
const SPARK_COUNT: u32 = 9;
const WARM_SPARK_COLOUR: Colour = rgb(255, 236, 170);
const SPARK_FALL_ACCELERATION: f32 = 70.0;

const BRANCH_SEGMENT_PIXELS_RANGE: (f32, f32) = (3.0, 10.0);
const BRANCH_WANDER_RADIANS: f32 = 0.75;
const CHANCE_OF_A_TWIG: f32 = 0.22;
const TWIG_SHARE_OF_BRANCH_RANGE: (f32, f32) = (0.25, 0.5);
const TWIG_TURN_RADIANS_RANGE: (f32, f32) = (0.5, 1.0);
const SHARE_OF_HEADING_KEPT: f32 = 0.8;
const SHARE_OF_HEADING_TAKEN_FROM_LAST_SEGMENT: f32 = 0.2;
const SMALLEST_LEAN_PIXELS: f32 = 15.0;
const LARGEST_LEAN_PIXELS_FOR_A_TARGET_AT_THE_TOP: f32 = 35.0;
const LEAN_PIXELS_PER_PIXEL_OF_TARGET_DEPTH: f32 = 0.5;
const TARGET_DEPTH_BELOW_WHICH_NO_EXTRA_LEAN: f32 = 140.0;
const EXTRA_LEAN_SHARE_RANGE: (f32, f32) = (0.3, 1.0);
const FLATTEST_LEAN_PER_PIXEL_OF_DROP: f32 = 2.2;
const EDGE_MARGIN_FOR_BOLT_START_PIXELS: f32 = 4.0;
const CHANCE_OF_A_SHARP_KINK: f32 = 0.3;
const SHARP_KINK_RADIANS: f32 = 1.15;
const GENTLE_KINK_RADIANS: f32 = 0.5;
const CHANCE_OF_A_LONG_RUN: f32 = 0.25;
const LONG_RUN_PIXELS_RANGE: (f32, f32) = (10.0, 22.0);
const SHORT_RUN_PIXELS_RANGE: (f32, f32) = (4.0, 11.0);
const LARGEST_STEP_SHARE_OF_REMAINING_DISTANCE: f32 = 0.7;
const SHALLOWEST_BRANCH_POINT_PIXELS: f32 = 2.0;
const DISTANCE_LEFT_NEEDED_TO_BRANCH_PIXELS: f32 = 30.0;
const CHANCE_OF_A_BRANCH: f32 = 0.27;
const BRANCH_SHARE_OF_REMAINING_DISTANCE_RANGE: (f32, f32) = (0.25, 0.6);
const BRANCH_TURN_RADIANS_RANGE: (f32, f32) = (0.45, 1.05);
const EDGE_SHARE_OF_CORE_BRIGHTNESS: f32 = 0.85;
const INNER_HALO_STRENGTH: f32 = 0.62;
const OUTER_HALO_STRENGTH: f32 = 0.30;
const AFTERIMAGE_STRENGTH: f32 = 0.34;
const AFTERIMAGE_COVERAGE: f32 = 1.2;
const IMPACT_GLOW_SIZE_WHEN_DIM: f32 = 0.7;
const IMPACT_GLOW_SIZE_GAINED_WHEN_BRIGHT: f32 = 0.3;
const IMPACT_CORE_RADIUS: f32 = 0.2;
const IMPACT_INNER_GLOW_RADIUS: f32 = 0.45;
const IMPACT_MIDDLE_GLOW_RADIUS: f32 = 0.75;
const IMPACT_MIDDLE_GLOW_COVERAGE: f32 = 0.75;
const IMPACT_OUTER_GLOW_COVERAGE: f32 = 0.4;
const IMPACT_INNER_GLOW_STRENGTH: f32 = 0.7;
const IMPACT_MIDDLE_GLOW_STRENGTH: f32 = 0.38;
const IMPACT_OUTER_GLOW_STRENGTH: f32 = 0.26;
const SLOWEST_SPARK_PIXELS_PER_SECOND: f32 = 26.0;
const SPARK_SPEED_RANGE_PIXELS_PER_SECOND: f32 = 60.0;
const SPARK_RISE_SHARE_OF_SPEED: f32 = 0.6;

fn grow_branch(
    segments: &mut Vec<BoltSegment>,
    random: &mut RandomNumberGenerator,
    mut tip_x: f32,
    mut tip_y: f32,
    mut heading: f32,
    length_budget: f32,
    part: BoltPart,
) {
    let mut length_left = length_budget;
    while length_left > SHORTEST_BRANCH_REMAINDER_PIXELS {
        let segment_length =
            random.float_between(BRANCH_SEGMENT_PIXELS_RANGE.0, BRANCH_SEGMENT_PIXELS_RANGE.1).min(length_left);
        let angle = (heading + random.float_between(-BRANCH_WANDER_RADIANS, BRANCH_WANDER_RADIANS))
            .clamp(-STEEPEST_TURN_RADIANS, STEEPEST_TURN_RADIANS);
        let end_x = tip_x + segment_length * angle.sin();
        let end_y = tip_y + segment_length * angle.cos();
        segments.push(BoltSegment { start_x: tip_x, start_y: tip_y, end_x, end_y, half_width: 0.0, part });
        let may_sprout_a_twig = part == BoltPart::Branch && length_left > BRANCH_LENGTH_NEEDED_TO_SPROUT_TWIGS;
        if may_sprout_a_twig && random.succeeds_with_probability(CHANCE_OF_A_TWIG) {
            let twig_side = if random.succeeds_with_probability(0.5) { 1.0 } else { -1.0 };
            let twig_length =
                length_left * random.float_between(TWIG_SHARE_OF_BRANCH_RANGE.0, TWIG_SHARE_OF_BRANCH_RANGE.1);
            let twig_turn = random.float_between(TWIG_TURN_RADIANS_RANGE.0, TWIG_TURN_RADIANS_RANGE.1);
            grow_branch(segments, random, end_x, end_y, angle + twig_side * twig_turn, twig_length, BoltPart::Twig);
        }
        tip_x = end_x;
        tip_y = end_y;
        length_left -= segment_length;
        heading = heading * SHARE_OF_HEADING_KEPT + angle * SHARE_OF_HEADING_TAKEN_FROM_LAST_SEGMENT;
    }
}

fn lay_out_bolt(
    strike: &LightningStrike,
    scene: &SceneInfo,
    canvas_width: i32,
    canvas_height: i32,
    segments: &mut Vec<BoltSegment>,
) -> (i32, i32) {
    segments.clear();
    let mut random = RandomNumberGenerator::new(strike.shape_seed as u64 ^ BOLT_SEED_MIX);
    let target = if scene.lightning_strike_points.is_empty() {
        let target_x = random.integer_between(canvas_width * 15 / 100, canvas_width * 85 / 100);
        let target_y = random.integer_between(canvas_height * 30 / 100, canvas_height * 80 / 100);
        (target_x, target_y)
    } else {
        let strike_point = *random.choose_from(&scene.lightning_strike_points);
        (strike_point.0.clamp(0, canvas_width - 1), strike_point.1.clamp(0, canvas_height - 1))
    };
    let target_x = target.0 as f32;
    let target_y = target.1 as f32;

    let side_of_approach = if random.succeeds_with_probability(0.5) { 1.0 } else { -1.0 };
    let ordinary_lean = random.float_between(
        SMALLEST_LEAN_PIXELS,
        LARGEST_LEAN_PIXELS_FOR_A_TARGET_AT_THE_TOP + LEAN_PIXELS_PER_PIXEL_OF_TARGET_DEPTH * target_y,
    );
    let extra_lean_for_high_targets = (TARGET_DEPTH_BELOW_WHICH_NO_EXTRA_LEAN - target_y).max(0.0)
        * random.float_between(EXTRA_LEAN_SHARE_RANGE.0, EXTRA_LEAN_SHARE_RANGE.1);
    let flattest_allowed_lean = FLATTEST_LEAN_PER_PIXEL_OF_DROP * (target_y - BOLT_STARTS_ABOVE_CANVAS_PIXELS);
    let lean = (ordinary_lean + extra_lean_for_high_targets).min(flattest_allowed_lean);
    let mut start_x = target_x + side_of_approach * lean;
    if start_x < EDGE_MARGIN_FOR_BOLT_START_PIXELS || start_x > canvas_width as f32 - EDGE_MARGIN_FOR_BOLT_START_PIXELS
    {
        start_x = target_x - side_of_approach * lean;
    }
    let start_x = start_x.clamp(0.0, (canvas_width as f32 - 1.0).max(0.0));

    let mut tip_x = start_x;
    let mut tip_y = BOLT_STARTS_ABOVE_CANVAS_PIXELS;
    let whole_distance = ((target_x - tip_x).hypot(target_y - tip_y)).max(1.0);
    loop {
        let across_to_target = target_x - tip_x;
        let down_to_target = target_y - tip_y;
        let distance_to_target = across_to_target.hypot(down_to_target);
        let progress = (1.0 - distance_to_target / whole_distance).clamp(0.0, 1.0);
        let half_width = HALF_WIDTH_AT_TOP - HALF_WIDTH_LOST_BY_BOTTOM * progress;
        let close_enough_to_connect = distance_to_target < DISTANCE_AT_WHICH_BOLT_CONNECTS
            || down_to_target < HEIGHT_ABOVE_TARGET_AT_WHICH_BOLT_CONNECTS;
        if close_enough_to_connect {
            segments.push(BoltSegment {
                start_x: tip_x,
                start_y: tip_y,
                end_x: target_x,
                end_y: target_y,
                half_width,
                part: BoltPart::MainChannel,
            });
            break;
        }

        let angle_to_target = across_to_target.atan2(down_to_target);
        let kink_roll = random.float_between(-1.0f32, 1.0);
        let kink = if random.succeeds_with_probability(CHANCE_OF_A_SHARP_KINK) {
            kink_roll * SHARP_KINK_RADIANS
        } else {
            kink_roll * GENTLE_KINK_RADIANS
        };
        let angle = (angle_to_target + kink).clamp(-STEEPEST_TURN_RADIANS, STEEPEST_TURN_RADIANS);
        let rolled_length = if random.succeeds_with_probability(CHANCE_OF_A_LONG_RUN) {
            random.float_between(LONG_RUN_PIXELS_RANGE.0, LONG_RUN_PIXELS_RANGE.1)
        } else {
            random.float_between(SHORT_RUN_PIXELS_RANGE.0, SHORT_RUN_PIXELS_RANGE.1)
        };
        let segment_length = rolled_length.min(distance_to_target * LARGEST_STEP_SHARE_OF_REMAINING_DISTANCE);
        let end_x = tip_x + segment_length * angle.sin();
        let end_y = tip_y + segment_length * angle.cos();
        segments.push(BoltSegment {
            start_x: tip_x,
            start_y: tip_y,
            end_x,
            end_y,
            half_width,
            part: BoltPart::MainChannel,
        });

        let room_for_a_branch =
            tip_y > SHALLOWEST_BRANCH_POINT_PIXELS && distance_to_target > DISTANCE_LEFT_NEEDED_TO_BRANCH_PIXELS;
        if room_for_a_branch && random.succeeds_with_probability(CHANCE_OF_A_BRANCH) {
            let branch_side = if random.succeeds_with_probability(0.5) { 1.0 } else { -1.0 };
            let branch_length = (distance_to_target
                * random.float_between(
                    BRANCH_SHARE_OF_REMAINING_DISTANCE_RANGE.0,
                    BRANCH_SHARE_OF_REMAINING_DISTANCE_RANGE.1,
                ))
            .min(LONGEST_BRANCH_PIXELS);
            let branch_turn = random.float_between(BRANCH_TURN_RADIANS_RANGE.0, BRANCH_TURN_RADIANS_RANGE.1);
            grow_branch(
                segments,
                &mut random,
                end_x,
                end_y,
                angle + branch_side * branch_turn,
                branch_length,
                BoltPart::Branch,
            );
        }
        tip_x = end_x;
        tip_y = end_y;
        if segments.len() > MOST_SEGMENTS_IN_MAIN_CHANNEL {
            segments.push(BoltSegment {
                start_x: tip_x,
                start_y: tip_y,
                end_x: target_x,
                end_y: target_y,
                half_width,
                part: BoltPart::MainChannel,
            });
            break;
        }
    }
    target
}

#[inline]
fn raise_mask(mask: &mut [u8], canvas_width: i32, canvas_height: i32, pixel_x: i32, pixel_y: i32, level: u8) {
    let is_on_canvas = pixel_x >= 0 && pixel_y >= 0 && pixel_x < canvas_width && pixel_y < canvas_height;
    if !is_on_canvas {
        return;
    }
    let mask_index = (pixel_y * canvas_width + pixel_x) as usize;
    if mask[mask_index] < level {
        mask[mask_index] = level;
    }
}

fn stamp_bolt_into_mask(
    segments: &[BoltSegment],
    mask: &mut [u8],
    canvas_width: i32,
    canvas_height: i32,
    branches_are_lit: bool,
) -> (i32, i32, i32, i32) {
    let mut left = canvas_width;
    let mut top = canvas_height;
    let mut right = 0;
    let mut bottom = 0;
    for segment in segments {
        if segment.part != BoltPart::MainChannel && !branches_are_lit {
            continue;
        }
        let across = segment.end_x - segment.start_x;
        let down = segment.end_y - segment.start_y;
        let sample_count = (across.abs().max(down.abs()) * SAMPLES_PER_PIXEL_ALONG_SEGMENT).ceil().max(1.0) as i32;
        for sample_index in 0..=sample_count {
            let along_segment = sample_index as f32 / sample_count as f32;
            let sample_x = (segment.start_x + across * along_segment).round() as i32;
            let sample_y = (segment.start_y + down * along_segment).round() as i32;
            left = left.min(sample_x - MASK_MARGIN_PIXELS);
            top = top.min(sample_y - MASK_MARGIN_PIXELS);
            right = right.max(sample_x + MASK_MARGIN_PIXELS);
            bottom = bottom.max(sample_y + MASK_MARGIN_PIXELS);

            if segment.part == BoltPart::Branch {
                raise_mask(mask, canvas_width, canvas_height, sample_x, sample_y, CHANNEL_EDGE);
                for (offset_x, offset_y) in NEAREST_NEIGHBOURS {
                    raise_mask(mask, canvas_width, canvas_height, sample_x + offset_x, sample_y + offset_y, INNER_HALO);
                }
                for (offset_x, offset_y) in SECOND_NEIGHBOURS {
                    raise_mask(mask, canvas_width, canvas_height, sample_x + offset_x, sample_y + offset_y, OUTER_HALO);
                }
                continue;
            }
            if segment.part == BoltPart::Twig {
                raise_mask(mask, canvas_width, canvas_height, sample_x, sample_y, INNER_HALO);
                for (offset_x, offset_y) in NEAREST_NEIGHBOURS {
                    raise_mask(mask, canvas_width, canvas_height, sample_x + offset_x, sample_y + offset_y, OUTER_HALO);
                }
                continue;
            }

            let (core_left, core_right) = if segment.half_width > HALF_WIDTH_FOR_THREE_PIXELS {
                (-1, 1)
            } else if segment.half_width > HALF_WIDTH_FOR_TWO_PIXELS {
                (0, 1)
            } else {
                (0, 0)
            };
            for offset_x in core_left - 4..=core_right + 4 {
                for offset_y in -3..=3i32 {
                    let pixels_beside_core = if offset_x < core_left {
                        core_left - offset_x
                    } else if offset_x > core_right {
                        offset_x - core_right
                    } else {
                        0
                    };
                    let farthest_axis_distance = pixels_beside_core.max(offset_y.abs());
                    let stepped_distance = pixels_beside_core + offset_y.abs();
                    let level = if farthest_axis_distance == 0 {
                        CHANNEL_CORE
                    } else if farthest_axis_distance == 1 && stepped_distance == 1 {
                        CHANNEL_EDGE
                    } else if stepped_distance <= 2 {
                        INNER_HALO
                    } else if stepped_distance <= 5 {
                        OUTER_HALO
                    } else {
                        continue;
                    };
                    raise_mask(mask, canvas_width, canvas_height, sample_x + offset_x, sample_y + offset_y, level);
                }
            }
        }
    }
    (left.max(0), top.max(0), right.min(canvas_width - 1), bottom.min(canvas_height - 1))
}

pub(crate) fn draw_lightning_bolt(
    effect_state: &mut EffectState,
    strike: &LightningStrike,
    strike_age: f32,
    bolt_brightness: f32,
    scene: &SceneInfo,
    canvas: &mut Canvas,
) {
    if !strike.has_bolt || strike_age < 0.0 {
        return;
    }
    let canvas_width = canvas.width;
    let canvas_height = canvas.height;
    let seconds_since_last_stroke = strike_age - strike.last_return_stroke_age;
    let afterimage_strength = if seconds_since_last_stroke > 0.0 {
        (1.0 - seconds_since_last_stroke / AFTERIMAGE_SECONDS).clamp(0.0, 1.0)
    } else {
        0.0
    };
    if bolt_brightness < DIMMEST_VISIBLE_BOLT && afterimage_strength <= 0.0 {
        return;
    }

    let mask_length = (canvas_width * canvas_height) as usize;
    if effect_state.bolt_mask.len() != mask_length {
        effect_state.bolt_mask.clear();
        effect_state.bolt_mask.resize(mask_length, 0);
    }
    let target = lay_out_bolt(strike, scene, canvas_width, canvas_height, &mut effect_state.bolt_segments);
    let branches_are_lit = strike_age < strike.branches_visible_until_age + BRANCH_LINGER_SECONDS;
    let (left, top, right, bottom) = stamp_bolt_into_mask(
        &effect_state.bolt_segments,
        &mut effect_state.bolt_mask,
        canvas_width,
        canvas_height,
        branches_are_lit,
    );

    let brightness = bolt_brightness.clamp(0.0, 1.0);
    let bolt_is_lit = brightness >= DIMMEST_VISIBLE_BOLT;
    let core_weight = blend_weight(brightness.sqrt());
    let edge_weight = blend_weight(brightness.sqrt() * EDGE_SHARE_OF_CORE_BRIGHTNESS);
    let inner_halo_weight = blend_weight(INNER_HALO_STRENGTH * brightness);
    let outer_halo_weight = blend_weight(OUTER_HALO_STRENGTH * brightness);
    let afterimage_weight = blend_weight(AFTERIMAGE_STRENGTH * afterimage_strength);
    for pixel_y in top..=bottom {
        for pixel_x in left..=right {
            let pixel_index = (pixel_y * canvas_width + pixel_x) as usize;
            let mask_level = effect_state.bolt_mask[pixel_index];
            if mask_level == 0 {
                continue;
            }
            effect_state.bolt_mask[pixel_index] = 0;
            let pixel = &mut canvas.pixels[pixel_index];
            if bolt_is_lit {
                if mask_level == CHANNEL_CORE {
                    *pixel = blend_pixels(*pixel, CORE_COLOUR, core_weight);
                } else if mask_level == CHANNEL_EDGE {
                    *pixel = blend_pixels(*pixel, EDGE_COLOUR, edge_weight);
                } else if mask_level == INNER_HALO {
                    *pixel = add_to_pixel(*pixel, INNER_HALO_COLOUR, inner_halo_weight);
                } else if (pixel_x + pixel_y) & 1 == 0 {
                    *pixel = add_to_pixel(*pixel, OUTER_HALO_COLOUR, outer_halo_weight);
                }
            }
            let is_part_of_afterimage = afterimage_strength > 0.0
                && mask_level >= CHANNEL_EDGE
                && COARSE_DITHER.threshold(pixel_x, pixel_y) < afterimage_strength * AFTERIMAGE_COVERAGE;
            if is_part_of_afterimage {
                *pixel = add_to_pixel(*pixel, AFTERIMAGE_COLOUR, afterimage_weight);
            }
        }
    }

    let (target_x, target_y) = target;
    if bolt_is_lit {
        let glow_size = IMPACT_GLOW_SIZE_WHEN_DIM + IMPACT_GLOW_SIZE_GAINED_WHEN_BRIGHT * brightness;
        let glow_half_width = IMPACT_GLOW_WIDTH_PIXELS * glow_size;
        let glow_half_height = IMPACT_GLOW_HEIGHT_PIXELS * glow_size;
        let widest_offset = glow_half_width as i32 + 1;
        let tallest_offset = glow_half_height as i32 + 1;
        for offset_y in -tallest_offset..=tallest_offset {
            for offset_x in -widest_offset..=widest_offset {
                let across = offset_x as f32 / glow_half_width;
                let down = offset_y as f32 / glow_half_height;
                let distance_from_centre = (across.powi(2) + down.powi(2)).sqrt();
                let pixel_x = target_x + offset_x;
                let pixel_y = target_y + offset_y;
                if distance_from_centre < IMPACT_CORE_RADIUS {
                    blend_onto_canvas(canvas, pixel_x, pixel_y, CORE_COLOUR, blend_weight(brightness));
                } else if distance_from_centre < IMPACT_INNER_GLOW_RADIUS {
                    glow_onto_canvas(
                        canvas,
                        pixel_x,
                        pixel_y,
                        IMPACT_INNER_GLOW_COLOUR,
                        blend_weight(IMPACT_INNER_GLOW_STRENGTH * brightness),
                    );
                } else if distance_from_centre < IMPACT_MIDDLE_GLOW_RADIUS {
                    if COARSE_DITHER.threshold(pixel_x, pixel_y) < IMPACT_MIDDLE_GLOW_COVERAGE {
                        glow_onto_canvas(
                            canvas,
                            pixel_x,
                            pixel_y,
                            IMPACT_MIDDLE_GLOW_COLOUR,
                            blend_weight(IMPACT_MIDDLE_GLOW_STRENGTH * brightness),
                        );
                    }
                } else if distance_from_centre < 1.0
                    && COARSE_DITHER.threshold(pixel_x, pixel_y) < IMPACT_OUTER_GLOW_COVERAGE
                {
                    glow_onto_canvas(
                        canvas,
                        pixel_x,
                        pixel_y,
                        IMPACT_OUTER_GLOW_COLOUR,
                        blend_weight(IMPACT_OUTER_GLOW_STRENGTH * brightness),
                    );
                }
            }
        }
    }

    if strike_age < SPARK_SECONDS {
        let spark_weight = blend_weight(1.0 - strike_age / SPARK_SECONDS);
        for spark_index in 0..SPARK_COUNT {
            let direction = hash_to_unit(spark_index, 1, strike.shape_seed) * std::f32::consts::TAU;
            let speed = SLOWEST_SPARK_PIXELS_PER_SECOND
                + SPARK_SPEED_RANGE_PIXELS_PER_SECOND * hash_to_unit(spark_index, 2, strike.shape_seed);
            let spark_x = target_x as f32 + direction.cos() * speed * strike_age;
            let spark_y = target_y as f32 - (direction.sin().abs() * speed * SPARK_RISE_SHARE_OF_SPEED) * strike_age
                + SPARK_FALL_ACCELERATION * strike_age * strike_age;
            let spark_colour = if spark_index % 3 == 0 { WARM_SPARK_COLOUR } else { CORE_COLOUR };
            blend_onto_canvas(canvas, spark_x.floor() as i32, spark_y.floor() as i32, spark_colour, spark_weight);
        }
    }
}
