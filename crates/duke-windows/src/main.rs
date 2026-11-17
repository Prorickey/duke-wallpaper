#![cfg_attr(windows, windows_subsystem = "windows")]

#[cfg(windows)]
mod host;

#[cfg(windows)]
fn main() {
    std::process::exit(host::run());
}

#[cfg(not(windows))]
fn main() {
    eprintln!("duke-windows is the Windows build of the wallpaper; on a Mac build and run duke-mac instead.");
    std::process::exit(1);
}
