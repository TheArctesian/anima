//! The menu bar or tray item (AWK-004): the three levels as check lines, and "Open Anima.ing".
//! It shows the same state as the Settings page: each change redraws both.

use tauri::{
    AppHandle, Manager,
    menu::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem},
    tray::TrayIconBuilder,
};

use super::{control::Status, level::Level, lid_set_up::Lid};
use crate::web_view::MAIN;

const OFF: &str = "stay-awake-off";
const AWAKE: &str = "stay-awake-awake";
const LID_CLOSED: &str = "stay-awake-lid-closed";
const OPEN: &str = "open";

/// The three level lines, kept so a change of state can redraw them.
pub struct Lines {
    off: CheckMenuItem<tauri::Wry>,
    awake: CheckMenuItem<tauri::Wry>,
    lid_closed: CheckMenuItem<tauri::Wry>,
}

/// The level a menu line chooses.
#[must_use]
pub fn level_of(id: &str) -> Option<Level> {
    match id {
        OFF => Some(Level::Off),
        AWAKE => Some(Level::Awake),
        LID_CLOSED => Some(Level::LidClosed),
        _ => None,
    }
}

/// The text of the lid-closed line: it says why it cannot be chosen.
#[must_use]
pub fn lid_text(lid: Lid) -> &'static str {
    match lid {
        Lid::Ready => "Keep awake with the lid closed",
        Lid::NeedsSetUp => "Keep awake with the lid closed (set up in Settings)",
        Lid::Unsupported => "Keep awake with the lid closed (not available on this desktop)",
    }
}

/// Makes the tray item with the lines for `status`.
///
/// # Errors
/// Tauri's error when the menu or the tray item could not be made.
pub fn build(app: &AppHandle, status: Status) -> tauri::Result<()> {
    let none: Option<&str> = None;
    let lines = Lines {
        off: CheckMenuItem::with_id(app, OFF, "Stay awake: off", true, false, none)?,
        awake: CheckMenuItem::with_id(app, AWAKE, "Keep awake", true, false, none)?,
        lid_closed: CheckMenuItem::with_id(
            app,
            LID_CLOSED,
            lid_text(status.lid),
            true,
            false,
            none,
        )?,
    };
    let open = MenuItem::with_id(app, OPEN, "Open Anima.ing", true, none)?;
    let menu = Menu::with_items(
        app,
        &[
            &lines.off,
            &lines.awake,
            &lines.lid_closed,
            &PredefinedMenuItem::separator(app)?,
            &open,
        ],
    )?;
    let mut tray = TrayIconBuilder::with_id("anima")
        .menu(&menu)
        .tooltip("Anima.ing")
        .on_menu_event(|app, event| on_menu_event(app, &event));
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app)?;
    app.manage(lines);
    show(app, status);
    Ok(())
}

/// Redraws the lines for `status`. Without a tray item (it could not be made) it does nothing.
pub fn show(app: &AppHandle, status: Status) {
    let Some(lines) = app.try_state::<Lines>() else {
        return;
    };
    // A line that could not be redrawn keeps its old mark until the next change.
    let _ = lines.off.set_checked(status.level == Level::Off);
    let _ = lines.awake.set_checked(status.level == Level::Awake);
    let _ = lines
        .lid_closed
        .set_checked(status.level == Level::LidClosed);
    let _ = lines.lid_closed.set_enabled(status.lid == Lid::Ready);
    let _ = lines.lid_closed.set_text(lid_text(status.lid));
}

fn on_menu_event(app: &AppHandle, event: &MenuEvent) {
    let id = event.id().as_ref();
    if let Some(level) = level_of(id) {
        super::choose_from_tray(app, level);
    } else if id == OPEN
        && let Some(window) = app.get_window(MAIN)
    {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}
