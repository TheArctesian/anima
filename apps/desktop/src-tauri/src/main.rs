//! Anima desktop shell: the main window and OS services only (REPO-003).
//! The UI is the static `apps/client` build; no business logic lives here.

fn main() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("the Tauri application failed to start");
}
