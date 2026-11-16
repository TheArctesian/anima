//! The steps that build, put in place and check what a release carries (S12 2.2 steps 2 to 4,
//! 2.3). Each runs planned commands under the step's claim.

use std::path::{Path, PathBuf};

use super::{
    joined, link,
    mach_o::{self, ARCHS, Signer, TeamId},
    on_disk, rust_programs, staged, tree,
};
use crate::builder::{
    artifact,
    error::BuilderError,
    job::{Target, Version},
    steps::Workshop,
};

impl Workshop<'_> {
    /// The identity that signs a macOS release, read from the clone's `tauri.conf.json`, and its
    /// Team ID (D-5); none on Linux, which signs nothing.
    pub fn signer(&self, target: Target) -> Result<Option<Signer>, BuilderError> {
        match target {
            Target::LinuxX86_64 => Ok(None),
            Target::DarwinUniversal => {
                Signer::from_identity(&on_disk::signing_identity(&self.clone)?).map(Some)
            }
        }
    }

    /// Builds the six programs and the PostgreSQL tree for `target` from the clone, and puts each
    /// where the bundle takes it (INS-002, S12 2.2): the programs universal on macOS and x86-64
    /// ELF files on Linux; the tree signed by `signer` on macOS.
    pub async fn carried(
        &self,
        target: Target,
        version: &Version,
        signer: Option<&Signer>,
    ) -> Result<(), BuilderError> {
        self.run(&rust_programs::build(&self.clone, target, version))
            .await?;
        self.stage_all(&rust_programs::copies(&self.clone, target))?;
        for (_, program) in rust_programs::PROGRAMS {
            self.join_and_check(target, program).await?;
        }
        self.run(&link::build(&self.clone)).await?;
        self.stage_all(&link::copies(&self.clone, target))?;
        self.join_and_check(target, link::PROGRAM).await?;
        let work = tree::work_folder(&self.clone)?;
        on_disk::renew(&work)?;
        let identity = signer.map(|signer| signer.identity.as_str());
        self.run(&tree::build(&self.clone, target, &work, identity)?)
            .await?;
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
