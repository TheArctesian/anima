//! Registers the native packages without exposing plugin commands to the web view.

fn main() {
    tauri_plugin::Builder::new(&[])
        .ios_path("ios")
        .android_path("android")
        .build();
}
