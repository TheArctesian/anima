//! The OS work of a builder computer (BLD-026, BLD-027): fixed build steps in the builder's own
//! clone, the tool check, and signing the updater file. The builder loop is the client's
//! (`$lib/builds/`); it names a step and the job's commit, version and target, never a program.
//!
//! Each step's output goes to the main web view as `builder-line` events, which the client
//! cleans and forwards to the hub (BLD-048, BLD-049). Nothing here logs a line (LOG-007).

mod artifact;
mod child;
mod environment;
mod error;
mod job;
mod lines;
mod plan;
mod steps;
mod tools;

#[cfg(test)]
mod tests;

use std::{
    path::PathBuf,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, Runtime, State};

use crate::updater_key::{self, Signed};
pub use child::Running;
use environment::Environment;
pub use error::BuilderError;
use job::{Commit, Target, Version};
use steps::Workshop;
pub use tools::Tool;

/// The event that carries one output line of a step to the main web view.
const LINE_EVENT: &str = "builder-line";
/// The builder's clone, under the app data folder (BLD-040).
const CLONE_FOLDER: &str = "builds/anima";

/// The step a line came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Step {
    Checkout,
    Gates,
    Bundle,
}

#[derive(Debug, Clone, Serialize)]
struct LineEvent {
    step: Step,
    text: String,
}

/// What the tool check found (BLD-028).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolCheck {
    /// K1 `BuilderOs` and `BuilderArch` of this computer.
    pub os: &'static str,
    pub arch: &'static str,
    pub missing: Vec<Tool>,
}

/// A signed updater file: its name and the `.sig` text (public).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SignedFile {
    pub file: String,
    pub signature: String,
}

fn workshop<'a, R: Runtime>(
    app: &AppHandle<R>,
    running: &'a Running,
    step: Step,
) -> Result<Workshop<'a>, BuilderError> {
    let emitter = app.clone();
    let claim = running.claim(Arc::new(move |text: String| {
        let _ = emitter.emit_to("main", LINE_EVENT, LineEvent { step, text });
    }))?;
    Ok(Workshop {
        claim,
        env: Environment::from_parent(std::env::vars_os()),
        clone: clone_folder(app)?,
    })
}

fn clone_folder<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf, BuilderError> {
    let data = app
        .path()
        .app_data_dir()
        .map_err(|_| BuilderError::Failed)?;
    Ok(data.join(CLONE_FOLDER))
}

/// Takes the step slot for a step with no output of its own (signing, verifying).
fn quiet(running: &Running) -> Result<child::Claim<'_>, BuilderError> {
    running.claim(Arc::new(|_| {}))
}

/// The target this computer builds, or `Platform` when `target` is another.
fn target_here(target: &str) -> Result<Target, BuilderError> {
    let target = Target::parse(target)?;
    (Target::here()? == target)
        .then_some(target)
        .ok_or(BuilderError::Platform)
}

/// This computer's platform and the tools it lacks (BLD-028).
#[tauri::command]
pub async fn builder_tools_check() -> Result<ToolCheck, BuilderError> {
    let target = Target::here()?;
    let (os, arch) = match target {
        Target::LinuxX86_64 => ("LINUX", "X86_64"),
        Target::DarwinUniversal => ("MACOS", "AARCH64"),
    };
    let missing = tauri::async_runtime::spawn_blocking(move || {
        let env = Environment::from_parent(std::env::vars_os());
        let key_present = updater_key::is_present().unwrap_or(false);
        tools::missing(target, &env, key_present, std::path::Path::exists)
    })
    .await
    .map_err(|_| BuilderError::Failed)?;
    Ok(ToolCheck { os, arch, missing })
}

/// Fetches and checks out `commit` with a clean tree; refuses a commit not on `main`.
#[tauri::command]
pub async fn builder_checkout<R: Runtime>(
    app: AppHandle<R>,
    running: State<'_, Running>,
    commit: String,
) -> Result<(), BuilderError> {
    let commit = Commit::parse(&commit)?;
    workshop(&app, &running, Step::Checkout)?
        .checkout(&commit)
        .await
}

/// Runs the gates at `commit` (BLD-042).
#[tauri::command]
pub async fn builder_gates_run<R: Runtime>(
    app: AppHandle<R>,
    running: State<'_, Running>,
    commit: String,
) -> Result<(), BuilderError> {
    let commit = Commit::parse(&commit)?;
    workshop(&app, &running, Step::Gates)?.gates(&commit).await
}

/// The job fields a bundle needs, as the hub sent them; each is checked before use.
#[derive(Debug, Clone, Deserialize)]
pub struct BundleJob {
    commit: String,
    target: String,
    version: String,
}

/// Compiles and bundles the job's target at its version from its commit; returns the updater
/// file's name.
#[tauri::command]
pub async fn builder_bundle<R: Runtime>(
    app: AppHandle<R>,
    running: State<'_, Running>,
    job: BundleJob,
) -> Result<String, BuilderError> {
    let commit = Commit::parse(&job.commit)?;
    let version = Version::parse(&job.version)?;
    let target = target_here(&job.target)?;
    let file = workshop(&app, &running, Step::Bundle)?
        .bundle(&commit, target, &version)
        .await?;
    file_name(&file)
}

fn file_name(path: &std::path::Path) -> Result<String, BuilderError> {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(str::to_owned)
        .ok_or(BuilderError::Bundle)
}

/// The updater file of `target` at `version`, once it is the build's own regular file.
fn updater_path<R: Runtime>(
    app: &AppHandle<R>,
    target: Target,
    version: &Version,
) -> Result<PathBuf, BuilderError> {
    let clone = clone_folder(app)?;
    artifact::own_file(&clone, &artifact::updater_file(&clone, target, version))
}

/// Signs the updater file of `target` at `version` in this process, checks the signature with
/// the version (BLD-045, BLD-046) and writes `<file>.sig` beside it. The web view names no file.
#[tauri::command]
pub async fn builder_sign<R: Runtime>(
    app: AppHandle<R>,
    running: State<'_, Running>,
    target: String,
    version: String,
) -> Result<SignedFile, BuilderError> {
    let version = Version::parse(&version)?;
    let target = target_here(&target)?;
    let _claim = quiet(&running)?;
    let path = updater_path(&app, target, &version)?;
    let file = file_name(&path)?;
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs());
    let name = file.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let signed = Signed {
            timestamp,
            file_name: &name,
            version: version.as_str(),
        };
        let signature = updater_key::sign_file(&path, &signed)?;
        artifact::write_signature(&path, &signature)?;
        Ok(SignedFile { file, signature })
    })
    .await
    .map_err(|_| BuilderError::Failed)?
}

/// Checks the written `<file>.sig` over the updater file for `version` with the stored key
/// (BLD-046), as the upload step will before it sends anything.
#[tauri::command]
pub async fn builder_signature_verify<R: Runtime>(
    app: AppHandle<R>,
    running: State<'_, Running>,
    target: String,
    version: String,
) -> Result<(), BuilderError> {
    let version = Version::parse(&version)?;
    let target = target_here(&target)?;
    let _claim = quiet(&running)?;
    let path = updater_path(&app, target, &version)?;
    tauri::async_runtime::spawn_blocking(move || {
        let signature = artifact::read_signature(&path)?;
        updater_key::verify_file(&path, &signature, version.as_str()).map_err(BuilderError::from)
    })
    .await
    .map_err(|_| BuilderError::Failed)?
}

/// Stops the running step and every process it started.
#[expect(
    clippy::needless_pass_by_value,
    reason = "Tauri hands a command its state by value"
)]
#[tauri::command]
pub fn builder_cancel(running: State<'_, Running>) {
    running.cancel();
}
