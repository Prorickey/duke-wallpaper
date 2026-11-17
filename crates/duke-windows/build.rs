use std::{env, fs, path::PathBuf};

const MANIFEST_DECLARING_PER_MONITOR_PIXELS_AND_LAYERED_CHILD_WINDOWS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0" xmlns:asmv3="urn:schemas-microsoft-com:asm.v3">
  <assemblyIdentity type="win32" name="DukePixelArtBackground" version="0.1.0.0"/>
  <compatibility xmlns="urn:schemas-microsoft-com:compatibility.v1">
    <application>
      <supportedOS Id="{8e0f7a12-bfb3-4fe8-b9a5-48fd50a15a9a}"/>
      <supportedOS Id="{1f676c76-80e1-4239-95bb-83d0f6d0da78}"/>
      <supportedOS Id="{4a2f28e3-53b9-4441-ba9c-d69d4a4a6e38}"/>
      <supportedOS Id="{35138b9a-5d96-4fbd-8e2d-a2440225f93a}"/>
    </application>
  </compatibility>
  <asmv3:application>
    <asmv3:windowsSettings>
      <dpiAware xmlns="http://schemas.microsoft.com/SMI/2005/WindowsSettings">true/pm</dpiAware>
      <dpiAwareness xmlns="http://schemas.microsoft.com/SMI/2016/WindowsSettings">PerMonitorV2, PerMonitor</dpiAwareness>
    </asmv3:windowsSettings>
  </asmv3:application>
</assembly>
"#;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    let target_operating_system = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let target_toolchain = env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    let linker_can_embed_a_manifest = target_operating_system == "windows" && target_toolchain == "msvc";
    if !linker_can_embed_a_manifest {
        return;
    }
    let build_output_directory = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let manifest_path = build_output_directory.join("duke-wallpaper.manifest");
    fs::write(&manifest_path, MANIFEST_DECLARING_PER_MONITOR_PIXELS_AND_LAYERED_CHILD_WINDOWS).expect("write manifest");
    println!("cargo:rustc-link-arg-bins=/MANIFEST:EMBED");
    println!("cargo:rustc-link-arg-bins=/MANIFESTINPUT:{}", manifest_path.display());
}
