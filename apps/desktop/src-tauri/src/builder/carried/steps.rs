//! The steps that build, put in place and check what a release carries (S12 2.2 and 2.3). Each
//! runs planned commands under the step's claim. The two recipes run before `cargo build`, whose
//! build scripts could rewrite them, and each file a recipe runs or reads is compared with the
//! commit directly before.

use std::path::{Path, PathBuf};

use super::{
    committed, joined, link,
    mach_o::{self, ARCHS, Readings, TeamId},
    on_disk, rust_programs,
    signer::{self, Signer},
    staged, tree,
};
use crate::builder::{
    artifact,
    error::BuilderError,
    job::{Commit, Target, Version},
    steps::Workshop,
    tools::readings,
};

impl Workshop<'_> {
    /// The identity that signs a macOS release and its Team ID: the one this builder was built
    /// with, when the clone's committed `tauri.conf.json` names the same (BLD-083). None on
    /// Linux, which signs nothing.
    pub async fn signer(
        &self,
        commit: &Commit,
        target: Target,
    ) -> Result<Option<Signer>, BuilderError> {
        match target {
            Target::LinuxX86_64 => Ok(None),
            Target::DarwinUniversal => {
                self.committed(commit, &[signer::CONFIG]).await?;
                let clone = signer::in_clone(&self.clone)?;
                Signer::pinned(readings::signing_identity().as_deref(), &clone).map(Some)
            }
        }
    }

    /// Fails with `Dirty` unless each of `paths` is a regular file whose bytes are the blob of
    /// `commit` at that path.
    pub async fn committed(&self, commit: &Commit, paths: &[&str]) -> Result<(), BuilderError> {
        for path in paths {
            let kind = std::fs::symlink_metadata(self.clone.join(path))
                .map_err(|_| BuilderError::Dirty)?;
            let on_disk = self.read(&committed::on_disk(&self.clone, path)).await?;
            let in_commit = self
                .read(&committed::in_commit(&self.clone, commit, path))
                .await?;
            if !kind.is_file() || !committed::same_blob(&on_disk, &in_commit) {
                return Err(BuilderError::Dirty);
            }
        }
        Ok(())
    }

    /// Builds the PostgreSQL tree with the committed recipe in a work folder made anew, signed by
    /// `signer` on macOS (S3 T11, D-1).
    pub async fn tree(
        &self,
        commit: &Commit,
        target: Target,
        signer: Option<&Signer>,
    ) -> Result<(), BuilderError> {
        self.committed(commit, &tree::COMMITTED).await?;
        let work = tree::work_folder(&self.clone)?;
        on_disk::renew(&work)?;
        let identity = signer.map(|signer| signer.identity.as_str());
        self.run(&tree::build(&self.clone, target, &work, identity)?)
            .await
    }

    /// Builds `anima-link` for every platform with the committed recipe and modules.
    pub async fn link(&self, commit: &Commit) -> Result<(), BuilderError> {
        self.committed(commit, &link::COMMITTED).await?;
        for path in link::ABSENT {
            on_disk::absent(&self.clone.join(path))?;
        }
        self.run(&link::verify(&self.clone)).await?;
        self.run(&link::build(&self.clone)).await
    }

    /// Builds the six Rust programs with the release version (INS-002, OD-17).
    pub async fn rust_programs(
        &self,
        target: Target,
        version: &Version,
    ) -> Result<(), BuilderError> {
        self.run(&rust_programs::build(&self.clone, target, version))
            .await
    }

    /// Puts each built file of `copies` where the bundle takes it, then on macOS joins the two
    /// halves of each of `programs` into one file.
    pub async fn stage(
        &self,
        target: Target,
        copies: &[(PathBuf, PathBuf)],
        programs: &[&str],
    ) -> Result<(), BuilderError> {
        for (built, staged) in copies {
            artifact::stage(&self.clone, built, staged)?;
        }
        for program in programs {
            if let Some(lipo) = joined(&self.clone, target, program) {
                self.run(&lipo).await?;
            }
        }
        Ok(())
    }

    /// Fails with `Bundle` unless the file the bundle takes of each of `programs` is universal
    /// on macOS, or an x86-64 ELF program on Linux.
    pub async fn check_staged(
        &self,
        target: Target,
        programs: &[&str],
    ) -> Result<(), BuilderError> {
        for program in programs {
            let staged = staged(&self.clone, target, program);
            let carried = match target {
                Target::DarwinUniversal => {
                    artifact::universal(&self.read(&mach_o::archs(&staged)).await?)
                }
                Target::LinuxX86_64 => {
                    super::is_x86_64_elf(&artifact::header(&self.clone, &staged, 20)?)
                }
            };
            if !carried {
                return Err(BuilderError::Bundle);
            }
        }
        Ok(())
    }

    /// Checks every Mach-O file of the signed `app` (S12 2.3, D-4); the first that fails fails
    /// the step.
    pub async fn signed_mach_o(&self, app: &Path, team: &TeamId) -> Result<(), BuilderError> {
        for file in on_disk::mach_o_files(&self.clone, app)? {
            if !mach_o::released(&self.mach_o_readings(&file, team).await?) {
                return Err(BuilderError::Bundle);
            }
        }
        Ok(())
    }

    /// What the checks of one Mach-O `file` read (UPD-011, D-3). A file that is not exactly
    /// universal is not asked about either architecture: `codesign --arch` fails on a missing one.
    pub async fn mach_o_readings(
        &self,
        file: &Path,
        team: &TeamId,
    ) -> Result<Readings, BuilderError> {
        let archs = self.read(&mach_o::archs(file)).await?;
        if !artifact::universal(&archs) {
            return Ok(Readings {
                archs,
                ..Readings::default()
            });
        }
        let verified = match self.run(&mach_o::verify(file, team)).await {
            Err(BuilderError::StepFailed) => false,
            other => other.map(|()| true)?,
        };
        let mut readings = Readings {
            archs,
            verified,
            ..Readings::default()
        };
        for arch in ARCHS {
            let display = self.read(&mach_o::display(file, arch)).await?;
            readings.displays.push((arch, display));
            let entitlements = self.read(&mach_o::entitlements(file, arch)).await?;
            readings.entitlements.push((arch, entitlements));
        }
        Ok(readings)
    }
}
