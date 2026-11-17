use crate::east::campus_layout::*;
use crate::east::geometry::Solid;
use crate::east::trinity_park_houses::TRINITY_PARK_HOUSE_FOOTPRINTS;
use crate::hash_position;

pub const HALL_EAVE_HEIGHT: f32 = 8.4;
pub const MANSARD_SLOPE: f32 = 0.95;
pub const MANSARD_RISE: f32 = 4.6;
pub const HALL_WINDOW_BAY: f32 = 3.4;
pub const HALL_CHIMNEY_HEIGHT: f32 = HALL_EAVE_HEIGHT + MANSARD_RISE + 2.0;
pub const HALL_CHIMNEY_FROM_AXIS: f32 = 44.5;
pub const ENTRANCE_GABLE_RISE: f32 = 2.9;
pub const ENTRANCE_GABLE_HALF_WIDTH: f32 = 4.05;

pub const BALDWIN_UPPER_TIER_HEIGHT: f32 = 16.5;
pub const BALDWIN_LOWER_TIER_HEIGHT: f32 = 12.5;
pub const DRUM_RADIUS: f32 = 12.3;
pub const DRUM_TOP_HEIGHT: f32 = 20.7;
pub const GUTTER_RING_RADIUS: f32 = 12.75;
pub const DOME_SPRING_HEIGHT: f32 = 21.3;
pub const DOME_RADIUS: f32 = 11.9;
pub const DOME_RISE: f32 = 6.6;
pub const PORTICO_FLOOR_HEIGHT: f32 = 1.3;
pub const PORTICO_COLUMNS_SOUTH: f32 = 22.0;
pub const PORTICO_COLUMNS_EAST: [f32; 6] = [-8.1, -4.95, -1.65, 1.65, 4.95, 8.1];
pub const PORTICO_COLUMN_TOP_HEIGHT: f32 = 9.95;
pub const ENTABLATURE_BASE_HEIGHT: f32 = 10.6;
pub const PEDIMENT_BASE_HEIGHT: f32 = 12.45;
pub const PEDIMENT_RISE: f32 = 3.3;
pub const PEDIMENT_HALF_WIDTH: f32 = 9.7;
pub const PEDIMENT_NORTH_EDGE: f32 = 19.5;

pub const LIBRARY_ROOF_HEIGHT: f32 = 12.6;
pub const LIBRARY_ATTIC_HEIGHT: f32 = LIBRARY_ROOF_HEIGHT + 1.1;
pub const LIBRARY_COLUMN_TOP_HEIGHT: f32 = 9.7;
pub const LIBRARY_SKYLIGHT_RISE: f32 = 2.4;
pub const BIDDLE_ROOF_HEIGHT: f32 = 9.0;
pub const GREEN_ROOF_BAY_LENGTH: f32 = 13.2;
pub const GREEN_ROOF_BAY_PITCH: f32 = GREEN_ROOF_BAY_LENGTH + 0.16;
pub const GREEN_ROOF_OVERHANG: f32 = 0.4;
pub const GREEN_HALL_EAVE_HEIGHT: f32 = 11.4;

const BISHOPS_HOUSE: Rectangle = rectangle(114.6, 136.5, 20.2, 39.3);
const PRE_MAJOR_ADVISING_CENTER: Rectangle = rectangle(105.1, 119.1, 77.0, 119.5);
const THE_ARK: Rectangle = rectangle(84.8, 115.0, 206.6, 220.6);
const CROWELL_BUILDING: Rectangle = rectangle(87.1, 126.3, 256.0, 278.0);
const BIDDLE_MUSIC_BUILDING: Rectangle = rectangle(-147.5, -76.5, -34.9, 16.6);
const JARVIS_WEST_EDGE: f32 = -64.0;
const JARVIS_EAST_EDGE: f32 = -49.5;
const JARVIS_NORTH_END: f32 = 334.0;
const EAST_HOUSE_WEST_EDGE: f32 = 49.0;
const EAST_HOUSE_EAST_EDGE: f32 = 63.5;
const EAST_HOUSE_NORTH_END: f32 = 338.0;

const ARCADES_BETWEEN_NEIGHBOURS: [Rectangle; 8] = [
    rectangle(-43.6, -40.4, 63.6, 74.4),
    rectangle(40.4, 43.6, 65.5, 77.4),
    rectangle(-56.0, -52.8, 130.7, 141.2),
    rectangle(-56.0, -52.8, 185.7, 197.3),
    rectangle(46.0, 49.2, 132.7, 145.3),
    rectangle(46.0, 49.2, 189.9, 198.8),
    rectangle(-43.6, -40.4, 253.0, 265.5),
    rectangle(40.4, 43.6, 254.9, 267.5),
];

#[derive(Copy, Clone, PartialEq)]
pub enum DormerFacing {
    East,
    West,
    South,
    North,
}

#[derive(Copy, Clone, PartialEq)]
pub enum HouseStyle {
    BishopsHouse,
    BrickInstitutional,
    TheArk,
    TrinityParkHouse,
}

#[derive(Copy, Clone)]
pub enum Material {
    BaldwinBrickTier { faces_portico: bool },
    DomeDrum,
    DomeGutterRing,
    Dome,
    DomeOculus,
    PorticoFloor,
    PorticoSteps,
    PorticoColumn { centre_east: f32 },
    ColumnCapital { centre_east: f32 },
    ColumnBase,
    Entablature,
    Pediment,
    ArcadeRoof { runs_north_south: bool, ridge: f32 },
    ArcadeWall { first_bay_start: f32, bay_length: f32, runs_north_south: bool },
    HallWall { entrance_south: f32, is_entrance_pavilion: bool },
    HallRoof,
    Dormer { facing: DormerFacing, centre: f32 },
    Chimney,
    EntranceGable { ridge_south: f32 },
    DoorSurround { door_south: f32 },
    Hedge { variation: f32 },
    BiddleMusicBuilding,
    StatuePlinth { is_base_step: bool },
    TerracePlanter,
    RoofPlant { is_white_penthouse: bool },
    LibraryWall { is_union: bool, is_low_pavilion: bool },
    LibraryStylobate,
    LibrarySteps,
    LibraryColumn,
    LibraryAttic,
    LibrarySkylight,
    House { style: HouseStyle, ridge_runs_east_west: bool, wall_height: f32 },
    GreenHallWall,
    GreenHallRoof { north_end: f32 },
}

const NOTHING_HERE: (f32, f32) = (0.0, 0.0);

fn solid(
    material: Material,
    bounds: Rectangle,
    top_height: f32,
    march_step_metres: f32,
    vertical_extent_at: impl Fn(f32, f32) -> (f32, f32) + Send + Sync + 'static,
) -> Solid {
    Solid {
        material,
        west_edge: bounds.west,
        east_edge: bounds.east,
        north_edge: bounds.north,
        south_edge: bounds.south,
        top_height,
        march_step_metres,
        casts_shadow: !matches!(material, Material::Hedge { .. }),
        vertical_extent_at: Box::new(vertical_extent_at),
    }
}

fn block(material: Material, bounds: Rectangle, bottom: f32, top: f32) -> Solid {
    let narrowest_side = (bounds.east - bounds.west).min(bounds.south - bounds.north);
    let march_step = (0.3f32).min(narrowest_side * 0.45).max(0.05);
    solid(material, bounds, top, march_step, move |east, south| {
        if east >= bounds.west && east <= bounds.east && south >= bounds.north && south <= bounds.south {
            (bottom, top)
        } else {
            NOTHING_HERE
        }
    })
}

fn cylinder(material: Material, centre_east: f32, centre_south: f32, radius: f32, bottom: f32, top: f32) -> Solid {
    let bounds = rectangle(centre_east - radius, centre_east + radius, centre_south - radius, centre_south + radius);
    solid(material, bounds, top, (0.3f32).min(radius * 0.4), move |east, south| {
        if (east - centre_east) * (east - centre_east) + (south - centre_south) * (south - centre_south)
            <= radius * radius
        {
            (bottom, top)
        } else {
            NOTHING_HERE
        }
    })
}

fn mansard_roof(bounds: Rectangle) -> Solid {
    solid(Material::HallRoof, bounds, HALL_EAVE_HEIGHT + MANSARD_RISE, 0.3, move |east, south| {
        let from_eave =
            (east - bounds.west).min(bounds.east - east).min(south - bounds.north).min(bounds.south - south);
        if from_eave <= 0.0 {
            NOTHING_HERE
        } else {
            (HALL_EAVE_HEIGHT - 0.2, HALL_EAVE_HEIGHT + (from_eave * MANSARD_SLOPE).min(MANSARD_RISE))
        }
    })
}

fn dormer(bounds: Rectangle, facing: DormerFacing, centre: f32) -> Solid {
    let half_width = 0.8;
    let arch_runs_east_west = matches!(facing, DormerFacing::South | DormerFacing::North);
    solid(Material::Dormer { facing, centre }, bounds, HALL_EAVE_HEIGHT + 3.25, 0.15, move |east, south| {
        if east < bounds.west || east > bounds.east || south < bounds.north || south > bounds.south {
            return NOTHING_HERE;
        }
        let across = if arch_runs_east_west { (east - centre) / half_width } else { (south - centre) / half_width };
        (HALL_EAVE_HEIGHT, HALL_EAVE_HEIGHT + 2.55 + 0.7 * (1.0 - across * across).max(0.0).sqrt())
    })
}

fn mirrored(side: f32, nearer_axis: f32, farther_from_axis: f32, north: f32, south: f32) -> Rectangle {
    let (first, second) = (side * nearer_axis, side * farther_from_axis);
    rectangle(first.min(second), first.max(second), north, south)
}

fn arch_opening_bottom(along: f32, bay_length: f32) -> f32 {
    let across_arch = (along.rem_euclid(bay_length) - 0.5 * bay_length) / (0.5 * bay_length - 0.42);
    if across_arch.abs() < 1.0 {
        2.0 + 1.2 * (1.0 - across_arch * across_arch).sqrt()
    } else {
        0.0
    }
}

fn add_arcade_running_east_west(solids: &mut Vec<Solid>, footprint: Rectangle) {
    let (west, east, north, south) = (footprint.west, footprint.east, footprint.north, footprint.south);
    let ridge = 0.5 * (north + south);
    let half_width = 0.5 * (south - north) + 0.25;
    let roof = Material::ArcadeRoof { runs_north_south: false, ridge };
    solids.push(solid(
        roof,
        rectangle(west, east, north - 0.25, south + 0.25),
        4.25,
        0.2,
        move |metres_east, metres_south| {
            if metres_east < west || metres_east > east || (metres_south - ridge).abs() > half_width {
                return NOTHING_HERE;
            }
            (3.5, 3.6 + 0.65 * (1.0 - (metres_south - ridge).abs() / half_width))
        },
    ));
    let bay_count = ((east - west) / 3.3).round().max(1.0);
    let bay_length = (east - west) / bay_count;
    let wall = Material::ArcadeWall { first_bay_start: west, bay_length, runs_north_south: false };
    for (wall_north, wall_south) in [(north, north + 0.4), (south - 0.4, south)] {
        solids.push(solid(
            wall,
            rectangle(west, east, wall_north, wall_south),
            3.5,
            0.12,
            move |metres_east, metres_south| {
                if metres_east < west || metres_east > east || metres_south < wall_north || metres_south > wall_south {
                    return NOTHING_HERE;
                }
                (arch_opening_bottom(metres_east - west, bay_length), 3.5)
            },
        ));
    }
}

fn add_arcade_running_north_south(solids: &mut Vec<Solid>, footprint: Rectangle) {
    let (west, east, north, south) = (footprint.west, footprint.east, footprint.north, footprint.south);
    let ridge = 0.5 * (west + east);
    let half_width = 0.5 * (east - west) + 0.25;
    let roof = Material::ArcadeRoof { runs_north_south: true, ridge };
    solids.push(solid(
        roof,
        rectangle(west - 0.25, east + 0.25, north, south),
        4.25,
        0.2,
        move |metres_east, metres_south| {
            if metres_south < north || metres_south > south || (metres_east - ridge).abs() > half_width {
                return NOTHING_HERE;
            }
            (3.5, 3.6 + 0.65 * (1.0 - (metres_east - ridge).abs() / half_width))
        },
    ));
    let bay_count = ((south - north) / 3.3).round().max(1.0);
    let bay_length = (south - north) / bay_count;
    let wall = Material::ArcadeWall { first_bay_start: north, bay_length, runs_north_south: true };
    for (wall_west, wall_east) in [(west, west + 0.4), (east - 0.4, east)] {
        solids.push(solid(
            wall,
            rectangle(wall_west, wall_east, north, south),
            3.5,
            0.12,
            move |metres_east, metres_south| {
                if metres_east < wall_west || metres_east > wall_east || metres_south < north || metres_south > south {
                    return NOTHING_HERE;
                }
                (arch_opening_bottom(metres_south - north, bay_length), 3.5)
            },
        ));
    }
}

fn add_baldwin_auditorium(solids: &mut Vec<Solid>) {
    let plain_tier = Material::BaldwinBrickTier { faces_portico: false };
    solids.push(block(plain_tier, rectangle(-16.3, 16.3, -18.5, 17.1), 0.0, BALDWIN_LOWER_TIER_HEIGHT));
    solids.push(block(
        Material::BaldwinBrickTier { faces_portico: true },
        rectangle(-9.0, 9.0, -18.5, 19.5),
        0.0,
        BALDWIN_UPPER_TIER_HEIGHT,
    ));
    solids.push(block(plain_tier, rectangle(-16.3, 16.3, -9.8, 10.4), 0.0, BALDWIN_UPPER_TIER_HEIGHT));

    solids.push(cylinder(Material::DomeDrum, 0.0, 0.0, DRUM_RADIUS, BALDWIN_LOWER_TIER_HEIGHT, DRUM_TOP_HEIGHT));
    solids.push(cylinder(Material::DomeGutterRing, 0.0, 0.0, GUTTER_RING_RADIUS, DRUM_TOP_HEIGHT, DOME_SPRING_HEIGHT));
    let sphere_radius = (DOME_RADIUS * DOME_RADIUS + DOME_RISE * DOME_RISE) / (2.0 * DOME_RISE);
    let dome_bounds = rectangle(-DOME_RADIUS, DOME_RADIUS, -DOME_RADIUS, DOME_RADIUS);
    solids.push(solid(Material::Dome, dome_bounds, DOME_SPRING_HEIGHT + DOME_RISE, 0.25, move |east, south| {
        let squared_from_centre = east * east + south * south;
        if squared_from_centre >= DOME_RADIUS * DOME_RADIUS {
            NOTHING_HERE
        } else {
            (
                DOME_SPRING_HEIGHT - 0.3,
                DOME_SPRING_HEIGHT + (sphere_radius * sphere_radius - squared_from_centre).sqrt()
                    - (sphere_radius - DOME_RISE),
            )
        }
    }));
    solids.push(cylinder(
        Material::DomeOculus,
        0.0,
        0.0,
        1.45,
        DOME_SPRING_HEIGHT + DOME_RISE - 0.3,
        DOME_SPRING_HEIGHT + DOME_RISE + 0.45,
    ));

    solids.push(block(Material::PorticoFloor, rectangle(-9.5, 9.5, 19.5, 23.0), 0.0, PORTICO_FLOOR_HEIGHT));
    solids.push(solid(
        Material::PorticoSteps,
        rectangle(-9.5, 9.5, 23.0, 25.45),
        PORTICO_FLOOR_HEIGHT,
        0.1,
        |east, south| {
            if !(-9.5..=9.5).contains(&east) || !(23.0..=25.45).contains(&south) {
                return NOTHING_HERE;
            }
            let step_number = ((south - 23.0) / 0.35).floor();
            (0.0, (1.3 - 0.1857 * (step_number + 1.0)).max(0.02))
        },
    ));
    for &column_east in PORTICO_COLUMNS_EAST.iter() {
        solids.push(cylinder(
            Material::PorticoColumn { centre_east: column_east },
            column_east,
            PORTICO_COLUMNS_SOUTH,
            0.56,
            PORTICO_FLOOR_HEIGHT,
            PORTICO_COLUMN_TOP_HEIGHT,
        ));
        let capital =
            rectangle(column_east - 0.8, column_east + 0.8, PORTICO_COLUMNS_SOUTH - 0.62, PORTICO_COLUMNS_SOUTH + 0.62);
        solids.push(block(
            Material::ColumnCapital { centre_east: column_east },
            capital,
            PORTICO_COLUMN_TOP_HEIGHT,
            ENTABLATURE_BASE_HEIGHT,
        ));
        let base = rectangle(
            column_east - 0.78,
            column_east + 0.78,
            PORTICO_COLUMNS_SOUTH - 0.62,
            PORTICO_COLUMNS_SOUTH + 0.62,
        );
        solids.push(block(Material::ColumnBase, base, PORTICO_FLOOR_HEIGHT, 1.62));
    }
    solids.push(block(
        Material::Entablature,
        rectangle(-9.3, 9.3, 19.5, 22.85),
        ENTABLATURE_BASE_HEIGHT,
        PEDIMENT_BASE_HEIGHT,
    ));
    let pediment_bounds = rectangle(-PEDIMENT_HALF_WIDTH, PEDIMENT_HALF_WIDTH, PEDIMENT_NORTH_EDGE, 23.1);
    solids.push(solid(Material::Pediment, pediment_bounds, 15.75, 0.25, |east, south| {
        if !(19.5..=23.1).contains(&south) {
            return NOTHING_HERE;
        }
        (PEDIMENT_BASE_HEIGHT, PEDIMENT_BASE_HEIGHT + PEDIMENT_RISE * (1.0 - east.abs() / PEDIMENT_HALF_WIDTH))
    }));

    solids.push(block(Material::TerracePlanter, rectangle(-27.5, -16.3, 1.6, 2.2), 0.0, 0.9));
    solids.push(block(Material::TerracePlanter, rectangle(-28.0, -27.4, -13.5, 2.2), 0.0, 0.9));

    add_arcade_running_east_west(solids, rectangle(-33.0, -16.3, 11.8, 15.0));
    add_arcade_running_east_west(solids, rectangle(16.3, 33.0, 12.4, 15.6));
}

fn add_quad_hall(solids: &mut Vec<Solid>, hall: &QuadHall, hall_number: f32) {
    let side = hall.side_of_quad;
    let (north_end, south_end) = (hall.north_end, hall.south_end);
    let entrance_south = hall.entrance_south();
    let wing_length = 10.4;
    let wall = Material::HallWall { entrance_south, is_entrance_pavilion: false };

    solids.push(block(wall, mirrored(side, 33.0, 49.0, north_end, south_end), 0.0, HALL_EAVE_HEIGHT));
    let mut wings = vec![(north_end, north_end + wing_length), (south_end - wing_length, south_end)];
    let has_middle_wing = hall.name == "Friedl";
    if has_middle_wing {
        wings.push((entrance_south - wing_length * 0.5, entrance_south + wing_length * 0.5));
    }
    for &(wing_north, wing_south) in wings.iter() {
        solids.push(block(wall, mirrored(side, 48.5, 68.0, wing_north, wing_south), 0.0, HALL_EAVE_HEIGHT));
    }
    solids.push(mansard_roof(mirrored(side, 32.65, 49.35, north_end - 0.35, south_end + 0.35)));
    for &(wing_north, wing_south) in wings.iter() {
        solids.push(mansard_roof(mirrored(side, 41.0, 68.35, wing_north - 0.35, wing_south + 0.35)));
    }

    let pavilion = Material::HallWall { entrance_south, is_entrance_pavilion: true };
    solids.push(block(
        pavilion,
        mirrored(side, 32.0, 33.2, entrance_south - 3.7, entrance_south + 3.7),
        0.0,
        HALL_EAVE_HEIGHT,
    ));
    let gable_bounds = mirrored(
        side,
        31.7,
        38.6,
        entrance_south - ENTRANCE_GABLE_HALF_WIDTH,
        entrance_south + ENTRANCE_GABLE_HALF_WIDTH,
    );
    let gable = Material::EntranceGable { ridge_south: entrance_south };
    solids.push(solid(gable, gable_bounds, HALL_EAVE_HEIGHT + ENTRANCE_GABLE_RISE, 0.2, move |east, south| {
        if east < gable_bounds.west || east > gable_bounds.east {
            return NOTHING_HERE;
        }
        (
            HALL_EAVE_HEIGHT - 0.2,
            HALL_EAVE_HEIGHT + ENTRANCE_GABLE_RISE * (1.0 - (south - entrance_south).abs() / ENTRANCE_GABLE_HALF_WIDTH),
        )
    }));
    let door_surround = Material::DoorSurround { door_south: entrance_south };
    solids.push(block(door_surround, mirrored(side, 31.0, 32.0, entrance_south - 1.3, entrance_south + 1.3), 0.0, 3.5));

    let faces_quad = if side < 0.0 { DormerFacing::East } else { DormerFacing::West };
    let faces_courtyard = if side < 0.0 { DormerFacing::West } else { DormerFacing::East };
    let dormer_count = (((south_end - north_end) - 6.0) / 4.4).floor() as i32;
    let first_dormer_south = entrance_south - (dormer_count - 1) as f32 * 2.2;
    for index in 0..dormer_count {
        let dormer_south = first_dormer_south + index as f32 * 4.4;
        let clear_of_entrance_gable = (dormer_south - entrance_south).abs() > 5.0;
        if clear_of_entrance_gable {
            solids.push(dormer(
                mirrored(side, 33.5, 36.2, dormer_south - 0.8, dormer_south + 0.8),
                faces_quad,
                dormer_south,
            ));
        }
        solids.push(dormer(
            mirrored(side, 45.8, 48.5, dormer_south - 0.8, dormer_south + 0.8),
            faces_courtyard,
            dormer_south,
        ));
    }
    for &(wing_north, wing_south) in wings.iter() {
        for index in 0..4 {
            let from_axis = 52.3 + index as f32 * 4.3;
            let dormer_east = side * from_axis;
            solids.push(dormer(
                mirrored(side, from_axis - 0.8, from_axis + 0.8, wing_south - 3.2, wing_south - 0.5),
                DormerFacing::South,
                dormer_east,
            ));
            solids.push(dormer(
                mirrored(side, from_axis - 0.8, from_axis + 0.8, wing_north + 0.5, wing_north + 3.2),
                DormerFacing::North,
                dormer_east,
            ));
        }
    }
    for from_axis in [38.6f32, 43.4] {
        let dormer_east = side * from_axis;
        solids.push(dormer(
            mirrored(side, from_axis - 0.8, from_axis + 0.8, south_end - 3.2, south_end - 0.5),
            DormerFacing::South,
            dormer_east,
        ));
    }

    for (index, fraction_along) in [0.2f32, 0.5, 0.8].iter().enumerate() {
        let chimney_south = north_end + (south_end - north_end) * fraction_along;
        let from_axis = if index % 2 == 0 { 37.3 } else { HALL_CHIMNEY_FROM_AXIS };
        let bounds = mirrored(side, from_axis - 0.55, from_axis + 0.55, chimney_south - 0.75, chimney_south + 0.75);
        solids.push(block(Material::Chimney, bounds, HALL_EAVE_HEIGHT, HALL_CHIMNEY_HEIGHT));
    }
    for &(wing_north, wing_south) in wings.iter() {
        let chimney_south = 0.5 * (wing_north + wing_south);
        let bounds = mirrored(side, 61.0, 62.5, chimney_south - 0.55, chimney_south + 0.55);
        solids.push(block(Material::Chimney, bounds, HALL_EAVE_HEIGHT, HALL_EAVE_HEIGHT + MANSARD_RISE + 1.8));
    }

    let north_hedge = mirrored(side, 31.6, 32.9, north_end + 1.0, entrance_south - 5.2);
    let south_hedge = mirrored(side, 31.6, 32.9, entrance_south + 5.2, south_end - 1.0);
    solids.push(block(Material::Hedge { variation: hall_number }, north_hedge, 0.0, 1.1));
    solids.push(block(Material::Hedge { variation: hall_number + 3.0 }, south_hedge, 0.0, 1.1));
}

fn add_portico_building(solids: &mut Vec<Solid>, building: &PorticoBuilding, is_union: bool) {
    let side = building.side_of_quad;
    let facade = building.facade_from_axis;
    let rear = building.rear_from_axis;
    let (north_end, south_end) = (building.north_end, building.south_end);
    let portico_south = 0.5 * (north_end + south_end);
    let main_block = Material::LibraryWall { is_union, is_low_pavilion: false };
    let low_pavilion = Material::LibraryWall { is_union, is_low_pavilion: true };
    let pavilion_height = 9.4;

    solids.push(block(
        main_block,
        mirrored(side, facade + 3.4, rear - 4.0, north_end + 5.5, south_end - 5.5),
        0.0,
        LIBRARY_ROOF_HEIGHT,
    ));
    solids.push(block(
        low_pavilion,
        mirrored(side, facade + 8.0, rear - 8.0, north_end, north_end + 5.6),
        0.0,
        pavilion_height,
    ));
    solids.push(block(
        low_pavilion,
        mirrored(side, facade + 8.0, rear - 8.0, south_end - 5.6, south_end),
        0.0,
        pavilion_height,
    ));
    solids.push(block(
        low_pavilion,
        mirrored(side, rear - 4.1, rear, north_end + 9.0, south_end - 9.0),
        0.0,
        pavilion_height,
    ));

    solids.push(block(
        Material::LibraryStylobate,
        mirrored(side, facade, facade + 3.4, portico_south - 12.6, portico_south + 12.6),
        0.0,
        1.4,
    ));
    let steps_bounds = mirrored(side, facade - 2.4, facade, portico_south - 11.0, portico_south + 11.0);
    solids.push(solid(Material::LibrarySteps, steps_bounds, 1.4, 0.1, move |east, south| {
        if east < steps_bounds.west || east > steps_bounds.east || (south - portico_south).abs() > 11.0 {
            return NOTHING_HERE;
        }
        let fraction_down_the_flight = (facade - east * side) / 2.4;
        (0.0, (1.4 * (1.0 - (fraction_down_the_flight * 7.0).ceil() / 7.0)).max(0.02))
    }));
    for index in 0..6 {
        let column_south = portico_south - 11.0 + index as f32 * 4.4;
        solids.push(cylinder(
            Material::LibraryColumn,
            side * (facade + 1.0),
            column_south,
            0.6,
            1.4,
            LIBRARY_COLUMN_TOP_HEIGHT,
        ));
    }
    let attic = mirrored(side, facade + 0.1, facade + 3.4, portico_south - 12.9, portico_south + 12.9);
    solids.push(block(Material::LibraryAttic, attic, LIBRARY_COLUMN_TOP_HEIGHT, LIBRARY_ATTIC_HEIGHT));

    if is_union {
        let penthouse = mirrored(side, facade + 22.0, facade + 39.0, portico_south - 11.0, portico_south + 11.0);
        solids.push(block(
            Material::RoofPlant { is_white_penthouse: true },
            penthouse,
            LIBRARY_ROOF_HEIGHT,
            LIBRARY_ROOF_HEIGHT + 2.6,
        ));
        let plant = mirrored(side, facade + 9.0, facade + 13.0, portico_south - 4.0, portico_south + 2.0);
        solids.push(block(
            Material::RoofPlant { is_white_penthouse: false },
            plant,
            LIBRARY_ROOF_HEIGHT,
            LIBRARY_ROOF_HEIGHT + 1.4,
        ));
    } else {
        let skylight_east = side * (facade + 17.0);
        let half_width = 3.2;
        let bounds = rectangle(
            skylight_east - half_width,
            skylight_east + half_width,
            portico_south - half_width,
            portico_south + half_width,
        );
        solids.push(solid(
            Material::LibrarySkylight,
            bounds,
            LIBRARY_ROOF_HEIGHT + LIBRARY_SKYLIGHT_RISE,
            0.2,
            move |east, south| {
                let from_apex = (east - skylight_east).abs().max((south - portico_south).abs());
                if from_apex >= half_width {
                    NOTHING_HERE
                } else {
                    (LIBRARY_ROOF_HEIGHT, LIBRARY_ROOF_HEIGHT + 0.3 + 2.1 * (1.0 - from_apex / half_width))
                }
            },
        ));
    }

    let hedge_variation = if is_union { 1.0 } else { 0.0 };
    let north_hedge = mirrored(side, facade - 1.6, facade - 0.2, portico_south - 20.0, portico_south - 12.2);
    let south_hedge = mirrored(side, facade - 1.6, facade - 0.2, portico_south + 12.2, portico_south + 20.0);
    solids.push(block(Material::Hedge { variation: hedge_variation + 7.0 }, north_hedge, 0.0, 1.1));
    solids.push(block(Material::Hedge { variation: hedge_variation + 9.0 }, south_hedge, 0.0, 1.1));
}

fn pitched_roof_house(
    footprint: Rectangle,
    wall_height: f32,
    roof_rise: f32,
    is_hipped: bool,
    style: HouseStyle,
) -> Solid {
    let (west, east, north, south) = (footprint.west, footprint.east, footprint.north, footprint.south);
    let ridge_runs_east_west = (east - west) >= (south - north);
    let (centre_east, centre_south) = (0.5 * (west + east), 0.5 * (north + south));
    let (half_east, half_south) = (0.5 * (east - west), 0.5 * (south - north));
    let material = Material::House { style, ridge_runs_east_west, wall_height };
    solid(material, footprint, wall_height + roof_rise, 0.3, move |metres_east, metres_south| {
        if metres_east < west || metres_east > east || metres_south < north || metres_south > south {
            return NOTHING_HERE;
        }
        let (across_ridge, along_ridge, half_span, half_length) = if ridge_runs_east_west {
            ((metres_south - centre_south).abs(), (metres_east - centre_east).abs(), half_south, half_east)
        } else {
            ((metres_east - centre_east).abs(), (metres_south - centre_south).abs(), half_east, half_south)
        };
        let mut roof_height = roof_rise * (1.0 - across_ridge / half_span);
        if is_hipped {
            roof_height = roof_height.min(roof_rise * (half_length - along_ridge) / half_span);
        }
        (0.0, wall_height + roof_height.max(0.0))
    })
}

fn add_green_roofed_hall(solids: &mut Vec<Solid>, west_edge: f32, east_edge: f32, north_end: f32) {
    let south_end = north_end + 5.0 * GREEN_ROOF_BAY_LENGTH;
    solids.push(block(
        Material::GreenHallWall,
        rectangle(west_edge, east_edge, north_end, south_end),
        0.0,
        GREEN_HALL_EAVE_HEIGHT,
    ));
    let roof_bounds = rectangle(
        west_edge - GREEN_ROOF_OVERHANG,
        east_edge + GREEN_ROOF_OVERHANG,
        north_end - GREEN_ROOF_OVERHANG,
        south_end + GREEN_ROOF_OVERHANG,
    );
    solids.push(solid(
        Material::GreenHallRoof { north_end },
        roof_bounds,
        GREEN_HALL_EAVE_HEIGHT + 4.2,
        0.3,
        move |east, south| {
            if east < roof_bounds.west
                || east > roof_bounds.east
                || south < north_end - GREEN_ROOF_OVERHANG
                || south > south_end + GREEN_ROOF_OVERHANG
            {
                return NOTHING_HERE;
            }
            let along_bay = ((south - (north_end - GREEN_ROOF_OVERHANG)) / GREEN_ROOF_BAY_PITCH).rem_euclid(1.0)
                * GREEN_ROOF_BAY_PITCH;
            let from_bay_edge = (east - roof_bounds.west)
                .min(roof_bounds.east - east)
                .min(along_bay)
                .min(GREEN_ROOF_BAY_PITCH - along_bay);
            (GREEN_HALL_EAVE_HEIGHT - 0.2, GREEN_HALL_EAVE_HEIGHT + (from_bay_edge * 0.6).min(4.2))
        },
    ));
}

pub fn all_solids() -> Vec<Solid> {
    let mut solids = Vec::new();
    add_baldwin_auditorium(&mut solids);
    for (index, hall) in QUAD_HALLS.iter().enumerate() {
        add_quad_hall(&mut solids, hall, index as f32 + 1.0);
    }
    add_portico_building(&mut solids, &LILLY_LIBRARY, false);
    add_portico_building(&mut solids, &EAST_CAMPUS_UNION, true);
    for arcade in ARCADES_BETWEEN_NEIGHBOURS {
        add_arcade_running_north_south(&mut solids, arcade);
    }
    add_green_roofed_hall(&mut solids, JARVIS_WEST_EDGE, JARVIS_EAST_EDGE, JARVIS_NORTH_END);
    add_green_roofed_hall(&mut solids, EAST_HOUSE_WEST_EDGE, EAST_HOUSE_EAST_EDGE, EAST_HOUSE_NORTH_END);

    solids.push(block(Material::Hedge { variation: 11.0 }, rectangle(-25.0, -8.4, 328.0, 331.7), 0.0, 1.4));
    solids.push(block(Material::Hedge { variation: 13.0 }, rectangle(8.4, 25.0, 328.6, 331.9), 0.0, 1.4));

    solids.push(block(Material::BiddleMusicBuilding, BIDDLE_MUSIC_BUILDING, 0.0, BIDDLE_ROOF_HEIGHT));
    solids.push(block(
        Material::RoofPlant { is_white_penthouse: false },
        rectangle(-108.0, -98.0, -20.0, -10.0),
        BIDDLE_ROOF_HEIGHT,
        10.6,
    ));
    solids.push(block(
        Material::RoofPlant { is_white_penthouse: false },
        rectangle(-92.0, -86.0, -6.0, 2.0),
        BIDDLE_ROOF_HEIGHT,
        10.2,
    ));

    solids.push(pitched_roof_house(BISHOPS_HOUSE, 6.8, 4.0, false, HouseStyle::BishopsHouse));
    solids.push(pitched_roof_house(PRE_MAJOR_ADVISING_CENTER, 7.2, 3.2, true, HouseStyle::BrickInstitutional));
    solids.push(pitched_roof_house(THE_ARK, 6.6, 3.6, false, HouseStyle::TheArk));
    solids.push(pitched_roof_house(CROWELL_BUILDING, 8.0, 3.8, true, HouseStyle::BrickInstitutional));
    for (index, footprint) in TRINITY_PARK_HOUSE_FOOTPRINTS.iter().enumerate() {
        let house_hash = hash_position(index as i32, 7, 0x7121);
        let wall_height = 5.8 + (house_hash % 5) as f32 * 0.55;
        let roof_rise = 2.8 + ((house_hash >> 4) % 4) as f32 * 0.4;
        let is_hipped = (house_hash >> 8).is_multiple_of(3);
        solids.push(pitched_roof_house(*footprint, wall_height, roof_rise, is_hipped, HouseStyle::TrinityParkHouse));
    }

    solids.push(block(Material::StatuePlinth { is_base_step: false }, rectangle(-0.95, 0.95, 68.7, 70.3), 0.0, 1.5));
    solids.push(block(Material::StatuePlinth { is_base_step: true }, rectangle(-1.3, 1.3, 68.35, 70.65), 0.0, 0.35));
    solids
}
