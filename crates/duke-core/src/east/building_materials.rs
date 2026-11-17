use crate::east::buildings::*;
use crate::east::conditions::{palette, Conditions};
use crate::east::geometry::*;
use crate::east::projection::*;
use crate::east::shading_tools::*;
use crate::{blend_colours, hash_position, rgb, Colour};
use crate::{fractal_noise, value_noise};

pub const NO_GLOW_GROUP: u16 = 0;
pub const ALWAYS_LIT_GROUP: u16 = 1;
const FIRST_WINDOW_GROUP: u32 = 2;
const WINDOW_GROUP_COUNT: u32 = 60000;
const ENTRANCE_GABLE_WINDOW_GROUP_COUNT: u32 = 50000;
const BIDDLE_WINDOW_GROUP_COUNT: u32 = 40000;
const BALDWIN_FLANK_WINDOW_GROUPS_START: u16 = FIRST_WINDOW_GROUP as u16 + 950;
const BALDWIN_EAST_FLANK_GROUP_OFFSET: u16 = 20;

#[derive(Copy, Clone, Default)]
pub struct BuildingPixel {
    pub colour: Colour,
    pub glow_colour: Colour,
    pub glow_group: u16,
    pub floodlight: u8,
}

fn plain(colour: Colour) -> BuildingPixel {
    BuildingPixel { colour, glow_colour: 0, glow_group: NO_GLOW_GROUP, floodlight: 0 }
}

fn floodlit(colour: Colour, floodlight: u8) -> BuildingPixel {
    BuildingPixel { colour, glow_colour: 0, glow_group: NO_GLOW_GROUP, floodlight }
}

fn glowing(colour: Colour, glow_colour: Colour, glow_group: u16) -> BuildingPixel {
    BuildingPixel { colour, glow_colour, glow_group, floodlight: 0 }
}

fn window_group(window_hash: u32) -> u16 {
    window_group_among(window_hash, WINDOW_GROUP_COUNT)
}

fn window_group_among(window_hash: u32, group_count: u32) -> u16 {
    (FIRST_WINDOW_GROUP + window_hash % group_count) as u16
}

struct Shading<'a> {
    solid: &'a Solid,
    solid_index: usize,
    hit: &'a SurfaceHit,
    screen_x: i32,
    screen_y: i32,
    conditions: &'a Conditions,
    pixels_per_metre: f32,
    rows_per_metre: f32,
    row_above_ground: i32,
    seed: u32,
}

impl Shading<'_> {
    fn row_at(&self, height: f32) -> i32 {
        (height * self.rows_per_metre).round() as i32
    }

    fn screen_column_at(&self, metres_east: f32) -> i32 {
        (QUAD_AXIS_SCREEN_X + metres_east * self.pixels_per_metre).floor() as i32
    }

    fn distance_from_roof_edge(&self) -> f32 {
        let hit = self.hit;
        let solid = self.solid;
        (hit.metres_east - solid.west_edge)
            .min(solid.east_edge - hit.metres_east)
            .min(hit.metres_south - solid.north_edge)
            .min(solid.south_edge - hit.metres_south)
    }

    fn wall_light(&self, south_facing_light: f32) -> f32 {
        if self.hit.face == FACE_SOUTH {
            return south_facing_light;
        }
        side_wall_light(self.conditions, self.hit.metres_east)
    }

    fn snow_on_top(&self, exposure: f32) -> Option<Colour> {
        let conditions = self.conditions;
        let hit = self.hit;
        if conditions.snow_cover <= 0.0 {
            return None;
        }
        let steepness = (hit.slope_east * hit.slope_east + hit.slope_south * hit.slope_south).sqrt();
        let exposure_after_sliding = exposure * (1.0 - (steepness - 0.4).clamp(0.0, 0.5) * 0.5);
        let (patch_across, patch_along) = if steepness < 0.3 {
            (hit.metres_east * 1.7, hit.metres_south * 1.7)
        } else if hit.slope_east.abs() > hit.slope_south.abs() {
            (hit.metres_south * 5.0, hit.metres_up * 1.2)
        } else {
            (hit.metres_east * 5.0, hit.metres_up * 1.2)
        };
        if !conditions.snow_covers_point(patch_across, patch_along, exposure_after_sliding) {
            return None;
        }
        let light = conditions.direction_to_light;
        let brightness = (0.62 + (-hit.slope_east * light[0] - hit.slope_south * light[1]) * 1.25).clamp(0.0, 1.0);
        let drift = fractal_noise(hit.metres_east * 0.2, hit.metres_south * 0.2, 2, 911);
        Some(banded_ramp_colour(
            &palette::SNOW,
            0.12 + brightness * 0.72 + (drift - 0.5) * 0.3,
            self.screen_x,
            self.screen_y,
            0.1,
        ))
    }

    fn snow_on_ledge(&self) -> Option<Colour> {
        let snow_cover = self.conditions.snow_cover;
        let line_is_unbroken =
            snow_cover > 0.5 || !hash_position(self.screen_x >> 1, self.screen_y, 77).is_multiple_of(3);
        if snow_cover > 0.12 && line_is_unbroken {
            Some(palette::SNOW[3])
        } else {
            None
        }
    }

    fn top_light(&self) -> f32 {
        let hit = self.hit;
        let light = self.conditions.direction_to_light;
        let normal_length = (hit.slope_east * hit.slope_east + hit.slope_south * hit.slope_south + 1.0).sqrt();
        ((-hit.slope_east * light[0] - hit.slope_south * light[1] + light[2]) / normal_length).clamp(0.0, 1.0)
    }

    fn flat_roof_deck(&self, seed: u32) -> Colour {
        let hit = self.hit;
        let staining = fractal_noise(hit.metres_east * 0.14, hit.metres_south * 0.06, 2, seed);
        let mut deck = banded_ramp_colour(
            &palette::FLAT_ROOF_DECK[1..4],
            0.5 + (staining - 0.5) * 1.3,
            self.screen_x,
            self.screen_y,
            0.07,
        );
        let on_membrane_seam = ((hit.metres_east + 200.0) / 4.3).rem_euclid(1.0) < 0.06;
        if on_membrane_seam {
            deck = palette::FLAT_ROOF_DECK[1];
        }
        let (cell_east, cell_south) = ((hit.metres_east / 2.8).floor(), (hit.metres_south / 8.0).floor());
        let cell_hash = hash_position(cell_east as i32, cell_south as i32, seed ^ 606);
        let cell_has_roof_furniture = cell_hash.is_multiple_of(7);
        if cell_has_roof_furniture {
            let (within_east, within_south) = (hit.metres_east / 2.8 - cell_east, hit.metres_south / 8.0 - cell_south);
            let (furniture_east, furniture_south) =
                (0.25 + ((cell_hash >> 8) & 3) as f32 * 0.12, 0.3 + ((cell_hash >> 11) & 3) as f32 * 0.1);
            if (within_east - furniture_east).abs() < 0.14 && (within_south - furniture_south).abs() < 0.2 {
                deck = if within_south < furniture_south {
                    palette::FLAT_ROOF_DECK[3]
                } else {
                    palette::FLAT_ROOF_DECK[0]
                };
            }
        }
        deck
    }

    fn brick_wall(&self, tone: i32, brick_column: i32, wetness_factor: f32) -> BuildingPixel {
        let brick = brick_colour(&palette::BRICK, tone, brick_column, self.row_above_ground, self.seed);
        plain(rain_darkened(brick, self.conditions.wetness * wetness_factor))
    }
}

fn side_wall_light(conditions: &Conditions, metres_east: f32) -> f32 {
    if metres_east < 0.0 {
        conditions.east_facing_wall_light
    } else {
        1.0 - conditions.east_facing_wall_light
    }
}

fn brick_colour(ramp: &[Colour], tone: i32, brick_column: i32, course: i32, seed: u32) -> Colour {
    let brick_hash = hash_position((brick_column + (course & 1)) >> 1, course, seed);
    let mut index = tone;
    if brick_hash.is_multiple_of(9) {
        index -= 1;
    } else if brick_hash.is_multiple_of(19) {
        index += 1;
    }
    ramp[index.clamp(0, ramp.len() as i32 - 1) as usize]
}

const DRAWN_BLIND: Colour = rgb(206, 200, 184);

fn sash_window(columns_from_centre: i32, row: i32, row_count: i32, half_width: i32, window_hash: u32) -> BuildingPixel {
    let in_upper_sash = row * 2 >= row_count;
    let on_meeting_rail = row_count >= 4 && row == row_count / 2;
    let on_centre_bar = half_width >= 3 && columns_from_centre == 0;
    let mut colour = if in_upper_sash { palette::GLASS[0] } else { palette::GLASS[1] };
    if on_meeting_rail {
        colour = palette::MARBLE[2];
    } else if on_centre_bar {
        colour = blend_colours(colour, palette::MARBLE[2], 0.55);
    } else if columns_from_centre == -(half_width - 1) && row == row_count - 1 {
        colour = palette::GLASS[2];
    }
    let blind_is_drawn = window_hash.is_multiple_of(5) && in_upper_sash;
    if blind_is_drawn {
        colour = DRAWN_BLIND;
    }
    let lamp_colour = if window_hash % 7 == 3 { palette::COOL_LIT_WINDOW } else { palette::LIT_WINDOW };
    let glow_colour = if on_meeting_rail || on_centre_bar { 0 } else { lamp_colour };
    glowing(colour, glow_colour, window_group(window_hash))
}

const OCULUS_GLASS: Colour = rgb(150, 172, 184);
const CHIMNEY_FLUE: Colour = rgb(44, 38, 44);
const DOOR_PAINT: Colour = rgb(236, 232, 222);
const PENTHOUSE_ROOF: Colour = rgb(228, 228, 224);
const PENTHOUSE_ROOF_EDGE: Colour = rgb(200, 202, 206);
const SKYLIGHT_SOUTH_FACET: Colour = rgb(110, 140, 166);
const SKYLIGHT_NORTH_FACET: Colour = rgb(176, 206, 220);
const SKYLIGHT_LIT_FACET: Colour = rgb(140, 174, 196);
const SKYLIGHT_SHADED_FACET: Colour = rgb(92, 122, 150);
const SKYLIGHT_GLOW: Colour = rgb(150, 164, 150);
const DARKEST_BUILDING_COLOUR: Colour = rgb(1, 1, 1);

pub fn shade_building(
    solids: &[Solid],
    hit: &SurfaceHit,
    screen_x: i32,
    screen_y: i32,
    conditions: &Conditions,
) -> BuildingPixel {
    let solid_index = hit.solid_index as usize;
    let solid = &solids[solid_index];
    let rows_per_metre = screen_rows_per_metre_of_height_at(hit.metres_south);
    let shading = Shading {
        solid,
        solid_index,
        hit,
        screen_x,
        screen_y,
        conditions,
        pixels_per_metre: pixels_per_metre_at(hit.metres_south),
        rows_per_metre,
        row_above_ground: (hit.metres_up * rows_per_metre).floor() as i32,
        seed: conditions.seed ^ (solid_index as u32).wrapping_mul(0x9E37),
    };
    let is_top = hit.face == FACE_TOP;
    let is_south = hit.face == FACE_SOUTH;
    let wetness = conditions.wetness;

    let mut pixel = match solid.material {
        Material::BaldwinBrickTier { faces_portico } => baldwin_brick_tier(&shading, faces_portico),
        Material::DomeDrum => dome_drum(&shading),
        Material::DomeGutterRing => {
            if is_top {
                plain(palette::BRONZE[1])
            } else {
                let across = hit.metres_east / GUTTER_RING_RADIUS;
                let brightness = (0.5
                    - across * 0.5 * (conditions.direction_to_light[0] * 3.0).clamp(-1.0, 1.0)
                    - across.abs() * 0.3)
                    .clamp(0.0, 1.0);
                floodlit(nearest_ramp_colour(&palette::BRONZE, brightness * 0.8), 70)
            }
        }
        Material::Dome => dome(&shading),
        Material::DomeOculus => {
            if is_top {
                let from_centre = (hit.metres_east * hit.metres_east + hit.metres_south * hit.metres_south).sqrt();
                let glass_or_rim = if from_centre < 0.85 { OCULUS_GLASS } else { palette::DOME[5] };
                floodlit(shading.snow_on_top(1.3).unwrap_or(glass_or_rim), 60)
            } else {
                plain(palette::DOME[2])
            }
        }
        Material::PorticoFloor => {
            if is_top {
                let at_lip = hit.metres_south > 22.5;
                let on_joint = ((hit.metres_east + 20.0) / 3.3).rem_euclid(1.0) < 0.08;
                let marble = if at_lip {
                    palette::MARBLE[4]
                } else if on_joint {
                    palette::MARBLE[2]
                } else {
                    palette::MARBLE[3]
                };
                floodlit(marble, 150)
            } else {
                plain(palette::MARBLE[2])
            }
        }
        Material::PorticoSteps => {
            let is_tread_row = screen_y & 1 == 1;
            let mut marble = if is_tread_row { palette::MARBLE[4] } else { palette::MARBLE[2] };
            if is_tread_row
                && conditions.snow_cover > 0.0
                && conditions.snow_covers_point(hit.metres_east, hit.metres_south * 3.0, 0.75)
            {
                marble = palette::SNOW[3];
            }
            floodlit(rain_darkened(marble, wetness * 0.5), 120)
        }
        Material::PorticoColumn { centre_east } => {
            let columns_from_centre = screen_x - shading.screen_column_at(centre_east);
            let highlight_shift = if conditions.direction_to_light[0] < -0.1 { 1 } else { 0 };
            let fluting_tone = match columns_from_centre + highlight_shift {
                i32::MIN..=-2 => 2,
                -1 => 4,
                0 => 3,
                1 => 2,
                _ => 1,
            };
            let under_capital = shading.row_above_ground >= shading.row_at(PORTICO_COLUMN_TOP_HEIGHT) - 1;
            floodlit(palette::MARBLE[if under_capital { 2 } else { fluting_tone }], 230)
        }
        Material::ColumnCapital { centre_east } => {
            let capital_column =
                (QUAD_AXIS_SCREEN_X + centre_east * pixels_per_metre_at(PORTICO_COLUMNS_SOUTH)).floor() as i32;
            let on_volute = (screen_x - capital_column).abs() >= 2;
            let marble = if is_top {
                palette::MARBLE[3]
            } else if on_volute {
                palette::MARBLE[1]
            } else {
                palette::MARBLE[4]
            };
            floodlit(marble, 220)
        }
        Material::ColumnBase => floodlit(palette::MARBLE[if is_top { 3 } else { 2 }], 220),
        Material::Entablature => entablature(&shading),
        Material::Pediment => pediment(&shading),
        Material::ArcadeRoof { runs_north_south, ridge } => {
            if is_top {
                arcade_roof(&shading, runs_north_south, ridge)
            } else {
                plain(palette::MARBLE[2])
            }
        }
        Material::ArcadeWall { first_bay_start, bay_length, runs_north_south } => {
            arcade_wall(&shading, first_bay_start, bay_length, runs_north_south)
        }
        Material::HallWall { entrance_south, is_entrance_pavilion } => {
            hall_wall(&shading, entrance_south, is_entrance_pavilion)
        }
        Material::HallRoof => hall_roof(&shading),
        Material::Dormer { facing, centre } => hall_dormer(&shading, facing, centre),
        Material::Chimney => {
            let below_top = hit.metres_up - solid.top_height;
            if is_top {
                let on_coping = shading.distance_from_roof_edge() < 0.28;
                if !on_coping {
                    plain(CHIMNEY_FLUE)
                } else {
                    plain(shading.snow_on_ledge().unwrap_or(palette::MARBLE[2]))
                }
            } else if below_top > -0.35 {
                plain(palette::MARBLE[if is_south { 3 } else { 2 }])
            } else {
                let tone =
                    if is_south { 2 } else { 1 + (side_wall_light(conditions, hit.metres_east) * 2.0).round() as i32 };
                shading.brick_wall(tone, screen_x, 0.7)
            }
        }
        Material::EntranceGable { ridge_south } => entrance_gable(&shading, ridge_south),
        Material::DoorSurround { door_south } => {
            if is_top {
                plain(palette::MARBLE[3])
            } else if is_south {
                plain(palette::MARBLE[2])
            } else {
                let from_door_centre = (hit.metres_south - door_south).abs();
                let in_doorway = from_door_centre < 0.55 && hit.metres_up < 2.5;
                let in_fanlight = hit.metres_up > 2.0;
                if in_doorway && in_fanlight {
                    glowing(palette::GLASS[1], palette::LIT_WINDOW, ALWAYS_LIT_GROUP)
                } else if in_doorway {
                    plain(DOOR_PAINT)
                } else if shading.row_above_ground == shading.row_at(2.9) {
                    plain(palette::MARBLE[1])
                } else {
                    plain(palette::MARBLE[if from_door_centre > 0.9 { 4 } else { 3 }])
                }
            }
        }
        Material::Hedge { variation } => {
            let leafiness = value_noise(hit.metres_east * 1.3 + variation * 9.0, hit.metres_south * 1.3, shading.seed);
            if is_top {
                let foliage = dithered_ramp_colour(&palette::HEDGE[1..4], 0.2 + leafiness * 0.8, screen_x, screen_y);
                plain(shading.snow_on_top(1.1).unwrap_or(foliage))
            } else {
                plain(dithered_ramp_colour(&palette::HEDGE[0..3], 0.1 + leafiness * 0.6, screen_x, screen_y))
            }
        }
        Material::BiddleMusicBuilding => biddle_music_building(&shading),
        Material::StatuePlinth { is_base_step } => {
            let mut granite = match hit.face {
                FACE_TOP => palette::GRANITE[3],
                FACE_SOUTH => palette::GRANITE[2],
                _ => palette::GRANITE[if side_wall_light(conditions, hit.metres_east.signum()) > 0.5 { 2 } else { 1 }],
            };
            if is_south && !is_base_step {
                let columns_from_axis = screen_x - QUAD_AXIS_SCREEN_X as i32;
                let on_inscription_line =
                    shading.row_above_ground == shading.row_at(0.8) || shading.row_above_ground == shading.row_at(1.15);
                if columns_from_axis.abs() <= 2 && on_inscription_line {
                    granite = palette::GRANITE[0];
                }
                if hash_position(screen_x, screen_y, 5).is_multiple_of(6) {
                    granite = palette::GRANITE[1];
                }
            }
            if is_top {
                granite = shading.snow_on_top(1.2).unwrap_or(granite);
            }
            plain(rain_darkened(granite, wetness * 0.6))
        }
        Material::TerracePlanter => {
            if is_top {
                let leafiness = value_noise(hit.metres_east * 1.5, hit.metres_south * 1.5, shading.seed);
                let planting = if leafiness > 0.45 { palette::HEDGE[2] } else { palette::HEDGE[1] };
                plain(shading.snow_on_top(1.1).unwrap_or(planting))
            } else {
                plain(palette::CONCRETE[if is_south { 2 } else { 1 }])
            }
        }
        Material::RoofPlant { is_white_penthouse } => {
            if is_top {
                let from_edge = shading.distance_from_roof_edge();
                let roof = if is_white_penthouse {
                    if from_edge < 0.5 {
                        PENTHOUSE_ROOF_EDGE
                    } else {
                        PENTHOUSE_ROOF
                    }
                } else if from_edge < 0.35 {
                    palette::FLAT_ROOF_DECK[3]
                } else {
                    palette::FLAT_ROOF_DECK[2]
                };
                plain(shading.snow_on_top(1.0).unwrap_or(roof))
            } else {
                let on_louvre_slat = shading.row_above_ground & 1 == 0;
                plain(palette::FLAT_ROOF_DECK[if on_louvre_slat { 1 } else { 0 }])
            }
        }
        Material::LibraryWall { is_union, is_low_pavilion } => library_wall(&shading, is_union, is_low_pavilion),
        Material::LibraryStylobate => {
            let marble = if is_top || side_wall_light_for_portico(&shading) <= 0.3 {
                palette::MARBLE[2]
            } else {
                palette::MARBLE[3]
            };
            floodlit(marble, 110)
        }
        Material::LibrarySteps => {
            let mut marble = if is_top {
                palette::MARBLE[4]
            } else if (shading.row_above_ground & 1) == 0 {
                palette::MARBLE[2]
            } else {
                palette::MARBLE[1]
            };
            if is_top && conditions.snow_cover > 0.3 {
                marble = palette::SNOW[3];
            }
            floodlit(rain_darkened(marble, wetness * 0.4), 90)
        }
        Material::LibraryColumn => {
            let column_centre = (QUAD_AXIS_SCREEN_X
                + 0.5 * (solid.west_edge + solid.east_edge) * pixels_per_metre_at(hit.metres_south))
            .floor() as i32;
            let shaft_tone = match screen_x - column_centre {
                i32::MIN..=-1 => 4,
                0 => 3,
                _ => 2,
            };
            let on_capital = shading.row_above_ground >= shading.row_at(LIBRARY_COLUMN_TOP_HEIGHT) - 1;
            floodlit(palette::MARBLE[if on_capital { 4 } else { shaft_tone }], 150)
        }
        Material::LibraryAttic => {
            let row_count = ((solid.top_height - LIBRARY_COLUMN_TOP_HEIGHT) * rows_per_metre).round() as i32;
            let row = ((hit.metres_up - LIBRARY_COLUMN_TOP_HEIGHT) * rows_per_metre).floor() as i32;
            let marble = if is_top {
                shading.snow_on_top(1.0).unwrap_or(palette::MARBLE[3])
            } else if row >= row_count - 1 {
                palette::MARBLE[4]
            } else if row == 0 || row == row_count / 2 {
                palette::MARBLE[1]
            } else if is_south {
                palette::MARBLE[3]
            } else {
                palette::MARBLE[if side_wall_light_for_portico(&shading) > 0.3 { 4 } else { 2 }]
            };
            floodlit(marble, 110)
        }
        Material::LibrarySkylight => {
            let on_rib = (hit.slope_east.abs() - hit.slope_south.abs()).abs() < 0.12
                || hit.metres_up > LIBRARY_ROOF_HEIGHT + 2.2;
            let facet = if hit.slope_south.abs() > hit.slope_east.abs() {
                if hit.slope_south < 0.0 {
                    SKYLIGHT_NORTH_FACET
                } else {
                    SKYLIGHT_SOUTH_FACET
                }
            } else if (hit.slope_east > 0.0) == (conditions.direction_to_light[0] < 0.0) {
                SKYLIGHT_LIT_FACET
            } else {
                SKYLIGHT_SHADED_FACET
            };
            match shading.snow_on_top(0.6) {
                Some(snow) if on_rib => plain(snow),
                Some(snow) => glowing(snow, 0, ALWAYS_LIT_GROUP),
                None if on_rib => plain(palette::MARBLE[4]),
                None => glowing(facet, SKYLIGHT_GLOW, ALWAYS_LIT_GROUP),
            }
        }
        Material::House { style, ridge_runs_east_west, wall_height } => {
            house(&shading, style, ridge_runs_east_west, wall_height)
        }
        Material::GreenHallWall => green_hall_wall(&shading),
        Material::GreenHallRoof { north_end } => green_hall_roof(&shading, north_end),
    };
    if pixel.colour == 0 {
        pixel.colour = DARKEST_BUILDING_COLOUR;
    }
    pixel
}

fn side_wall_light_for_portico(shading: &Shading) -> f32 {
    if shading.hit.face == FACE_SIDE {
        side_wall_light(shading.conditions, shading.hit.metres_east)
    } else {
        0.5
    }
}

fn baldwin_brick_tier(shading: &Shading, faces_portico: bool) -> BuildingPixel {
    let hit = shading.hit;
    let conditions = shading.conditions;
    let screen_x = shading.screen_x;
    let row = shading.row_above_ground;
    let is_south = hit.face == FACE_SOUTH;
    if hit.face == FACE_TOP {
        let from_edge = shading.distance_from_roof_edge();
        if from_edge < 0.55 {
            return plain(shading.snow_on_ledge().unwrap_or(palette::MARBLE[4]));
        }
        if from_edge < 0.95 {
            return plain(palette::FLAT_ROOF_DECK[0]);
        }
        let deck = shading.snow_on_top(1.05).unwrap_or(shading.flat_roof_deck(shading.seed));
        return plain(rain_darkened(deck, conditions.wetness * 0.5));
    }

    let top_row = shading.row_at(shading.solid.top_height);
    let light = shading.wall_light(0.5);
    let mut tone = 1 + (light * 2.0).round() as i32;
    if row >= top_row - 1 {
        let coping = if light < 0.3 {
            palette::MARBLE[2]
        } else if is_south {
            palette::MARBLE[4]
        } else {
            palette::MARBLE[3]
        };
        return floodlit(coping, 60);
    }
    if row == top_row - 2 {
        return plain(palette::MARBLE[1]);
    }
    if row == top_row - 3 {
        tone -= 1;
    }
    let behind_the_columns = faces_portico && is_south && hit.metres_south > 19.0;
    if behind_the_columns {
        return portico_wall(shading);
    }

    let belt_row = shading.row_at(8.9);
    let water_table_row = shading.row_at(PORTICO_FLOOR_HEIGHT);
    if row == belt_row || row == water_table_row {
        return plain(palette::MARBLE[if light < 0.3 { 2 } else { 3 }]);
    }
    if row < water_table_row {
        return plain(rain_darkened(palette::MARBLE[if light < 0.3 { 1 } else { 2 }], conditions.wetness * 0.6));
    }

    let on_corner_block_front = is_south && hit.metres_up < 12.4 && hit.metres_east.abs() > 9.0;
    if on_corner_block_front {
        let window_column = shading.screen_column_at(hit.metres_east.signum() * 12.65);
        let columns_from_centre = screen_x - window_column;
        let (sill_row, head_row) = (shading.row_at(3.4), shading.row_at(7.6));
        let half_width = (0.85 * shading.pixels_per_metre).round() as i32;
        if columns_from_centre.abs() <= half_width && row >= sill_row - 1 && row <= head_row {
            let on_frame = row == sill_row - 1 || row == head_row || columns_from_centre.abs() == half_width;
            if on_frame {
                return plain(palette::MARBLE[4]);
            }
            let window_hash = 900 + (hit.metres_east > 0.0) as u32;
            return sash_window(columns_from_centre, row - sill_row, head_row - sill_row, half_width, window_hash);
        }
    }
    if hit.face == FACE_SIDE && hit.metres_up < 12.4 {
        let bay_position = (hit.metres_south + 18.5) / 4.4;
        let bay = bay_position.floor();
        let from_bay_centre = (bay_position - bay - 0.5).abs();
        if from_bay_centre < 0.2 && hit.metres_up > 3.2 && hit.metres_up < 7.8 {
            let group = BALDWIN_FLANK_WINDOW_GROUPS_START
                + bay.max(0.0) as u16
                + if hit.metres_east > 0.0 { BALDWIN_EAST_FLANK_GROUP_OFFSET } else { 0 };
            return glowing(palette::GLASS[0], palette::LIT_WINDOW, group);
        }
        if from_bay_centre < 0.3 && hit.metres_up > 2.9 && hit.metres_up < 8.1 {
            return plain(palette::MARBLE[3]);
        }
    }

    let brick_column = if is_south { screen_x } else { (hit.metres_south * 3.0) as i32 };
    let mut pixel = shading.brick_wall(tone, brick_column, 0.75);
    pixel.floodlight = if is_south { 40 } else { 0 };
    pixel
}

fn portico_wall(shading: &Shading) -> BuildingPixel {
    let hit = shading.hit;
    let screen_x = shading.screen_x;
    let row = shading.row_above_ground;
    let floor_row = shading.row_at(PORTICO_FLOOR_HEIGHT);
    let under_portico_roof = hit.metres_up < 12.0;
    let brick_ramp: &[Colour] = if under_portico_roof { &palette::SHADED_BRICK } else { &palette::BRICK };
    let mut pixel =
        floodlit(brick_colour(brick_ramp, 2, screen_x, row, shading.seed), if under_portico_roof { 170 } else { 60 });
    if hit.metres_up > 12.0 {
        return pixel;
    }
    for side in [-1.0f32, 1.0] {
        let pilaster_column = shading.screen_column_at(side * 8.1);
        if (screen_x - pilaster_column).abs() <= 1 && row >= floor_row {
            pixel.colour = palette::MARBLE[if screen_x - pilaster_column == 1 { 1 } else { 2 }];
            return pixel;
        }
    }
    if row == shading.row_at(7.0) {
        pixel.colour = palette::MARBLE[2];
    }
    for door in -1..=1 {
        let door_column = shading.screen_column_at(door as f32 * 3.3);
        let columns_from_centre = screen_x - door_column;
        let half_width = (1.15 * shading.pixels_per_metre).round() as i32;
        if columns_from_centre.abs() > half_width {
            continue;
        }
        let lintel_row = shading.row_at(4.75);
        let (window_sill_row, window_head_row) = (shading.row_at(5.3), shading.row_at(8.9));
        if row >= floor_row && row < lintel_row {
            pixel.colour = if columns_from_centre.abs() == half_width {
                palette::MARBLE[2]
            } else if columns_from_centre == 0 {
                palette::MARBLE[1]
            } else if (row - floor_row) == (lintel_row - floor_row) / 2 {
                palette::MARBLE[2]
            } else {
                palette::MARBLE[3]
            };
            pixel.floodlight = 200;
        } else if row == lintel_row {
            pixel.colour = palette::MARBLE[2];
        } else if row >= window_sill_row && row < window_head_row {
            let rows_below_head = window_head_row - 1 - row;
            let arch_half_width = match rows_below_head {
                0 => half_width - 2,
                1 => half_width - 1,
                _ => half_width,
            };
            if columns_from_centre.abs() == arch_half_width {
                pixel.colour = palette::MARBLE[2];
            } else if columns_from_centre.abs() < arch_half_width {
                let window = sash_window(
                    columns_from_centre,
                    row - window_sill_row,
                    window_head_row - window_sill_row,
                    half_width - 1,
                    (door + 9) as u32,
                );
                pixel.colour = window.colour;
                pixel.glow_colour = window.glow_colour;
                pixel.glow_group = ALWAYS_LIT_GROUP;
            }
        }
    }
    pixel
}

fn dome_drum(shading: &Shading) -> BuildingPixel {
    let hit = shading.hit;
    if hit.face == FACE_TOP {
        return plain(palette::FLAT_ROOF_DECK[1]);
    }
    let row_below_top = ((DRUM_TOP_HEIGHT - hit.metres_up) * shading.rows_per_metre).floor() as i32;
    let across = (hit.metres_east / DRUM_RADIUS).clamp(-1.0, 1.0);
    let light_east = shading.conditions.direction_to_light[0];
    let light = (0.55 - across * (light_east * 2.4).clamp(-0.8, 0.8) - across * across * 0.25).clamp(0.0, 1.0);
    let colour = if row_below_top <= 0 {
        palette::MARBLE[if light > 0.35 { 4 } else { 3 }]
    } else if row_below_top == 1 {
        let on_dentil = shading.screen_x & 1 == 0;
        palette::MARBLE[if on_dentil { 2 } else { 1 }]
    } else if row_below_top == 5 {
        palette::MARBLE[if light > 0.35 { 3 } else { 2 }]
    } else {
        let tone = 1 + (light * 2.4) as i32;
        let brick_column = ((hit.metres_east / DRUM_RADIUS).asin() * 28.0) as i32;
        rain_darkened(
            brick_colour(&palette::BRICK, tone, brick_column, row_below_top, 91),
            shading.conditions.wetness * 0.7,
        )
    };
    floodlit(colour, 110)
}

fn dome(shading: &Shading) -> BuildingPixel {
    let hit = shading.hit;
    let conditions = shading.conditions;
    let (screen_x, screen_y) = (shading.screen_x, shading.screen_y);
    let from_centre = (hit.metres_east * hit.metres_east + hit.metres_south * hit.metres_south).sqrt();

    let leaning_key_light = (conditions.direction_to_light[0] * 1.2).clamp(-0.55, 0.55);
    let key_light_east = if leaning_key_light.abs() < 0.2 { -0.3 } else { leaning_key_light };
    let (key_light_south, key_light_up) = (0.42f32, 0.74f32);
    let normal_length = (hit.slope_east * hit.slope_east + hit.slope_south * hit.slope_south + 1.0).sqrt();
    let key_light_length =
        (key_light_east * key_light_east + key_light_south * key_light_south + key_light_up * key_light_up).sqrt();
    let key_light = ((-hit.slope_east * key_light_east - hit.slope_south * key_light_south + key_light_up)
        / (normal_length * key_light_length))
        .clamp(0.0, 1.0);

    let tone = ((key_light - 0.42) / 0.55).clamp(0.0, 1.0).powf(1.15) * 0.94 + 0.03;
    let on_slate_course = (from_centre / 2.7).fract() < 0.14 && from_centre > 5.0 && from_centre < DOME_RADIUS - 0.6;
    let bearing = hit.metres_east.atan2(hit.metres_south);
    let tile_hash = hash_position((bearing * from_centre * 0.9) as i32, (from_centre * 1.6) as i32, 733);
    let one_ramp_step = 1.0 / (palette::DOME.len() as f32 - 1.0);
    let two_ramp_steps = 2.0 / (palette::DOME.len() as f32 - 1.0);
    let dome_colour = |level: f32| banded_ramp_colour(&palette::DOME, level, screen_x, screen_y, 0.16);

    let mut colour = dome_colour(tone);
    if on_slate_course {
        colour = dome_colour((tone - one_ramp_step).max(0.0));
    } else if tile_hash.is_multiple_of(19) {
        colour = dome_colour((tone + one_ramp_step).min(1.0));
    }
    let on_oculus_flashing = from_centre < 2.75 && from_centre > 2.2;
    if on_oculus_flashing {
        colour = dome_colour((tone + two_ramp_steps).min(1.0));
    }
    if from_centre > DOME_RADIUS - 0.45 {
        colour = palette::DOME[1];
    }
    if conditions.snow_cover > 0.0 {
        let steepness = (hit.slope_east * hit.slope_east + hit.slope_south * hit.slope_south).sqrt();
        let drift = fractal_noise(hit.metres_east * 0.3, hit.metres_south * 0.3, 3, 4242);
        let steepest_snowy_slope = conditions.snow_cover * 2.1 - 0.15 + (drift - 0.5) * 0.5;
        if steepness < steepest_snowy_slope {
            let brightness = (0.5 + (key_light - 0.6) * 1.6).clamp(0.0, 1.0);
            colour = dithered_ramp_colour(&palette::SNOW, 0.2 + brightness * 0.8, screen_x, screen_y);
            if on_slate_course && conditions.snow_cover < 0.7 {
                colour = palette::SNOW[1];
            }
        }
    }
    colour = rain_darkened(colour, conditions.wetness * 0.45);
    let fraction_to_skirt = from_centre / DOME_RADIUS;
    floodlit(colour, (26.0 + 120.0 * fraction_to_skirt * fraction_to_skirt * fraction_to_skirt) as u8)
}

fn entablature(shading: &Shading) -> BuildingPixel {
    let hit = shading.hit;
    let base_row = shading.row_at(ENTABLATURE_BASE_HEIGHT);
    let row = shading.row_above_ground - base_row;
    let row_count = shading.row_at(PEDIMENT_BASE_HEIGHT) - base_row;
    if hit.face == FACE_TOP {
        return floodlit(palette::MARBLE[3], 200);
    }
    if hit.face == FACE_SIDE {
        return floodlit(palette::MARBLE[2], 200);
    }
    let columns_from_axis = shading.screen_x - QUAD_AXIS_SCREEN_X as i32;
    let marble = if row >= row_count - 1 {
        palette::MARBLE[4]
    } else if row == row_count - 2 {
        palette::MARBLE[if shading.screen_x & 1 == 0 { 1 } else { 2 }]
    } else if row == 0 {
        palette::MARBLE[2]
    } else {
        let on_inscription = row == row_count / 2
            && columns_from_axis.abs() <= 16
            && columns_from_axis.abs() % 3 != 2
            && (columns_from_axis + 40) % 7 != 0;
        palette::MARBLE[if on_inscription { 1 } else { 3 }]
    };
    floodlit(marble, 200)
}

fn pediment(shading: &Shading) -> BuildingPixel {
    let hit = shading.hit;
    let conditions = shading.conditions;
    if hit.face == FACE_TOP {
        let is_west_slope = hit.slope_east > 0.0;
        let light_east = conditions.direction_to_light[0];
        let light = if is_west_slope { 0.5 + light_east * -0.9 } else { 0.5 + light_east * 0.9 };
        let mut copper = nearest_ramp_colour(&palette::COPPER[1..4], light.clamp(0.0, 1.0));
        let on_seam = ((hit.metres_south - PEDIMENT_NORTH_EDGE) / 0.85).fract() < 0.22;
        if on_seam {
            copper = palette::COPPER[0];
        }
        let on_ridge_cap = hit.metres_east.abs() < 0.22;
        if on_ridge_cap {
            copper = palette::COPPER[3];
        }
        copper = shading.snow_on_top(0.95).unwrap_or(copper);
        return floodlit(rain_darkened(copper, conditions.wetness * 0.5), 40);
    }
    if hit.face == FACE_SIDE {
        return plain(palette::MARBLE[2]);
    }
    let raking_edge_height = PEDIMENT_BASE_HEIGHT + PEDIMENT_RISE * (1.0 - hit.metres_east.abs() / PEDIMENT_HALF_WIDTH);
    let below_raking_edge = raking_edge_height - hit.metres_up;
    let row = ((hit.metres_up - PEDIMENT_BASE_HEIGHT) * shading.rows_per_metre).floor() as i32;
    let columns_from_axis = (shading.screen_x - QUAD_AXIS_SCREEN_X as i32).abs();
    let marble = if below_raking_edge < 0.42 {
        shading.snow_on_ledge().unwrap_or(palette::MARBLE[4])
    } else if below_raking_edge < 0.85 {
        palette::MARBLE[1]
    } else if row == 0 {
        palette::MARBLE[4]
    } else {
        let on_shield =
            (columns_from_axis <= 1 && (2..=5).contains(&row)) || (columns_from_axis == 2 && (3..=4).contains(&row));
        let on_swag = (3..=6).contains(&columns_from_axis) && row == 3 + ((columns_from_axis - 3) & 1)
            || (columns_from_axis == 7 && row == 4);
        if on_shield {
            palette::MARBLE[if columns_from_axis == 0 && row == 4 { 4 } else { 1 }]
        } else if on_swag {
            palette::MARBLE[2]
        } else {
            palette::MARBLE[3]
        }
    };
    floodlit(marble, 210)
}

fn arcade_roof(shading: &Shading, runs_north_south: bool, ridge: f32) -> BuildingPixel {
    let hit = shading.hit;
    let conditions = shading.conditions;
    let (from_ridge, along_roof) = if runs_north_south {
        (hit.metres_east - ridge, hit.metres_south)
    } else {
        (hit.metres_south - ridge, hit.metres_east)
    };
    let light = if runs_north_south {
        (0.5 - hit.slope_east.signum() * conditions.direction_to_light[0] * 1.2).clamp(0.0, 1.0)
    } else if hit.slope_south > 0.0 {
        0.35
    } else {
        0.8
    };
    let mut copper = nearest_ramp_colour(&palette::COPPER[1..4], light);
    let on_seam = (along_roof / 0.9).rem_euclid(1.0) < 0.2;
    if on_seam {
        copper = palette::COPPER[0];
    }
    if from_ridge.abs() < 0.2 {
        copper = palette::COPPER[3];
    }
    copper = shading.snow_on_top(1.0).unwrap_or(copper);
    plain(rain_darkened(copper, conditions.wetness * 0.5))
}

fn arcade_wall(shading: &Shading, first_bay_start: f32, bay_length: f32, runs_north_south: bool) -> BuildingPixel {
    let hit = shading.hit;
    let along_wall = if runs_north_south { hit.metres_south } else { hit.metres_east };
    let from_bay_centre = ((along_wall - first_bay_start).rem_euclid(bay_length) - 0.5 * bay_length).abs();
    let on_pier = from_bay_centre > 0.5 * bay_length - 0.42;
    let light = shading.wall_light(0.5);
    if hit.metres_up > 3.15 {
        return plain(palette::MARBLE[if light > 0.3 { 4 } else { 2 }]);
    }
    let on_impost = on_pier && (hit.metres_up - 2.0).abs() < 0.2;
    if on_impost {
        return plain(palette::MARBLE[3]);
    }
    let on_pier_base = on_pier && hit.metres_up < 0.35;
    if on_pier_base {
        return plain(palette::MARBLE[2]);
    }
    let tone = 1 + (light * 2.0).round() as i32;
    let brick_column = if runs_north_south { (hit.metres_south * 3.0) as i32 } else { shading.screen_x };
    shading.brick_wall(tone, brick_column, 0.7)
}

const HALL_WINDOW_STOREYS: [(f32, f32); 2] = [(1.75, 3.95), (5.3, 7.3)];

fn hall_wall(shading: &Shading, entrance_south: f32, is_entrance_pavilion: bool) -> BuildingPixel {
    let hit = shading.hit;
    let solid = shading.solid;
    let screen_x = shading.screen_x;
    let row = shading.row_above_ground;
    if hit.face == FACE_TOP {
        return plain(palette::MARBLE[3]);
    }
    let light = shading.wall_light(0.5);
    let tone = 1 + (light * 2.0).round() as i32;
    let bright_marble = palette::MARBLE[if light > 0.3 { 4 } else { 3 }];
    let dim_marble = palette::MARBLE[if light > 0.3 { 2 } else { 1 }];
    let eave_row = shading.row_at(HALL_EAVE_HEIGHT);
    if row >= eave_row - 1 {
        return plain(bright_marble);
    }
    if row == eave_row - 2 {
        let on_modillion = (screen_x + shading.screen_y) & 1 == 0;
        return plain(if on_modillion { dim_marble } else { palette::MARBLE[1] });
    }
    let water_table_row = shading.row_at(1.0);
    if row == water_table_row {
        return plain(dim_marble);
    }
    if row == shading.row_at(4.45) {
        return plain(bright_marble);
    }
    let on_darker_course = row < water_table_row || row == eave_row - 3;
    let brick_tone = tone - on_darker_course as i32;

    if hit.face == FACE_SOUTH {
        let (west, east) = (solid.west_edge, solid.east_edge);
        let centre = 0.5 * (west + east);
        let bay_count = (((east - west) - 1.4) / HALL_WINDOW_BAY).floor().max(1.0);
        let bay = ((hit.metres_east - centre) / HALL_WINDOW_BAY + bay_count * 0.5).floor();
        if bay >= 0.0 && bay < bay_count {
            let window_column = shading.screen_column_at(centre + (bay + 0.5 - bay_count * 0.5) * HALL_WINDOW_BAY);
            let columns_from_centre = screen_x - window_column;
            let half_width = ((1.3 * shading.pixels_per_metre) * 0.5).round() as i32;
            for (storey, (sill_height, head_height)) in HALL_WINDOW_STOREYS.iter().enumerate() {
                let (sill_row, head_row) = (shading.row_at(*sill_height), shading.row_at(*head_height));
                if columns_from_centre.abs() > half_width || row < sill_row - 1 || row > head_row {
                    continue;
                }
                if row == sill_row - 1 {
                    return plain(palette::MARBLE[4]);
                }
                if row == head_row {
                    return plain(if columns_from_centre == 0 { palette::MARBLE[4] } else { palette::BRICK[4] });
                }
                if columns_from_centre.abs() == half_width {
                    return plain(palette::MARBLE[4]);
                }
                let window_hash = hash_position(bay as i32, storey as i32, shading.solid_index as u32 * 31 + 5);
                return sash_window(columns_from_centre, row - sill_row, head_row - sill_row, half_width, window_hash);
            }
        }
        return shading.brick_wall(brick_tone, screen_x, 0.75);
    }

    let wall_face_east = if hit.metres_east < 0.0 { solid.east_edge } else { solid.west_edge };
    let bay_count = ((solid.south_edge - solid.north_edge - 1.4) / HALL_WINDOW_BAY).floor().max(1.0);
    let middle = 0.5 * (solid.north_edge + solid.south_edge);
    let bay = ((hit.metres_south - middle) / HALL_WINDOW_BAY + bay_count * 0.5).floor();
    let screen_column_of_wall_at =
        |metres_south: f32| (QUAD_AXIS_SCREEN_X + wall_face_east * pixels_per_metre_at(metres_south)).floor() as i32;
    if bay >= 0.0 && bay < bay_count {
        let bay_south = middle + (bay + 0.5 - bay_count * 0.5) * HALL_WINDOW_BAY;
        let window_column = screen_column_of_wall_at(bay_south);
        let door_surround_stands_here =
            is_entrance_pavilion && (bay_south - entrance_south).abs() < 0.5 * HALL_WINDOW_BAY;
        let bay_width_in_pixels = (screen_column_of_wall_at(bay_south + HALL_WINDOW_BAY) - window_column).abs();
        let bay_is_drawn = bay_width_in_pixels >= 3 || (bay as i32) & 1 == 0;
        if bay_is_drawn && screen_x == window_column {
            for (storey, (sill_height, head_height)) in HALL_WINDOW_STOREYS.iter().enumerate() {
                if door_surround_stands_here && storey == 0 {
                    continue;
                }
                let (sill_row, head_row) = (shading.row_at(*sill_height), shading.row_at(*head_height));
                if row >= sill_row && row < head_row {
                    let window_hash = hash_position(bay as i32, storey as i32 + 9, shading.solid_index as u32 * 31 + 5);
                    let mut window = sash_window(0, row - sill_row, head_row - sill_row, 1, window_hash);
                    window.colour = palette::GLASS[0];
                    return window;
                }
                if row == sill_row - 1 {
                    return plain(bright_marble);
                }
            }
        }
    }
    shading.brick_wall(brick_tone, (hit.metres_south * 2.2) as i32, 0.75)
}

const PURPLE_SLATE: Colour = rgb(86, 70, 84);

fn hall_roof(shading: &Shading) -> BuildingPixel {
    let hit = shading.hit;
    let solid = shading.solid;
    let conditions = shading.conditions;
    if hit.face != FACE_TOP {
        return plain(palette::MARBLE[if hit.face == FACE_SOUTH { 4 } else { 3 }]);
    }
    let from_side_eaves = (hit.metres_east - solid.west_edge).min(solid.east_edge - hit.metres_east);
    let from_end_eaves = (hit.metres_south - solid.north_edge).min(solid.south_edge - hit.metres_south);
    let from_eave = from_side_eaves.min(from_end_eaves);
    let on_flat_deck = from_eave * MANSARD_SLOPE >= MANSARD_RISE - 0.02;
    if on_flat_deck {
        let on_curb = (from_eave - MANSARD_RISE / MANSARD_SLOPE) < 0.4;
        let deck = match shading.snow_on_top(1.05) {
            Some(_) if on_curb => palette::SNOW[1],
            Some(snow) => snow,
            None if on_curb => palette::SLATE[4],
            None => shading.flat_roof_deck(conditions.seed ^ 0xDEC),
        };
        return plain(rain_darkened(deck, conditions.wetness * 0.5));
    }

    let tone = ((shading.top_light() * 4.6 - 0.6).round() as i32).clamp(0, 3);
    let above_eave = hit.metres_up - HALL_EAVE_HEIGHT;
    let on_slate_course = (above_eave / 0.92).fract() < 0.3;
    let tile_hash =
        hash_position((hit.metres_east * 2.0 + hit.metres_south * 2.0) as i32, (above_eave * 3.0) as i32, 311);
    let mut slate = palette::SLATE[tone as usize];
    if on_slate_course {
        slate = palette::SLATE[(tone - 1).max(0) as usize];
    } else if tile_hash.is_multiple_of(8) {
        slate = palette::SLATE[(tone + 1).min(4) as usize];
    } else if tile_hash % 13 == 1 {
        slate = blend_colours(palette::SLATE[tone as usize], PURPLE_SLATE, 0.5);
    }
    let on_hip = (from_side_eaves - from_end_eaves).abs() < 0.2;
    if on_hip {
        slate = palette::SLATE[4];
    }
    if from_eave < 0.32 {
        let gutter = match shading.snow_on_ledge() {
            Some(_) if conditions.snow_cover > 0.55 => palette::SNOW[0],
            Some(snow) => snow,
            None => palette::MARBLE[4],
        };
        return plain(gutter);
    }
    if let Some(snow) = shading.snow_on_top(0.9) {
        slate = if on_hip {
            palette::SNOW[3]
        } else if on_slate_course {
            blend_colours(snow, palette::SNOW[0], 0.45)
        } else {
            snow
        };
    } else if conditions.snow_cover > 0.15
        && conditions.snow_cover * 0.8 > ordered_dither_threshold(shading.screen_x, shading.screen_y)
    {
        slate = blend_colours(slate, palette::SNOW[1], 0.62);
    }
    plain(rain_darkened(slate, conditions.wetness * 0.55))
}

fn hall_dormer(shading: &Shading, facing: DormerFacing, centre: f32) -> BuildingPixel {
    let hit = shading.hit;
    let solid = shading.solid;
    let above_eave = hit.metres_up - HALL_EAVE_HEIGHT;
    let faces_south_or_north = matches!(facing, DormerFacing::South | DormerFacing::North);
    if hit.face == FACE_TOP {
        let light = shading.top_light();
        let roof = if faces_south_or_north {
            let from_front_arch = if facing == DormerFacing::South {
                solid.south_edge - hit.metres_south
            } else {
                hit.metres_south - solid.north_edge
            };
            if from_front_arch < 0.3 {
                palette::MARBLE[4]
            } else {
                palette::SLATE[((light * 4.2).round() as usize).clamp(2, 4)]
            }
        } else {
            palette::SLATE[if light > 0.7 { 3 } else { 2 }]
        };
        return plain(shading.snow_on_top(1.0).unwrap_or(roof));
    }
    if facing == DormerFacing::South && hit.face == FACE_SOUTH {
        let columns_from_centre = shading.screen_x - shading.screen_column_at(centre);
        let row = ((above_eave - 1.1) * shading.rows_per_metre).floor() as i32;
        if columns_from_centre.abs() <= 1 && (0..=3).contains(&row) && above_eave < 2.75 {
            let mut window =
                sash_window(columns_from_centre, row, 4, 2, hash_position(shading.solid_index as i32, 3, 17));
            if row == 3 && columns_from_centre != 0 {
                window.colour = palette::MARBLE[4];
            }
            return window;
        }
        return plain(palette::MARBLE[4]);
    }
    let front_is_seen =
        !faces_south_or_north && hit.face == FACE_SIDE && (facing == DormerFacing::East) == (hit.metres_east < 0.0);
    if front_is_seen {
        let in_window = (hit.metres_south - centre).abs() < 0.42 && above_eave > 1.15 && above_eave < 2.7;
        if in_window {
            let mut window = sash_window(0, 0, 2, 1, hash_position(shading.solid_index as i32, 4, 17));
            window.colour = palette::GLASS[0];
            return window;
        }
        return plain(palette::MARBLE[if side_wall_light(shading.conditions, hit.metres_east) > 0.3 { 4 } else { 3 }]);
    }
    plain(palette::SLATE[if hit.face == FACE_SOUTH { 2 } else { 1 }])
}

fn entrance_gable(shading: &Shading, ridge_south: f32) -> BuildingPixel {
    let hit = shading.hit;
    let conditions = shading.conditions;
    let from_ridge = (hit.metres_south - ridge_south).abs();
    if hit.face == FACE_TOP {
        let tone = ((shading.top_light() * 4.6 - 0.6).round() as i32).clamp(0, 3);
        let mut slate = palette::SLATE[tone as usize];
        if ((hit.metres_up - HALL_EAVE_HEIGHT) / 0.92).fract() < 0.3 {
            slate = palette::SLATE[(tone - 1).max(0) as usize];
        }
        if from_ridge < 0.22 {
            slate = palette::SLATE[4];
        }
        if from_ridge > 3.7 {
            slate = palette::MARBLE[4];
        }
        slate = shading.snow_on_top(0.9).unwrap_or(slate);
        return plain(rain_darkened(slate, conditions.wetness * 0.55));
    }
    if hit.face == FACE_SOUTH {
        return plain(palette::MARBLE[4]);
    }
    let raking_edge_height = HALL_EAVE_HEIGHT + ENTRANCE_GABLE_RISE * (1.0 - from_ridge / ENTRANCE_GABLE_HALF_WIDTH);
    let light = side_wall_light(conditions, hit.metres_east);
    if raking_edge_height - hit.metres_up < 0.5 || hit.metres_up < HALL_EAVE_HEIGHT + 0.25 {
        return plain(palette::MARBLE[if light > 0.3 { 4 } else { 3 }]);
    }
    let from_round_window_centre = (hit.metres_up - (HALL_EAVE_HEIGHT + 1.15)).abs();
    if from_ridge < 0.55 && from_round_window_centre < 0.5 {
        return glowing(
            palette::GLASS[1],
            palette::LIT_WINDOW,
            window_group_among(hash_position(shading.seed as i32, 1, 9), ENTRANCE_GABLE_WINDOW_GROUP_COUNT),
        );
    }
    if from_ridge < 0.95 && from_round_window_centre < 0.8 {
        return plain(palette::MARBLE[3]);
    }
    let tone = 1 + (light * 2.0).round() as i32;
    let brick = brick_colour(
        &palette::BRICK,
        tone,
        (hit.metres_south * 2.2) as i32,
        (hit.metres_up * 3.0) as i32,
        shading.seed,
    );
    plain(rain_darkened(brick, conditions.wetness * 0.7))
}

fn biddle_music_building(shading: &Shading) -> BuildingPixel {
    let hit = shading.hit;
    let conditions = shading.conditions;
    let row = shading.row_above_ground;
    let roof_row = shading.row_at(BIDDLE_ROOF_HEIGHT);
    if hit.face == FACE_TOP {
        let from_edge = shading.distance_from_roof_edge();
        let mottling = fractal_noise(hit.metres_east * 0.15, hit.metres_south * 0.15, 2, 88);
        let on_roof_joint = ((hit.metres_south + 60.0) / 6.0).rem_euclid(1.0) < 0.05;
        let roof = if from_edge < 0.7 {
            palette::BIDDLE_STONE[3]
        } else if on_roof_joint {
            palette::BIDDLE_STONE[0]
        } else {
            dithered_ramp_colour(&palette::BIDDLE_STONE[1..3], 0.3 + mottling * 0.6, shading.screen_x, shading.screen_y)
        };
        return plain(rain_darkened(shading.snow_on_top(1.05).unwrap_or(roof), conditions.wetness * 0.4));
    }
    let light = shading.wall_light(0.6);
    let pier_stone = palette::BIDDLE_STONE[if light > 0.3 { 3 } else { 2 }];
    if row >= roof_row - 2 {
        return plain(pier_stone);
    }
    let along_wall =
        if hit.face == FACE_SOUTH { hit.metres_east * shading.pixels_per_metre } else { hit.metres_south * 1.2 };
    let bay = (along_wall / 2.0).floor() as i32;
    if bay % 3 == 0 {
        return plain(pier_stone);
    }
    if row <= 0 {
        return plain(palette::BIDDLE_STONE[0]);
    }
    if row >= roof_row - 4 {
        return plain(palette::BIDDLE_STONE[1]);
    }
    let glazing = blend_colours(palette::GLASS[1], palette::BIDDLE_STONE[0], 0.45);
    let bay_hash = hash_position(bay, 77, 3);
    if bay_hash.is_multiple_of(4) && row < roof_row - 5 {
        return glowing(
            glazing,
            blend_colours(palette::LIT_WINDOW, palette::BIDDLE_STONE[0], 0.35),
            window_group_among(bay_hash, BIDDLE_WINDOW_GROUP_COUNT),
        );
    }
    plain(glazing)
}

const WHITE_MEMBRANE: Colour = rgb(234, 234, 230);
const STAINED_WHITE_MEMBRANE: Colour = rgb(214, 216, 216);
const WHITE_MEMBRANE_SEAM: Colour = rgb(200, 202, 206);

fn library_wall(shading: &Shading, is_union: bool, is_low_pavilion: bool) -> BuildingPixel {
    let hit = shading.hit;
    let solid = shading.solid;
    let conditions = shading.conditions;
    let screen_x = shading.screen_x;
    let row = shading.row_above_ground;
    if hit.face == FACE_TOP {
        let from_edge = shading.distance_from_roof_edge();
        let on_parapet = from_edge < 0.6;
        let roof = if on_parapet {
            palette::MARBLE[4]
        } else if from_edge < 1.0 {
            palette::FLAT_ROOF_DECK[0]
        } else if is_union {
            let staining = fractal_noise(hit.metres_east * 0.12, hit.metres_south * 0.05, 2, shading.seed);
            let on_seam = ((hit.metres_east + 200.0) / 5.0).rem_euclid(1.0) < 0.05;
            if on_seam {
                WHITE_MEMBRANE_SEAM
            } else if staining > 0.56 {
                STAINED_WHITE_MEMBRANE
            } else {
                WHITE_MEMBRANE
            }
        } else {
            shading.flat_roof_deck(shading.seed)
        };
        if on_parapet {
            return plain(shading.snow_on_ledge().unwrap_or(roof));
        }
        return plain(rain_darkened(shading.snow_on_top(1.05).unwrap_or(roof), conditions.wetness * 0.4));
    }

    let light = shading.wall_light(0.5);
    let tone = 1 + (light * 2.0).round() as i32;
    let bright_marble = palette::MARBLE[if light > 0.3 { 4 } else { 3 }];
    let dim_marble = palette::MARBLE[if light > 0.3 { 2 } else { 1 }];
    let roof_row = shading.row_at(solid.top_height);
    if row >= roof_row - 2 {
        return plain(bright_marble);
    }
    if row == roof_row - 3 {
        return plain(if screen_x & 1 == 0 { dim_marble } else { palette::MARBLE[1] });
    }
    if row <= shading.row_at(1.3) {
        return plain(rain_darkened(dim_marble, conditions.wetness * 0.5));
    }
    let (sill_height, head_height) = if is_low_pavilion { (2.6f32, 6.8f32) } else { (3.2f32, 9.4f32) };
    let (sill_row, head_row) = (shading.row_at(sill_height), shading.row_at(head_height));

    if hit.face == FACE_SOUTH {
        let (west, east) = (solid.west_edge, solid.east_edge);
        let pixels_from_corner = ((hit.metres_east - west) * shading.pixels_per_metre)
            .min((east - hit.metres_east) * shading.pixels_per_metre);
        let on_quoin = pixels_from_corner < 3.0 && (row / 2) & 1 == 0;
        if on_quoin {
            return plain(bright_marble);
        }
        let bay_length = 5.2;
        let centre = 0.5 * (west + east);
        let bay_count = (((east - west) - 3.0) / bay_length).floor().max(1.0);
        let bay = ((hit.metres_east - centre) / bay_length + bay_count * 0.5).floor();
        if bay >= 0.0 && bay < bay_count {
            let window_column = shading.screen_column_at(centre + (bay + 0.5 - bay_count * 0.5) * bay_length);
            let columns_from_centre = screen_x - window_column;
            let half_width = (1.05 * shading.pixels_per_metre).round() as i32;
            if columns_from_centre.abs() <= half_width && row >= sill_row - 1 && row <= head_row {
                let rows_below_head = head_row - row;
                let arch_half_width = match rows_below_head {
                    0 => half_width - 2,
                    1 => half_width - 1,
                    _ => half_width,
                };
                if columns_from_centre.abs() > arch_half_width {
                    return shading.brick_wall(tone, screen_x, 0.75);
                }
                if row == sill_row - 1 || columns_from_centre.abs() == arch_half_width || rows_below_head == 0 {
                    return plain(palette::MARBLE[4]);
                }
                let window_hash = hash_position(bay as i32, 3, shading.solid_index as u32 * 31 + 5);
                let mut window =
                    sash_window(columns_from_centre, row - sill_row, head_row - sill_row, half_width, window_hash);
                window.glow_group = window_group(shading.solid_index as u32 * 7 + (bay as u32 & 1));
                return window;
            }
        }
        if row == shading.row_at(head_height + 1.1) {
            return plain(dim_marble);
        }
        return shading.brick_wall(tone, screen_x, 0.75);
    }

    let bays_along = (hit.metres_south - solid.north_edge) / 4.4;
    let from_bay_centre = (bays_along - bays_along.floor() - 0.5).abs();
    if from_bay_centre < 0.16 && row >= sill_row && row < head_row {
        return glowing(palette::GLASS[0], palette::LIT_WINDOW, window_group(shading.solid_index as u32 * 7));
    }
    shading.brick_wall(tone, (hit.metres_south * 2.2) as i32, 0.75)
}

const WHITE_CLAPBOARD: [Colour; 3] = [rgb(170, 174, 188), rgb(214, 216, 216), rgb(240, 240, 234)];
const CREAM_SIDING: [Colour; 3] = [rgb(160, 150, 112), rgb(206, 194, 142), rgb(232, 222, 170)];
const BLUE_GREY_SIDING: [Colour; 3] = [rgb(98, 112, 128), rgb(132, 150, 164), rgb(164, 182, 192)];
const TAN_SIDING: [Colour; 3] = [rgb(120, 104, 84), rgb(160, 140, 110), rgb(190, 172, 138)];
const PAINTED_BRICK: [Colour; 3] = [rgb(104, 50, 46), rgb(140, 64, 50), rgb(168, 84, 60)];
const SAGE_SIDING: [Colour; 3] = [rgb(96, 116, 92), rgb(128, 150, 118), rgb(158, 178, 146)];
const YELLOW_SIDING: [Colour; 3] = [rgb(150, 132, 60), rgb(196, 176, 84), rgb(224, 206, 118)];
const TRINITY_PARK_WALLS: [[Colour; 3]; 7] =
    [WHITE_CLAPBOARD, CREAM_SIDING, BLUE_GREY_SIDING, TAN_SIDING, PAINTED_BRICK, SAGE_SIDING, YELLOW_SIDING];

const CHARCOAL_SHINGLE: [Colour; 3] = [rgb(54, 58, 72), rgb(74, 80, 94), rgb(98, 106, 118)];
const BROWN_SHINGLE: [Colour; 3] = [rgb(72, 56, 52), rgb(98, 76, 66), rgb(126, 100, 84)];
const GREY_SHINGLE: [Colour; 3] = [rgb(84, 88, 96), rgb(112, 116, 122), rgb(140, 144, 146)];
const GREEN_SHINGLE: [Colour; 3] = [rgb(44, 70, 62), rgb(58, 92, 78), rgb(80, 116, 96)];
const RED_SHINGLE: [Colour; 3] = [rgb(92, 48, 44), rgb(122, 62, 52), rgb(150, 82, 64)];
const TRINITY_PARK_ROOFS: [[Colour; 3]; 5] =
    [CHARCOAL_SHINGLE, BROWN_SHINGLE, GREY_SHINGLE, GREEN_SHINGLE, RED_SHINGLE];

fn house(shading: &Shading, style: HouseStyle, ridge_runs_east_west: bool, wall_height: f32) -> BuildingPixel {
    let hit = shading.hit;
    let solid = shading.solid;
    let conditions = shading.conditions;
    let screen_x = shading.screen_x;
    let row = shading.row_above_ground;
    let house_hash = hash_position(shading.solid_index as i32, 9, 0x40A5E);
    let (walls, roof) = match style {
        HouseStyle::BishopsHouse => (CREAM_SIDING, GREY_SHINGLE),
        HouseStyle::BrickInstitutional => (PAINTED_BRICK, GREY_SHINGLE),
        HouseStyle::TheArk => (WHITE_CLAPBOARD, GREY_SHINGLE),
        HouseStyle::TrinityParkHouse => {
            (TRINITY_PARK_WALLS[(house_hash % 7) as usize], TRINITY_PARK_ROOFS[((house_hash >> 5) % 5) as usize])
        }
    };
    if hit.face == FACE_TOP {
        let light = shading.top_light();
        let mut shingle = roof[if light > 0.78 {
            2
        } else if light > 0.52 {
            1
        } else {
            0
        }];
        if hash_position(screen_x >> 1, shading.screen_y, house_hash).is_multiple_of(7) {
            shingle = roof[1];
        }
        let from_ridge = if ridge_runs_east_west {
            (hit.metres_south - 0.5 * (solid.north_edge + solid.south_edge)).abs()
        } else {
            (hit.metres_east - 0.5 * (solid.west_edge + solid.east_edge)).abs()
        };
        if from_ridge < 0.3 {
            shingle = roof[2];
        }
        return plain(rain_darkened(shading.snow_on_top(0.95).unwrap_or(shingle), conditions.wetness * 0.5));
    }

    let is_south = hit.face == FACE_SOUTH;
    let light = shading.wall_light(0.5);
    let mut wall = walls[if light > 0.66 {
        2
    } else if light > 0.3 {
        1
    } else {
        0
    }];
    let eave_row = shading.row_at(wall_height);
    let on_eaves = row == eave_row - 1 || row == eave_row;
    if on_eaves {
        wall = palette::MARBLE[if is_south { 4 } else { 3 }];
    }
    if is_south && hit.metres_up < wall_height - 0.4 {
        let bay_length = 3.6;
        let centre = 0.5 * (solid.west_edge + solid.east_edge);
        let bay_count = (((solid.east_edge - solid.west_edge) - 1.6) / bay_length).floor().max(1.0);
        let bay = ((hit.metres_east - centre) / bay_length + bay_count * 0.5).floor();
        if bay >= 0.0 && bay < bay_count {
            let columns_from_centre =
                screen_x - shading.screen_column_at(centre + (bay + 0.5 - bay_count * 0.5) * bay_length);
            let storeys: &[(f32, f32)] = if wall_height > 5.6 { &[(1.1, 2.6), (3.9, 5.3)] } else { &[(1.1, 2.6)] };
            for (storey, (sill_height, head_height)) in storeys.iter().enumerate() {
                if columns_from_centre.abs() <= 1
                    && row >= shading.row_at(*sill_height)
                    && row < shading.row_at(*head_height)
                {
                    if columns_from_centre == 1 {
                        return plain(palette::MARBLE[4]);
                    }
                    let glass = palette::GLASS[if columns_from_centre == 0 { 0 } else { 1 }];
                    return glowing(
                        glass,
                        palette::LIT_WINDOW,
                        window_group(hash_position(bay as i32, storey as i32, house_hash)),
                    );
                }
            }
        }
        let has_porch_roof = style == HouseStyle::TrinityParkHouse && (house_hash >> 9) & 1 == 0;
        if has_porch_roof && row == shading.row_at(2.9) {
            wall = palette::MARBLE[3];
        }
        if row == 0 {
            wall = walls[0];
        }
    }
    if style == HouseStyle::BrickInstitutional && is_south && row > 0 {
        wall = rain_darkened(brick_colour(&palette::BRICK, 2, screen_x, row, house_hash), conditions.wetness * 0.7);
        if on_eaves {
            wall = palette::MARBLE[4];
        }
    }
    plain(wall)
}

const GREEN_TILE: [Colour; 4] = [rgb(30, 70, 60), rgb(40, 92, 70), rgb(56, 114, 84), rgb(92, 146, 108)];
const VALLEY_FLASHING: Colour = rgb(150, 168, 150);
const GREEN_HALL_WINDOW_STOREYS: [(f32, f32); 3] = [(1.7, 3.6), (5.0, 7.0), (8.2, 10.0)];

fn green_hall_roof(shading: &Shading, north_end: f32) -> BuildingPixel {
    let hit = shading.hit;
    let solid = shading.solid;
    if hit.face != FACE_TOP {
        return plain(palette::MARBLE[if hit.face == FACE_SOUTH { 4 } else { 3 }]);
    }
    let along_bay = ((hit.metres_south - (north_end - GREEN_ROOF_OVERHANG)) / GREEN_ROOF_BAY_PITCH).rem_euclid(1.0)
        * GREEN_ROOF_BAY_PITCH;
    let from_side_eaves = (hit.metres_east - solid.west_edge).min(solid.east_edge - hit.metres_east);
    let from_valley = along_bay.min(GREEN_ROOF_BAY_PITCH - along_bay);
    let from_edge = from_side_eaves.min(from_valley);
    let mut tile = GREEN_TILE[((shading.top_light() * 3.4 - 0.5).round() as usize).clamp(0, 2)];
    let on_standing_seam = if from_side_eaves < from_valley {
        (hit.metres_south / 1.1).fract().abs() < 0.2
    } else {
        ((hit.metres_east + 200.0) / 1.1).fract() < 0.2
    };
    if on_standing_seam {
        tile = GREEN_TILE[0];
    }
    let on_hip_or_cap = (from_side_eaves - from_valley).abs() < 0.3 || from_edge * 0.6 >= 4.1;
    if on_hip_or_cap {
        tile = GREEN_TILE[3];
    }
    if from_valley < 0.3 && from_side_eaves > 0.6 {
        tile = VALLEY_FLASHING;
    }
    if from_side_eaves < 0.38 {
        tile = palette::MARBLE[4];
    }
    plain(rain_darkened(shading.snow_on_top(0.95).unwrap_or(tile), shading.conditions.wetness * 0.5))
}

fn green_hall_wall(shading: &Shading) -> BuildingPixel {
    let hit = shading.hit;
    let solid = shading.solid;
    let row = shading.row_above_ground;
    if hit.face == FACE_TOP {
        return plain(palette::MARBLE[3]);
    }
    let is_south = hit.face == FACE_SOUTH;
    let light = shading.wall_light(0.5);
    let tone = 1 + (light * 2.0).round() as i32;
    let marble = palette::MARBLE[if light > 0.3 { 4 } else { 3 }];
    if row >= shading.row_at(GREEN_HALL_EAVE_HEIGHT) - 2 || row == shading.row_at(4.3) || row == shading.row_at(1.0) {
        return plain(marble);
    }
    let bays_along =
        if is_south { (hit.metres_east - solid.west_edge) / 3.3 } else { (hit.metres_south - solid.north_edge) / 3.3 };
    let from_bay_centre = (bays_along.fract() - 0.5).abs();
    let in_window_bay = from_bay_centre < if is_south { 0.2 } else { 0.16 };
    for (storey, (sill_height, head_height)) in GREEN_HALL_WINDOW_STOREYS.iter().enumerate() {
        if in_window_bay && hit.metres_up > *sill_height && hit.metres_up < *head_height {
            let glass = palette::GLASS[if hit.metres_up > 0.5 * (sill_height + head_height) { 0 } else { 1 }];
            return glowing(
                glass,
                palette::LIT_WINDOW,
                window_group(hash_position(bays_along as i32, storey as i32, shading.seed)),
            );
        }
    }
    let brick_column = if is_south { shading.screen_x } else { (hit.metres_south * 2.0) as i32 };
    shading.brick_wall(tone, brick_column, 0.7)
}
