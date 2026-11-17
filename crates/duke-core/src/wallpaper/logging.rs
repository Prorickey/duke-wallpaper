use crate::wallpaper::clock::LocalTime;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

const ROLL_OVER_AFTER_BYTES: u64 = 256 * 1024;

struct Log {
    file: Option<File>,
    path: Option<PathBuf>,
    bytes_in_file: u64,
    also_print_to_stderr: bool,
    local_time_now: fn() -> LocalTime,
}

fn time_not_yet_known() -> LocalTime {
    LocalTime { year: 0, month: 0, day: 0, hour: 0, minute: 0, second: 0, millisecond: 0 }
}

static LOG: Mutex<Log> = Mutex::new(Log {
    file: None,
    path: None,
    bytes_in_file: 0,
    also_print_to_stderr: false,
    local_time_now: time_not_yet_known,
});

fn lock_log_even_after_a_panic_elsewhere() -> MutexGuard<'static, Log> {
    LOG.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn open_for_appending_after_setting_a_large_file_aside(path: &Path) -> Option<(File, u64)> {
    if let Some(directory) = path.parent() {
        let _ = std::fs::create_dir_all(directory);
    }
    let bytes_already_there = std::fs::metadata(path).map(|metadata| metadata.len()).unwrap_or(0);
    if bytes_already_there > ROLL_OVER_AFTER_BYTES {
        let mut previous_log_path = path.as_os_str().to_owned();
        previous_log_path.push(".old");
        let _ = std::fs::remove_file(&previous_log_path);
        let _ = std::fs::rename(path, &previous_log_path);
    }
    let file = OpenOptions::new().create(true).append(true).open(path).ok()?;
    let bytes_in_file = file.metadata().map(|metadata| metadata.len()).unwrap_or(0);
    Some((file, bytes_in_file))
}

pub fn use_clock(local_time_now: fn() -> LocalTime) {
    lock_log_even_after_a_panic_elsewhere().local_time_now = local_time_now;
}

pub fn start(log_file_path: Option<&Path>, also_print_to_stderr: bool) {
    let mut log = lock_log_even_after_a_panic_elsewhere();
    log.also_print_to_stderr = also_print_to_stderr;
    let Some(path) = log_file_path else {
        return;
    };
    let Some((file, bytes_in_file)) = open_for_appending_after_setting_a_large_file_aside(path) else {
        return;
    };
    log.file = Some(file);
    log.bytes_in_file = bytes_in_file;
    log.path = Some(path.to_path_buf());
}

pub fn write_line(level: &str, message: std::fmt::Arguments) {
    let mut log = lock_log_even_after_a_panic_elsewhere();
    let timestamp = (log.local_time_now)().as_log_timestamp();
    let line = format!("{timestamp} {level:<5} {message}\n");
    if log.also_print_to_stderr {
        let _ = std::io::stderr().write_all(line.as_bytes());
    }
    if log.bytes_in_file > ROLL_OVER_AFTER_BYTES {
        if let Some(path) = log.path.clone() {
            log.file = None;
            if let Some((file, bytes_in_file)) = open_for_appending_after_setting_a_large_file_aside(&path) {
                log.file = Some(file);
                log.bytes_in_file = bytes_in_file;
            }
        }
    }
    let Some(file) = log.file.as_mut() else {
        return;
    };
    if file.write_all(line.as_bytes()).is_ok() {
        log.bytes_in_file += line.len() as u64;
    }
}

#[macro_export]
macro_rules! info {
    ($($message:tt)*) => { $crate::wallpaper::logging::write_line("INFO", format_args!($($message)*)) };
}

#[macro_export]
macro_rules! warn {
    ($($message:tt)*) => { $crate::wallpaper::logging::write_line("WARN", format_args!($($message)*)) };
}

pub fn record_panics_in_the_log() {
    let print_panic_as_usual = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic| {
        write_line("PANIC", format_args!("{panic}"));
        print_panic_as_usual(panic);
    }));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_large_log_is_set_aside_and_a_fresh_one_started() {
        let directory = std::env::temp_dir().join(format!("duke-wallpaper-log-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        let path = directory.join("nested").join("duke-wallpaper.log");

        let (mut file, bytes_in_file) = open_for_appending_after_setting_a_large_file_aside(&path).unwrap();
        assert_eq!(bytes_in_file, 0);
        file.write_all(&vec![b'x'; ROLL_OVER_AFTER_BYTES as usize + 1]).unwrap();
        drop(file);

        let (_, bytes_in_file) = open_for_appending_after_setting_a_large_file_aside(&path).unwrap();
        assert_eq!(bytes_in_file, 0);
        let previous_log_path = directory.join("nested").join("duke-wallpaper.log.old");
        assert_eq!(std::fs::metadata(previous_log_path).unwrap().len(), ROLL_OVER_AFTER_BYTES + 1);
        let _ = std::fs::remove_dir_all(&directory);
    }
}
