//! The fixed commands of each step (BLD-027, BLD-040 to BLD-043, BLD-047).

use std::path::Path;

use super::COMMIT;
use crate::builder::{
    job::{Commit, Target, Version},
    plan::{self, Program},
};

fn args(command: &plan::Command) -> Vec<&str> {
    command.args.iter().map(|a| a.to_str().unwrap()).collect()
}

#[test]
fn bld_040_every_git_command_runs_with_hooks_and_helpers_off() {
    let clone = Path::new("/data/builds/anima");
    let commit = Commit::parse(COMMIT).unwrap();
    let mut commands = vec![
        plan::clone(clone),
        plan::ancestry(clone, &commit),
        plan::status(clone),
        plan::head(clone),
    ];
    commands.extend(plan::fetch(clone));
    commands.extend(plan::checkout(clone, &commit));
    for command in &commands {
        assert_eq!(command.program, Program::Git);
        let args = args(command);
        for setting in [
            "core.hooksPath=/dev/null",
            "core.fsmonitor=false",
            "credential.helper=",
            "protocol.file.allow=never",
        ] {
            assert!(args.contains(&setting), "{args:?}");
        }
    }
    let clone_args = args(&commands[0]);
    assert_eq!(
        clone_args[clone_args.len() - 4..],
        [
            "--no-checkout",
            "--",
            plan::REPOSITORY,
            "/data/builds/anima"
        ]
    );
    assert_eq!(commands[0].cwd, Path::new("/data/builds"));
}

#[test]
fn bld_041_ancestry_asks_git_whether_the_commit_is_on_main() {
    let commit = Commit::parse(COMMIT).unwrap();
    let command = plan::ancestry(Path::new("/c"), &commit);
    let args = args(&command);
    assert_eq!(
        args[args.len() - 4..],
        [
            "merge-base",
            "--is-ancestor",
            COMMIT,
            "refs/remotes/origin/main"
        ]
    );
    let fetch = plan::fetch(Path::new("/c"));
    assert_eq!(
        args_tail(&fetch[0], 4),
        ["remote", "set-url", "origin", plan::REPOSITORY]
    );
}

fn args_tail(command: &plan::Command, n: usize) -> Vec<&str> {
    let all = args(command);
    all[all.len() - n..].to_vec()
}

#[test]
fn bld_042_gates_run_with_base_ref_set_to_the_commit() {
    let commit = Commit::parse(COMMIT).unwrap();
    let gates = plan::gates(Path::new("/c"), &commit);
    assert_eq!(
        (gates.program, args(&gates)),
        (Program::Bun, vec!["run", "gates"])
    );
    assert_eq!(gates.env, [("BASE_REF", COMMIT.to_owned())]);
}

#[test]
fn bld_043_linux_compiles_then_bundles_an_appimage_with_the_version_in_config() {
    let version = Version::parse("0.1.12").unwrap();
    let [compile, bundle] = &plan::bundle(Path::new("/c"), Target::LinuxX86_64, &version)[..]
    else {
        panic!("two commands")
    };
    assert_eq!(
        args(compile),
        [
            "run",
            "--cwd",
            "apps/desktop",
            "tauri",
            "build",
            "--no-bundle",
            "--config",
            r#"{"version":"0.1.12"}"#
        ]
    );
    assert_eq!(
        args(bundle),
        [
            "run",
            "--cwd",
            "apps/desktop",
            "tauri",
            "bundle",
            "--bundles",
            "appimage",
            "--no-sign",
            "--config",
            r#"{"bundle":{"createUpdaterArtifacts":false},"version":"0.1.12"}"#
        ]
    );
    assert_eq!(bundle.env, [("APPIMAGE_EXTRACT_AND_RUN", "1".to_owned())]);
}

#[test]
fn bld_043_macos_builds_universal_and_bundles_app_and_dmg_ad_hoc_signed() {
    let version = Version::parse("0.1.12").unwrap();
    let commands = plan::bundle(Path::new("/c"), Target::DarwinUniversal, &version);
    assert_eq!(
        args(&commands[0])[4..8],
        ["build", "--no-bundle", "--target", "universal-apple-darwin"]
    );
    assert_eq!(
        args(&commands[1])[4..9],
        [
            "bundle",
            "--target",
            "universal-apple-darwin",
            "--bundles",
            "app,dmg"
        ]
    );
    assert_eq!(
        commands[1].env,
        [("APPLE_SIGNING_IDENTITY", "-".to_owned())]
    );
    let app = Path::new("/c/bundle/macos/Anima.app");
    let [codesign, lipo] = plan::bundle_checks(app, &app.join("Contents/MacOS/Anima"));
    assert_eq!(
        args(&codesign),
        [
            "--verify",
            "--deep",
            "--strict",
            "/c/bundle/macos/Anima.app"
        ]
    );
    assert_eq!(
        args(&lipo),
        ["-archs", "/c/bundle/macos/Anima.app/Contents/MacOS/Anima"]
    );
    let tar = plan::app_archive(app, Path::new("/c/bundle/macos/Anima.app.tar.gz"));
    assert_eq!(
        args(&tar),
        [
            "-czf",
            "/c/bundle/macos/Anima.app.tar.gz",
            "-C",
            "/c/bundle/macos",
            "Anima.app"
        ]
    );
}
