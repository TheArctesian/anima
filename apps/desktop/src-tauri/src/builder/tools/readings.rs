//! What the tool check reads from this computer at the OS edge (BLD-028, BLD-083): the signing
//! identities in the login keychain, the identity this app was built to sign with, and the
//! sysroot of the Rust toolchain the steps run. The decision on these readings is in `tools`.

use std::path::{Path, PathBuf};

use crate::builder::environment::Environment;

/// The lines `security find-identity -v -p codesigning` prints for the login keychain; none when
/// it cannot run (BLD-083).
pub fn keychain_identities() -> Vec<String> {
    std::process::Command::new("/usr/bin/security")
        .args(["find-identity", "-v", "-p", "codesigning"])
        .output()
        .map_or_default(|out| {
            String::from_utf8_lossy(&out.stdout)
                .lines()
                .map(str::to_owned)
                .collect()
        })
}

/// The identity `bundle.macOS.signingIdentity` names in the `tauri.conf.json` this app was built
/// from (BLD-083). Tauri keeps no bundle settings in the running app's config, so the file is
/// read at compile time.
pub fn signing_identity() -> Option<String> {
    let config: serde_json::Value =
        serde_json::from_str(include_str!("../../../tauri.conf.json")).ok()?;
    config["bundle"]["macOS"]["signingIdentity"]
        .as_str()
        .map(str::to_owned)
}

/// What `rustc --print sysroot` prints for the toolchain the build steps run: the `rustc` found
/// in the steps' program folders, with the steps' environment, in `folder` (rustup reads a folder
/// override there). Correct for a rustup proxy and for a system toolchain alike. Rustup may not
/// install a toolchain for it (`RUSTUP_AUTO_INSTALL=0`). `None` when `rustc` is not found or fails.
pub fn rust_sysroot(env: &Environment, folder: &Path) -> Option<PathBuf> {
    let rustc = env.find("rustc", Path::is_file)?;
    let out = std::process::Command::new(rustc)
        .args(["--print", "sysroot"])
        .env_clear()
        .envs(env.for_step(&[("RUSTUP_AUTO_INSTALL", "0".to_owned())]))
        .current_dir(folder)
        .stdin(std::process::Stdio::null())
        .output()
        .ok()?;
    let text = String::from_utf8(out.stdout).ok()?;
    let line = text.lines().next()?.trim();
    (out.status.success() && Path::new(line).is_absolute()).then(|| PathBuf::from(line))
}
