//! The session bus `systemctl --user` reaches the user's manager through (MET-019): this
//! shell's `XDG_RUNTIME_DIR`, else the user's own `/run/user/<uid>`.

use std::fs;
use std::os::unix::fs::MetadataExt as _;

/// Where each user's runtime folder (and so their session bus) is, by user ID.
const RUNTIME_ROOT: &str = "/run/user";

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
#[error(
    "no user session bus: run from a login session, or enable lingering with `loginctl enable-linger`"
)]
pub struct NoSessionBus;

/// The runtime folder `systemctl --user` finds the user's manager through: `set`
/// (`XDG_RUNTIME_DIR`) when it is not empty, else `/run/user/<uid>` when `owner` (that folder's
/// owner; `None` when it is not a folder) is `uid`.
///
/// # Errors
/// [`NoSessionBus`] when neither is there.
fn session_folder(
    set: Option<String>,
    uid: u32,
    owner: Option<u32>,
) -> Result<String, NoSessionBus> {
    match set.filter(|dir| !dir.is_empty()) {
        Some(dir) => Ok(dir),
        None if owner == Some(uid) => Ok(fallback_session_folder(uid)),
        None => Err(NoSessionBus),
    }
}

/// `/run/user/<uid>`: where logind puts a user's runtime folder.
fn fallback_session_folder(uid: u32) -> String {
    format!("{RUNTIME_ROOT}/{uid}")
}

/// The variables `systemctl --user` reaches the user's manager through.
pub(super) fn session_bus(
    env: impl Fn(&str) -> Option<String>,
    uid: u32,
) -> Result<Vec<(&'static str, String)>, NoSessionBus> {
    let fallback = fs::metadata(fallback_session_folder(uid))
        .ok()
        .filter(fs::Metadata::is_dir)
        .map(|folder| folder.uid());
    let folder = session_folder(env("XDG_RUNTIME_DIR"), uid, fallback)?;
    let mut vars = vec![("XDG_RUNTIME_DIR", folder)];
    if let Some(address) = env("DBUS_SESSION_BUS_ADDRESS").filter(|a| !a.is_empty()) {
        vars.push(("DBUS_SESSION_BUS_ADDRESS", address));
    }
    Ok(vars)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn met_019_systemctl_user_finds_the_session_bus_or_says_how_to_get_one() {
        let set = Some("/run/user/1000".to_owned());
        assert_eq!(
            session_folder(set, 1000, None).as_deref(),
            Ok("/run/user/1000")
        );
        assert_eq!(
            session_folder(None, 1000, Some(1000)).as_deref(),
            Ok("/run/user/1000")
        );
        assert_eq!(
            session_folder(Some(String::new()), 1000, Some(1000)).as_deref(),
            Ok("/run/user/1000")
        );
        assert_eq!(session_folder(None, 1000, Some(0)), Err(NoSessionBus));
        assert_eq!(session_folder(None, 1000, None), Err(NoSessionBus));
        assert_eq!(
            NoSessionBus.to_string(),
            "no user session bus: run from a login session, or enable lingering with `loginctl enable-linger`"
        );
    }
}
