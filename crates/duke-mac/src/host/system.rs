use duke_core::wallpaper::clock::LocalTime;
use std::ffi::{c_char, c_void, CString};
use std::fs::File;
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};

const TIME_REMAINING_UNLIMITED_MEANS_PLUGGED_IN: f64 = -2.0;
const LAUNCH_AGENT_LABEL: &str = "edu.duke.pixel-art-background";
const UTF8_STRING_ENCODING: u32 = 0x0800_0100;

#[link(name = "IOKit", kind = "framework")]
extern "C" {
    fn IOPSGetTimeRemainingEstimate() -> f64;
}

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGMainDisplayID() -> u32;
    fn CGDisplayIsAsleep(display: u32) -> u32;
    fn CGSessionCopyCurrentDictionary() -> *const c_void;
}

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    fn CFStringCreateWithCString(allocator: *const c_void, text: *const c_char, encoding: u32) -> *const c_void;
    fn CFDictionaryGetValue(dictionary: *const c_void, key: *const c_void) -> *const c_void;
    fn CFBooleanGetValue(boolean: *const c_void) -> bool;
    fn CFRelease(object: *const c_void);
}

pub fn local_time_now() -> LocalTime {
    let mut time_since_1970 = libc::timeval { tv_sec: 0, tv_usec: 0 };
    let mut calendar: libc::tm = unsafe { std::mem::zeroed() };
    unsafe {
        libc::gettimeofday(&mut time_since_1970, std::ptr::null_mut());
        libc::localtime_r(&time_since_1970.tv_sec, &mut calendar);
    }
    LocalTime {
        year: (calendar.tm_year + 1900) as u32,
        month: (calendar.tm_mon + 1) as u32,
        day: calendar.tm_mday as u32,
        hour: calendar.tm_hour as u32,
        minute: calendar.tm_min as u32,
        second: calendar.tm_sec as u32,
        millisecond: (time_since_1970.tv_usec / 1000) as u32,
    }
}

pub fn running_on_battery() -> bool {
    unsafe { IOPSGetTimeRemainingEstimate() != TIME_REMAINING_UNLIMITED_MEANS_PLUGGED_IN }
}

pub fn main_display_is_asleep() -> bool {
    unsafe { CGDisplayIsAsleep(CGMainDisplayID()) != 0 }
}

pub fn screen_is_locked() -> bool {
    unsafe {
        let session = CGSessionCopyCurrentDictionary();
        if session.is_null() {
            return false;
        }
        let key_name = CString::new("CGSSessionScreenIsLocked").expect("no nul in the key");
        let key = CFStringCreateWithCString(std::ptr::null(), key_name.as_ptr(), UTF8_STRING_ENCODING);
        let value = CFDictionaryGetValue(session, key);
        let is_locked = !value.is_null() && CFBooleanGetValue(value);
        CFRelease(key);
        CFRelease(session);
        is_locked
    }
}

pub struct SingleInstanceLock {
    _file_held_open_while_locked: File,
}

pub fn claim_single_instance(lock_path: &Path) -> Option<SingleInstanceLock> {
    if let Some(directory) = lock_path.parent() {
        let _ = std::fs::create_dir_all(directory);
    }
    let file = File::create(lock_path).ok()?;
    let locked = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } == 0;
    locked.then_some(SingleInstanceLock { _file_held_open_while_locked: file })
}

fn launch_agent_path() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    Some(PathBuf::from(home).join("Library/LaunchAgents").join(format!("{LAUNCH_AGENT_LABEL}.plist")))
}

fn launch_agent_for(program: &Path) -> String {
    let program = program.display().to_string().replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;");
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>{LAUNCH_AGENT_LABEL}</string>
    <key>ProgramArguments</key>
    <array>
        <string>{program}</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>ProcessType</key>
    <string>Interactive</string>
</dict>
</plist>
"#
    )
}

pub fn opens_at_login() -> bool {
    launch_agent_path().is_some_and(|path| path.exists())
}

pub fn set_opens_at_login(enable: bool) -> std::io::Result<()> {
    let path = launch_agent_path().ok_or_else(|| std::io::Error::other("HOME is not set"))?;
    if !enable {
        return match std::fs::remove_file(&path) {
            Err(error) if error.kind() != std::io::ErrorKind::NotFound => Err(error),
            _ => Ok(()),
        };
    }
    if let Some(directory) = path.parent() {
        std::fs::create_dir_all(directory)?;
    }
    std::fs::write(&path, launch_agent_for(&std::env::current_exe()?))
}

pub fn open_in_text_editor(path: &Path) {
    let _ = std::process::Command::new("open").arg("-t").arg(path).spawn();
}
