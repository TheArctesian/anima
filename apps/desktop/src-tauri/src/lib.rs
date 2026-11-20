//! Anima shell: the window and OS services only (REPO-003, MOB-001).
//! The UI is the static `apps/client` build; no business logic lives here.
//! One crate builds the desktop application and the mobile application. A module of a desktop
//! feature does not compile for a mobile target (MOB-003).

#[cfg(desktop)]
mod builder;
#[cfg(desktop)]
mod carried;
mod cloud;
#[cfg(desktop)]
#[cfg_attr(windows, path = "command_line/windows.rs")]
mod command_line;
#[cfg(desktop)]
mod daemon;
#[cfg(desktop)]
#[cfg_attr(target_os = "windows", path = "desktop/windows.rs")]
mod desktop;
#[cfg(desktop)]
#[cfg_attr(windows, path = "hub_listener/windows.rs")]
mod hub_listener;
#[cfg(desktop)]
mod keystore;
#[cfg(desktop)]
mod link;
#[cfg(desktop)]
mod local_hub;
#[cfg(desktop)]
mod mcp_agents;
mod mcp_permissions;
#[cfg(mobile)]
mod memory_tokens;
#[cfg(mobile)]
#[cfg_attr(
    target_os = "android",
    expect(
        clippy::too_many_arguments,
        reason = "the Android binding that Tauri generates for the entry point has five arguments"
    )
)]
mod mobile;
#[cfg(desktop)]
mod os_program;
#[cfg(any(mobile, test))]
mod phone_browser;
#[cfg(mobile)]
mod phone_link;
#[cfg(desktop)]
mod process_log;
#[cfg(desktop)]
mod stay_awake;
#[cfg(desktop)]
mod system_browser;
#[cfg(all(desktop, unix))]
mod system_settings;
#[cfg(desktop)]
mod this_machine;
#[cfg(desktop)]
mod user_folders;
#[cfg(desktop)]
mod web_view;

/// Starts the desktop application and returns when it ends. A phone starts at the entry
/// point in `mobile`, which the operating system calls.
#[cfg(desktop)]
pub fn run() {
    #[cfg(windows)]
    if stay_awake::guard_entry() {
        return;
    }
    desktop::run();
}
