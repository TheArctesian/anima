use tauri_build::{AppManifest, Attributes};

fn main() {
    tauri_build::try_build(
        Attributes::new().app_manifest(AppManifest::new().commands(&["spike_probe"])),
    )
    .expect("failed to run tauri-build");
}
