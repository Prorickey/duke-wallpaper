use crate::east::figures::{FramePainter, SHIRTS, SKIN_TONES};
use crate::east::projection::*;
use crate::{blend_colours, rgb, Colour};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Rider {
    pub facing: i8,
    pub pedal_frame: i32,
    pub look: u32,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PropKind {
    Volleyball,
    Snowball,
    SnowballBeingRolled,
    Frisbee,
    Spikeball,
    Football,
    VolleyballNet,
    SpikeballNet,
    SnowFort,
    Snowman { stage: i32 },
    PicnicBlanket { pattern: usize },
    BeachTowel { colour: usize },
    Book,
    Bike(Rider),
    Scooter(Rider),
    GroundsCart { seen_side_on: bool },
}

#[derive(Copy, Clone, Debug)]
pub struct Prop {
    pub metres_east: f32,
    pub metres_south: f32,
    pub metres_up: f32,
    pub kind: PropKind,
}

const GROUND_SHADOW: Colour = rgb(16, 22, 40);
const SNOW_SHADOW: Colour = rgb(120, 138, 184);
const SNOW_SHADE: Colour = rgb(150, 166, 204);
const SNOW_WHITE: Colour = rgb(250, 252, 255);
const PICNIC_BLANKETS: [(Colour, Colour); 4] = [
    (rgb(206, 60, 60), rgb(236, 232, 222)),
    (rgb(40, 80, 160), rgb(236, 232, 222)),
    (rgb(240, 200, 70), rgb(250, 246, 230)),
    (rgb(60, 140, 96), rgb(232, 236, 222)),
];
const BEACH_TOWELS: [Colour; 4] = [rgb(240, 120, 60), rgb(80, 170, 220), rgb(250, 220, 90), rgb(226, 150, 190)];
const TYRE: Colour = rgb(28, 28, 36);
const BIKE_FRAME: Colour = rgb(180, 60, 50);
const SCOOTER_DECK: Colour = rgb(90, 200, 120);
const SCOOTER_STEM: Colour = rgb(60, 64, 76);

pub fn draw_prop(painter: &mut FramePainter, prop: &Prop) {
    let (x, y) = project_to_pixel(prop.metres_east, prop.metres_south, prop.metres_up);
    let (ground_x, ground_y) = project_to_pixel(prop.metres_east, prop.metres_south, 0.0);
    let depth = prop.metres_south;
    let in_front = depth + 1.0;
    let is_in_the_air = prop.metres_up > 0.4;
    match prop.kind {
        PropKind::Volleyball => {
            let leather = rgb(250, 248, 236);
            painter.put(x, y - 1, in_front, leather);
            painter.put(x, y - 2, in_front, leather);
            painter.put(x + 1, y - 2, in_front, leather);
            painter.put(x + 1, y - 1, in_front, rgb(206, 206, 196));
            if is_in_the_air {
                painter.blend(ground_x, ground_y, depth - 0.3, GROUND_SHADOW, 0.25);
            }
        }
        PropKind::Snowball => {
            painter.put(x, y - 1, in_front, SNOW_WHITE);
            painter.put(x, y, in_front, SNOW_SHADOW);
            painter.put(x + 1, y - 1, in_front, SNOW_SHADE);
            if is_in_the_air {
                painter.blend(ground_x, ground_y, depth - 0.3, GROUND_SHADOW, 0.25);
            }
        }
        PropKind::SnowballBeingRolled => {
            painter.put(x, y - 1, in_front, SNOW_WHITE);
            painter.put(x + 1, y - 1, depth, SNOW_WHITE);
            painter.put(x, y - 2, depth, SNOW_WHITE);
            painter.put(x + 1, y - 2, depth, rgb(198, 210, 232));
        }
        PropKind::Frisbee => {
            painter.put(x, y - 1, in_front, rgb(250, 250, 250));
            painter.put(x + 1, y - 1, in_front, rgb(214, 60, 60));
            if is_in_the_air {
                painter.blend(ground_x, ground_y, depth - 0.3, GROUND_SHADOW, 0.25);
            }
        }
        PropKind::Spikeball => {
            painter.put(x, y - 1, in_front, rgb(250, 220, 60));
            if is_in_the_air {
                painter.blend(ground_x, ground_y, depth - 0.3, GROUND_SHADOW, 0.25);
            }
        }
        PropKind::Football => {
            painter.put(x, y - 1, in_front, rgb(140, 78, 40));
            painter.put(x + 1, y - 1, in_front, rgb(176, 104, 56));
            if is_in_the_air {
                painter.blend(ground_x, ground_y, depth - 0.3, GROUND_SHADOW, 0.25);
                painter.blend(ground_x + 1, ground_y, depth - 0.3, GROUND_SHADOW, 0.25);
            }
        }
        PropKind::VolleyballNet => {
            let (west_post_x, _) = project_to_pixel(prop.metres_east - 4.6, prop.metres_south, 0.0);
            let (east_post_x, _) = project_to_pixel(prop.metres_east + 4.6, prop.metres_south, 0.0);
            let net_height = (2.45 * screen_rows_per_metre_of_height_at(prop.metres_south)).round() as i32;
            let mesh_rows = (net_height * 2 / 5).max(2);
            for up in 0..=net_height {
                painter.put(west_post_x, y - up, depth, rgb(52, 52, 62));
                painter.put(east_post_x, y - up, depth, rgb(52, 52, 62));
            }
            for column in west_post_x + 1..east_post_x {
                painter.put(column, y - net_height, depth, rgb(250, 250, 250));
                for mesh_row in 1..mesh_rows {
                    if (column + mesh_row) & 1 == 0 {
                        painter.blend(column, y - net_height + mesh_row, depth, rgb(236, 236, 236), 0.55);
                    }
                }
                painter.blend(column, y - net_height + mesh_rows, depth, rgb(250, 250, 250), 0.7);
            }
            painter.blend(west_post_x - 1, y, depth - 0.2, GROUND_SHADOW, 0.3);
            painter.blend(east_post_x + 1, y, depth - 0.2, GROUND_SHADOW, 0.3);
        }
        PropKind::SpikeballNet => {
            for across in -1..=1 {
                painter.put(x + across, y - 1, depth, if across == 0 { rgb(30, 30, 36) } else { rgb(250, 214, 50) });
            }
            painter.put(x, y - 2, depth, rgb(250, 214, 50));
        }
        PropKind::SnowFort => {
            for across in -3..=3 {
                painter.put(x + across, y - 1, depth, SNOW_SHADE);
                painter.put(x + across, y - 2, depth, rgb(176, 190, 222));
            }
            for across in -3..=3 {
                painter.put(x + across, y - 3, depth, if across.abs() == 3 { rgb(176, 190, 222) } else { SNOW_WHITE });
            }
            for across in -3..=4 {
                painter.blend(x + across, y, depth - 0.2, SNOW_SHADOW, 0.7);
            }
        }
        PropKind::Snowman { stage } => {
            let (lit_snow, shaded_snow) = (rgb(246, 249, 253), rgb(198, 210, 232));
            painter.put(x - 1, y - 1, depth, lit_snow);
            painter.put(x, y - 1, depth, lit_snow);
            painter.put(x + 1, y - 1, depth, shaded_snow);
            painter.put(x - 1, y - 2, depth, lit_snow);
            painter.put(x, y - 2, depth, lit_snow);
            painter.put(x + 1, y - 2, depth, shaded_snow);
            if stage >= 2 {
                painter.put(x, y - 3, depth, lit_snow);
                painter.put(x + 1, y - 3, depth, shaded_snow);
                painter.put(x, y - 4, depth, lit_snow);
                painter.put(x + 1, y - 4, depth, shaded_snow);
            }
            if stage >= 3 {
                painter.put(x, y - 5, depth, lit_snow);
                painter.put(x + 1, y - 5, depth, rgb(236, 130, 40));
                painter.put(x, y - 6, depth, rgb(36, 60, 150));
                painter.put(x + 1, y - 6, depth, rgb(36, 60, 150));
            }
            for across in -1..=2 {
                painter.blend(x + across, y, depth - 0.2, rgb(160, 176, 210), 0.6);
            }
        }
        PropKind::PicnicBlanket { pattern } => {
            let (check, cream) = PICNIC_BLANKETS[pattern & 3];
            for across in -4..=4 {
                painter.put(x + across, y, depth - 0.4, if across & 1 == 0 { check } else { cream });
                painter.put(x + across, y - 1, depth - 0.4, if across & 1 == 0 { cream } else { check });
            }
            painter.put(x, y - 2, depth, rgb(150, 110, 60));
        }
        PropKind::BeachTowel { colour } => {
            let towel = BEACH_TOWELS[colour & 3];
            for across in -3..=3 {
                painter.put(x + across, y, depth - 0.4, towel);
            }
            painter.put(x - 3, y - 1, depth - 0.4, towel);
            painter.put(x + 3, y - 1, depth - 0.4, towel);
        }
        PropKind::Book => {
            painter.put(x, y - 1, depth + 0.2, rgb(240, 238, 228));
        }
        PropKind::Bike(rider) | PropKind::Scooter(rider) => {
            let shirt = SHIRTS[(rider.look % 12) as usize];
            let skin = SKIN_TONES[((rider.look >> 4) % 4) as usize];
            let seen_side_on = rider.facing != 0;
            let forward = if rider.facing >= 0 { 1 } else { -1 };
            painter.blend(x, y, depth - 0.3, GROUND_SHADOW, 0.25);
            if matches!(prop.kind, PropKind::Scooter(_)) {
                if seen_side_on {
                    painter.put(x - 1, y - 1, depth, TYRE);
                    painter.put(x, y - 1, depth, SCOOTER_DECK);
                    painter.put(x + 1, y - 1, depth, TYRE);
                    painter.put(x + forward, y - 2, depth, SCOOTER_STEM);
                    painter.put(x + forward, y - 3, depth, SCOOTER_STEM);
                } else {
                    painter.put(x, y - 1, depth, TYRE);
                    painter.put(x + 1, y - 1, depth, SCOOTER_DECK);
                }
                painter.put(x, y - 2, depth, rgb(44, 50, 78));
                painter.put(x, y - 3, depth, shirt);
                painter.put(x, y - 4, depth, shirt);
                painter.put(x, y - 5, depth, skin);
            } else if seen_side_on {
                painter.put(x - 2, y - 1, depth, TYRE);
                painter.put(x + 2, y - 1, depth, TYRE);
                painter.put(x - 2 + rider.pedal_frame, y, depth, TYRE);
                painter.put(x + 2 - rider.pedal_frame, y, depth, TYRE);
                for along in -1..=1 {
                    painter.put(x + along, y - 2, depth, BIKE_FRAME);
                }
                painter.put(x, y - 3, depth, shirt);
                painter.put(x + forward, y - 4, depth, shirt);
                painter.put(x + forward, y - 5, depth, skin);
            } else {
                let wears_a_yellow_helmet = (rider.look >> 7) & 1 == 0;
                painter.put(x, y, depth, TYRE);
                painter.put(x, y - 1, depth, TYRE);
                painter.put(x, y - 2, depth, BIKE_FRAME);
                painter.put(x, y - 3, depth, shirt);
                painter.put(x + 1, y - 3, depth, blend_colours(shirt, 0, 0.2));
                painter.put(x, y - 4, depth, shirt);
                painter.put(x, y - 5, depth, if wears_a_yellow_helmet { rgb(230, 230, 60) } else { skin });
            }
        }
        PropKind::GroundsCart { seen_side_on } => {
            let (width, height) = if seen_side_on { (6, 4) } else { (4, 5) };
            for row in 0..height {
                let colour = match row {
                    0 => rgb(236, 236, 228),
                    1 => rgb(60, 70, 90),
                    _ => rgb(40, 96, 60),
                };
                for across in -width / 2..=width / 2 {
                    painter.put(x + across, y - height + row, depth, colour);
                }
            }
            painter.put(x - width / 2, y, depth, rgb(24, 24, 30));
            painter.put(x + width / 2, y, depth, rgb(24, 24, 30));
        }
    }
}
