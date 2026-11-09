//! The OS work of a builder computer (BLD-026, BLD-027): fixed build steps in the builder's own
//! clone, the tool check, and signing the updater file. The builder loop is the client's
//! (`$lib/builds/`); it names a step and the job's commit, version and target, never a program.
//!
//! Each step's output goes to the main web view as `builder-line` events, which the client
//! cleans and forwards to the hub (BLD-048, BLD-049). Nothing here logs a line (LOG-007).

mod artifact;
mod bundled;
mod child;
mod environment;
mod error;
mod job;
mod lines;
mod plan;
mod reaper;
mod steps;
mod tools;

#[cfg(test)]
mod tests;

use std::{
    fs::File,
    io::{Seek, SeekFrom},
    path::PathBuf,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, Runtime, State, Webview};

use crate::{
    updater_key::{self, Signed, UnlockedKey, prehash},
    web_view::MAIN,
};
use bundled::Bundle;
pub use bundled::Bundled;
pub use child::Running;
use environment::Environment;
pub use error::BuilderError;
use job::{Commit, Target, Version};
use plan::Repository;
use steps::Workshop;
use tools::KeyReadiness;
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
    /// Whether a build process is kept from reading this process's memory, where the unlocked
    /// updater key is (D-19). The builder still runs when it is not; the row warns.
    pub memory_guarded: bool,
}

/// A signed updater file: its name and the `.sig` text (public).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SignedFile {
    pub file: String,
    pub signature: String,
}

/// Refuses every caller but the main web view; the capability already does, this is the second
/// wall (C-5).
fn from_main<R: Runtime>(webview: &Webview<R>) -> Result<(), BuilderError> {
    (webview.label() == MAIN)
        .then_some(())
        .ok_or(BuilderError::Caller)
}

fn workshop<'a, R: Runtime>(
    app: &AppHandle<R>,
    running: &'a Running,
    step: Step,
) -> Result<Workshop<'a>, BuilderError> {
    let emitter = app.clone();
    let claim = running.claim(Arc::new(move |text: String| {
        let _ = emitter.emit_to(MAIN, LINE_EVENT, LineEvent { step, text });
    }))?;
    Ok(Workshop {
        claim,
        env: Environment::from_parent(std::env::vars_os()),
        repository: Repository::anima(),
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

/// Where the updater key stands; a key store that cannot be read counts as no key.
fn key_readiness(unlocked: &UnlockedKey) -> KeyReadiness {
    if !updater_key::is_present().unwrap_or(false) {
        KeyReadiness::Missing
    } else if unlocked.is_unlocked() {
        KeyReadiness::Unlocked
    } else {
        KeyReadiness::Locked
    }
}

/// Yama's `ptrace_scope`, read at the edge (D-19).
fn memory_guarded() -> bool {
    let scope = std::fs::read_to_string("/proc/sys/kernel/yama/ptrace_scope").ok();
    cfg!(target_os = "macos") || tools::memory_guarded(scope.as_deref())
}

/// This computer's platform and the tools it lacks (BLD-028).
#[tauri::command]
pub async fn builder_tools_check<R: Runtime>(
    webview: Webview<R>,
) -> Result<ToolCheck, BuilderError> {
    from_main(&webview)?;
    let app = webview.app_handle().clone();
    let target = Target::here()?;
    let (os, arch) = match target {
        Target::LinuxX86_64 => ("LINUX", "X86_64"),
        Target::DarwinUniversal => ("MACOS", "AARCH64"),
    };
    let missing = tauri::async_runtime::spawn_blocking(move || {
        let env = Environment::from_parent(std::env::vars_os());
        let key = key_readiness(&app.state::<UnlockedKey>());
        tools::missing(target, &env, key, std::path::Path::exists)
    })
    .await
    .map_err(|_| BuilderError::Failed)?;
    let memory_guarded = memory_guarded();
    Ok(ToolCheck {
        os,
        arch,
        missing,
        memory_guarded,
    })
}

/// Fetches and checks out `commit` with a clean tree; refuses a commit not on `main`. The last
/// bundle is forgotten first, so nothing built before can be signed after.
#[tauri::command]
pub async fn builder_checkout<R: Runtime>(
    webview: Webview<R>,
    running: State<'_, Running>,
    bundled: State<'_, Bundled>,
    commit: String,
) -> Result<(), BuilderError> {
    from_main(&webview)?;
    let app = webview.app_handle().clone();
    let commit = Commit::parse(&commit)?;
    bundled.forget();
    workshop(&app, &running, Step::Checkout)?
        .checkout(&commit)
        .await
}

/// Runs the gates at `commit` (BLD-042).
#[tauri::command]
pub async fn builder_gates_run<R: Runtime>(
    webview: Webview<R>,
    running: State<'_, Running>,
    commit: String,
) -> Result<(), BuilderError> {
    from_main(&webview)?;
    let app = webview.app_handle().clone();
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

/// Compiles and bundles the job's target at its version from its commit, records the updater
/// file and the hash of its bytes (D-13), and returns its name.
#[tauri::command]
pub async fn builder_bundle<R: Runtime>(
    webview: Webview<R>,
    running: State<'_, Running>,
    job: BundleJob,
) -> Result<String, BuilderError> {
    from_main(&webview)?;
    let app = webview.app_handle().clone();
    let commit = Commit::parse(&job.commit)?;
    let version = Version::parse(&job.version)?;
    let target = target_here(&job.target)?;
    let path = workshop(&app, &running, Step::Bundle)?
        .bundle(&commit, target, &version)
        .await?;
    let name = file_name(&path)?;
    let clone = clone_folder(&app)?;
    let recorder = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let digest = prehash(artifact::open_own(&clone, &path)?)?;
        recorder.state::<Bundled>().record(Bundle {
            commit,
            target,
            version,
            path,
            digest,
        });
        Ok::<_, BuilderError>(())
    })
    .await
    .map_err(|_| BuilderError::Failed)??;
    Ok(name)
}

fn file_name(path: &std::path::Path) -> Result<String, BuilderError> {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(str::to_owned)
        .ok_or(BuilderError::Bundle)
}

/// Opens the recorded bundle's file and checks its bytes are still the ones bundled (D-13).
/// Returns the open file, at its start, and the hash. Blocking.
fn reopened(clone: &std::path::Path, bundle: &Bundle) -> Result<(File, [u8; 64]), BuilderError> {
    let mut file = artifact::open_own(clone, &bundle.path)?;
    let digest = prehash(&file)?;
    if digest != bundle.digest {
        return Err(BuilderError::Bundle);
    }
    file.seek(SeekFrom::Start(0))
        .map_err(|_| BuilderError::Bundle)?;
    Ok((file, digest))
}

/// The bundle this job's `target` and `version` name, when it is the last one made.
fn recorded<R: Runtime>(
    app: &AppHandle<R>,
    target: &str,
    version: &str,
) -> Result<Bundle, BuilderError> {
    let version = Version::parse(version)?;
    let target = target_here(target)?;
    app.state::<Bundled>()
        .of(target, &version)
        .ok_or(BuilderError::Bundle)
}

/// Signs the last bundle's updater file in this process with the unlocked key, from the one
/// descriptor whose hash matched the bundle's (D-13), checks the signature against the stored
/// public key with the version (BLD-045, BLD-046) and writes `<file>.sig` beside it.
#[tauri::command]
pub async fn builder_sign<R: Runtime>(
    webview: Webview<R>,
    running: State<'_, Running>,
    target: String,
    version: String,
) -> Result<SignedFile, BuilderError> {
    from_main(&webview)?;
    let app = webview.app_handle().clone();
    let bundle = recorded(&app, &target, &version)?;
    let _claim = quiet(&running)?;
    let clone = clone_folder(&app)?;
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs());
    let keys = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let file = file_name(&bundle.path)?;
        let (mut opened, digest) = reopened(&clone, &bundle)?;
        let signed = Signed {
            timestamp,
            file_name: &file,
            version: bundle.version.as_str(),
        };
        let signature = keys.state::<UnlockedKey>().sign(&digest, &signed)?;
        updater_key::verify_file(&mut opened, &signature, bundle.version.as_str())?;
        artifact::write_signature(&bundle.path, &signature)?;
        Ok(SignedFile { file, signature })
    })
    .await
    .map_err(|_| BuilderError::Failed)?
}

/// Checks the written `<file>.sig` over the last bundle's file for its version with the stored
/// public key (BLD-046), as the upload step will before it sends anything.
#[tauri::command]
pub async fn builder_signature_verify<R: Runtime>(
    webview: Webview<R>,
    running: State<'_, Running>,
    target: String,
    version: String,
) -> Result<(), BuilderError> {
    from_main(&webview)?;
    let app = webview.app_handle().clone();
    let bundle = recorded(&app, &target, &version)?;
    let _claim = quiet(&running)?;
    let clone = clone_folder(&app)?;
    tauri::async_runtime::spawn_blocking(move || {
        let (opened, _) = reopened(&clone, &bundle)?;
        let signature = artifact::read_signature(&bundle.path)?;
        updater_key::verify_file(opened, &signature, bundle.version.as_str())
            .map_err(BuilderError::from)
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
pub fn builder_cancel<R: Runtime>(
    webview: Webview<R>,
    running: State<'_, Running>,
) -> Result<(), BuilderError> {
    from_main(&webview)?;
    running.cancel();
    Ok(())
}
