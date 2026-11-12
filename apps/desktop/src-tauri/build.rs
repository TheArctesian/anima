use tauri_build::{AppManifest, Attributes};

/// Generates `allow-*` permissions for the shell commands so the capability files list them.
const APP_COMMANDS: &[&str] = &[
    "access_token_read",
    "access_token_write",
    "access_token_clear",
    "builder_token_read",
    "builder_token_write",
    "builder_token_clear",
    "builder_tools_check",
    "builder_checkout",
    "builder_gates_run",
    "builder_bundle",
    "builder_cancel",
    "builder_release_files",
    "builder_upload",
    "process_log_folder_open",
    "sharing_settings_open",
    "this_machine_read",
    "agent_bundled",
    "agent_install",
    "system_browser_open",
    "web_view_pinning",
    "web_view_open",
    "web_view_place",
    "web_view_show",
    "web_view_go",
    "web_view_pin",
    "web_view_pop_out",
    "web_view_close",
    "web_certificate_read",
];

fn main() {
    tauri_build::try_build(
        Attributes::new().app_manifest(AppManifest::new().commands(APP_COMMANDS)),
    )
    .expect("failed to run tauri-build");
}
