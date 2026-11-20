use tauri_build::{AppManifest, Attributes};

/// Generates `allow-*` permissions for the shell commands so the capability files list them.
const APP_COMMANDS: &[&str] = &[
    "access_token_read",
    "access_token_write",
    "access_token_clear",
    "cloud_sign_in_start",
    "cloud_sign_in_poll",
    "cloud_sign_in_resume",
    "cloud_sign_out",
    "mobile_metrics_enrolled",
    "mobile_metrics_enrol",
    "mobile_metrics_push",
    "cloud_sign_in_session",
    "cloud_hub",
    "cloud_hub_access",
    "cloud_device",
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
    "mcp_agents_read",
    "mcp_agents_change",
    "mcp_permissions_read",
    "mcp_permissions_write",
    "process_log_folder_open",
    "sharing_settings_open",
    "this_machine_read",
    "daemon_bundled",
    "daemon_install",
    "local_hub_carried",
    "local_hub_install",
    "local_hub_remove",
    "local_hub_status",
    "local_hub_start",
    "local_hub_stop",
    "local_hub_account",
    "local_hub_totp_new",
    "local_hub_account_create",
    "local_hub_key_save",
    "link_join",
    "link_status",
    "link_leave",
    "hub_listener_check",
    "command_line_read",
    "command_line_install",
    "command_line_remove",
    "system_browser_open",
    "runner_capabilities_read",
    "stay_awake_read",
    "stay_awake_choose",
    "stay_awake_floor",
    "stay_awake_lid_set_up",
    "stay_awake_lid_remove",
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

/// Links the Go device link that `link/build-phone.sh` built for this phone target (mobile plan M3-1,
/// MOB-031): a static archive for iOS, a shared library for Android, which also goes beside the app's own
/// library in the Android project. A desktop target needs nothing: it runs the `anima-link` program.
fn link_phone_library() {
    let target = std::env::var("TARGET").unwrap_or_default();
    let os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if os != "ios" && os != "android" {
        return;
    }
    let manifest = std::path::PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR").expect("cargo sets CARGO_MANIFEST_DIR"),
    );
    let folder = manifest.join("../../../target/phonelink").join(&target);
    let file = if os == "ios" {
        "libphonelink.a"
    } else {
        "libphonelink.so"
    };
    println!("cargo:rerun-if-changed={}", folder.join(file).display());
    assert!(
        folder.join(file).is_file(),
        "the device link for {target} is not built: run link/build-phone.sh {target}"
    );
    println!("cargo:rustc-link-search=native={}", folder.display());
    if os == "ios" {
        println!("cargo:rustc-link-lib=static=phonelink");
        return;
    }
    println!("cargo:rustc-link-lib=dylib=phonelink");
    let libraries = manifest.join("gen/android/app/src/main/jniLibs/arm64-v8a");
    std::fs::create_dir_all(&libraries).expect("create the Android native libraries directory");
    std::fs::copy(folder.join(file), libraries.join(file))
        .expect("copy the phone link into the Android package");
}

fn main() {
    link_phone_library();
    tauri_build::try_build(
        Attributes::new().app_manifest(AppManifest::new().commands(APP_COMMANDS)),
    )
    .expect("failed to run tauri-build");
}
