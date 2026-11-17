use crate::east::buildings::Material;
use crate::east::projection::*;

pub const CANVAS_WIDTH: usize = 640;
pub const CANVAS_HEIGHT: usize = 360;

pub const FACE_TOP: u8 = 0;
pub const FACE_SOUTH: u8 = 1;
pub const FACE_SIDE: u8 = 2;

pub const BARE_GROUND: u16 = u16::MAX;

const EXTENT_TOLERANCE_METRES: f32 = 1e-3;
const TOP_FACE_TOLERANCE_METRES: f32 = 0.035;
const SOUTH_FACE_PROBE_METRES: f32 = 0.03;
const SLOPE_PROBE_METRES: f32 = 0.06;
const STEEPEST_SLOPE: f32 = 4.0;
const TOP_ENTRY_BISECTIONS: usize = 22;
const SURFACE_BISECTIONS: usize = 12;

pub type VerticalExtentAt = Box<dyn Fn(f32, f32) -> (f32, f32) + Send + Sync>;

pub struct Solid {
    pub material: Material,
    pub west_edge: f32,
    pub east_edge: f32,
    pub north_edge: f32,
    pub south_edge: f32,
    pub top_height: f32,
    pub march_step_metres: f32,
    pub casts_shadow: bool,
    pub vertical_extent_at: VerticalExtentAt,
}

#[derive(Copy, Clone)]
pub struct SurfaceHit {
    pub solid_index: u16,
    pub face: u8,
    pub ground_surface: u8,
    pub metres_east: f32,
    pub metres_south: f32,
    pub metres_up: f32,
    pub slope_east: f32,
    pub slope_south: f32,
}

impl SurfaceHit {
    #[inline]
    pub fn is_bare_ground(&self) -> bool {
        self.solid_index == BARE_GROUND
    }
}

pub fn bare_ground_hits() -> Vec<SurfaceHit> {
    let mut hits = Vec::with_capacity(CANVAS_WIDTH * CANVAS_HEIGHT);
    for screen_y in 0..CANVAS_HEIGHT {
        for screen_x in 0..CANVAS_WIDTH {
            let (metres_east, metres_south) = ground_point_at_screen(screen_x as f32 + 0.5, screen_y as f32 + 0.5);
            hits.push(SurfaceHit {
                solid_index: BARE_GROUND,
                face: FACE_TOP,
                ground_surface: 0,
                metres_east,
                metres_south,
                metres_up: 0.0,
                slope_east: 0.0,
                slope_south: 0.0,
            });
        }
    }
    hits
}

#[inline]
fn extent_contains(extent: (f32, f32), metres_up: f32) -> bool {
    let (bottom, top) = extent;
    top > bottom && metres_up >= bottom - EXTENT_TOLERANCE_METRES && metres_up <= top + EXTENT_TOLERANCE_METRES
}

#[inline]
fn ray_height_at(row_centre_y: f32, metres_south: f32) -> f32 {
    (ground_screen_y(metres_south) - row_centre_y) / screen_rows_per_metre_of_height_at(metres_south)
}

pub fn march_solid_into_hits(solid_index: usize, solid: &Solid, hits: &mut [SurfaceHit]) {
    let scale_at_north_edge = pixels_per_metre_at(solid.north_edge);
    let scale_at_south_edge = pixels_per_metre_at(solid.south_edge);
    let corner_screen_xs = [
        QUAD_AXIS_SCREEN_X + solid.west_edge * scale_at_north_edge,
        QUAD_AXIS_SCREEN_X + solid.east_edge * scale_at_north_edge,
        QUAD_AXIS_SCREEN_X + solid.west_edge * scale_at_south_edge,
        QUAD_AXIS_SCREEN_X + solid.east_edge * scale_at_south_edge,
    ];
    let mut leftmost = f32::MAX;
    let mut rightmost = f32::MIN;
    for corner_screen_x in corner_screen_xs {
        leftmost = leftmost.min(corner_screen_x);
        rightmost = rightmost.max(corner_screen_x);
    }
    let first_column = leftmost.floor().max(0.0) as i32;
    let last_column = rightmost.ceil().min(CANVAS_WIDTH as f32 - 1.0) as i32;
    let top_row_at_north_edge =
        ground_screen_y(solid.north_edge) - solid.top_height * screen_rows_per_metre_of_height_at(solid.north_edge);
    let top_row_at_south_edge =
        ground_screen_y(solid.south_edge) - solid.top_height * screen_rows_per_metre_of_height_at(solid.south_edge);
    let first_row = top_row_at_north_edge.min(top_row_at_south_edge).floor().max(0.0) as i32;
    let last_row = ground_screen_y(solid.south_edge).ceil().min(CANVAS_HEIGHT as f32 - 1.0) as i32;
    if first_column > last_column || first_row > last_row {
        return;
    }

    for screen_y in first_row..=last_row {
        let row_centre_y = screen_y as f32 + 0.5;
        let farthest_south = metres_south_at_screen_row(row_centre_y).max(solid.north_edge);
        let mut nearest_south = solid.south_edge;
        let ray_enters_through_top = ray_height_at(row_centre_y, solid.south_edge) > solid.top_height;
        if ray_enters_through_top {
            if ray_height_at(row_centre_y, farthest_south) > solid.top_height {
                continue;
            }
            let mut below_top = farthest_south;
            let mut above_top = solid.south_edge;
            for _ in 0..TOP_ENTRY_BISECTIONS {
                let middle = 0.5 * (below_top + above_top);
                if ray_height_at(row_centre_y, middle) > solid.top_height {
                    above_top = middle;
                } else {
                    below_top = middle;
                }
            }
            nearest_south = below_top;
        }
        if nearest_south <= farthest_south {
            continue;
        }
        let scale_at_nearest = pixels_per_metre_at(nearest_south);
        let scale_at_farthest = pixels_per_metre_at(farthest_south);

        for screen_x in first_column..=last_column {
            let pixels_east_of_axis = screen_x as f32 + 0.5 - QUAD_AXIS_SCREEN_X;
            let east_at_nearest = pixels_east_of_axis / scale_at_nearest;
            let east_at_farthest = pixels_east_of_axis / scale_at_farthest;
            if east_at_nearest.max(east_at_farthest) < solid.west_edge
                || east_at_nearest.min(east_at_farthest) > solid.east_edge
            {
                continue;
            }
            let hit_index = screen_y as usize * CANVAS_WIDTH + screen_x as usize;
            let already_hit_a_solid = !hits[hit_index].is_bare_ground();
            let nearest_so_far = hits[hit_index].metres_south;
            if already_hit_a_solid && nearest_so_far >= nearest_south {
                continue;
            }
            let ray_point_at = |metres_south: f32| -> (f32, f32) {
                (pixels_east_of_axis / pixels_per_metre_at(metres_south), ray_height_at(row_centre_y, metres_south))
            };
            let ray_is_inside_at = |metres_south: f32| -> bool {
                let (metres_east, metres_up) = ray_point_at(metres_south);
                metres_east >= solid.west_edge
                    && metres_east <= solid.east_edge
                    && extent_contains((solid.vertical_extent_at)(metres_east, metres_south), metres_up)
            };

            let mut probe_south = nearest_south;
            let mut previous_probe_south = nearest_south;
            let mut probing_entry_point = true;
            let mut inside_at_entry_point = false;
            let mut crossing: Option<(f32, f32)> = None;
            loop {
                if ray_is_inside_at(probe_south) {
                    inside_at_entry_point = probing_entry_point;
                    crossing = Some((probe_south, previous_probe_south));
                    break;
                }
                probing_entry_point = false;
                if probe_south <= farthest_south {
                    break;
                }
                previous_probe_south = probe_south;
                probe_south = (probe_south - solid.march_step_metres).max(farthest_south);
            }
            let Some((mut inside_south, mut outside_south)) = crossing else { continue };

            let face;
            if inside_at_entry_point {
                face = if ray_enters_through_top { FACE_TOP } else { FACE_SOUTH };
            } else {
                for _ in 0..SURFACE_BISECTIONS {
                    let middle = 0.5 * (inside_south + outside_south);
                    if ray_is_inside_at(middle) {
                        inside_south = middle;
                    } else {
                        outside_south = middle;
                    }
                }
                let (metres_east, metres_up) = ray_point_at(inside_south);
                let (_, top_here) = (solid.vertical_extent_at)(metres_east, inside_south);
                if top_here - metres_up < TOP_FACE_TOLERANCE_METRES {
                    face = FACE_TOP;
                } else {
                    let just_south = inside_south + SOUTH_FACE_PROBE_METRES;
                    let open_to_the_south = just_south > solid.south_edge
                        || !extent_contains((solid.vertical_extent_at)(metres_east, just_south), metres_up);
                    face = if open_to_the_south { FACE_SOUTH } else { FACE_SIDE };
                }
            }
            if already_hit_a_solid && inside_south <= nearest_so_far {
                continue;
            }

            let (metres_east, metres_up) = ray_point_at(inside_south);
            let mut slope_east = 0.0;
            let mut slope_south = 0.0;
            if face == FACE_TOP {
                let top_height_at = |east: f32, south: f32| -> Option<f32> {
                    let (bottom, top) = (solid.vertical_extent_at)(east, south);
                    if top > bottom {
                        Some(top)
                    } else {
                        None
                    }
                };
                let west_height = top_height_at(metres_east - SLOPE_PROBE_METRES, inside_south);
                let east_height = top_height_at(metres_east + SLOPE_PROBE_METRES, inside_south);
                let north_height = top_height_at(metres_east, inside_south - SLOPE_PROBE_METRES);
                let south_height = top_height_at(metres_east, inside_south + SLOPE_PROBE_METRES);
                if let (Some(west_height), Some(east_height)) = (west_height, east_height) {
                    slope_east = ((east_height - west_height) / (2.0 * SLOPE_PROBE_METRES))
                        .clamp(-STEEPEST_SLOPE, STEEPEST_SLOPE);
                }
                if let (Some(north_height), Some(south_height)) = (north_height, south_height) {
                    slope_south = ((south_height - north_height) / (2.0 * SLOPE_PROBE_METRES))
                        .clamp(-STEEPEST_SLOPE, STEEPEST_SLOPE);
                }
            }
            hits[hit_index] = SurfaceHit {
                solid_index: solid_index as u16,
                face,
                ground_surface: 0,
                metres_east,
                metres_south: inside_south,
                metres_up,
                slope_east,
                slope_south,
            };
        }
    }
}

const SHADOW_MAP_WEST_EDGE: f32 = -150.0;
const SHADOW_MAP_EAST_EDGE: f32 = 150.0;
const SHADOW_MAP_NORTH_EDGE: f32 = -60.0;
const SHADOW_MAP_SOUTH_EDGE: f32 = 408.0;
const SHADOW_MAP_CELL_METRES: f32 = 0.5;
const NO_SHADOW_CEILING: f32 = -1.0e6;
const SUN_OVERHEAD_TOLERANCE: f32 = 1e-4;

pub struct HeightMap {
    pub columns: usize,
    pub rows: usize,
    pub heights: Vec<f32>,
}

impl HeightMap {
    pub fn of_shadow_casters(solids: &[Solid]) -> HeightMap {
        let columns = ((SHADOW_MAP_EAST_EDGE - SHADOW_MAP_WEST_EDGE) / SHADOW_MAP_CELL_METRES) as usize;
        let rows = ((SHADOW_MAP_SOUTH_EDGE - SHADOW_MAP_NORTH_EDGE) / SHADOW_MAP_CELL_METRES) as usize;
        let mut heights = vec![0.0f32; columns * rows];
        for solid in solids {
            if !solid.casts_shadow {
                continue;
            }
            let outside_the_map = solid.east_edge < SHADOW_MAP_WEST_EDGE
                || solid.west_edge > SHADOW_MAP_EAST_EDGE
                || solid.south_edge < SHADOW_MAP_NORTH_EDGE
                || solid.north_edge > SHADOW_MAP_SOUTH_EDGE;
            if outside_the_map {
                continue;
            }
            let first_column =
                ((solid.west_edge - SHADOW_MAP_WEST_EDGE) / SHADOW_MAP_CELL_METRES).floor().max(0.0) as usize;
            let last_column = ((solid.east_edge - SHADOW_MAP_WEST_EDGE) / SHADOW_MAP_CELL_METRES)
                .ceil()
                .min(columns as f32 - 1.0)
                .max(0.0) as usize;
            let first_row =
                ((solid.north_edge - SHADOW_MAP_NORTH_EDGE) / SHADOW_MAP_CELL_METRES).floor().max(0.0) as usize;
            let last_row = ((solid.south_edge - SHADOW_MAP_NORTH_EDGE) / SHADOW_MAP_CELL_METRES)
                .ceil()
                .min(rows as f32 - 1.0)
                .max(0.0) as usize;
            for row in first_row..=last_row {
                for column in first_column..=last_column {
                    let metres_east = SHADOW_MAP_WEST_EDGE + (column as f32 + 0.5) * SHADOW_MAP_CELL_METRES;
                    let metres_south = SHADOW_MAP_NORTH_EDGE + (row as f32 + 0.5) * SHADOW_MAP_CELL_METRES;
                    if metres_east < solid.west_edge
                        || metres_east > solid.east_edge
                        || metres_south < solid.north_edge
                        || metres_south > solid.south_edge
                    {
                        continue;
                    }
                    let (bottom, top) = (solid.vertical_extent_at)(metres_east, metres_south);
                    let cell = &mut heights[row * columns + column];
                    if top > bottom && top > *cell {
                        *cell = top;
                    }
                }
            }
        }
        HeightMap { columns, rows, heights }
    }

    pub fn shadow_ceilings(
        &self,
        metres_east_per_metre_of_sun_rise: f32,
        metres_south_per_metre_of_sun_rise: f32,
    ) -> Vec<f32> {
        let (columns, rows) = (self.columns, self.rows);
        let east_run = metres_east_per_metre_of_sun_rise;
        let south_run = metres_south_per_metre_of_sun_rise;
        let mut ceilings = vec![NO_SHADOW_CEILING; columns * rows];
        if east_run.abs() < SUN_OVERHEAD_TOLERANCE && south_run.abs() < SUN_OVERHEAD_TOLERANCE {
            return ceilings;
        }
        if east_run.abs() >= south_run.abs() {
            let sun_rise_per_cell = SHADOW_MAP_CELL_METRES / east_run.abs();
            let rows_per_column = south_run / east_run.abs();
            let sun_is_east = east_run > 0.0;
            for step in 0..columns - 1 {
                let column = if sun_is_east { columns - 2 - step } else { 1 + step };
                let sunward_column = if sun_is_east { column + 1 } else { column - 1 };
                for row in 0..rows {
                    let sunward_row = row as f32 + rows_per_column;
                    let whole_row = sunward_row.floor();
                    let fraction = sunward_row - whole_row;
                    let whole_row = whole_row as i64;
                    if whole_row < 0 || whole_row + 1 >= rows as i64 {
                        continue;
                    }
                    let first = whole_row as usize * columns + sunward_column;
                    let second = first + columns;
                    let first_ceiling = self.heights[first].max(ceilings[first]);
                    let second_ceiling = self.heights[second].max(ceilings[second]);
                    ceilings[row * columns + column] =
                        first_ceiling + (second_ceiling - first_ceiling) * fraction - sun_rise_per_cell;
                }
            }
        } else {
            let sun_rise_per_cell = SHADOW_MAP_CELL_METRES / south_run.abs();
            let columns_per_row = east_run / south_run.abs();
            let sun_is_south = south_run > 0.0;
            for step in 0..rows - 1 {
                let row = if sun_is_south { rows - 2 - step } else { 1 + step };
                let sunward_row = if sun_is_south { row + 1 } else { row - 1 };
                for column in 0..columns {
                    let sunward_column = column as f32 + columns_per_row;
                    let whole_column = sunward_column.floor();
                    let fraction = sunward_column - whole_column;
                    let whole_column = whole_column as i64;
                    if whole_column < 0 || whole_column + 1 >= columns as i64 {
                        continue;
                    }
                    let first = sunward_row * columns + whole_column as usize;
                    let second = first + 1;
                    let first_ceiling = self.heights[first].max(ceilings[first]);
                    let second_ceiling = self.heights[second].max(ceilings[second]);
                    ceilings[row * columns + column] =
                        first_ceiling + (second_ceiling - first_ceiling) * fraction - sun_rise_per_cell;
                }
            }
        }
        ceilings
    }

    #[inline]
    pub fn cell_index_at(&self, metres_east: f32, metres_south: f32) -> Option<usize> {
        let column = (metres_east - SHADOW_MAP_WEST_EDGE) / SHADOW_MAP_CELL_METRES;
        let row = (metres_south - SHADOW_MAP_NORTH_EDGE) / SHADOW_MAP_CELL_METRES;
        if column < 0.0 || row < 0.0 || column >= self.columns as f32 || row >= self.rows as f32 {
            return None;
        }
        Some(row as usize * self.columns + column as usize)
    }
}
