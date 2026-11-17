#[cfg(target_os = "macos")]
mod host;

#[cfg(target_os = "macos")]
fn main() {
    std::process::exit(host::run());
}

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("duke-mac is the macOS build of the wallpaper; on Windows build and run duke-windows instead.");
    std::process::exit(1);
}
