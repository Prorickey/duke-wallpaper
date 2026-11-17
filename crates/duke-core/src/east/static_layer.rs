use crate::east::building_materials::shade_building;
use crate::east::buildings::all_solids;
use crate::east::campus_layout::hall_entrances_south;
use crate::east::conditions::{CacheKey, Conditions};
use crate::east::geometry::*;
use crate::east::ground;
use crate::east::projection::*;
use crate::east::shading_tools::ordered_dither_threshold;
use crate::east::street_furniture::*;
use crate::east::trees::*;
use crate::{blend_colours, rgb, tint_colour, Colour};

const PIXEL_COUNT: usize = CANVAS_WIDTH * CANVAS_HEIGHT;

#[derive(Copy, Clone, Default)]
pub struct LeafLitter {
    pub density: u8,
    pub tree_index: u16,
}

pub struct SeedGeometry {
    pub seed: u32,
    pub solids: Vec<Solid>,
    pub surface_hits: Vec<SurfaceHit>,
    pub shadow_height_map: HeightMap,
    pub trees: Vec<Tree>,
    pub crown_maps: Vec<CrownMap>,
    pub leaf_litter: Vec<LeafLitter>,
    pub ground_noise: Vec<ground::GroundNoise>,
}

impl SeedGeometry {
    pub fn build(seed: u32) -> SeedGeometry {
        let solids = all_solids();
        let mut surface_hits = bare_ground_hits();
        for hit in surface_hits.iter_mut() {
            hit.ground_surface = ground::surface_at(hit.metres_east, hit.metres_south);
        }
        for (solid_index, solid) in solids.iter().enumerate() {
            march_solid_into_hits(solid_index, solid, &mut surface_hits);
        }
        let shadow_height_map = HeightMap::of_shadow_casters(&solids);
        let trees = all_trees();
        let crown_maps = trees.iter().map(crown_map).collect();

        let mut leaf_litter = vec![LeafLitter::default(); PIXEL_COUNT];
        for (tree_index, tree) in trees.iter().enumerate() {
            if tree.is_evergreen() || tree.crown_radius < 1.5 {
                continue;
            }
            let reach = tree.crown_radius * 1.9;
            let (left, _) = project_to_screen(tree.metres_east - reach, tree.metres_south, 0.0);
            let (right, _) = project_to_screen(tree.metres_east + reach, tree.metres_south, 0.0);
            let top = ground_screen_y(tree.metres_south - reach);
            let bottom = ground_screen_y(tree.metres_south + reach);
            for y in (top.floor() as i32).max(0)..=(bottom.ceil() as i32).min(CANVAS_HEIGHT as i32 - 1) {
                for x in (left.floor() as i32 - 4).max(0)..=(right.ceil() as i32 + 4).min(CANVAS_WIDTH as i32 - 1) {
                    let pixel = y as usize * CANVAS_WIDTH + x as usize;
                    let hit = &surface_hits[pixel];
                    if !hit.is_bare_ground() {
                        continue;
                    }
                    let fraction_of_reach = ((hit.metres_east - tree.metres_east).powi(2)
                        + (hit.metres_south - tree.metres_south).powi(2))
                    .sqrt()
                        / reach;
                    if fraction_of_reach >= 1.0 {
                        continue;
                    }
                    let density = ((1.0 - fraction_of_reach * fraction_of_reach) * 230.0) as u8;
                    if density > leaf_litter[pixel].density {
                        leaf_litter[pixel] = LeafLitter { density, tree_index: tree_index as u16 };
                    }
                }
            }
        }

        let ground_noise = surface_hits
            .iter()
            .map(|hit| {
                if hit.is_bare_ground() {
                    ground::ground_noise_at(hit.ground_surface, hit.metres_east, hit.metres_south, seed)
                } else {
                    ground::GroundNoise::default()
                }
            })
            .collect();
        SeedGeometry { seed, solids, surface_hits, shadow_height_map, trees, crown_maps, leaf_litter, ground_noise }
    }
}

pub struct StaticPainter<'a> {
    pub colours: &'a mut [Colour],
    pub depths: &'a mut [f32],
    pub tree_ids: &'a mut [u16],
    pub glow_colours: &'a mut [Colour],
    pub floodlights: &'a mut [u8],
    pub current_tree_id: u16,
}

impl StaticPainter<'_> {
    fn is_visible(&self, x: i32, y: i32, depth: f32) -> bool {
        x >= 0
            && y >= 0
            && (x as usize) < CANVAS_WIDTH
            && (y as usize) < CANVAS_HEIGHT
            && depth >= self.depths[y as usize * CANVAS_WIDTH + x as usize] - 0.02
    }

    pub fn put(&mut self, x: i32, y: i32, depth: f32, colour: Colour) {
        if !self.is_visible(x, y, depth) {
            return;
        }
        let pixel = y as usize * CANVAS_WIDTH + x as usize;
        self.colours[pixel] = colour;
        self.depths[pixel] = depth;
        self.tree_ids[pixel] = self.current_tree_id;
        self.glow_colours[pixel] = 0;
        self.floodlights[pixel] = 0;
    }

    pub fn blend(&mut self, x: i32, y: i32, depth: f32, colour: Colour, opacity: f32) {
        if !self.is_visible(x, y, depth) {
            return;
        }
        let pixel = y as usize * CANVAS_WIDTH + x as usize;
        self.colours[pixel] = blend_colours(self.colours[pixel], colour, opacity);
    }

    pub fn line(&mut self, from_x: i32, from_y: i32, to_x: i32, to_y: i32, depth: f32, colour: Colour) {
        let (mut x, mut y) = (from_x, from_y);
        let width = (to_x - from_x).abs();
        let negative_height = -(to_y - from_y).abs();
        let step_x = if from_x < to_x { 1 } else { -1 };
        let step_y = if from_y < to_y { 1 } else { -1 };
        let mut error = width + negative_height;
        loop {
            self.put(x, y, depth, colour);
            if x == to_x && y == to_y {
                return;
            }
            let doubled_error = 2 * error;
            if doubled_error >= negative_height {
                error += negative_height;
                x += step_x;
            }
            if doubled_error <= width {
                error += width;
                y += step_y;
            }
        }
    }
}

#[derive(Copy, Clone)]
pub struct LampLight {
    pub globe_x: i32,
    pub globe_y: i32,
    pub foot_x: i32,
    pub foot_y: i32,
    pub pixels_per_metre: f32,
    pub metres_south: f32,
    pub is_arcade_lantern: bool,
}

#[derive(Copy, Clone)]
pub struct TreeSway {
    pub phase: f32,
    pub amplitude: f32,
    pub crown_foot_y: f32,
}

#[derive(Copy, Clone)]
pub struct GlowingPixel {
    pub pixel: u32,
    pub glow_colour: Colour,
    pub glow_group: u16,
}

pub struct StaticLayer {
    pub key: CacheKey,
    pub colours: Vec<Colour>,
    pub depths: Vec<f32>,
    pub tree_ids: Vec<u16>,
    pub swaying_foliage: Vec<(u32, u16)>,
    pub tree_sway: Vec<TreeSway>,
    pub glowing_pixels: Vec<GlowingPixel>,
    pub floodlit_pixels: Vec<(u32, u8)>,
    pub puddle_pixels: Vec<u32>,
    pub lamps: Vec<LampLight>,
    pub lamp_pools: Vec<(u32, u8)>,
}

enum Sprite {
    Tree(usize),
    LampPost(f32, f32),
    Bench(f32, f32),
    BikeRack(usize),
    Statue,
    ParkedCar(ParkedCar),
    BaldwinStepHandrails,
    Snowman,
}

const SHADOW_TINT: [f32; 3] = [0.61, 0.64, 0.79];
const HAZE_OVER_SNOW: Colour = rgb(196, 206, 222);
const HAZE: Colour = rgb(150, 172, 176);
const HAZE_STARTS_NORTH_OF: f32 = -52.0;

impl StaticLayer {
    pub fn build(geometry: &SeedGeometry, key: CacheKey) -> StaticLayer {
        let conditions = Conditions::new(&key);
        let mut colours = vec![0u32; PIXEL_COUNT];
        let mut depths = vec![0f32; PIXEL_COUNT];
        let mut glow_colours = vec![0u32; PIXEL_COUNT];
        let mut glow_groups = vec![0u16; PIXEL_COUNT];
        let mut floodlights = vec![0u8; PIXEL_COUNT];
        let mut tree_ids = vec![0u16; PIXEL_COUNT];
        let mut is_puddle = vec![false; PIXEL_COUNT];

        let fallen_leaf_colours: Vec<Colour> =
            geometry.trees.iter().map(|tree| foliage_in_season(tree, &conditions).leaf_colours[2]).collect();
        for y in 0..CANVAS_HEIGHT {
            for x in 0..CANVAS_WIDTH {
                let pixel = y * CANVAS_WIDTH + x;
                let hit = &geometry.surface_hits[pixel];
                depths[pixel] = hit.metres_south;
                if !hit.is_bare_ground() {
                    let building = shade_building(&geometry.solids, hit, x as i32, y as i32, &conditions);
                    colours[pixel] = building.colour;
                    glow_colours[pixel] = building.glow_colour;
                    glow_groups[pixel] = building.glow_group;
                    floodlights[pixel] = building.floodlight;
                    continue;
                }
                let surface_above = match y {
                    0 => hit.ground_surface,
                    _ if geometry.surface_hits[pixel - CANVAS_WIDTH].is_bare_ground() => {
                        geometry.surface_hits[pixel - CANVAS_WIDTH].ground_surface
                    }
                    _ => 254,
                };
                let litter = geometry.leaf_litter[pixel];
                let fallen_leaf_colour =
                    if litter.density > 0 { fallen_leaf_colours[litter.tree_index as usize] } else { 0 };
                let ground = ground::shade_ground(
                    hit,
                    x as i32,
                    y as i32,
                    &conditions,
                    litter.density,
                    fallen_leaf_colour,
                    surface_above,
                    &geometry.ground_noise[pixel],
                );
                colours[pixel] = ground.colour;
                is_puddle[pixel] = ground.is_puddle;
            }
        }

        if conditions.shadow_strength >= 0.05 {
            let east_run = -conditions.shadow_x_per_height * TYPICAL_HEIGHT_FORESHORTENING;
            let south_run =
                -conditions.shadow_y_per_height * TYPICAL_HEIGHT_FORESHORTENING / TYPICAL_GROUND_FORESHORTENING;
            let shadow_ceilings = geometry.shadow_height_map.shadow_ceilings(east_run, south_run);
            let mut tree_shadow_mask = vec![0u8; PIXEL_COUNT];
            stamp_tree_shadows(&geometry.trees, &conditions, &mut tree_shadow_mask);
            for pixel in 0..PIXEL_COUNT {
                let hit = &geometry.surface_hits[pixel];
                let on_ground = hit.is_bare_ground();
                let mut in_shadow = on_ground && tree_shadow_mask[pixel] > 0;
                if !in_shadow && (on_ground || hit.face == FACE_TOP) {
                    if let Some(cell) = geometry.shadow_height_map.cell_index_at(hit.metres_east, hit.metres_south) {
                        in_shadow = shadow_ceilings[cell] > hit.metres_up + 0.35;
                    }
                }
                if in_shadow {
                    let colour = colours[pixel];
                    colours[pixel] =
                        blend_colours(colour, tint_colour(colour, SHADOW_TINT), conditions.shadow_strength * 0.92);
                }
            }
        }

        let lamp_posts = lamp_post_positions();
        let mut sprites: Vec<(f32, Sprite)> = Vec::new();
        for (tree_index, tree) in geometry.trees.iter().enumerate() {
            sprites.push((tree.metres_south, Sprite::Tree(tree_index)));
        }
        for &(metres_east, metres_south) in lamp_posts.iter() {
            sprites.push((metres_south, Sprite::LampPost(metres_east, metres_south)));
        }
        for (metres_east, metres_south) in bench_positions() {
            sprites.push((metres_south, Sprite::Bench(metres_east, metres_south)));
        }
        for (rack_index, rack) in BIKE_RACKS.iter().enumerate() {
            sprites.push((rack.metres_south, Sprite::BikeRack(rack_index)));
        }
        sprites.push((STATUE_DEPTH, Sprite::Statue));
        for car in parked_cars(geometry.seed) {
            sprites.push((car.metres_south, Sprite::ParkedCar(car)));
        }
        sprites.push((BALDWIN_HANDRAIL_DEPTH, Sprite::BaldwinStepHandrails));
        if conditions.snow_cover > 0.55 {
            sprites.push((SNOWMAN_DEPTH, Sprite::Snowman));
        }
        sprites.sort_by(|first, second| first.0.total_cmp(&second.0));

        let mut tree_sway = vec![TreeSway { phase: 0.0, amplitude: 0.0, crown_foot_y: 0.0 }];
        let mut painter = StaticPainter {
            colours: &mut colours,
            depths: &mut depths,
            tree_ids: &mut tree_ids,
            glow_colours: &mut glow_colours,
            floodlights: &mut floodlights,
            current_tree_id: 0,
        };
        for (_, sprite) in sprites.iter() {
            match sprite {
                Sprite::Tree(tree_index) => {
                    let tree = &geometry.trees[*tree_index];
                    let sway_id = tree_sway.len() as u16;
                    let crown = tree.crown_on_screen();
                    let amplitude =
                        if tree.is_evergreen() { 0.7 } else { 1.0 } * (tree.crown_radius / 5.0).clamp(0.5, 1.4);
                    tree_sway.push(TreeSway {
                        phase: (tree.seed % 628) as f32 / 100.0,
                        amplitude,
                        crown_foot_y: crown.centre_y + crown.half_height,
                    });
                    draw_tree(&mut painter, tree, &geometry.crown_maps[*tree_index], sway_id, &conditions);
                }
                Sprite::LampPost(metres_east, metres_south) => {
                    draw_lamp_post(&mut painter, *metres_east, *metres_south, &conditions)
                }
                Sprite::Bench(metres_east, metres_south) => {
                    draw_bench(&mut painter, *metres_east, *metres_south, &conditions)
                }
                Sprite::BikeRack(rack_index) => draw_bike_rack(&mut painter, &BIKE_RACKS[*rack_index]),
                Sprite::Statue => draw_benjamin_duke_statue(&mut painter, &conditions),
                Sprite::ParkedCar(car) => draw_parked_car(&mut painter, car, &conditions),
                Sprite::BaldwinStepHandrails => draw_baldwin_step_handrails(&mut painter),
                Sprite::Snowman => draw_snowman(&mut painter),
            }
        }

        let haze = if conditions.snow_cover > 0.5 { HAZE_OVER_SNOW } else { HAZE };
        for pixel in 0..PIXEL_COUNT {
            let depth = depths[pixel];
            if depth < HAZE_STARTS_NORTH_OF {
                let distance_into_haze = ((HAZE_STARTS_NORTH_OF - depth) / 150.0).clamp(0.0, 1.0);
                let haze_step = ((distance_into_haze * 3.0).ceil() / 3.0) * 0.34 + 0.08;
                colours[pixel] = blend_colours(colours[pixel], haze, haze_step);
            }
        }

        let mut swaying_foliage = Vec::new();
        let mut glowing_pixels = Vec::new();
        let mut floodlit_pixels = Vec::new();
        let mut puddle_pixels = Vec::new();
        for pixel in 0..PIXEL_COUNT {
            if tree_ids[pixel] != 0 && depths[pixel] > -70.0 {
                swaying_foliage.push((pixel as u32, tree_ids[pixel]));
            }
            if glow_colours[pixel] != 0 && glow_groups[pixel] != 0 {
                glowing_pixels.push(GlowingPixel {
                    pixel: pixel as u32,
                    glow_colour: glow_colours[pixel],
                    glow_group: glow_groups[pixel],
                });
            }
            if floodlights[pixel] != 0 {
                floodlit_pixels.push((pixel as u32, floodlights[pixel]));
            }
            let ground_still_shows = depths[pixel] == geometry.surface_hits[pixel].metres_south
                && geometry.surface_hits[pixel].is_bare_ground();
            if is_puddle[pixel] && ground_still_shows {
                puddle_pixels.push(pixel as u32);
            }
        }

        let mut lamps: Vec<LampLight> = lamp_posts
            .iter()
            .map(|&(metres_east, metres_south)| {
                let (foot_x, foot_y) = project_to_pixel(metres_east, metres_south, 0.0);
                let (globe_x, globe_y) = project_to_pixel(metres_east, metres_south, LAMP_POST_HEIGHT);
                LampLight {
                    globe_x,
                    globe_y,
                    foot_x,
                    foot_y,
                    pixels_per_metre: pixels_per_metre_at(metres_south),
                    metres_south,
                    is_arcade_lantern: false,
                }
            })
            .collect();
        for (metres_east, metres_south) in arcade_lantern_positions() {
            let (foot_x, foot_y) = project_to_pixel(metres_east, metres_south, 0.0);
            lamps.push(LampLight {
                globe_x: foot_x,
                globe_y: foot_y,
                foot_x,
                foot_y,
                pixels_per_metre: pixels_per_metre_at(metres_south),
                metres_south,
                is_arcade_lantern: true,
            });
        }

        let mut pool_light = vec![0f32; PIXEL_COUNT];
        let mut stamp_pool =
            |centre_x: f32, centre_y: f32, half_width: f32, half_height: f32, metres_south: f32, brightness: f32| {
                let (left, right) = ((centre_x - half_width) as i32, (centre_x + half_width) as i32);
                let (top, bottom) = ((centre_y - half_height) as i32, (centre_y + half_height) as i32);
                for y in top.max(0)..=bottom.min(CANVAS_HEIGHT as i32 - 1) {
                    for x in left.max(0)..=right.min(CANVAS_WIDTH as i32 - 1) {
                        let (across, down) =
                            ((x as f32 + 0.5 - centre_x) / half_width, (y as f32 + 0.5 - centre_y) / half_height);
                        let distance = (across * across + down * down).sqrt();
                        if distance >= 1.0 {
                            continue;
                        }
                        let pixel = y as usize * CANVAS_WIDTH + x as usize;
                        if (depths[pixel] - metres_south).abs() > 9.0 {
                            continue;
                        }
                        let dither = ordered_dither_threshold(x, y);
                        let closeness = 1.0 - distance;
                        let level = if closeness > 0.62 + dither * 0.12 {
                            0.5
                        } else if closeness > 0.32 + dither * 0.14 {
                            0.3
                        } else if closeness > dither * 0.2 {
                            0.14
                        } else {
                            0.0
                        };
                        pool_light[pixel] += level * brightness;
                    }
                }
            };
        for lamp in lamps.iter() {
            let pool_half_width = if lamp.is_arcade_lantern { 3.4 } else { 6.2 } * lamp.pixels_per_metre;
            let brightness = if lamp.is_arcade_lantern { 1.25 } else { 1.0 };
            stamp_pool(
                lamp.foot_x as f32 + 0.5,
                lamp.foot_y as f32 + 0.5,
                pool_half_width,
                pool_half_width * ground_foreshortening_at(lamp.metres_south),
                lamp.metres_south,
                brightness,
            );
        }
        for side in [-1.0f32, 1.0] {
            for entrance_south in hall_entrances_south(side) {
                let (centre_x, centre_y) = project_to_screen(side * 30.0, entrance_south, 0.0);
                let pixels_per_metre = pixels_per_metre_at(entrance_south);
                stamp_pool(
                    centre_x,
                    centre_y,
                    3.0 * pixels_per_metre,
                    3.0 * pixels_per_metre * ground_foreshortening_at(entrance_south) * 1.6,
                    entrance_south,
                    0.6,
                );
            }
        }
        let lamp_pools = pool_light
            .iter()
            .enumerate()
            .filter(|(_, light)| **light > 0.0)
            .map(|(pixel, light)| (pixel as u32, (light.min(1.0) * 255.0) as u8))
            .collect();

        StaticLayer {
            key,
            colours,
            depths,
            tree_ids,
            swaying_foliage,
            tree_sway,
            glowing_pixels,
            floodlit_pixels,
            puddle_pixels,
            lamps,
            lamp_pools,
        }
    }
}
