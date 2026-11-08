//! The fixed commands of each build step (BLD-027, BLD-040 to BLD-043, BLD-047). The web view
//! cannot name a program or an argument: it names a step, and each step is one of these plans
//! built from a checked `Commit`, `Version` and `Target`. Pure.

use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

use super::job::{Commit, Target, Version};

/// The only repository a builder builds (BLD-040).
pub const REPOSITORY: &str = "https://github.com/TheArctesian/anima.git";
/// Where `fetch` keeps `main` of `REPOSITORY`.
const MAIN: &str = "refs/remotes/origin/main";

/// The programs a step may start. Found in the step's program folders, never in the clone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Program {
    Git,
    Bun,
    Tar,
    Codesign,
    Lipo,
}

impl Program {
    pub fn name(self) -> &'static str {
        match self {
            Self::Git => "git",
            Self::Bun => "bun",
            Self::Tar => "tar",
            Self::Codesign => "codesign",
            Self::Lipo => "lipo",
        }
    }
}

/// One process of a step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Command {
    pub program: Program,
    pub args: Vec<OsString>,
    pub cwd: PathBuf,
    /// Variables the step adds to the scrubbed environment (BLD-044).
    pub env: Vec<(&'static str, String)>,
}

fn command(program: Program, cwd: &Path, args: &[&str]) -> Command {
    Command {
        program,
        args: args.iter().map(OsString::from).collect(),
        cwd: cwd.to_owned(),
        env: Vec::new(),
    }
}

/// git with hooks, fsmonitor, credential helpers, submodules and local transports off, whatever
/// the clone's own `.git/config` says: a build writes to the clone (BLD-044).
fn git(cwd: &Path, args: &[&str]) -> Command {
    let mut all = vec![
        "-c",
        "core.hooksPath=/dev/null",
        "-c",
        "core.fsmonitor=false",
        "-c",
        "credential.helper=",
        "-c",
        "protocol.file.allow=never",
        "-c",
        "protocol.ext.allow=never",
        "-c",
        "submodule.recurse=false",
        "-c",
        "advice.detachedHead=false",
    ];
    all.extend_from_slice(args);
    command(Program::Git, cwd, &all)
}

/// A fresh clone into `clone`, run from its parent folder. No checkout yet.
pub fn clone(clone: &Path) -> Command {
    let parent = clone.parent().unwrap_or(clone);
    let mut step = git(parent, &["clone", "--no-checkout", "--", REPOSITORY]);
    step.args.push(clone.as_os_str().to_owned());
    step
}

/// Points `origin` at `REPOSITORY` again, then fetches `main` (BLD-040).
pub fn fetch(clone: &Path) -> Vec<Command> {
    vec![
        git(clone, &["remote", "set-url", "origin", REPOSITORY]),
        git(
            clone,
            &[
                "fetch",
                "--no-tags",
                "--prune",
                "origin",
                &format!("+refs/heads/main:{MAIN}"),
            ],
        ),
    ]
}

/// Exits 0 only when `commit` is an ancestor of `main` (BLD-041).
pub fn ancestry(clone: &Path, commit: &Commit) -> Command {
    git(
        clone,
        &["merge-base", "--is-ancestor", commit.as_str(), MAIN],
    )
}

/// Checks out `commit` and removes every file git does not track, apart from cargo's `target/`
/// cache (BLD-040).
pub fn checkout(clone: &Path, commit: &Commit) -> Vec<Command> {
    vec![
        git(clone, &["checkout", "--force", "--detach", commit.as_str()]),
        git(clone, &["clean", "-ffdx", "--exclude=/target/"]),
    ]
}

/// Prints one line for each changed or untracked file; none means a clean tree.
pub fn status(clone: &Path) -> Command {
    git(clone, &["status", "--porcelain", "--untracked-files=all"])
}

/// Prints the commit the clone is at.
pub fn head(clone: &Path) -> Command {
    git(clone, &["rev-parse", "HEAD"])
}

/// `bun run gates` with `BASE_REF` set to the job commit (BLD-042). `DATABASE_URL` is never in
/// a step's environment, so the gates start their own PostgreSQL.
pub fn gates(clone: &Path, commit: &Commit) -> Command {
    let mut step = command(Program::Bun, clone, &["run", "gates"]);
    step.env.push(("BASE_REF", commit.as_str().to_owned()));
    step
}

fn tauri(clone: &Path, args: &[&str], config: &serde_json::Value) -> Command {
    let mut all = vec!["run", "--cwd", "apps/desktop", "tauri"];
    all.extend_from_slice(args);
    let mut step = command(Program::Bun, clone, &all);
    step.args.push("--config".into());
    step.args.push(config.to_string().into());
    step
}

/// Compile with `--no-bundle`, then bundle, with the version in `--config` and no updater
/// artifact: the shell signs the updater file itself (BLD-043, BLD-045).
pub fn bundle(clone: &Path, target: Target, version: &Version) -> Vec<Command> {
    let compile_config = serde_json::json!({ "version": version.as_str() });
    let bundle_config = serde_json::json!({
        "version": version.as_str(),
        "bundle": { "createUpdaterArtifacts": false }
    });
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
        Target::DarwinUniversal => {
            let universal = ["--target", "universal-apple-darwin"];
            let mut bundle = tauri(
                clone,
                &["bundle", universal[0], universal[1], "--bundles", "app,dmg"],
                &bundle_config,
            );
            // Ad-hoc signing until there is an Apple Developer ID (OD-13).
            bundle.env.push(("APPLE_SIGNING_IDENTITY", "-".into()));
            let compile = ["build", "--no-bundle", universal[0], universal[1]];
            vec![tauri(clone, &compile, &compile_config), bundle]
        }
    }
}

/// The macOS bundle checks of UPD-011 (BLD-047): the signature, then the architectures of the
/// app's binary, whose output must name `x86_64` and `arm64`.
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
