//! The build steps, each a fixed run of planned commands in the builder's clone (BLD-040 to
//! BLD-047). Every command runs under the step's claim, so one step runs at a time and a cancel
//! stops it.

use std::{
    ffi::OsString,
    fs, io,
    path::{Path, PathBuf},
};

use super::{
    artifact,
    carried::{mach_o::Signer, tree},
    child::Claim,
    environment::Environment,
    error::BuilderError,
    job::{Commit, Target, Version},
    plan::{self, CLONE_CONFIG_KEYS, Command, Program, Repository},
};

/// The claim, the environment, the repository and the clone of one step.
pub struct Workshop<'a> {
    pub claim: Claim<'a>,
    pub env: Environment,
    pub repository: Repository,
    pub clone: PathBuf,
}

/// The files in `.git` that make git read objects, parents or history from somewhere other than
/// the clone's own objects, or that hide a changed or untracked file from `git status`. The
/// checkout removes `info/exclude`, which `git clone` writes; a build may write any (D-15, D-21).
const UNTRUSTED_FILES: &[&str] = &[
    "info/grafts",
    "info/exclude",
    "info/attributes",
    "info/sparse-checkout",
    "shallow",
    "commondir",
    "objects/info/alternates",
    "objects/info/http-alternates",
    "objects/info/commit-graph",
    "objects/info/commit-graphs",
];

/// The files in `.git` the checkout and each later step remove: the index, which a build can mark
/// to hide a changed file (`--skip-worktree`, `--assume-unchanged`), and the exclude rules,
/// which hide an untracked one. git builds the index anew from the commit.
const FORGOTTEN_FILES: &[&str] = &["index", "info/exclude"];

/// Whether every key `git config --local --name-only --list` printed is one `git clone` writes.
pub fn config_is_clones(keys: &[String]) -> bool {
    keys.iter()
        .all(|key| CLONE_CONFIG_KEYS.contains(&key.to_ascii_lowercase().as_str()))
}

impl Workshop<'_> {
    /// Runs one command and returns the lines of its kept stream.
    pub(super) async fn run(&self, command: &Command) -> Result<Vec<String>, BuilderError> {
        let program = self.program(command.program)?;
        self.claim
            .run(command, &program, self.variables(command))
            .await
    }

    /// Where `program` is: its fixed path (`/bin/sh`), or the first in the step's program folders.
    pub fn program(&self, program: Program) -> Result<PathBuf, BuilderError> {
        match program.fixed_path() {
            Some(path) => path.is_file().then(|| path.to_owned()),
            None => self.env.find(program.name(), Path::is_file),
        }
        .ok_or(BuilderError::ToolMissing)
    }

    /// The environment of `command`: the step's own, and cargo's output folder inside the clone,
    /// whatever the builder's cargo configuration names (BLD-044).
    pub fn variables(&self, command: &Command) -> Vec<(OsString, OsString)> {
        let mut variables = self.env.for_step(&command.env);
        variables.push((
            "CARGO_TARGET_DIR".into(),
            artifact::target_folder(&self.clone).into(),
        ));
        variables
    }

    async fn run_all(&self, commands: &[Command]) -> Result<(), BuilderError> {
        for command in commands {
            self.run(command).await?;
        }
        Ok(())
    }

    /// Whether the clone is one `git clone` made: a `.git` folder, none of the files that make
    /// git read history from elsewhere, and no config key a build could have added to redirect
    /// or rewrite it (D-15, D-21).
    async fn trusted_clone(&self) -> bool {
        let git = self.clone.join(".git");
        if !git.is_dir() || UNTRUSTED_FILES.iter().any(|file| git.join(file).exists()) {
            return false;
        }
        self.run(&plan::config_keys(&self.clone))
            .await
            .is_ok_and(|keys| config_is_clones(&keys))
    }

    /// Removes `FORGOTTEN_FILES` from the clone's `.git`; a link there is removed as a link.
    fn forget_index(&self) -> Result<(), BuilderError> {
        for file in FORGOTTEN_FILES {
            let path = self.clone.join(".git").join(file);
            match fs::symlink_metadata(&path) {
                Ok(kind) if kind.is_dir() => fs::remove_dir_all(&path),
                Ok(_) => fs::remove_file(&path),
                Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
                Err(error) => Err(error),
            }
            .map_err(|_| BuilderError::Failed)?;
        }
        Ok(())
    }

    /// Deletes the clone, if there is one, and clones the repository anew.
    async fn clone_anew(&self) -> Result<(), BuilderError> {
        if self.clone.exists() {
            fs::remove_dir_all(&self.clone).map_err(|_| BuilderError::Failed)?;
        }
        let parent = self.clone.parent().ok_or(BuilderError::Failed)?;
        fs::create_dir_all(parent).map_err(|_| BuilderError::Failed)?;
        self.run(&plan::clone(&self.repository, &self.clone))
            .await?;
        Ok(())
    }

    /// A trusted clone with `main` fetched by URL, whose objects `git fsck` finds whole. A clone
    /// that fails either check is cloned anew once; a fresh clone that fails `fsck` is refused
    /// (D-15, D-21).
    async fn whole_clone(&self) -> Result<(), BuilderError> {
        let fresh = !self.trusted_clone().await;
        if fresh {
            self.clone_anew().await?;
        }
        self.run(&plan::fetch(&self.repository, &self.clone))
            .await?;
        match self.run(&plan::fsck(&self.clone)).await {
            Err(BuilderError::StepFailed) if !fresh => {}
            other => return other.map(drop),
        }
        self.clone_anew().await?;
        self.run(&plan::fetch(&self.repository, &self.clone))
            .await?;
        self.run(&plan::fsck(&self.clone)).await.map(drop)
    }

    /// Makes the clone whole, refuses a commit that is not on `main`, then checks the commit out
    /// with a clean tree (BLD-040, BLD-041).
    pub async fn checkout(&self, commit: &Commit) -> Result<(), BuilderError> {
        self.whole_clone().await?;
        match self.run(&plan::ancestry(&self.clone, commit)).await {
            Err(BuilderError::StepFailed) => return Err(BuilderError::NotOnMain),
            other => other?,
        };
        self.forget_index()?;
        self.run_all(&plan::checkout(&self.clone, commit)).await?;
        if self.run(&plan::status(&self.clone)).await?.is_empty() {
            Ok(())
        } else {
            Err(BuilderError::Dirty)
        }
    }

    /// Fails unless the clone is still one `git clone` made, at `commit`, with a clean tree, so a
    /// later step cannot build another tree than the one checked out. The index is built anew
    /// from the commit first, so no mark a build put on an entry hides a change.
    pub(super) async fn at(&self, commit: &Commit) -> Result<(), BuilderError> {
        if !self.trusted_clone().await {
            return Err(BuilderError::Dirty);
        }
        let head = self.run(&plan::head(&self.clone)).await?;
        if !matches!(head.as_slice(), [line] if line == commit.as_str()) {
            return Err(BuilderError::Dirty);
        }
        self.forget_index()?;
        self.run(&plan::read_tree(&self.clone, commit)).await?;
        if self.run(&plan::status(&self.clone)).await?.is_empty() {
            Ok(())
        } else {
            Err(BuilderError::Dirty)
        }
    }

    /// `bun run gates` at `commit` (BLD-042).
    pub async fn gates(&self, commit: &Commit) -> Result<(), BuilderError> {
        self.at(commit).await?;
        self.run(&plan::gates(&self.clone, commit)).await?;
        Ok(())
    }

    /// Notarizes `dmg`, then staples and validates the ticket on it and on `app`, so the
    /// updater file is made from the stapled app (BLD-084, BLD-085). A submission Apple does not
    /// accept fails the step; its status is in the step's log.
    async fn notarize(&self, dmg: &Path, app: &Path) -> Result<(), BuilderError> {
        if !artifact::notarized(&self.run(&plan::notarize(dmg)).await?) {
            return Err(BuilderError::StepFailed);
        }
        self.run_all(&plan::staple(dmg)).await?;
        self.run_all(&plan::staple(app)).await
    }

    /// Builds what the release carries, compiles and bundles `target` at `version` with it,
    /// checks, notarizes and staples a macOS bundle, and returns the updater file (INS-002,
    /// BLD-043, BLD-047, BLD-083 to BLD-085, S12 2.2). In this order: the clone at `commit`; the
    /// signing identity; the PostgreSQL tree; `anima-link`; then `cargo build`, whose build
    /// scripts run code from the commit; the committed `tauri.conf.json` again; then Tauri.
    pub async fn bundle(
        &self,
        commit: &Commit,
        target: Target,
        version: &Version,
    ) -> Result<PathBuf, BuilderError> {
        self.at(commit).await?;
        let signer = self.signer(commit, target).await?;
        self.tree(commit, target, signer.as_ref()).await?;
        self.link(commit, target).await?;
        self.rust_programs(target, version).await?;
        self.committed(commit, &[tree::CONFIG]).await?;
        artifact::clear_bundle(&self.clone, target)?;
        let identity = signer.as_ref().map(|signer| signer.identity.as_str());
        self.run_all(&plan::bundle(&self.clone, target, version, identity))
            .await?;
        artifact::stabilize_names(&self.clone, target, version)?;
        let updater = artifact::updater_file(&self.clone, target, version);
        if let Some(signer) = &signer {
            self.release_mac(version, signer, &updater).await?;
        }
        artifact::own_file(&self.clone, &updater)
    }

    /// Checks the signed app (UPD-011): exactly the expected programs, `codesign --verify --deep
    /// --strict` of the app, its own executable universal, then every Mach-O file of it (S12
    /// 2.3). Then notarizes and staples the DMG and the app and archives the app as `updater`.
    async fn release_mac(
        &self,
        version: &Version,
        signer: &Signer,
        updater: &Path,
    ) -> Result<(), BuilderError> {
        let app = artifact::own_folder(&self.clone, &artifact::mac_app(&self.clone))?;
        let executable = artifact::mac_executable(&self.clone, &app)?;
        let [codesign, lipo] = plan::bundle_checks(&app, &executable);
        self.run(&codesign).await?;
        if !artifact::universal(&self.run(&lipo).await?) {
            return Err(BuilderError::Bundle);
        }
        self.signed_mach_o(&app, &signer.team).await?;
        let dmg = artifact::dmg_file(&self.clone, Target::DarwinUniversal, version)
            .ok_or(BuilderError::Bundle)?;
        self.notarize(&artifact::own_file(&self.clone, &dmg)?, &app)
            .await?;
        self.run(&plan::app_archive(&app, updater)).await.map(drop)
    }
}
