pub type Point = (f32, f32);

#[derive(Copy, Clone)]
pub struct PlaceOnPath {
    pub metres_east: f32,
    pub metres_south: f32,
    pub heading_east: f32,
    pub heading_south: f32,
}

pub fn path_length(points: &[Point]) -> f32 {
    points.windows(2).map(|leg| ((leg[1].0 - leg[0].0).powi(2) + (leg[1].1 - leg[0].1).powi(2)).sqrt()).sum()
}

pub fn place_along(points: &[Point], distance: f32) -> PlaceOnPath {
    let mut distance_left = distance;
    for leg in points.windows(2) {
        let (leg_east, leg_south) = (leg[1].0 - leg[0].0, leg[1].1 - leg[0].1);
        let leg_length = (leg_east * leg_east + leg_south * leg_south).sqrt();
        if distance_left <= leg_length || leg_length == 0.0 {
            let fraction = if leg_length > 0.0 { (distance_left / leg_length).max(0.0) } else { 0.0 };
            return PlaceOnPath {
                metres_east: leg[0].0 + leg_east * fraction,
                metres_south: leg[0].1 + leg_south * fraction,
                heading_east: leg_east,
                heading_south: leg_south,
            };
        }
        distance_left -= leg_length;
    }
    let end = points[points.len() - 1];
    PlaceOnPath { metres_east: end.0, metres_south: end.1, heading_east: 0.0, heading_south: 0.0 }
}

impl PlaceOnPath {
    pub fn turned_around(self) -> PlaceOnPath {
        PlaceOnPath { heading_east: -self.heading_east, heading_south: -self.heading_south, ..self }
    }

    pub fn facing_on_screen(&self) -> i8 {
        facing_on_screen(self.heading_east, self.heading_south)
    }
}

pub fn facing_on_screen(heading_east: f32, heading_south: f32) -> i8 {
    let mostly_across_the_screen = heading_east.abs() > heading_south.abs() * 0.45;
    if !mostly_across_the_screen {
        0
    } else if heading_east > 0.0 {
        1
    } else {
        -1
    }
}
