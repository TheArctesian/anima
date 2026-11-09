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
};

/// The claim, the environment, the repository and the clone of one step.
pub struct Workshop<'a> {
    pub claim: Claim<'a>,
    pub env: Environment,
    pub repository: Repository,
    pub clone: PathBuf,
}

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

    /// Whether the clone is one `git clone` made: a `.git` folder, no graft or shallow file,
    /// and no config key a build could have added to redirect or rewrite it (D-15).
    async fn trusted_clone(&self) -> bool {
        let git = self.clone.join(".git");
        if !git.is_dir() || git.join("info/grafts").exists() || git.join("shallow").exists() {
            return false;
        }
        self.run(&plan::config_keys(&self.clone))
            .await
            .is_ok_and(|keys| config_is_clones(&keys))
    }

    /// Clones the repository anew when the clone is missing or no longer trusted.
    async fn fresh_clone(&self) -> Result<(), BuilderError> {
        if self.trusted_clone().await {
            return Ok(());
        }
        if self.clone.exists() {
            fs::remove_dir_all(&self.clone).map_err(|_| BuilderError::Failed)?;
        }
        let parent = self.clone.parent().ok_or(BuilderError::Failed)?;
        fs::create_dir_all(parent).map_err(|_| BuilderError::Failed)?;
        self.run(&plan::clone(&self.repository, &self.clone))
            .await?;
        Ok(())
    }

    /// Clones when there is no trusted clone, fetches `main` by URL, refuses a commit that is
    /// not on it, then checks the commit out with a clean tree (BLD-040, BLD-041).
    pub async fn checkout(&self, commit: &Commit) -> Result<(), BuilderError> {
        self.fresh_clone().await?;
        self.run(&plan::fetch(&self.repository, &self.clone))
            .await?;
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

    /// Compiles and bundles `target` at `version`, checks a macOS bundle, and returns the
    /// updater file (BLD-043, BLD-047).
    pub async fn bundle(
        &self,
        commit: &Commit,
        target: Target,
        version: &Version,
    ) -> Result<PathBuf, BuilderError> {
        self.at(commit).await?;
        self.run_all(&plan::bundle(&self.clone, target, version))
            .await?;
        let updater = artifact::updater_file(&self.clone, target, version);
        if target == Target::DarwinUniversal {
            let app = artifact::mac_app(&self.clone);
            let binary = artifact::mac_binary(&self.clone, &app)?;
            let [codesign, lipo] = plan::bundle_checks(&app, &binary);
            self.run(&codesign).await?;
            if !artifact::universal(&self.run(&lipo).await?) {
                return Err(BuilderError::Bundle);
            }
            self.run(&plan::app_archive(&app, &updater)).await?;
        }
        artifact::own_file(&self.clone, &updater)
    }
}
