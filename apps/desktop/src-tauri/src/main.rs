//! Anima desktop shell: the main window and OS services only (REPO-003).
//! The UI is the static `apps/client` build; no business logic lives here.

mod keystore;

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            keystore::access_token_read,
            keystore::access_token_write,
            keystore::access_token_clear,
        ])
        .run(tauri::generate_context!())
        .expect("the Tauri application failed to start");
}
