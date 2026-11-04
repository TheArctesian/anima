use tauri_build::{AppManifest, Attributes};

/// Generates `allow-*` permissions for the key-store commands so the capability file lists them.
const APP_COMMANDS: &[&str] = &["access_token_read", "access_token_write", "access_token_clear"];

fn main() {
    tauri_build::try_build(Attributes::new().app_manifest(AppManifest::new().commands(APP_COMMANDS)))
        .expect("failed to run tauri-build");
}
