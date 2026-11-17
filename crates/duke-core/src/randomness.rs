const SPLITMIX_INCREMENT: u64 = 0x9E37_79B9_7F4A_7C15;
const SPLITMIX_FIRST_MULTIPLIER: u64 = 0xBF58_476D_1CE4_E5B9;
const SPLITMIX_SECOND_MULTIPLIER: u64 = 0x94D0_49BB_1331_11EB;
const HASH_X_MULTIPLIER: u32 = 0x85EB_CA6B;
const HASH_Y_MULTIPLIER: u32 = 0xC2B2_AE35;
const HASH_SEED_MULTIPLIER: u32 = 0x27D4_EB2F;
const HASH_FIRST_FINAL_MULTIPLIER: u32 = 0x2C1B_3C6D;
const HASH_SECOND_FINAL_MULTIPLIER: u32 = 0x297A_2D39;
const FRACTION_BITS: u32 = 24;
const OCTAVE_SEED_STEP: u32 = 101;
const AVERAGE_NOISE_VALUE: f32 = 0.5;

#[derive(Clone, Debug)]
pub struct RandomNumberGenerator {
    state: u64,
}

impl RandomNumberGenerator {
    pub fn new(seed: u64) -> Self {
        RandomNumberGenerator { state: seed ^ SPLITMIX_INCREMENT }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(SPLITMIX_INCREMENT);
        let mut mixed = self.state;
        mixed = (mixed ^ (mixed >> 30)).wrapping_mul(SPLITMIX_FIRST_MULTIPLIER);
        mixed = (mixed ^ (mixed >> 27)).wrapping_mul(SPLITMIX_SECOND_MULTIPLIER);
        mixed ^ (mixed >> 31)
    }

    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }

    pub fn next_fraction(&mut self) -> f32 {
        (self.next_u64() >> (64 - FRACTION_BITS)) as f32 / (1u64 << FRACTION_BITS) as f32
    }

    pub fn float_between(&mut self, low: f32, high: f32) -> f32 {
        low + (high - low) * self.next_fraction()
    }

    pub fn integer_between(&mut self, lowest: i32, highest: i32) -> i32 {
        if highest <= lowest {
            return lowest;
        }
        let possible_values = (highest as i64 - lowest as i64 + 1) as u64;
        (lowest as i64 + (self.next_u64() % possible_values) as i64) as i32
    }

    pub fn succeeds_with_probability(&mut self, probability: f32) -> bool {
        self.next_fraction() < probability
    }

    pub fn choose_from<'items, Item>(&mut self, items: &'items [Item]) -> &'items Item {
        &items[(self.next_u64() % items.len() as u64) as usize]
    }
}

pub fn fold_seed_to_32_bits(seed: u64) -> u32 {
    (seed as u32) ^ ((seed >> 32) as u32)
}

#[inline]
pub fn hash_position(x: i32, y: i32, seed: u32) -> u32 {
    let mut hash = (x as u32).wrapping_mul(HASH_X_MULTIPLIER)
        ^ (y as u32).wrapping_mul(HASH_Y_MULTIPLIER)
        ^ seed.wrapping_mul(HASH_SEED_MULTIPLIER);
    hash ^= hash >> 15;
    hash = hash.wrapping_mul(HASH_FIRST_FINAL_MULTIPLIER);
    hash ^= hash >> 12;
    hash = hash.wrapping_mul(HASH_SECOND_FINAL_MULTIPLIER);
    hash ^ (hash >> 15)
}

#[inline]
pub fn hash_fraction(x: i32, y: i32, seed: u32) -> f32 {
    (hash_position(x, y, seed) >> (32 - FRACTION_BITS)) as f32 / (1u32 << FRACTION_BITS) as f32
}

pub fn value_noise(x: f32, y: f32, seed: u32) -> f32 {
    let (cell_left, cell_top) = (x.floor(), y.floor());
    let fraction_within_cell = |coordinate: f32, cell_start: f32| {
        let fraction = coordinate - cell_start;
        if fraction.is_nan() {
            0.0
        } else {
            fraction
        }
    };
    let (fraction_x, fraction_y) = (fraction_within_cell(x, cell_left), fraction_within_cell(y, cell_top));
    let (column, row) = (cell_left as i32, cell_top as i32);
    let (next_column, next_row) = (column.wrapping_add(1), row.wrapping_add(1));
    let smooth_x = fraction_x * fraction_x * (3.0 - 2.0 * fraction_x);
    let smooth_y = fraction_y * fraction_y * (3.0 - 2.0 * fraction_y);
    let top_left = hash_fraction(column, row, seed);
    let top_right = hash_fraction(next_column, row, seed);
    let bottom_left = hash_fraction(column, next_row, seed);
    let bottom_right = hash_fraction(next_column, next_row, seed);
    let top = top_left + (top_right - top_left) * smooth_x;
    let bottom = bottom_left + (bottom_right - bottom_left) * smooth_x;
    top + (bottom - top) * smooth_y
}

pub fn fractal_noise(x: f32, y: f32, octaves: u32, seed: u32) -> f32 {
    if octaves == 0 {
        return AVERAGE_NOISE_VALUE;
    }
    let (mut total, mut amplitude, mut frequency, mut total_amplitude) = (0.0, 0.5, 1.0, 0.0);
    for octave in 0..octaves {
        total += amplitude * value_noise(x * frequency, y * frequency, seed.wrapping_add(octave * OCTAVE_SEED_STEP));
        total_amplitude += amplitude;
        amplitude *= 0.5;
        frequency *= 2.0;
    }
    total / total_amplitude
}
