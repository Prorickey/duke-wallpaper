pub const QUAD_AXIS_SCREEN_X: f32 = 320.0;

const CAMERA_PITCH_SINE: f32 = 0.241_922;
const CAMERA_PITCH_COSINE: f32 = 0.970_296;
const CAMERA_HEIGHT_METRES: f32 = 304.16;
const FOCAL_LENGTH_PIXELS: f32 = 4016.0;
const CAMERA_METRES_SOUTH_OF_DOME: f32 = 1528.42;
const PRINCIPAL_POINT_SCREEN_Y: f32 = 294.56;
const HORIZON_GUARD_PIXELS: f32 = 1.0;
const METRES_SOUTH_ABOVE_HORIZON: f32 = -1.0e5;

pub const TYPICAL_GROUND_FORESHORTENING: f32 = 0.225;
pub const TYPICAL_HEIGHT_FORESHORTENING: f32 = 0.975;

#[inline]
fn distance_along_view_axis(metres_south: f32) -> f32 {
    (CAMERA_METRES_SOUTH_OF_DOME - metres_south) * CAMERA_PITCH_COSINE + CAMERA_HEIGHT_METRES * CAMERA_PITCH_SINE
}

#[inline]
pub fn pixels_per_metre_at(metres_south: f32) -> f32 {
    FOCAL_LENGTH_PIXELS / distance_along_view_axis(metres_south)
}

#[inline]
pub fn ground_screen_y(metres_south: f32) -> f32 {
    let metres_below_view_axis =
        CAMERA_HEIGHT_METRES * CAMERA_PITCH_COSINE - (CAMERA_METRES_SOUTH_OF_DOME - metres_south) * CAMERA_PITCH_SINE;
    PRINCIPAL_POINT_SCREEN_Y + FOCAL_LENGTH_PIXELS * metres_below_view_axis / distance_along_view_axis(metres_south)
}

#[inline]
pub fn ground_foreshortening_at(metres_south: f32) -> f32 {
    CAMERA_HEIGHT_METRES / distance_along_view_axis(metres_south)
}

#[inline]
pub fn height_foreshortening_at(metres_south: f32) -> f32 {
    (CAMERA_METRES_SOUTH_OF_DOME - metres_south) / distance_along_view_axis(metres_south)
}

#[inline]
pub fn screen_rows_per_metre_of_height_at(metres_south: f32) -> f32 {
    let distance = distance_along_view_axis(metres_south);
    FOCAL_LENGTH_PIXELS * (CAMERA_METRES_SOUTH_OF_DOME - metres_south) / (distance * distance)
}

#[inline]
pub fn metres_south_at_screen_row(screen_y: f32) -> f32 {
    let rows_below_principal_point = screen_y - PRINCIPAL_POINT_SCREEN_Y;
    let denominator = rows_below_principal_point * CAMERA_PITCH_COSINE + FOCAL_LENGTH_PIXELS * CAMERA_PITCH_SINE;
    if denominator <= HORIZON_GUARD_PIXELS {
        return METRES_SOUTH_ABOVE_HORIZON;
    }
    CAMERA_METRES_SOUTH_OF_DOME
        - CAMERA_HEIGHT_METRES
            * (FOCAL_LENGTH_PIXELS * CAMERA_PITCH_COSINE - rows_below_principal_point * CAMERA_PITCH_SINE)
            / denominator
}

#[inline]
pub fn project_to_screen(metres_east: f32, metres_south: f32, metres_up: f32) -> (f32, f32) {
    let distance = distance_along_view_axis(metres_south);
    let pixels_per_metre = FOCAL_LENGTH_PIXELS / distance;
    let metres_below_view_axis =
        CAMERA_HEIGHT_METRES * CAMERA_PITCH_COSINE - (CAMERA_METRES_SOUTH_OF_DOME - metres_south) * CAMERA_PITCH_SINE;
    let screen_x = QUAD_AXIS_SCREEN_X + metres_east * pixels_per_metre;
    let screen_y = PRINCIPAL_POINT_SCREEN_Y + FOCAL_LENGTH_PIXELS * metres_below_view_axis / distance
        - metres_up * pixels_per_metre * (CAMERA_METRES_SOUTH_OF_DOME - metres_south) / distance;
    (screen_x, screen_y)
}

#[inline]
pub fn project_to_pixel(metres_east: f32, metres_south: f32, metres_up: f32) -> (i32, i32) {
    let (screen_x, screen_y) = project_to_screen(metres_east, metres_south, metres_up);
    (screen_x.floor() as i32, screen_y.floor() as i32)
}

#[inline]
pub fn ground_point_at_screen(screen_x: f32, screen_y: f32) -> (f32, f32) {
    let metres_south = metres_south_at_screen_row(screen_y);
    ((screen_x - QUAD_AXIS_SCREEN_X) / pixels_per_metre_at(metres_south), metres_south)
}
