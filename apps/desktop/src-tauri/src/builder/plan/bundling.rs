//! Platform-specific compilation, packaging and signing plans.
use super::{Command, Program, command};
use crate::builder::{
    carried,
    job::{Target, Version},
};
use std::{ffi::OsString, path::Path};

fn tauri(clone: &Path, args: &[&str], config: &serde_json::Value) -> Command {
    let mut all = vec!["run", "--cwd", "apps/desktop", "tauri"];
    all.extend_from_slice(args);
    let mut step = command(Program::Bun, clone, &all);
    step.args.push("--config".into());
    step.args.push(config.to_string().into());
    step
}

/// Compile with `--no-bundle`, then bundle, with the version in `--config` and no updater
/// artifact: the builder signs nothing, and the hub signs the updater file (BLD-043, R-117). On
/// macOS Tauri signs the app with `identity`, the one this builder was built with, also in
/// `--config` (BLD-083). Both carry the seven programs and the PostgreSQL tree the carried steps
/// put in place (INS-002, S12 2.1).
pub fn bundle(
    clone: &Path,
    target: Target,
    version: &Version,
    identity: Option<&str>,
) -> Vec<Command> {
    let mut carried = carried::bundle_settings();
    if let (Target::DarwinUniversal, Some(identity)) = (target, identity) {
        carried["macOS"] = serde_json::json!({ "signingIdentity": identity });
    }
    let mut bundled = carried.clone();
    bundled["createUpdaterArtifacts"] = false.into();
    let compile_config = serde_json::json!({ "version": version.as_str(), "bundle": carried });
    let bundle_config = serde_json::json!({ "version": version.as_str(), "bundle": bundled });
    match target {
        Target::LinuxX86_64 => {
            let mut bundle = tauri(
                clone,
                &["bundle", "--bundles", "appimage", "--no-sign"],
                &bundle_config,
            );
            bundle.env.push(("APPIMAGE_EXTRACT_AND_RUN", "1".into()));
            vec![
                tauri(clone, &["build", "--no-bundle"], &compile_config),
                bundle,
            ]
        }
        Target::WindowsX86_64 => windows_bundle(clone, &compile_config, &bundle_config),
        Target::DarwinUniversal => {
            let universal = ["--target", "universal-apple-darwin"];
            let bundle = tauri(
                clone,
                &["bundle", universal[0], universal[1], "--bundles", "app,dmg"],
                &bundle_config,
            );
            let compile = ["build", "--no-bundle", universal[0], universal[1]];
            vec![tauri(clone, &compile, &compile_config), bundle]
        }
    }
}

/// The macOS bundle checks of UPD-011 (BLD-047): the signature, then the architectures of the
/// app's binary, whose output must name exactly `x86_64` and `arm64`.
pub fn bundle_checks(app: &Path, binary: &Path) -> [Command; 2] {
    let folder = app.parent().unwrap_or(app);
    let mut codesign = command(
        Program::Codesign,
        folder,
        &["--verify", "--deep", "--strict"],
    );
    codesign.args.push(app.as_os_str().to_owned());
    let mut lipo = command(Program::Lipo, folder, &["-archs"]);
    lipo.args.push(binary.as_os_str().to_owned());
    [codesign, lipo]
}

/// The `notarytool` keychain profile that holds the Apple credentials; they never reach a step's
/// environment (BLD-084, R-116).
const NOTARY_PROFILE: &str = "anima-notary";

/// Sends the DMG to Apple's notary service and waits for its answer (BLD-084). `notarytool`
/// exits 0 for a rejected submission too, so the output's `status:` line decides (BLD-085).
pub fn notarize(dmg: &Path) -> Command {
    let folder = dmg.parent().unwrap_or(dmg);
    let mut notarize = command(Program::Xcrun, folder, &["notarytool", "submit"]);
    notarize.args.push(dmg.as_os_str().to_owned());
    notarize.args.extend(
        ["--keychain-profile", NOTARY_PROFILE, "--wait"]
            .iter()
            .map(OsString::from),
    );
    notarize
}

/// Staples the notarization ticket to `bundle`, a DMG or an app, then checks it is there
/// (BLD-084, UPD-011).
pub fn staple(bundle: &Path) -> [Command; 2] {
    let folder = bundle.parent().unwrap_or(bundle);
    [["stapler", "staple"], ["stapler", "validate"]].map(|args| {
        let mut stapler = command(Program::Xcrun, folder, &args);
        stapler.args.push(bundle.as_os_str().to_owned());
        stapler
    })
}

/// The macOS updater file: the app folder as `<app>.tar.gz`, with no `AppleDouble` files.
pub fn app_archive(app: &Path, archive: &Path) -> Command {
    let folder = app.parent().unwrap_or(app);
    let mut tar = command(Program::Tar, folder, &["-czf"]);
    tar.args.push(archive.as_os_str().to_owned());
    tar.args.push("-C".into());
    tar.args.push(folder.as_os_str().to_owned());
    tar.args
        .push(app.file_name().unwrap_or_default().to_owned());
    tar.env.push(("COPYFILE_DISABLE", "1".into()));
    tar
}

fn windows_bundle(
    clone: &Path,
    compile_config: &serde_json::Value,
    bundle_config: &serde_json::Value,
) -> Vec<Command> {
    let windows = [
        "--target",
        "x86_64-pc-windows-msvc",
        "--config",
        "src-tauri/tauri.windows.conf.json",
    ];
    vec![
        tauri(
            clone,
            &[
                "build",
                "--no-bundle",
                windows[0],
                windows[1],
                windows[2],
                windows[3],
            ],
            compile_config,
        ),
        tauri(
            clone,
            &[
                "bundle",
                "--bundles",
                "nsis",
                windows[0],
                windows[1],
                windows[2],
                windows[3],
            ],
            bundle_config,
        ),
    ]
}
