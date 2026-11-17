use crate::east::geometry::{CANVAS_HEIGHT, CANVAS_WIDTH};
use crate::east::projection::*;
use crate::{blend_colours, rgb, Canvas, Colour};

pub struct FramePainter<'a> {
    pub canvas: &'a mut Canvas,
    pub depths: &'a [f32],
}

impl FramePainter<'_> {
    fn visible_pixel(&self, x: i32, y: i32, depth: f32) -> Option<usize> {
        if x < 0 || y < 0 || x as usize >= CANVAS_WIDTH || y as usize >= CANVAS_HEIGHT {
            return None;
        }
        let pixel = y as usize * CANVAS_WIDTH + x as usize;
        if depth >= self.depths[pixel] - 0.25 {
            Some(pixel)
        } else {
            None
        }
    }

    pub fn put(&mut self, x: i32, y: i32, depth: f32, colour: Colour) {
        if let Some(pixel) = self.visible_pixel(x, y, depth) {
            self.canvas.pixels[pixel] = colour;
        }
    }

    pub fn blend(&mut self, x: i32, y: i32, depth: f32, colour: Colour, opacity: f32) {
        if let Some(pixel) = self.visible_pixel(x, y, depth) {
            self.canvas.pixels[pixel] = blend_colours(self.canvas.pixels[pixel], colour, opacity);
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub enum Pose {
    Walk,
    Run,
    #[default]
    Stand,
    Sit,
    Lie,
    ArmsUp,
    Crouch,
}

#[derive(Copy, Clone, Debug)]
pub struct Person {
    pub metres_east: f32,
    pub metres_south: f32,
    pub metres_up: f32,
    pub look: u32,
    pub pose: Pose,
    pub stride_frame: u8,
    pub heading_across_screen: i8,
    pub team_bib: u8,
}

impl Person {
    pub fn new(metres_east: f32, metres_south: f32, look: u32, pose: Pose) -> Person {
        Person {
            metres_east,
            metres_south,
            metres_up: 0.0,
            look,
            pose,
            stride_frame: 0,
            heading_across_screen: 0,
            team_bib: 0,
        }
    }
}

pub fn stride_frame(distance_walked: f32, stride_length: f32) -> u8 {
    ((distance_walked / stride_length) as i32 & 1) as u8
}

#[derive(Copy, Clone)]
pub struct PeopleConditions {
    pub wrapped_up_for_winter: bool,
    pub raining: bool,
    pub snowing: bool,
    pub shadow_direction_x: f32,
    pub shadow_strength: f32,
}

pub const SHIRTS: [Colour; 12] = [
    rgb(36, 60, 150),
    rgb(230, 230, 232),
    rgb(196, 56, 52),
    rgb(240, 200, 70),
    rgb(60, 140, 96),
    rgb(40, 40, 52),
    rgb(232, 140, 60),
    rgb(150, 100, 180),
    rgb(120, 170, 220),
    rgb(226, 150, 170),
    rgb(36, 60, 150),
    rgb(150, 156, 164),
];
const WINTER_COATS: [Colour; 7] = [
    rgb(40, 44, 60),
    rgb(120, 40, 44),
    rgb(60, 80, 70),
    rgb(90, 90, 100),
    rgb(36, 60, 150),
    rgb(150, 120, 80),
    rgb(210, 210, 216),
];
pub const SKIN_TONES: [Colour; 4] = [rgb(240, 200, 168), rgb(210, 160, 120), rgb(150, 100, 70), rgb(98, 66, 50)];
pub const HAIR_COLOURS: [Colour; 5] =
    [rgb(40, 30, 28), rgb(96, 62, 36), rgb(196, 160, 90), rgb(24, 22, 28), rgb(150, 70, 40)];
const TROUSERS: [Colour; 4] = [rgb(44, 50, 78), rgb(70, 62, 56), rgb(30, 30, 38), rgb(150, 140, 116)];
const BACKPACKS: [Colour; 4] = [rgb(30, 30, 40), rgb(150, 40, 40), rgb(40, 90, 70), rgb(90, 70, 130)];
const WOOLLY_HATS: [Colour; 5] =
    [rgb(196, 50, 50), rgb(36, 60, 150), rgb(230, 230, 232), rgb(60, 140, 96), rgb(240, 200, 70)];
const UMBRELLAS: [Colour; 5] =
    [rgb(196, 50, 50), rgb(30, 36, 60), rgb(40, 80, 170), rgb(236, 200, 60), rgb(230, 230, 236)];
const TEAM_BIBS: [Colour; 4] = [rgb(206, 52, 48), rgb(36, 60, 150), rgb(240, 240, 240), rgb(244, 206, 64)];
const FOOT_SHADOW: Colour = rgb(16, 22, 40);

pub const LOOK_NO_BACKPACK: u32 = 1 << 11;
pub const LOOK_WEARS_SHORTS_MASK: u32 = 3 << 20;

pub fn draw_person(painter: &mut FramePainter, person: &Person, conditions: &PeopleConditions) {
    let (foot_x, foot_y) = project_to_pixel(person.metres_east, person.metres_south, person.metres_up);
    let depth = person.metres_south;
    let look = person.look;
    let shirt = if person.team_bib > 0 {
        TEAM_BIBS[(person.team_bib as usize - 1) % 4]
    } else if conditions.wrapped_up_for_winter {
        WINTER_COATS[(look % 7) as usize]
    } else {
        SHIRTS[(look % 12) as usize]
    };
    let shirt_shadow = blend_colours(shirt, 0, 0.2);
    let skin = SKIN_TONES[((look >> 4) % 4) as usize];
    let hair = HAIR_COLOURS[((look >> 6) % 5) as usize];
    let trousers = TROUSERS[((look >> 9) % 4) as usize];
    let wears_shorts = !conditions.wrapped_up_for_winter && look & LOOK_WEARS_SHORTS_MASK == 0;
    let is_tall_on_screen = screen_rows_per_metre_of_height_at(depth) > 3.2;
    let back_column = if person.heading_across_screen > 0 { 0 } else { 1 };
    let front_column = 1 - back_column;

    if person.metres_up < 0.3 {
        if conditions.shadow_strength > 0.1 && person.pose != Pose::Lie {
            for length in 1..=2 {
                let shadow_x = foot_x + (conditions.shadow_direction_x * length as f32 * 1.6).round() as i32;
                painter.blend(shadow_x, foot_y, depth - 0.3, FOOT_SHADOW, 0.3 * conditions.shadow_strength);
            }
        } else {
            painter.blend(foot_x + 1, foot_y, depth - 0.3, FOOT_SHADOW, 0.22);
        }
    }

    match person.pose {
        Pose::Lie => {
            let lying_direction = if look & 1 == 0 { 1 } else { -1 };
            let head_to_toe = [hair, skin, shirt, shirt, if wears_shorts { skin } else { trousers }];
            for (along, colour) in head_to_toe.iter().enumerate() {
                painter.put(foot_x + lying_direction * (along as i32 - 2), foot_y - 1, depth, *colour);
            }
            return;
        }
        Pose::Sit => {
            painter.put(foot_x, foot_y - 1, depth, trousers);
            painter.put(foot_x + 1, foot_y - 1, depth, trousers);
            painter.put(foot_x, foot_y - 2, depth, shirt);
            painter.put(foot_x + 1, foot_y - 2, depth, shirt_shadow);
            painter.put(foot_x + front_column, foot_y - 3, depth, skin);
            painter.put(foot_x + back_column, foot_y - 3, depth, hair);
            return;
        }
        Pose::Crouch => {
            painter.put(foot_x, foot_y - 1, depth, shirt);
            painter.put(foot_x + 1, foot_y - 1, depth, shirt_shadow);
            painter.put(foot_x + front_column, foot_y - 2, depth, skin);
            painter.put(foot_x + back_column, foot_y - 2, depth, hair);
            return;
        }
        _ => {}
    }

    let is_moving = matches!(person.pose, Pose::Walk | Pose::Run);
    let feet = if wears_shorts { skin } else { trousers };
    if !is_moving {
        painter.put(foot_x, foot_y - 1, depth, feet);
        painter.put(foot_x + 1, foot_y - 1, depth, feet);
    } else if person.stride_frame == 0 {
        painter.put(foot_x, foot_y - 1, depth, feet);
    } else {
        painter.put(foot_x + 1, foot_y - 1, depth, feet);
    }
    if is_tall_on_screen {
        painter.put(foot_x, foot_y - 2, depth, trousers);
        painter.put(foot_x + 1, foot_y - 2, depth, trousers);
    }
    let waist_y = foot_y - 2 - is_tall_on_screen as i32;
    painter.put(foot_x, waist_y, depth, shirt);
    painter.put(foot_x + 1, waist_y, depth, shirt_shadow);
    painter.put(foot_x, waist_y - 1, depth, shirt);
    painter.put(foot_x + 1, waist_y - 1, depth, shirt_shadow);
    let carries_backpack = person.team_bib == 0 && (look >> 11) & 3 == 0 && person.pose != Pose::ArmsUp;
    if carries_backpack {
        let backpack = BACKPACKS[((look >> 15) % 4) as usize];
        painter.put(foot_x + back_column, waist_y - 1, depth, backpack);
        painter.put(foot_x + back_column, waist_y, depth, backpack);
    }
    let head_lean = if person.pose == Pose::Run { person.heading_across_screen as i32 } else { 0 };
    painter.put(foot_x + front_column + head_lean, waist_y - 2, depth, skin);
    painter.put(foot_x + back_column + head_lean, waist_y - 2, depth, hair);
    let wears_hat = (conditions.wrapped_up_for_winter && (look >> 13) & 1 == 0) || (look >> 13) & 7 == 5;
    if wears_hat {
        let hat = WOOLLY_HATS[((look >> 17) % 5) as usize];
        painter.put(foot_x + head_lean, waist_y - 3, depth, hat);
        painter.put(foot_x + 1 + head_lean, waist_y - 3, depth, hat);
    } else if (look >> 23) & 1 == 0 {
        painter.put(foot_x + back_column + head_lean, waist_y - 3, depth, hair);
    }
    if person.pose == Pose::ArmsUp {
        painter.put(foot_x - 1, waist_y - 2, depth, skin);
        painter.put(foot_x + 2, waist_y - 2, depth, skin);
        painter.put(foot_x - 1, waist_y - 3, depth, skin);
        painter.put(foot_x + 2, waist_y - 3, depth, skin);
    }
    let carries_umbrella = person.team_bib == 0
        && person.pose != Pose::Run
        && (conditions.raining || (conditions.snowing && look & 3 == 0));
    if carries_umbrella {
        let canopy = UMBRELLAS[((look >> 2) % 5) as usize];
        let canopy_shadow = blend_colours(canopy, 0, 0.3);
        for across in -2..=3 {
            painter.put(foot_x + across, waist_y - 3, depth, if across > 1 { canopy_shadow } else { canopy });
        }
        for across in -1..=2 {
            painter.put(foot_x + across, waist_y - 4, depth, canopy);
        }
        painter.put(foot_x, waist_y - 5, depth, blend_colours(canopy, 0xFFFFFF, 0.3));
        painter.put(foot_x + 1, waist_y - 5, depth, canopy);
    }
}
