//! The build steps, each a fixed run of planned commands in the builder's clone (BLD-040 to
//! BLD-047). Every command runs under the step's claim, so one step runs at a time and a cancel
//! stops it.

use std::{
    fs,
    path::{Path, PathBuf},
};

use super::{
    artifact,
    child::Claim,
    environment::Environment,
    error::BuilderError,
    job::{Commit, Target, Version},
    plan::{self, CLONE_CONFIG_KEYS, Command, Repository},
    sidecar,
};

/// The claim, the environment, the repository and the clone of one step.
pub struct Workshop<'a> {
    pub claim: Claim<'a>,
    pub env: Environment,
    pub repository: Repository,
    pub clone: PathBuf,
}

/// The files in `.git` that make git read objects, parents or history from somewhere other than
/// the clone's own objects. `git clone` writes none of them; a build may (D-15, D-21).
const UNTRUSTED_FILES: &[&str] = &[
    "info/grafts",
    "shallow",
    "commondir",
    "objects/info/alternates",
    "objects/info/http-alternates",
    "objects/info/commit-graph",
    "objects/info/commit-graphs",
];

/// Whether every key `git config --local --name-only --list` printed is one `git clone` writes.
pub fn config_is_clones(keys: &[String]) -> bool {
    keys.iter()
        .all(|key| CLONE_CONFIG_KEYS.contains(&key.to_ascii_lowercase().as_str()))
}

impl Workshop<'_> {
    /// Runs one command; the program is found in the step's program folders.
    async fn run(&self, command: &Command) -> Result<Vec<String>, BuilderError> {
        let program = self
            .env
            .find(command.program.name(), Path::is_file)
            .ok_or(BuilderError::ToolMissing)?;
        self.claim
            .run(command, &program, self.env.for_step(&command.env))
            .await
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
        self.run_all(&plan::checkout(&self.clone, commit)).await?;
        if self.run(&plan::status(&self.clone)).await?.is_empty() {
            Ok(())
        } else {
            Err(BuilderError::Dirty)
        }
    }

    /// Fails unless the clone is at `commit` with a clean tree, so a later step cannot build
    /// another tree than the one checked out.
    async fn at(&self, commit: &Commit) -> Result<(), BuilderError> {
        let head = self.run(&plan::head(&self.clone)).await?;
        let clean = self.run(&plan::status(&self.clone)).await?.is_empty();
        match head.as_slice() {
            [line] if line == commit.as_str() && clean => Ok(()),
            _ => Err(BuilderError::Dirty),
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

    /// Builds `anima` for `target` with the release version and puts it where the bundle takes it
    /// (INS-002, OD-17): the universal file on macOS, an x86-64 ELF file on Linux.
    async fn command_line(&self, target: Target, version: &Version) -> Result<(), BuilderError> {
        self.run(&sidecar::build(&self.clone, target, version))
            .await?;
        for (built, staged) in sidecar::copies(&self.clone, target) {
            artifact::stage(&self.clone, &built, &staged)?;
        }
        let staged = sidecar::staged(&self.clone, target);
        if let Some(lipo) = sidecar::universal(&self.clone, target) {
            self.run(&lipo).await?;
            if !artifact::universal(&self.run(&sidecar::archs(&staged)).await?) {
                return Err(BuilderError::Bundle);
            }
        } else if !sidecar::is_x86_64_elf(&artifact::header(&self.clone, &staged, 20)?) {
            return Err(BuilderError::Bundle);
        }
        Ok(())
    }

    /// Builds `anima`, compiles and bundles `target` at `version` with it, checks, notarizes and
    /// staples a macOS bundle, and returns the updater file (INS-002, BLD-043, BLD-047, BLD-083 to
    /// BLD-085). On macOS the bundled `anima` must pass `codesign --verify --strict` and be
    /// universal; notarization then refuses it unless it has the Developer ID signature, the
    /// hardened runtime and a secure timestamp (OD-16).
    pub async fn bundle(
        &self,
        commit: &Commit,
        target: Target,
        version: &Version,
    ) -> Result<PathBuf, BuilderError> {
        self.at(commit).await?;
        self.command_line(target, version).await?;
        self.run_all(&plan::bundle(&self.clone, target, version))
            .await?;
        let updater = artifact::updater_file(&self.clone, target, version);
        if target == Target::DarwinUniversal {
            let app = artifact::mac_app(&self.clone);
            let binaries = artifact::mac_binaries(&self.clone, &app)?;
            let [codesign, lipo] = plan::bundle_checks(&app, &binaries.app);
            self.run(&codesign).await?;
            self.run(&sidecar::signature(&binaries.command_line))
                .await?;
            for binary in [&lipo, &sidecar::archs(&binaries.command_line)] {
                if !artifact::universal(&self.run(binary).await?) {
                    return Err(BuilderError::Bundle);
                }
            }
            let dmg =
                artifact::dmg_file(&self.clone, target, version).ok_or(BuilderError::Bundle)?;
            self.notarize(&artifact::own_file(&self.clone, &dmg)?, &app)
                .await?;
            self.run(&plan::app_archive(&app, &updater)).await?;
        }
        artifact::own_file(&self.clone, &updater)
    }
}
