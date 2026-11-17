const CHAPEL_AXIS_COLUMN: f32 = 320.0;
const CAMERA_HEIGHT_METRES: f32 = 200.0;
const CAMERA_DISTANCE_SOUTH_OF_TOWER_METRES: f32 = 120.0;
const COSINE_OF_LOOK_DOWN_ANGLE: f32 = 0.573_576;
const SINE_OF_LOOK_DOWN_ANGLE: f32 = 0.819_152;
const FOCAL_LENGTH_PIXELS: f32 = 767.8;
const OPTICAL_AXIS_ROW: f32 = 200.5;
const NEAREST_DISTANCE_ALONG_OPTICAL_AXIS_METRES: f32 = 20.0;
const PLAN_VIEW_CENTRE_ROW: f32 = 180.0;
const DEPTH_TAGS_PER_METRE: f32 = 4.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlanView {
    pub centre_metres_east: f32,
    pub centre_metres_north: f32,
    pub pixels_per_metre: f32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Projection {
    #[default]
    Perspective,
    StraightDown(PlanView),
}

fn distance_along_optical_axis(metres_north: f32, metres_up: f32) -> f32 {
    ((metres_north + CAMERA_DISTANCE_SOUTH_OF_TOWER_METRES) * COSINE_OF_LOOK_DOWN_ANGLE
        + (CAMERA_HEIGHT_METRES - metres_up) * SINE_OF_LOOK_DOWN_ANGLE)
        .max(NEAREST_DISTANCE_ALONG_OPTICAL_AXIS_METRES)
}

pub fn depth_tag(metres_north: f32) -> i16 {
    (metres_north * DEPTH_TAGS_PER_METRE).round().clamp(-3000.0, 30000.0) as i16
}

impl Projection {
    pub fn is_plan_view(self) -> bool {
        matches!(self, Projection::StraightDown(_))
    }

    pub fn pixels_per_metre_across(self, metres_north: f32) -> f32 {
        match self {
            Projection::StraightDown(plan) => plan.pixels_per_metre,
            Projection::Perspective => FOCAL_LENGTH_PIXELS / distance_along_optical_axis(metres_north, 0.0),
        }
    }

    pub fn pixels_per_metre_of_depth(self, metres_north: f32) -> f32 {
        match self {
            Projection::StraightDown(plan) => plan.pixels_per_metre,
            Projection::Perspective => {
                let distance = distance_along_optical_axis(metres_north, 0.0);
                FOCAL_LENGTH_PIXELS * CAMERA_HEIGHT_METRES / (distance * distance)
            }
        }
    }

    pub fn pixel_x(self, metres_east: f32, metres_north: f32) -> i32 {
        match self {
            Projection::StraightDown(plan) => {
                (CHAPEL_AXIS_COLUMN + (metres_east - plan.centre_metres_east) * plan.pixels_per_metre).round() as i32
            }
            Projection::Perspective => {
                (CHAPEL_AXIS_COLUMN + metres_east * self.pixels_per_metre_across(metres_north)).round() as i32
            }
        }
    }

    pub fn exact_pixel_y(self, metres_north: f32, metres_up: f32) -> f32 {
        match self {
            Projection::StraightDown(plan) => {
                PLAN_VIEW_CENTRE_ROW - (metres_north - plan.centre_metres_north) * plan.pixels_per_metre
            }
            Projection::Perspective => {
                let metres_from_camera_along_ground = metres_north + CAMERA_DISTANCE_SOUTH_OF_TOWER_METRES;
                let metres_below_camera = CAMERA_HEIGHT_METRES - metres_up;
                OPTICAL_AXIS_ROW
                    + FOCAL_LENGTH_PIXELS
                        * (metres_below_camera * COSINE_OF_LOOK_DOWN_ANGLE
                            - metres_from_camera_along_ground * SINE_OF_LOOK_DOWN_ANGLE)
                        / distance_along_optical_axis(metres_north, metres_up)
            }
        }
    }

    pub fn pixel_y(self, metres_north: f32, metres_up: f32) -> i32 {
        self.exact_pixel_y(metres_north, metres_up).round() as i32
    }

    pub fn screen_point(self, metres_east: f32, metres_north: f32, metres_up: f32) -> (i32, i32) {
        (self.pixel_x(metres_east, metres_north), self.pixel_y(metres_north, metres_up))
    }

    pub fn ground_pixel(self, (metres_east, metres_north): (f32, f32)) -> (i32, i32) {
        self.screen_point(metres_east, metres_north, 0.0)
    }

    pub fn ground_polygon(self, corners: &[(f32, f32)]) -> Vec<(i32, i32)> {
        corners.iter().map(|&corner| self.ground_pixel(corner)).collect()
    }

    pub fn height_in_pixels(self, metres_north: f32, metres_up: f32) -> i32 {
        (self.exact_pixel_y(metres_north, 0.0) - self.exact_pixel_y(metres_north, metres_up)).round() as i32
    }

    pub fn world_point_under_pixel(self, pixel_x: f32, pixel_y: f32, metres_up: f32) -> (f32, f32) {
        match self {
            Projection::StraightDown(plan) => (
                plan.centre_metres_east + (pixel_x - CHAPEL_AXIS_COLUMN) / plan.pixels_per_metre,
                plan.centre_metres_north + (PLAN_VIEW_CENTRE_ROW - pixel_y) / plan.pixels_per_metre,
            ),
            Projection::Perspective => {
                let slope_below_optical_axis = (pixel_y - OPTICAL_AXIS_ROW) / FOCAL_LENGTH_PIXELS;
                let metres_below_camera = CAMERA_HEIGHT_METRES - metres_up;
                let metres_from_camera_along_ground = metres_below_camera
                    * (COSINE_OF_LOOK_DOWN_ANGLE - slope_below_optical_axis * SINE_OF_LOOK_DOWN_ANGLE)
                    / (SINE_OF_LOOK_DOWN_ANGLE + slope_below_optical_axis * COSINE_OF_LOOK_DOWN_ANGLE).max(1e-3);
                let metres_north = metres_from_camera_along_ground - CAMERA_DISTANCE_SOUTH_OF_TOWER_METRES;
                ((pixel_x - CHAPEL_AXIS_COLUMN) / self.pixels_per_metre_across(metres_north), metres_north)
            }
        }
    }
}
