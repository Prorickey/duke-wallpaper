pub const WEST_SIDE: f32 = -1.0;
pub const EAST_SIDE: f32 = 1.0;
pub const BOTH_SIDES: [f32; 2] = [WEST_SIDE, EAST_SIDE];

#[inline]
pub fn side_of_quad(metres_east: f32) -> f32 {
    if metres_east < 0.0 {
        WEST_SIDE
    } else {
        EAST_SIDE
    }
}

#[derive(Copy, Clone)]
pub struct Rectangle {
    pub west: f32,
    pub east: f32,
    pub north: f32,
    pub south: f32,
}

pub const fn rectangle(west: f32, east: f32, north: f32, south: f32) -> Rectangle {
    Rectangle { west, east, north, south }
}

pub struct QuadHall {
    pub name: &'static str,
    pub side_of_quad: f32,
    pub north_end: f32,
    pub south_end: f32,
}

impl QuadHall {
    pub fn entrance_south(&self) -> f32 {
        0.5 * (self.north_end + self.south_end)
    }
}

const fn quad_hall(name: &'static str, side_of_quad: f32, north_end: f32, south_end: f32) -> QuadHall {
    QuadHall { name, side_of_quad, north_end, south_end }
}

pub const QUAD_HALLS: [QuadHall; 8] = [
    quad_hall("Pegram", WEST_SIDE, 7.4, 63.6),
    quad_hall("Alspaugh", WEST_SIDE, 74.4, 130.7),
    quad_hall("Giles", WEST_SIDE, 197.3, 253.0),
    quad_hall("Classroom Building", WEST_SIDE, 265.5, 321.9),
    quad_hall("Bassett", EAST_SIDE, 10.2, 65.5),
    quad_hall("Brown", EAST_SIDE, 77.4, 132.7),
    quad_hall("Wilson", EAST_SIDE, 198.8, 254.9),
    quad_hall("Friedl", EAST_SIDE, 267.5, 324.2),
];

pub fn hall_entrances_south(side: f32) -> [f32; 4] {
    let mut entrances = [0.0; 4];
    let mut count = 0;
    for hall in QUAD_HALLS.iter() {
        if hall.side_of_quad == side {
            entrances[count] = hall.entrance_south();
            count += 1;
        }
    }
    entrances
}

const WEST_CROSS_WALKS_SOUTH: [f32; 4] = [69.0, 136.0, 191.5, 259.3];
const EAST_CROSS_WALKS_SOUTH: [f32; 4] = [71.5, 139.0, 194.3, 261.2];

pub fn cross_walks_south(side: f32) -> [f32; 4] {
    if side < 0.0 {
        WEST_CROSS_WALKS_SOUTH
    } else {
        EAST_CROSS_WALKS_SOUTH
    }
}

pub struct PorticoBuilding {
    pub side_of_quad: f32,
    pub facade_from_axis: f32,
    pub rear_from_axis: f32,
    pub north_end: f32,
    pub south_end: f32,
}

pub const LILLY_LIBRARY: PorticoBuilding = PorticoBuilding {
    side_of_quad: WEST_SIDE,
    facade_from_axis: 45.8,
    rear_from_axis: 81.8,
    north_end: 141.2,
    south_end: 185.7,
};
pub const EAST_CAMPUS_UNION: PorticoBuilding = PorticoBuilding {
    side_of_quad: EAST_SIDE,
    facade_from_axis: 39.5,
    rear_from_axis: 84.8,
    north_end: 145.3,
    south_end: 189.9,
};

pub const LAWN_HALF_WIDTH: f32 = 20.0;
pub const QUAD_DRIVE_WIDTH: f32 = 6.5;
pub const NORTH_LAWN_CAP_SOUTH: f32 = 71.5;
pub const UNION_CIRCLE_SOUTH: f32 = 166.0;
pub const UNION_CIRCLE_LAWN_RADIUS: f32 = 28.6;
pub const UNION_CIRCLE_OUTER_RADIUS: f32 = 35.2;
pub const SOUTH_LAWN_CAP_SOUTH: f32 = 291.0;
pub const BENJAMIN_DUKE_STATUE_SOUTH: f32 = 69.5;

pub const ENTRANCE_LOOP_NORTH_KERB: f32 = 335.6;
pub const ENTRANCE_LOOP_SOUTH_KERB: f32 = 342.4;
pub const ENTRANCE_LOOP_INNER_KERB_FROM_AXIS: f32 = 29.6;
pub const ENTRANCE_LOOP_OUTER_KERB_FROM_AXIS: f32 = 36.2;
pub const ENTRANCE_LOOP_OUTER_CORNER_RADIUS: f32 = 20.0;
pub const ENTRANCE_LOOP_INNER_CORNER_RADIUS: f32 = 15.0;
pub const QUAD_STEM_FROM_AXIS: f32 = 5.8;
pub const ZEBRA_CROSSING_FROM_AXIS: f32 = 18.6;

pub const BUS_PLAZA_HALF_WIDTH: f32 = 14.5;
pub const BUS_PLAZA_SOUTH_EDGE: f32 = 348.4;
pub const BUS_PLAZA_BENCHES_EAST: [f32; 4] = [-6.9, -4.2, 1.9, 4.7];
pub const BUS_PLAZA_BENCHES_SOUTH: f32 = 347.3;

pub const BALDWIN_PARKING_BAYS: Rectangle = rectangle(3.0, 56.0, -42.6, -37.6);
pub const BROWN_LOT: Rectangle = rectangle(70.8, 92.4, 26.4, 143.9);
pub const WILSON_COURTYARD: Rectangle = rectangle(50.5, 67.5, 211.5, 242.5);
pub const BIDDLE_LOT: Rectangle = rectangle(-76.2, -57.0, -41.7, -15.7);
pub const PEGRAM_LOT: Rectangle = rectangle(-170.5, -79.2, 68.0, 109.7);
pub const GILBERT_ADDOMS_LOT: Rectangle = rectangle(-191.0, -86.0, 238.3, 402.0);
pub const CAR_PARKS: [Rectangle; 6] =
    [BALDWIN_PARKING_BAYS, BROWN_LOT, WILSON_COURTYARD, BIDDLE_LOT, PEGRAM_LOT, GILBERT_ADDOMS_LOT];

pub const TENNIS_COURTS: [Rectangle; 6] = [
    rectangle(-104.9, -92.8, 114.3, 138.6),
    rectangle(-120.4, -108.4, 113.4, 137.7),
    rectangle(-136.0, -123.9, 112.6, 137.0),
    rectangle(-106.5, -94.5, 150.3, 174.6),
    rectangle(-122.1, -110.1, 149.3, 173.7),
    rectangle(-137.7, -125.7, 148.8, 173.2),
];

pub const ARCADE_FLOORS: [Rectangle; 8] = [
    rectangle(-43.8, -40.2, 63.0, 74.8),
    rectangle(-56.2, -52.6, 130.5, 141.4),
    rectangle(-56.2, -52.6, 185.5, 197.5),
    rectangle(-43.8, -40.2, 252.8, 265.7),
    rectangle(40.2, 43.8, 65.3, 77.6),
    rectangle(45.8, 49.4, 132.5, 145.5),
    rectangle(45.8, 49.4, 189.7, 199.0),
    rectangle(40.2, 43.8, 254.7, 267.7),
];

pub const BERKELEY_STREET_EAST: f32 = -114.0;
pub const ONSLOW_STREET_EAST: f32 = -5.0;
pub const LANCASTER_STREET_EAST: f32 = 105.5;
pub const TRINITY_PARK_STREETS_EAST: [f32; 3] = [BERKELEY_STREET_EAST, ONSLOW_STREET_EAST, LANCASTER_STREET_EAST];
pub const WEST_MARKHAM_AVENUE_SOUTH: f32 = -81.0;
pub const WEST_MARKHAM_AVENUE_HALF_WIDTH: f32 = 5.5;
pub const BALDWIN_SERVICE_ROAD_SOUTH: f32 = -34.0;
pub const BALDWIN_SERVICE_ROAD_WEST_LEG_EAST: f32 = -6.0;
pub const BALDWIN_SERVICE_ROAD_EAST_LEG_EAST: f32 = 60.0;

pub const DESIRE_LINES_ACROSS_THE_LAWNS: [((f32, f32), (f32, f32)); 10] = [
    ((-19.5, 58.0), (-1.5, 68.5)),
    ((1.5, 70.5), (19.5, 96.0)),
    ((19.0, 57.0), (2.0, 68.0)),
    ((-2.0, 71.0), (-19.0, 104.0)),
    ((-6.0, 52.5), (-1.0, 67.5)),
    ((-18.0, 112.0), (16.0, 128.0)),
    ((-22.0, 150.0), (22.0, 182.0)),
    ((21.0, 152.0), (-21.0, 180.0)),
    ((-19.0, 206.0), (18.0, 252.0)),
    ((18.5, 266.0), (-17.0, 304.0)),
];

pub const GRAVEL_PERIMETER_PATH: [(f32, f32); 5] =
    [(18.4, -6.0), (34.0, -14.0), (58.0, -23.0), (90.0, -27.0), (140.0, -24.0)];
