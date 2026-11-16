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

/// What the program did with the hub's programs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Updated {
    /// The hub runs the programs of this application now.
    Replaced,
    /// Nothing to do: no hub is installed, or it has these programs.
    Unchanged,
    /// The hub has programs of a higher version, and they stay.
    KeptNewer,
}

/// What the program answered: `{"updated":true}`, `{"updated":false}` or
/// `{"updated":false,"older":true}` when this application is older than the hub's programs.
pub(super) fn parse_updated(printed: &[u8]) -> Result<Updated, LocalHubError> {
    let answer: serde_json::Value =
        serde_json::from_slice(printed).map_err(|_| LocalHubError::Failed)?;
    let flag = |name| answer.get(name).and_then(serde_json::Value::as_bool);
    match (flag("updated"), flag("older")) {
        (Some(true), _) => Ok(Updated::Replaced),
        (Some(false), Some(true)) => Ok(Updated::KeptNewer),
        (Some(false), _) => Ok(Updated::Unchanged),
        (None, _) => Err(LocalHubError::Failed),
    }
}

/// Asks the carried program to update the hub, and tells what it did.
pub(super) fn update(carried: &Carried) -> Result<Updated, LocalHubError> {
    let args = update_args(&carried.tree);
    parse_updated(&ask(carried, &args.each_ref(), INSTALL_LIMIT)?)
}

/// The update as one piece of work that holds the one change until it ends, or `None` when
/// another change runs: an install, remove, start or stop must not run beside it.
pub(super) fn claimed(
    changing: &Changing,
    carried: Carried,
) -> Option<impl FnOnce() -> Result<Updated, LocalHubError> + Send + 'static> {
    let claim = changing.claim().ok()?;
    Some(move || {
        let _claim = claim;
        update(&carried)
    })
}

/// Updates the hub on this computer in the background, once, when this build carries the
/// programs.
pub fn at_start(changing: &Changing) {
    let Some(work) = carried()
        .ok()
        .and_then(|carried| claimed(changing, carried))
    else {
        return;
    };
    tauri::async_runtime::spawn_blocking(move || match work() {
        Ok(Updated::Replaced) => {
            log::info!("the hub on this computer got the programs of this version");
        }
        Ok(Updated::KeptNewer) => {
            log::info!("the hub on this computer has programs of a higher version; they stay");
        }
        Ok(Updated::Unchanged) => {}
        Err(error) => log::warn!("the hub's programs were not updated: {}", error.code()),
    });
}
