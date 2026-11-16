//! The hub's programs after an update of the application (one-click plan S12). The hub runs a
//! copy of the programs, so a new application alone changes nothing for it. At each start the
//! shell runs `anima-local-hub update` from the application: that program replaces the copy
//! when the application carries other programs, and starts the hub on them. A hub that is not
//! installed, or that has the same programs, is left as it is. The database, the key and the
//! link's state are never touched.

use std::{ffi::OsString, path::Path};

use tauri_plugin_log::log;

use super::{Carried, Changing, INSTALL_LIMIT, LocalHubError, ask, carried};

pub(super) fn update_args(tree: &Path) -> [OsString; 3] {
    ["update".into(), "--tree".into(), tree.into()]
}

/// Whether the program replaced the hub's programs: it prints `{"updated":true}` or
/// `{"updated":false}`.
pub(super) fn parse_updated(printed: &[u8]) -> Result<bool, LocalHubError> {
    let answer: serde_json::Value =
        serde_json::from_slice(printed).map_err(|_| LocalHubError::Failed)?;
    answer
        .get("updated")
        .and_then(serde_json::Value::as_bool)
        .ok_or(LocalHubError::Failed)
}

/// Asks the carried program to update the hub, and tells whether it replaced the programs.
pub(super) fn update(carried: &Carried) -> Result<bool, LocalHubError> {
    let args = update_args(&carried.tree);
    parse_updated(&ask(carried, &args.each_ref(), INSTALL_LIMIT)?)
}

/// Updates the hub on this computer in the background, once, when this build carries the
/// programs. It holds the one change, so no install, remove, start or stop runs meanwhile.
pub fn at_start(changing: &Changing) {
    let Ok(carried) = carried() else {
        return;
    };
    let Ok(claim) = changing.claim() else {
        return;
    };
    tauri::async_runtime::spawn_blocking(move || {
        let _claim = claim;
        match update(&carried) {
            Ok(true) => log::info!("the hub on this computer got the programs of this version"),
            Ok(false) => {}
            Err(error) => log::warn!("the hub's programs were not updated: {}", error.code()),
        }
    });
}
