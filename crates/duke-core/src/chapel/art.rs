use crate::Colour;

pub const NOTHING_DRAWN: i16 = i16::MAX;

pub const FLOODLIT_TOWER_STONE: u8 = 1;
pub const SOFTLY_FLOODLIT_STONE: u8 = 2;
pub const WINDOW_GLASS: u8 = 4;
pub const FOLIAGE: u8 = 8;

pub struct Layer {
    pub width: i32,
    pub height: i32,
    pub pixels: Vec<Colour>,
    pub depth: Vec<i16>,
    pub owner: Vec<u16>,
    pub flags: Vec<u8>,
    pub pen_depth: i16,
    pub pen_owner: u16,
    pub pen_flags: u8,
}

impl Layer {
    pub fn new(width: i32, height: i32) -> Self {
        let pixel_count = (width * height) as usize;
        Layer {
            width,
            height,
            pixels: vec![0; pixel_count],
            depth: vec![NOTHING_DRAWN; pixel_count],
            owner: vec![0; pixel_count],
            flags: vec![0; pixel_count],
            pen_depth: NOTHING_DRAWN,
            pen_owner: 0,
            pen_flags: 0,
        }
    }

    pub fn contains(&self, column: i32, row: i32) -> bool {
        column >= 0 && row >= 0 && column < self.width && row < self.height
    }

    pub fn put(&mut self, column: i32, row: i32, colour: Colour) {
        if self.contains(column, row) {
            let index = (row * self.width + column) as usize;
            self.pixels[index] = colour;
            self.depth[index] = self.pen_depth;
            self.owner[index] = self.pen_owner;
            self.flags[index] = self.pen_flags;
        }
    }

    pub fn colour_at(&self, column: i32, row: i32) -> Colour {
        if self.contains(column, row) {
            self.pixels[(row * self.width + column) as usize]
        } else {
            0
        }
    }

    pub fn is_drawn(&self, column: i32, row: i32) -> bool {
        self.contains(column, row) && self.depth[(row * self.width + column) as usize] != NOTHING_DRAWN
    }

    pub fn repaint(&mut self, column: i32, row: i32, colour: Colour) {
        if self.contains(column, row) {
            let index = (row * self.width + column) as usize;
            if self.pen_depth == NOTHING_DRAWN || self.depth[index] != NOTHING_DRAWN {
                self.pixels[index] = colour;
            }
        }
    }

    pub fn horizontal_line(&mut self, from_column: i32, to_column: i32, row: i32, colour: Colour) {
        let (left, right) = (from_column.min(to_column), from_column.max(to_column));
        for column in left.max(0)..=right.min(self.width - 1) {
            self.put(column, row, colour);
        }
    }

    pub fn vertical_line(&mut self, column: i32, from_row: i32, to_row: i32, colour: Colour) {
        let (top, bottom) = (from_row.min(to_row), from_row.max(to_row));
        for row in top.max(0)..=bottom.min(self.height - 1) {
            self.put(column, row, colour);
        }
    }

    pub fn fill_rect(&mut self, from_column: i32, from_row: i32, to_column: i32, to_row: i32, colour: Colour) {
        for row in from_row.min(to_row)..=from_row.max(to_row) {
            self.horizontal_line(from_column, to_column, row, colour);
        }
    }

    pub fn fill_rect_with(
        &mut self,
        from_column: i32,
        from_row: i32,
        to_column: i32,
        to_row: i32,
        mut colour_of: impl FnMut(i32, i32) -> Colour,
    ) {
        let (left, right) = (from_column.min(to_column).max(0), from_column.max(to_column).min(self.width - 1));
        let (top, bottom) = (from_row.min(to_row).max(0), from_row.max(to_row).min(self.height - 1));
        for row in top..=bottom {
            for column in left..=right {
                let colour = colour_of(column, row);
                self.put(column, row, colour);
            }
        }
    }

    pub fn line(&mut self, from_column: i32, from_row: i32, to_column: i32, to_row: i32, colour: Colour) {
        const LONGEST_LINE_STEPS: i32 = 4000;
        let (mut column, mut row) = (from_column, from_row);
        let columns_to_go = (to_column - from_column).abs();
        let rows_to_go = -(to_row - from_row).abs();
        let column_step = if from_column < to_column { 1 } else { -1 };
        let row_step = if from_row < to_row { 1 } else { -1 };
        let mut error = columns_to_go + rows_to_go;
        let mut steps_taken = 0;
        loop {
            self.put(column, row, colour);
            if (column == to_column && row == to_row) || steps_taken > LONGEST_LINE_STEPS {
                break;
            }
            steps_taken += 1;
            let doubled_error = 2 * error;
            if doubled_error >= rows_to_go {
                error += rows_to_go;
                column += column_step;
            }
            if doubled_error <= columns_to_go {
                error += columns_to_go;
                row += row_step;
            }
        }
    }

    pub fn fill_ellipse_with(
        &mut self,
        centre_column: i32,
        centre_row: i32,
        radius_columns: i32,
        radius_rows: i32,
        mut colour_of: impl FnMut(i32, i32) -> Option<Colour>,
    ) {
        if radius_columns <= 0 || radius_rows <= 0 {
            if let Some(colour) = colour_of(centre_column, centre_row) {
                self.put(centre_column, centre_row, colour);
            }
            return;
        }
        for rows_from_centre in -radius_rows..=radius_rows {
            let remaining =
                1.0 - (rows_from_centre as f32 * rows_from_centre as f32) / (radius_rows as f32 * radius_rows as f32);
            let half_width = (radius_columns as f32 * remaining.max(0.0).sqrt()).round() as i32;
            let row = centre_row + rows_from_centre;
            for column in centre_column - half_width..=centre_column + half_width {
                if let Some(colour) = colour_of(column, row) {
                    self.put(column, row, colour);
                }
            }
        }
    }

    pub fn fill_polygon(&mut self, corners: &[(i32, i32)], colour: Colour) {
        self.fill_polygon_with(corners, |_, _| Some(colour));
    }

    pub fn fill_polygon_with(&mut self, corners: &[(i32, i32)], mut colour_of: impl FnMut(i32, i32) -> Option<Colour>) {
        let (width, height) = (self.width, self.height);
        polygon_spans(corners, width, height, |left, right, row| {
            for column in left..=right {
                if let Some(colour) = colour_of(column, row) {
                    self.put(column, row, colour);
                }
            }
        });
    }
}

pub fn polygon_spans(corners: &[(i32, i32)], width: i32, height: i32, mut fill_span: impl FnMut(i32, i32, i32)) {
    if corners.len() < 3 {
        return;
    }
    let top_row = corners.iter().map(|corner| corner.1).min().unwrap().max(0);
    let bottom_row = corners.iter().map(|corner| corner.1).max().unwrap().min(height - 1);
    let mut crossings: Vec<i32> = Vec::with_capacity(8);
    for row in top_row..=bottom_row {
        crossings.clear();
        let row_centre = row as f32 + 0.5;
        for index in 0..corners.len() {
            let (start_column, start_row) = corners[index];
            let (end_column, end_row) = corners[(index + 1) % corners.len()];
            let (start_row, end_row) = (start_row as f32, end_row as f32);
            let edge_crosses_row =
                (start_row <= row_centre && end_row > row_centre) || (end_row <= row_centre && start_row > row_centre);
            if edge_crosses_row {
                let along_edge = (row_centre - start_row) / (end_row - start_row);
                crossings.push((start_column as f32 + along_edge * (end_column - start_column) as f32).round() as i32);
            }
        }
        crossings.sort_unstable();
        for pair in crossings.chunks_exact(2) {
            let left = pair[0].max(0);
            let right = pair[1].min(width - 1);
            if left <= right {
                fill_span(left, right, row);
            }
        }
    }
}
