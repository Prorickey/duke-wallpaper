pub mod animation;
pub mod building_materials;
pub mod buildings;
pub mod bus;
pub mod campus_layout;
pub mod conditions;
pub mod cycle_memory;
pub mod figures;
pub mod geometry;
pub mod ground;
pub mod lawn_games;
pub mod lights;
pub mod paths;
pub mod projection;
pub mod props;
pub mod shading_tools;
pub mod static_layer;
pub mod street_furniture;
pub mod trees;
pub mod trinity_park_houses;
pub mod vehicles;
pub mod walkers;

#[cfg(test)]
mod tests;

use crate::{fold_seed_to_32_bits, Canvas, Environment, Scene, SceneInfo};
use buildings::{DOME_RISE, DOME_SPRING_HEIGHT, HALL_CHIMNEY_FROM_AXIS, HALL_CHIMNEY_HEIGHT, LIBRARY_ATTIC_HEIGHT};
use campus_layout::{BENJAMIN_DUKE_STATUE_SOUTH, EAST_CAMPUS_UNION, LILLY_LIBRARY, QUAD_HALLS};
use geometry::{CANVAS_HEIGHT, CANVAS_WIDTH};
use static_layer::{SeedGeometry, StaticLayer};
use street_furniture::LAMP_POST_HEIGHT;

pub struct EastCampusScene {
    seed_geometry: Option<SeedGeometry>,
    static_layer: Option<StaticLayer>,
    memories: animation::Memories,
    strike_points: Vec<(i32, i32)>,
    standard_canvas: Canvas,
    times_static_layer_was_built: u32,
}

const LONGEST_ANIMATION_CLOCK_SECONDS: f64 = 4_294_967_296.0;

fn with_animation_clock_in_range(environment: &Environment) -> Environment {
    let mut environment = environment.clone();
    if !environment.animation_seconds.is_finite() {
        environment.animation_seconds = 0.0;
    } else if environment.animation_seconds.abs() > LONGEST_ANIMATION_CLOCK_SECONDS {
        environment.animation_seconds = environment.animation_seconds.rem_euclid(LONGEST_ANIMATION_CLOCK_SECONDS);
    }
    environment
}

fn resample(from: &Canvas, to: &mut Canvas) {
    for y in 0..to.height {
        let source_y = y * from.height / to.height;
        for x in 0..to.width {
            let source_x = x * from.width / to.width;
            to.pixels[(y * to.width + x) as usize] = from.pixels[(source_y * from.width + source_x) as usize];
        }
    }
}

fn is_standard_size(canvas: &Canvas) -> bool {
    canvas.width as usize == CANVAS_WIDTH && canvas.height as usize == CANVAS_HEIGHT
}

impl EastCampusScene {
    pub fn new() -> Self {
        EastCampusScene {
            seed_geometry: None,
            static_layer: None,
            memories: animation::Memories::default(),
            strike_points: lightning_strike_points(),
            standard_canvas: Canvas::full_size(),
            times_static_layer_was_built: 0,
        }
    }

    fn build_what_this_environment_needs(&mut self, environment: &Environment) {
        let seed = fold_seed_to_32_bits(environment.seed);
        if self.seed_geometry.as_ref().is_none_or(|geometry| geometry.seed != seed) {
            self.seed_geometry = Some(SeedGeometry::build(seed));
            self.static_layer = None;
        }
        let key = conditions::CacheKey::of(environment);
        if self.static_layer.as_ref().is_none_or(|layer| layer.key != key) {
            self.static_layer = Some(StaticLayer::build(self.seed_geometry.as_ref().unwrap(), key));
            self.times_static_layer_was_built += 1;
        }
    }
}

impl Default for EastCampusScene {
    fn default() -> Self {
        Self::new()
    }
}

fn lightning_strike_points() -> Vec<(i32, i32)> {
    let mut points: Vec<(i32, i32)> = Vec::new();
    let mut add_if_a_bolt_fits = |metres_east: f32, metres_south: f32, metres_up: f32| {
        let (x, y) = projection::project_to_pixel(metres_east, metres_south, metres_up);
        if (4..636).contains(&x) && (24..352).contains(&y) {
            points.push((x, y));
        }
    };
    add_if_a_bolt_fits(0.0, 0.0, DOME_SPRING_HEIGHT + DOME_RISE + 0.3);
    for hall in QUAD_HALLS.iter() {
        let middle_chimney_south = hall.north_end + (hall.south_end - hall.north_end) * 0.5;
        add_if_a_bolt_fits(hall.side_of_quad * HALL_CHIMNEY_FROM_AXIS, middle_chimney_south, HALL_CHIMNEY_HEIGHT);
    }
    add_if_a_bolt_fits(
        -(LILLY_LIBRARY.facade_from_axis + 1.7),
        0.5 * (LILLY_LIBRARY.north_end + LILLY_LIBRARY.south_end),
        LIBRARY_ATTIC_HEIGHT,
    );
    add_if_a_bolt_fits(
        EAST_CAMPUS_UNION.facade_from_axis + 1.7,
        0.5 * (EAST_CAMPUS_UNION.north_end + EAST_CAMPUS_UNION.south_end),
        LIBRARY_ATTIC_HEIGHT,
    );
    let mut tallest_first = trees::all_trees();
    tallest_first.sort_by(|first, second| second.height.total_cmp(&first.height));
    for tree in tallest_first.iter().filter(|tree| tree.height >= 16.0 && tree.metres_south > -70.0).take(12) {
        add_if_a_bolt_fits(tree.metres_east, tree.metres_south, tree.height);
    }
    let tall_trees_near_the_south_lawn = tallest_first.iter().filter(|tree| {
        tree.metres_south > 200.0 && tree.height >= 9.5 && tree.height < 16.0 && tree.metres_east.abs() < 25.0
    });
    for tree in tall_trees_near_the_south_lawn {
        add_if_a_bolt_fits(tree.metres_east, tree.metres_south, tree.height);
    }
    add_if_a_bolt_fits(-27.4, 234.0, LAMP_POST_HEIGHT);
    add_if_a_bolt_fits(27.4, 278.0, LAMP_POST_HEIGHT);
    add_if_a_bolt_fits(-11.5, 319.5, LAMP_POST_HEIGHT);
    add_if_a_bolt_fits(0.0, BENJAMIN_DUKE_STATUE_SOUTH, 4.0);
    points
}

impl Scene for EastCampusScene {
    fn info(&self) -> SceneInfo {
        SceneInfo { id: "east", title: "East Campus", lightning_strike_points: self.strike_points.clone() }
    }

    fn render(&mut self, environment: &Environment, canvas: &mut Canvas) {
        let environment = &with_animation_clock_in_range(environment);
        self.build_what_this_environment_needs(environment);
        let static_layer = self.static_layer.as_ref().unwrap();
        if canvas.width <= 0 || canvas.height <= 0 {
            return;
        }
        if is_standard_size(canvas) {
            canvas.pixels.copy_from_slice(&static_layer.colours);
            animation::render(environment, static_layer, canvas, &mut self.memories);
            return;
        }
        self.standard_canvas.pixels.copy_from_slice(&static_layer.colours);
        animation::render(environment, static_layer, &mut self.standard_canvas, &mut self.memories);
        resample(&self.standard_canvas, canvas);
    }

    fn render_lights(&mut self, environment: &Environment, canvas: &mut Canvas) {
        let environment = &with_animation_clock_in_range(environment);
        self.build_what_this_environment_needs(environment);
        let static_layer = self.static_layer.as_ref().unwrap();
        if canvas.width <= 0 || canvas.height <= 0 {
            return;
        }
        if is_standard_size(canvas) {
            lights::render(environment, static_layer, canvas, &mut self.memories);
            return;
        }
        resample(canvas, &mut self.standard_canvas);
        lights::render(environment, static_layer, &mut self.standard_canvas, &mut self.memories);
        resample(&self.standard_canvas, canvas);
    }
}
