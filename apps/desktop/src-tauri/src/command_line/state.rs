//! The state the Settings row shows (INS-006): not installed, installed at a path, or installed
//! but pointing at another app or `AppImage`. Pure: what each candidate folder holds is a parameter.

use std::path::{Path, PathBuf};

use serde::Serialize;

use super::{
    carrier::{self, Carrier},
    directory::{PROFILE_LINE, Proposal},
    error::Occupant,
    found::Found,
};

/// How what is at `<folder>/anima` relates to this app.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fit {
    Absent,
    /// The link or launcher of this app.
    Current,
    /// A link or launcher an Anima app made, for another app bundle or `AppImage` path.
    Elsewhere(PathBuf),
    /// Not made by an Anima app: never replaced or removed.
    Foreign(Occupant),
}

/// How `found` fits `carrier`. A link to the `anima` of some `.app` bundle and a launcher with
/// the app's mark are an Anima app's own; anything else is not.
pub fn fit(found: Found, carrier: &Carrier) -> Fit {
    match (found, carrier) {
        (Found::Nothing, _) => Fit::Absent,
        (Found::Link(target), Carrier::Bundle { program }) if &target == program => Fit::Current,
        (Found::Launcher(file), Carrier::AppImage { file: ours }) if &file == ours => Fit::Current,
        (Found::Link(target), _) if carrier::is_bundle_program(&target) => Fit::Elsewhere(target),
        (Found::Launcher(file), _) => Fit::Elsewhere(file),
        (Found::Link(target), _) => Fit::Foreign(Occupant::Link(target)),
        (Found::Other(occupant), _) => Fit::Foreign(occupant),
    }
}

/// The row's state. Crosses to the client as `{ "state": "...", ... }` in camelCase.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "state",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
pub enum Status {
    /// This app cannot install it here; `reason` is a [`super::CommandLineError`] code.
    Unavailable {
        reason: &'static str,
    },
    /// `path` is where Install puts it.
    NotInstalled {
        folder: String,
        path: String,
        on_path: bool,
        create: bool,
        profile_line: Option<&'static str>,
    },
    Installed {
        path: String,
    },
    /// `path`, in `folder`, is a link or launcher an Anima app made for `points_at`, not this
    /// app. Install there points it at this app.
    Elsewhere {
        folder: String,
        path: String,
        points_at: String,
    },
}

fn shown(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

/// The state from the fit of each candidate's `anima`, in candidate order: this app's own first,
/// then another Anima app's, else not installed with the proposal.
pub fn status(fits: &[(PathBuf, Fit)], proposal: &Proposal) -> Status {
    if let Some((path, _)) = fits.iter().find(|(_, fit)| *fit == Fit::Current) {
        return Status::Installed { path: shown(path) };
    }
    if let Some((path, Fit::Elsewhere(target))) = fits
        .iter()
        .find(|(_, fit)| matches!(fit, Fit::Elsewhere(_)))
    {
        return Status::Elsewhere {
            folder: path.parent().map_or_else(String::new, shown),
            path: shown(path),
            points_at: shown(target),
        };
    }
    Status::NotInstalled {
        folder: shown(&proposal.folder),
        path: shown(&proposal.folder.join(carrier::PROGRAM)),
        on_path: proposal.on_path,
        create: proposal.create,
        profile_line: (!proposal.on_path).then_some(PROFILE_LINE),
    }
}
