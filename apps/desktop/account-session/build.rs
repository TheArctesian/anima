//! Gives the app's build the iOS package in `ios/`. The web view has no command of this plugin: only the shell calls it.

fn main() {
    tauri_plugin::Builder::new(&[]).ios_path("ios").build();
}
