//! The PostgreSQL tree a release carries (S3 T11, S12 2.2 step 4): the clone's
//! `database-tree/build.sh`, run by `/bin/sh` (D-2) in a work folder the builder makes anew for
//! each release (D-1), writes the tree to the folder `bundle.resources` names. On macOS the recipe
//! signs each Mach-O file of the tree with the identity of the clone's `tauri.conf.json` (D-5).
//! Pure.

use std::path::{Path, PathBuf};

use crate::builder::{
    error::BuilderError,
    job::Target,
    plan::{Command, Program, command},
};

/// The recipe, relative to the clone.
const RECIPE: &str = "apps/desktop/src-tauri/database-tree/build.sh";
/// The folder the recipe writes the tree to, relative to the clone: `database` beside
/// `tauri.conf.json`, which `bundle.resources` names.
const OUT: &str = "apps/desktop/src-tauri/database";
/// The recipe's work folder, beside the clone, so `git clean` of the clone never reaches it.
const WORK: &str = "database-tree";
/// The Tauri configuration that names the signing identity, relative to the clone.
pub const CONFIG: &str = "apps/desktop/src-tauri/tauri.conf.json";
/// The variable the recipe reads the signing identity from.
const IDENTITY_VARIABLE: &str = "SIGN_IDENTITY";

/// The recipe's work folder for `clone`: a sibling of the clone.
pub fn work_folder(clone: &Path) -> Result<PathBuf, BuilderError> {
    clone
        .parent()
        .map(|parent| parent.join(WORK))
        .ok_or(BuilderError::Bundle)
}

/// `/bin/sh <recipe> <target> <work> <out>` from the clone. On macOS `identity`, the one that
/// signs the release, is in `SIGN_IDENTITY`: the recipe makes an ad hoc tree without it, so an
/// empty or missing one is refused. Linux signs nothing.
pub fn build(
    clone: &Path,
    target: Target,
    work: &Path,
    identity: Option<&str>,
) -> Result<Command, BuilderError> {
    let recipe_target = match target {
        Target::LinuxX86_64 => "linux",
        Target::DarwinUniversal => "darwin-universal",
    };
    let mut sh = command(Program::Sh, clone, &[RECIPE, recipe_target]);
    sh.args.push(work.as_os_str().to_owned());
    sh.args.push(clone.join(OUT).into_os_string());
    if target == Target::DarwinUniversal {
        let identity = identity
            .filter(|name| !name.is_empty())
            .ok_or(BuilderError::Bundle)?;
        sh.env.push((IDENTITY_VARIABLE, identity.to_owned()));
    }
    Ok(sh)
}

/// `bundle.macOS.signingIdentity` of a `tauri.conf.json`, when it names one (BLD-083).
pub fn signing_identity(config: &serde_json::Value) -> Option<&str> {
    config["bundle"]["macOS"]["signingIdentity"]
        .as_str()
        .filter(|name| !name.is_empty())
}
