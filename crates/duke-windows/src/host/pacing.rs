use std::ptr::null;
use std::time::{Duration, Instant};
use windows_sys::Win32::Foundation::{CloseHandle, FILETIME, HANDLE, WAIT_FAILED};
use windows_sys::Win32::System::Threading::{
    CreateWaitableTimerExW, GetCurrentProcess, GetProcessTimes, SetWaitableTimer, INFINITE, TIMER_ALL_ACCESS,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{MsgWaitForMultipleObjectsEx, MWMO_INPUTAVAILABLE, QS_ALLINPUT};

const SUB_MILLISECOND_TIMER_WITHOUT_RAISING_SYSTEM_TIMER_RATE: u32 = 0x0000_0002;
const ORDINARY_TIMER: u32 = 0;
const NANOSECONDS_PER_TIMER_UNIT: u128 = 100;
const LONGEST_SLEEP_AFTER_A_FAILED_WAIT: Duration = Duration::from_millis(10);
const LONGEST_WAIT_WITHOUT_A_TIMER_MILLISECONDS: u128 = 60_000;

pub struct FrameTimer {
    waitable_timer: HANDLE,
    pub is_high_resolution: bool,
}

impl FrameTimer {
    pub fn new() -> Self {
        let high_resolution_timer = unsafe {
            CreateWaitableTimerExW(
                null(),
                null(),
                SUB_MILLISECOND_TIMER_WITHOUT_RAISING_SYSTEM_TIMER_RATE,
                TIMER_ALL_ACCESS,
            )
        };
        if !high_resolution_timer.is_null() {
            return FrameTimer { waitable_timer: high_resolution_timer, is_high_resolution: true };
        }
        let ordinary_timer = unsafe { CreateWaitableTimerExW(null(), null(), ORDINARY_TIMER, TIMER_ALL_ACCESS) };
        FrameTimer { waitable_timer: ordinary_timer, is_high_resolution: false }
    }

    pub fn sleep_until_deadline_or_window_message(&self, deadline: Option<Instant>) {
        let Some(deadline) = deadline else {
            let wait_result =
                unsafe { MsgWaitForMultipleObjectsEx(0, null(), INFINITE, QS_ALLINPUT, MWMO_INPUTAVAILABLE) };
            if wait_result == WAIT_FAILED {
                std::thread::sleep(LONGEST_SLEEP_AFTER_A_FAILED_WAIT);
            }
            return;
        };
        let time_left = deadline.saturating_duration_since(Instant::now());
        if time_left.is_zero() {
            return;
        }
        let timer_units_left = (time_left.as_nanos() / NANOSECONDS_PER_TIMER_UNIT).max(1) as i64;
        let negative_means_relative_to_now = -timer_units_left;
        let timer_is_armed = !self.waitable_timer.is_null()
            && unsafe { SetWaitableTimer(self.waitable_timer, &negative_means_relative_to_now, 0, None, null(), 0) }
                != 0;
        let wait_result = if timer_is_armed {
            unsafe { MsgWaitForMultipleObjectsEx(1, &self.waitable_timer, INFINITE, QS_ALLINPUT, MWMO_INPUTAVAILABLE) }
        } else {
            let milliseconds_left = time_left.as_millis().clamp(1, LONGEST_WAIT_WITHOUT_A_TIMER_MILLISECONDS) as u32;
            unsafe { MsgWaitForMultipleObjectsEx(0, null(), milliseconds_left, QS_ALLINPUT, MWMO_INPUTAVAILABLE) }
        };
        if wait_result == WAIT_FAILED {
            std::thread::sleep(time_left.min(LONGEST_SLEEP_AFTER_A_FAILED_WAIT));
        }
    }
}

impl Drop for FrameTimer {
    fn drop(&mut self) {
        if !self.waitable_timer.is_null() {
            unsafe { CloseHandle(self.waitable_timer) };
        }
    }
}

pub fn processor_time_used_by_this_process() -> Duration {
    let zero = FILETIME { dwLowDateTime: 0, dwHighDateTime: 0 };
    let (mut created_at, mut exited_at, mut time_in_kernel, mut time_in_user_code) = (zero, zero, zero, zero);
    unsafe {
        GetProcessTimes(
            GetCurrentProcess(),
            &mut created_at,
            &mut exited_at,
            &mut time_in_kernel,
            &mut time_in_user_code,
        )
    };
    let timer_units = |time: FILETIME| ((time.dwHighDateTime as u64) << 32) | time.dwLowDateTime as u64;
    let total_timer_units = timer_units(time_in_kernel) + timer_units(time_in_user_code);
    Duration::from_nanos(total_timer_units * NANOSECONDS_PER_TIMER_UNIT as u64)
}

pub struct FrameStatistics {
    pub measuring_since: Instant,
    processor_time_at_start: Duration,
    pub frames: u64,
    pub main_loop_turns: u64,
    time_rendering: Duration,
    time_blitting: Duration,
    slowest_frame: Duration,
}

impl FrameStatistics {
    pub fn start() -> Self {
        FrameStatistics {
            measuring_since: Instant::now(),
            processor_time_at_start: processor_time_used_by_this_process(),
            frames: 0,
            main_loop_turns: 0,
            time_rendering: Duration::ZERO,
            time_blitting: Duration::ZERO,
            slowest_frame: Duration::ZERO,
        }
    }

    pub fn record_frame(&mut self, time_rendering: Duration, time_blitting: Duration) {
        self.frames += 1;
        self.time_rendering += time_rendering;
        self.time_blitting += time_blitting;
        self.slowest_frame = self.slowest_frame.max(time_rendering + time_blitting);
    }

    pub fn summary(&self) -> String {
        let seconds_measured = self.measuring_since.elapsed().as_secs_f64().max(1e-6);
        let processor_seconds =
            processor_time_used_by_this_process().saturating_sub(self.processor_time_at_start).as_secs_f64();
        let frames_or_one = self.frames.max(1) as f64;
        format!(
            "{} frames in {:.1} s ({:.1} fps): render {:.2} ms, blit {:.2} ms avg, worst frame {:.2} ms; {} loop turns; process CPU {:.2}% of one core",
            self.frames,
            seconds_measured,
            self.frames as f64 / seconds_measured,
            self.time_rendering.as_secs_f64() * 1000.0 / frames_or_one,
            self.time_blitting.as_secs_f64() * 1000.0 / frames_or_one,
            self.slowest_frame.as_secs_f64() * 1000.0,
            self.main_loop_turns,
            processor_seconds / seconds_measured * 100.0,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sleeping_ends_at_the_deadline_without_spinning_or_hanging() {
        let frame_timer = FrameTimer::new();
        let started = Instant::now();
        let sleep_time = Duration::from_millis(60);
        for _ in 0..5 {
            let woke_at = Instant::now() + sleep_time;
            loop {
                frame_timer.sleep_until_deadline_or_window_message(Some(woke_at));
                if Instant::now() >= woke_at {
                    break;
                }
                crate::host::win32::dispatch_waiting_window_messages();
            }
        }
        let elapsed = started.elapsed();
        assert!(elapsed >= sleep_time * 5, "woke early: {elapsed:?}");
        assert!(elapsed < sleep_time * 5 + Duration::from_millis(400), "overslept: {elapsed:?}");

        let already_passed = Instant::now() - Duration::from_millis(5);
        let started = Instant::now();
        frame_timer.sleep_until_deadline_or_window_message(Some(already_passed));
        assert!(started.elapsed() < Duration::from_millis(5));
    }

    #[test]
    fn statistics_average_over_the_recorded_frames() {
        let mut statistics = FrameStatistics::start();
        statistics.record_frame(Duration::from_millis(4), Duration::from_millis(1));
        statistics.record_frame(Duration::from_millis(2), Duration::from_millis(1));
        statistics.main_loop_turns = 3;
        let summary = statistics.summary();
        assert!(summary.starts_with("2 frames in "), "{summary}");
        assert!(summary.contains("render 3.00 ms, blit 1.00 ms avg, worst frame 5.00 ms; 3 loop turns"), "{summary}");
    }
}
