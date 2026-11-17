use crate::{Environment, Season, Weather};

const CLOCK_HOURS_BEFORE_A_CHOICE_IS_FORGOTTEN: f32 = 0.3;

pub fn cycle_and_seconds_into_it(time: f64, cycle_seconds: f64) -> (i32, f32) {
    let cycle = (time / cycle_seconds).floor();
    (cycle as i64 as i32, (time - cycle * cycle_seconds) as f32)
}

#[derive(Copy, Clone, PartialEq)]
struct Circumstances {
    seed: u32,
    season: Season,
    weather: Weather,
}

#[derive(Copy, Clone)]
struct RememberedChoice<Choice> {
    cycle: i64,
    clock_hour_at_cycle_start: f32,
    circumstances: Circumstances,
    choice: Choice,
}

pub struct CycleMemory<Choice> {
    remembered_by_slot: Vec<Option<RememberedChoice<Choice>>>,
}

impl<Choice: Copy> Default for CycleMemory<Choice> {
    fn default() -> Self {
        CycleMemory { remembered_by_slot: Vec::new() }
    }
}

impl<Choice: Copy> CycleMemory<Choice> {
    pub fn recall_or_decide(
        &mut self,
        slot: usize,
        cycle: i64,
        clock_hour_at_cycle_start: f32,
        environment: &Environment,
        seed: u32,
        decide: impl FnOnce() -> Choice,
    ) -> Choice {
        if slot >= self.remembered_by_slot.len() {
            self.remembered_by_slot.resize(slot + 1, None);
        }
        let circumstances = Circumstances { seed, season: environment.season, weather: environment.weather };
        if let Some(remembered) = self.remembered_by_slot[slot] {
            let clock_drift = (remembered.clock_hour_at_cycle_start - clock_hour_at_cycle_start).rem_euclid(24.0);
            let clock_is_close = clock_drift.min(24.0 - clock_drift) < CLOCK_HOURS_BEFORE_A_CHOICE_IS_FORGOTTEN;
            if remembered.cycle == cycle && remembered.circumstances == circumstances && clock_is_close {
                return remembered.choice;
            }
        }
        let choice = decide();
        self.remembered_by_slot[slot] =
            Some(RememberedChoice { cycle, clock_hour_at_cycle_start, circumstances, choice });
        choice
    }
}
