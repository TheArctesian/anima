//! The steps that build, put in place and check what a release carries (S12 2.2 and 2.3). Each
//! runs planned commands under the step's claim. The two recipes run before `cargo build`, whose
//! build scripts could rewrite them, and each file a recipe runs or reads is compared with the
//! commit directly before.

use std::path::{Path, PathBuf};

use super::{
    committed, joined, link,
    mach_o::{self, ARCHS, Signer, TeamId},
    on_disk, rust_programs, staged, tree,
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
                self.committed(commit, &[tree::CONFIG]).await?;
                let clone = on_disk::signing_identity(&self.clone)?;
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
            let on_disk = self.run(&committed::on_disk(&self.clone, path)).await?;
            let in_commit = self
                .run(&committed::in_commit(&self.clone, commit, path))
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
            .map(drop)
    }

    /// Builds `anima-link` with the committed recipe and modules, and puts the programs for
    /// `target` where the bundle takes them.
    pub async fn link(&self, commit: &Commit, target: Target) -> Result<(), BuilderError> {
        self.committed(commit, &link::COMMITTED).await?;
        for path in link::ABSENT {
            on_disk::absent(&self.clone.join(path))?;
        }
        self.run(&link::verify(&self.clone)).await?;
        self.run(&link::build(&self.clone)).await?;
        self.stage_all(&link::copies(&self.clone, target))?;
        self.join_and_check(target, link::PROGRAM).await
    }

    /// Builds the five Rust programs with the release version and puts them where the bundle
    /// takes them (INS-002, OD-17).
    pub async fn rust_programs(
        &self,
        target: Target,
        version: &Version,
    ) -> Result<(), BuilderError> {
        self.run(&rust_programs::build(&self.clone, target, version))
            .await?;
        self.stage_all(&rust_programs::copies(&self.clone, target))?;
        for (_, program) in rust_programs::PROGRAMS {
            self.join_and_check(target, program).await?;
        }
        Ok(())
    }

    fn stage_all(&self, copies: &[(PathBuf, PathBuf)]) -> Result<(), BuilderError> {
        copies
            .iter()
            .try_for_each(|(built, staged)| artifact::stage(&self.clone, built, staged))
    }

    /// Joins the macOS halves of `program` and checks the file the bundle takes: universal on
    /// macOS, an x86-64 ELF file on Linux.
    async fn join_and_check(&self, target: Target, program: &str) -> Result<(), BuilderError> {
        let staged = staged(&self.clone, target, program);
        let carried = if let Some(lipo) = joined(&self.clone, target, program) {
            self.run(&lipo).await?;
            artifact::universal(&self.run(&mach_o::archs(&staged)).await?)
        } else {
            super::is_x86_64_elf(&artifact::header(&self.clone, &staged, 20)?)
        };
        carried.then_some(()).ok_or(BuilderError::Bundle)
    }

    /// Checks every Mach-O file of the signed `app` (S12 2.3, D-4); the first that fails fails
    /// the step.
    pub async fn signed_mach_o(&self, app: &Path, team: &TeamId) -> Result<(), BuilderError> {
        for file in on_disk::mach_o_files(&self.clone, app)? {
            self.mach_o_signed(&file, team).await?;
        }
        Ok(())
    }

    /// Both architectures, the Developer ID signature of `team`, then for each architecture the
    /// hardened runtime, a secure timestamp and no entitlement (UPD-011, D-3).
    async fn mach_o_signed(&self, file: &Path, team: &TeamId) -> Result<(), BuilderError> {
        if !artifact::universal(&self.run(&mach_o::archs(file)).await?) {
            return Err(BuilderError::Bundle);
        }
        self.run(&mach_o::verify(file, team)).await?;
        for arch in ARCHS {
            let display = self.run(&mach_o::display(file, arch)).await?;
            let entitlements = self.run(&mach_o::entitlements(file, arch)).await?;
            if !mach_o::hardened_and_timestamped(&display)
                || !mach_o::no_entitlements(&entitlements)
            {
                return Err(BuilderError::Bundle);
            }
        }
        Ok(())
    }
}
