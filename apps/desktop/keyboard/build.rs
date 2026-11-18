//! Gives the app's build the Android library in `android/`. The plugin has no commands.

fn main() {
    tauri_plugin::Builder::new(&[])
        .android_path("android")
        .build();
}
