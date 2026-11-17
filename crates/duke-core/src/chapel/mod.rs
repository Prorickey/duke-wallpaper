mod anim;
mod arch;
mod art;
mod buildings;
mod chapel_building;
mod crowd;
mod events;
mod ground;
mod lights;
mod palette;
mod proj;
mod props;
mod shadow;
mod survey;
mod tex;
mod trees;

#[cfg(test)]
mod tests;

use crate::{fold_seed_to_32_bits, Canvas, Colour, Environment, Scene, SceneInfo, CANVAS_HEIGHT, CANVAS_WIDTH};
use art::{Layer, NOTHING_DRAWN};
use palette::{Look, Palette};
use tex::{DrawingContext, ShadowAndLightSources};

pub use proj::{PlanView, Projection};

const SEASON_PROGRESS_STEPS: f32 = 50.0;
const SNOW_COVER_STEPS: f32 = 24.0;
const WETNESS_STEPS: f32 = 8.0;
const SHADOW_SHIFT_WITH_THE_SUN_FULLY_TO_ONE_SIDE: f32 = 0.6;
const SHADOW_STEPS_PER_PIXEL: f32 = 96.0;
const SHADOW_STRETCH_ACROSS_THE_SCREEN: f32 = 2.1;
const SHADOW_STRETCH_DOWN_THE_SCREEN: f32 = 1.8;
const SHADOW_EXTRA_DROP_DOWN_THE_SCREEN: f32 = 0.2;
const GROUND_SHADOW_DARKNESS: f32 = 0.92;

#[derive(Clone, Copy, PartialEq)]
enum Downwind {
    Left,
    Right,
}

impl Downwind {
    fn of(wind: f32) -> Downwind {
        if wind < 0.0 {
            Downwind::Left
        } else {
            Downwind::Right
        }
    }

    fn sway(self) -> f32 {
        match self {
            Downwind::Left => -1.0,
            Downwind::Right => 1.0,
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
struct ShadowKey {
    shift_x_steps: i32,
    shift_y_steps: i32,
    strength_tenths: i32,
    downwind: Downwind,
}

impl ShadowKey {
    fn of(environment: &Environment) -> ShadowKey {
        let (shift_x, shift_y) = environment.shadow_direction();
        ShadowKey {
            shift_x_steps: (shift_x * SHADOW_STEPS_PER_PIXEL).round() as i32,
            shift_y_steps: (shift_y * SHADOW_STEPS_PER_PIXEL).round() as i32,
            strength_tenths: (environment.shadow_strength() * 10.0).round() as i32,
            downwind: Downwind::of(environment.wind),
        }
    }
}

enum ObjectToDraw {
    Tree(usize),
    ChapelPart(chapel_building::Part),
    Building(buildings::Building),
    Prop(props::Prop),
}

struct SwayingPixel {
    index: u32,
    tree: u16,
    leaning_colour: Colour,
}

struct TreeRhythm {
    phase: f32,
    amplitude: f32,
}

struct Cache {
    look: Look,
    palette: Palette,
    trees: Vec<trees::Tree>,
    ground: Vec<Colour>,
    resting_objects: Layer,
    objects_leaning_left: Option<Layer>,
    objects_leaning_right: Option<Layer>,
    shadow_and_light_sources: ShadowAndLightSources,
    composed_for: Option<ShadowKey>,
    composed_frame: Vec<Colour>,
    swaying_pixels: Vec<SwayingPixel>,
    lights: lights::Lights,
    world: anim::World,
    tree_rhythms: Vec<TreeRhythm>,
    tree_leans_downwind: Vec<bool>,
    sprites: Vec<anim::Sprite>,
}

pub struct ChapelScene {
    cache: Option<Box<Cache>>,
    strike_points: Vec<(i32, i32)>,
    standard_canvas: Canvas,
    times_rebuilt: u32,
    projection: Projection,
}

impl ChapelScene {
    pub fn new() -> Self {
        Self::seen_through(Projection::Perspective)
    }

    pub fn seen_straight_down(plan_view: PlanView) -> Self {
        Self::seen_through(Projection::StraightDown(plan_view))
    }

    fn seen_through(projection: Projection) -> Self {
        ChapelScene {
            cache: None,
            strike_points: strike_points(projection),
            standard_canvas: Canvas::full_size(),
            times_rebuilt: 0,
            projection,
        }
    }

    fn bring_cache_up_to_date(&mut self, environment: &Environment) {
        let look = look_of(environment);
        let is_stale = match &self.cache {
            Some(cache) => cache.look != look,
            None => true,
        };
        if is_stale {
            self.cache = Some(Box::new(Cache::build(look, environment.wind, self.projection)));
            self.times_rebuilt += 1;
        }
        let projection = self.projection;
        let cache = self.cache.as_mut().unwrap();
        let downwind = Downwind::of(environment.wind);
        let leaning_objects = match downwind {
            Downwind::Left => &mut cache.objects_leaning_left,
            Downwind::Right => &mut cache.objects_leaning_right,
        };
        if leaning_objects.is_none() {
            let (objects, _) = build_objects(&cache.look, &cache.palette, downwind.sway(), &cache.trees, projection);
            *leaning_objects = Some(objects);
            cache.composed_for = None;
        }
        let shadow_key = ShadowKey::of(environment);
        if cache.composed_for != Some(shadow_key) {
            cache.compose(shadow_key);
        }
    }

    fn draw_frame(&mut self, environment: &Environment, canvas: &mut Canvas) {
        let cache = self.cache.as_mut().unwrap();
        canvas.pixels.copy_from_slice(&cache.composed_frame);
        let wind = environment.wind;
        let wind_strength = wind.abs();
        let seconds = anim::seconds_on_wrapped_clock(environment);
        let slow_swell_angle = anim::angle_after_whole_cycles(seconds, 2750.0);
        let strong_swell_angle = anim::angle_after_whole_cycles(seconds, 5850.0);
        let gust_angle = anim::angle_after_whole_cycles(seconds, 9300.0);
        let downwind = Downwind::of(wind);
        for (rhythm, leans_downwind) in cache.tree_rhythms.iter().zip(cache.tree_leans_downwind.iter_mut()) {
            let swell = (1.0 - wind_strength) * (slow_swell_angle + rhythm.phase).sin()
                + wind_strength * (strong_swell_angle + rhythm.phase).sin();
            let lean = wind * 0.42
                + (0.26 + 0.62 * wind_strength) * swell * (0.6 + 0.4 * rhythm.amplitude)
                + 0.30 * wind_strength * (gust_angle + rhythm.phase * 1.7).sin();
            *leans_downwind = match downwind {
                Downwind::Left => lean < -0.40,
                Downwind::Right => lean > 0.40,
            };
        }
        for pixel in &cache.swaying_pixels {
            if cache.tree_leans_downwind[pixel.tree as usize - 1] {
                canvas.pixels[pixel.index as usize] = pixel.leaning_colour;
            }
        }
        anim::draw(environment, canvas, &cache.world, &cache.resting_objects, &mut cache.sprites);
    }
}

impl Default for ChapelScene {
    fn default() -> Self {
        Self::new()
    }
}

impl Cache {
    fn build(look: Look, wind: f32, projection: Projection) -> Cache {
        let palette = Palette::new(&look);
        let trees = trees::plant(look.seed);
        let mut ground = Layer::new(CANVAS_WIDTH, CANVAS_HEIGHT);
        let still_context =
            DrawingContext { projection, look: &look, palette: &palette, sway: 0.0, distant_stonework: false };
        ground::draw_ground(&mut ground, &still_context);
        let (resting_objects, shadow_and_light_sources) = build_objects(&look, &palette, 0.0, &trees, projection);
        let downwind = Downwind::of(wind);
        let (leaning_objects, _) = build_objects(&look, &palette, downwind.sway(), &trees, projection);
        let world = anim::World::new(&look, &ground.pixels, &resting_objects, projection);
        let lights = lights::Lights::build(&resting_objects, &shadow_and_light_sources, &look, projection);
        let mut tree_rhythms = Vec::with_capacity(trees.len());
        for tree in &trees {
            tree_rhythms.push(TreeRhythm {
                phase: tree.metres_east * 0.13 + tree.metres_north * 0.29,
                amplitude: (tree.height_metres / 20.0).clamp(0.25, 1.0),
            });
        }
        let (objects_leaning_left, objects_leaning_right) = match downwind {
            Downwind::Left => (Some(leaning_objects), None),
            Downwind::Right => (None, Some(leaning_objects)),
        };
        Cache {
            look,
            palette,
            tree_leans_downwind: vec![false; trees.len()],
            trees,
            ground: ground.pixels,
            resting_objects,
            objects_leaning_left,
            objects_leaning_right,
            shadow_and_light_sources,
            composed_for: None,
            composed_frame: vec![0; (CANVAS_WIDTH * CANVAS_HEIGHT) as usize],
            swaying_pixels: Vec::new(),
            lights,
            world,
            tree_rhythms,
            sprites: Vec::with_capacity(512),
        }
    }

    fn compose(&mut self, shadow_key: ShadowKey) {
        let shift_x = shadow_key.shift_x_steps as f32 / SHADOW_STEPS_PER_PIXEL * SHADOW_STRETCH_ACROSS_THE_SCREEN;
        let shift_y = shadow_key.shift_y_steps as f32 / SHADOW_STEPS_PER_PIXEL * SHADOW_STRETCH_DOWN_THE_SCREEN
            + SHADOW_EXTRA_DROP_DOWN_THE_SCREEN;
        let strength = shadow_key.strength_tenths as f32 / 10.0;
        let mask = if strength > 0.0 {
            shadow::shadow_mask(&self.shadow_and_light_sources.casters, shift_x, shift_y, CANVAS_WIDTH, CANVAS_HEIGHT)
        } else {
            vec![0u8; (CANVAS_WIDTH * CANVAS_HEIGHT) as usize]
        };
        self.swaying_pixels.clear();
        let resting = &self.resting_objects;
        let leaning = match shadow_key.downwind {
            Downwind::Left => self.objects_leaning_left.as_ref().unwrap(),
            Downwind::Right => self.objects_leaning_right.as_ref().unwrap(),
        };
        for (index, &shadow_coverage) in mask.iter().enumerate() {
            let ground_colour = if shadow_coverage > 0 {
                shadow::shaded(self.ground[index], strength * GROUND_SHADOW_DARKNESS)
            } else {
                self.ground[index]
            };
            let colour_through = |objects: &Layer| {
                if objects.depth[index] != NOTHING_DRAWN {
                    objects.pixels[index]
                } else {
                    ground_colour
                }
            };
            let (resting_colour, leaning_colour) = (colour_through(resting), colour_through(leaning));
            self.composed_frame[index] = resting_colour;
            if leaning_colour == resting_colour {
                continue;
            }
            let tree = if resting.owner[index] != 0 { resting.owner[index] } else { leaning.owner[index] };
            if tree != 0 {
                self.swaying_pixels.push(SwayingPixel { index: index as u32, tree, leaning_colour });
            }
        }
        self.composed_for = Some(shadow_key);
    }
}

fn resample(from: &Canvas, to: &mut Canvas) {
    for row in 0..to.height {
        let source_row = row * from.height / to.height;
        for column in 0..to.width {
            let source_column = column * from.width / to.width;
            to.pixels[(row * to.width + column) as usize] =
                from.pixels[(source_row * from.width + source_column) as usize];
        }
    }
}

fn quantised(value: f32, steps: f32) -> f32 {
    (value.clamp(0.0, 1.0) * steps).round() / steps
}

fn sanitised(environment: &Environment) -> Environment {
    let finite_or = |value: f32, fallback: f32| if value.is_finite() { value } else { fallback };
    let mut sane = environment.clone();
    sane.season_progress = finite_or(environment.season_progress, 0.5).clamp(0.0, 1.0);
    sane.weather_intensity = finite_or(environment.weather_intensity, 0.0).clamp(0.0, 1.0);
    sane.hour = finite_or(environment.hour, 12.0).rem_euclid(24.0);
    sane.animation_seconds =
        if environment.animation_seconds.is_finite() { environment.animation_seconds } else { 0.0 };
    sane.wind = finite_or(environment.wind, 0.0).clamp(-1.0, 1.0);
    sane.snow_cover = finite_or(environment.snow_cover, 0.0).clamp(0.0, 1.0);
    sane.wetness = finite_or(environment.wetness, 0.0).clamp(0.0, 1.0);
    sane.lightning_flash = finite_or(environment.lightning_flash, 0.0).clamp(0.0, 1.0);
    sane
}

fn look_of(environment: &Environment) -> Look {
    let (shadow_shift_x, _) = environment.shadow_direction();
    let sun_side =
        ((-shadow_shift_x / SHADOW_SHIFT_WITH_THE_SUN_FULLY_TO_ONE_SIDE).clamp(-1.0, 1.0) * 2.0).round() / 2.0;
    Look {
        season: environment.season,
        season_progress: quantised(environment.season_progress, SEASON_PROGRESS_STEPS),
        snow_cover: quantised(environment.snow_cover, SNOW_COVER_STEPS),
        wetness: quantised(environment.wetness, WETNESS_STEPS),
        sun_side,
        seed: fold_seed_to_32_bits(environment.seed),
    }
}

fn strike_points(projection: Projection) -> Vec<(i32, i32)> {
    let mut points = chapel_building::strike_points(projection);
    let look = look_of(&Environment::default());
    let palette = Palette::new(&look);
    let context = DrawingContext { projection, look: &look, palette: &palette, sway: 0.0, distant_stonework: false };
    let trees = trees::plant(look.seed);
    let mut layer = Layer::new(CANVAS_WIDTH, CANVAS_HEIGHT);
    let mut unused_sources = ShadowAndLightSources::default();
    for &(east, north) in trees::TREES_LIGHTNING_CAN_STRIKE {
        let Some(tree) = trees
            .iter()
            .find(|tree| (tree.metres_east - east).abs() < 0.05 && (tree.metres_north - north).abs() < 0.05)
        else {
            continue;
        };
        layer.depth.fill(NOTHING_DRAWN);
        layer.pen_depth = 0;
        trees::draw_tree(&mut layer, &context, &mut unused_sources, tree, 1);
        let trunk_column = projection.pixel_x(tree.metres_east, tree.metres_north);
        for row in 0..CANVAS_HEIGHT {
            let mut nearest_column: Option<i32> = None;
            for column in 0..CANVAS_WIDTH {
                let is_drawn = layer.depth[(row * CANVAS_WIDTH + column) as usize] != NOTHING_DRAWN;
                if is_drawn
                    && nearest_column
                        .is_none_or(|nearest| (column - trunk_column).abs() < (nearest - trunk_column).abs())
                {
                    nearest_column = Some(column);
                }
            }
            if let Some(column) = nearest_column {
                points.push((column, row));
                break;
            }
        }
    }
    points
}

fn build_objects(
    look: &Look,
    palette: &Palette,
    sway: f32,
    trees: &[trees::Tree],
    projection: Projection,
) -> (Layer, ShadowAndLightSources) {
    let context = DrawingContext { projection, look, palette, sway, distant_stonework: false };
    let mut layer = Layer::new(CANVAS_WIDTH, CANVAS_HEIGHT);
    let mut sources = ShadowAndLightSources::default();
    let mut objects: Vec<(f32, ObjectToDraw)> = Vec::new();
    for (index, tree) in trees.iter().enumerate() {
        objects.push((tree.metres_north, ObjectToDraw::Tree(index)));
    }
    for (depth, part) in chapel_building::parts() {
        objects.push((depth, ObjectToDraw::ChapelPart(part)));
    }
    for (depth, building) in buildings::BUILDINGS {
        objects.push((depth, ObjectToDraw::Building(building)));
    }
    for (depth, prop) in props::props() {
        objects.push((depth, ObjectToDraw::Prop(prop)));
    }
    objects.sort_by(|nearer, farther| farther.0.total_cmp(&nearer.0));
    for (depth, object) in objects {
        layer.pen_depth = proj::depth_tag(depth);
        layer.pen_owner = 0;
        layer.pen_flags = 0;
        match object {
            ObjectToDraw::Tree(index) => {
                trees::draw_tree(&mut layer, &context, &mut sources, &trees[index], (index + 1) as u16)
            }
            ObjectToDraw::ChapelPart(part) => chapel_building::draw_part(part, &mut layer, &context, &mut sources),
            ObjectToDraw::Building(building) => buildings::draw_building(building, &mut layer, &context, &mut sources),
            ObjectToDraw::Prop(prop) => props::draw_prop(prop, &mut layer, &context, &mut sources),
        }
    }
    (layer, sources)
}

impl Scene for ChapelScene {
    fn info(&self) -> SceneInfo {
        SceneInfo { id: "chapel", title: "Duke Chapel", lightning_strike_points: self.strike_points.clone() }
    }

    fn render(&mut self, environment: &Environment, canvas: &mut Canvas) {
        let environment = &sanitised(environment);
        self.bring_cache_up_to_date(environment);
        if canvas.width <= 0 || canvas.height <= 0 {
            return;
        }
        if canvas.width == CANVAS_WIDTH && canvas.height == CANVAS_HEIGHT {
            self.draw_frame(environment, canvas);
            return;
        }
        let mut standard = std::mem::replace(&mut self.standard_canvas, Canvas::new(0, 0));
        self.draw_frame(environment, &mut standard);
        resample(&standard, canvas);
        self.standard_canvas = standard;
    }

    fn render_lights(&mut self, environment: &Environment, canvas: &mut Canvas) {
        if canvas.width <= 0 || canvas.height <= 0 {
            return;
        }
        let environment = &sanitised(environment);
        self.bring_cache_up_to_date(environment);
        let cache = self.cache.as_ref().unwrap();
        if canvas.width == CANVAS_WIDTH && canvas.height == CANVAS_HEIGHT {
            lights::draw(environment, canvas, &cache.lights, &cache.world);
            return;
        }
        let mut standard = std::mem::replace(&mut self.standard_canvas, Canvas::new(0, 0));
        resample(canvas, &mut standard);
        lights::draw(environment, &mut standard, &cache.lights, &cache.world);
        resample(&standard, canvas);
        self.standard_canvas = standard;
    }
}
