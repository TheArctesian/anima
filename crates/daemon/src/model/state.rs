//! The model state of the node (Section 8): the object of `model_report` and, later, of the
//! member `runtimes` of the push batch. It holds no prompt, no output, no file name of a copy
//! and no address (SEC-C-010).

use chrono::{SecondsFormat, Utc};
use daemon_core::model_work::folders::Folders;
use daemon_core::model_work::machine::MachineSettings;
use daemon_core::model_work::names::{Backend, Runtime, Word as _};
use daemon_core::model_work::releases::Release;
use serde_json::{Value, json};

use super::supervise::Servers;

/// The most installed runtimes and servers in the object.
const MAX_LISTED: usize = 16;

/// The installed releases: the folders `runtimes/<runtime>/<version>-<backend>/` whose names
/// read. A folder of an unpack in progress starts with `.` and does not read.
#[must_use]
pub fn installed(folders: &Folders) -> Vec<Release> {
    let mut found = Vec::new();
    for runtime in Runtime::ALL {
        let Ok(entries) = std::fs::read_dir(folders.runtimes.join(runtime.as_str())) else {
            continue;
        };
        for entry in entries.filter_map(Result::ok) {
            let name = entry.file_name().to_string_lossy().into_owned();
            let Some((version, backend)) = name.rsplit_once('-') else {
                continue;
            };
            let backend = Backend::parse(backend);
            if let (Some(backend), Some(_)) = (backend, Release::number(*runtime, version)) {
                found.push(Release {
                    runtime: *runtime,
                    version: version.to_owned(),
                    backend,
                });
            }
        }
    }
    found.sort_by(|a, b| (a.runtime.as_str(), &a.version).cmp(&(b.runtime.as_str(), &b.version)));
    found.truncate(MAX_LISTED);
    found
}

/// The object of Section 8, with the settings of the machine (Section 10.1). Copies come with
/// the pull of task L1-7; until then the list is empty.
#[must_use]
pub fn model_state(folders: &Folders, servers: &Servers, settings: &MachineSettings) -> Value {
    let installed: Vec<Value> = installed(folders)
        .iter()
        .map(|release| json!({ "runtime": release.runtime.as_str(), "version": release.version, "backend": release.backend.as_str() }))
        .collect();
    let servers: Vec<Value> = servers
        .map
        .iter()
        .take(MAX_LISTED)
        .map(|(id, server)| {
            json!({
                "server": id, "runtime": server.runtime, "state": server.state.as_str(),
                "restarts": server.record.restarts(), "loaded": [], "inProgress": 0, "queued": 0,
                "tokensPerSecond": null, "memoryBytes": 0,
                "tempBytes": super::folders::bytes_below(&folders.server(*id).join("tmp")),
            })
        })
        .collect();
    json!({
        "at": Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true),
        "installed": installed,
        "servers": servers,
        "copies": [],
        "cacheBytes": 0,
        "settings": settings.to_json(),
    })
}
