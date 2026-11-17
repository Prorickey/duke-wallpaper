use crate::atmosphere::WallClock;
use crate::Season;

#[derive(Copy, Clone, Debug)]
pub struct LocalTime {
    pub year: u32,
    pub month: u32,
    pub day: u32,
    pub hour: u32,
    pub minute: u32,
    pub second: u32,
    pub millisecond: u32,
}

impl LocalTime {
    pub fn as_atmosphere_clock(&self) -> WallClock {
        let seconds_into_the_hour = self.minute as f32 * 60.0 + self.second as f32 + self.millisecond as f32 / 1000.0;
        WallClock {
            month: self.month.clamp(1, 12),
            day: self.day.clamp(1, 31),
            hour: self.hour as f32 + seconds_into_the_hour / 3600.0,
        }
    }

    pub fn as_log_timestamp(&self) -> String {
        format!(
            "{:04}-{:02}-{:02} {:02}:{:02}:{:02}.{:03}",
            self.year, self.month, self.day, self.hour, self.minute, self.second, self.millisecond
        )
    }
}

pub fn season_of_month(month: u32) -> Season {
    match month {
        3..=5 => Season::Spring,
        6..=8 => Season::Summer,
        9..=11 => Season::Autumn,
        _ => Season::Winter,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_time_reads_as_a_log_timestamp_and_an_hour_of_the_day() {
        let one_billion_seconds_after_1970 =
            LocalTime { year: 2001, month: 9, day: 9, hour: 1, minute: 46, second: 40, millisecond: 7 };
        assert_eq!(one_billion_seconds_after_1970.as_log_timestamp(), "2001-09-09 01:46:40.007");
        assert!((one_billion_seconds_after_1970.as_atmosphere_clock().hour - 1.7777).abs() < 0.001);
    }
}
