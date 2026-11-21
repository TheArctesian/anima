//! One cycle of model work (Section 3, CAG-002, CAG-003): the claim through the work route of
//! K10, the pure check of the item, the step, and the report. The daemon asks; the hub never
//! opens a connection. The caller runs a cycle only while the switch at the machine is on.

use std::path::{Path, PathBuf};
use std::time::Instant;

use daemon_core::model_work::cause::Cause;
use daemon_core::model_work::command;
use daemon_core::model_work::folders::Folders;
use daemon_core::model_work::machine::MachineSettings;
use daemon_core::model_work::names::{Runtime, Word as _};
use daemon_core::model_work::order::{self, Checked, ModelMachine, Order, Parameters};
use daemon_core::model_work::releases::{self, Release, Sources};
use daemon_core::model_work::settings::{ServerSettings, SettingLimits};
use daemon_core::wire::{
    Claim, DaemonToken, Free, Switch, WorkArchitecture, WorkKind, WorkOs, WorkType,
};
use serde_json::{Value, json};
use uuid::Uuid;

use super::install::{self, Place};
use super::server::{self, Launch};
use super::state;
use super::supervise::{Server, ServerState, Servers};
use crate::hub::HubClient;
use crate::hub::answer::{Answer, Cause as HubCause};

/// The model types that this build does; the claim names only these.
pub const BUILT: [WorkType; 4] = [
    WorkType::ModelReport,
    WorkType::RuntimeInstall,
    WorkType::ServerStart,
    WorkType::ServerStop,
];

/// The model work of this node.
pub struct Node {
    pub device: Uuid,
    pub os: WorkOs,
    pub architecture: WorkArchitecture,
    pub folders: Folders,
    pub sources: Sources,
    pub settings: MachineSettings,
    pub servers: Servers,
    pub switch_on: bool,
    pub started: Instant,
    /// The free disk that model work leaves ([`install::DEFAULT_RESERVE`] by default).
    pub reserve: u64,
    /// The daemon program: the launcher of a runtime on Linux (R-NEW-containment).
    pub launcher: PathBuf,
    /// The range of loopback ports of the daemon configuration ([`server::PORTS`] by default).
    pub ports: std::ops::RangeInclusive<u16>,
}

/// The JSON report of attempt `attempt` of item `id` (K10 Section 7, with `model`).
#[must_use]
pub fn report(
    id: Uuid,
    attempt: u32,
    outcome: Result<Option<Value>, Cause>,
    seconds: u64,
) -> Value {
    let (state, cause, model) = match outcome {
        Ok(model) => ("succeeded", None, model),
        Err(cause) if cause.is_refusal() => ("refused", Some(cause.as_str()), None),
        Err(cause) => ("failed", Some(cause.as_str()), None),
    };
    let mut result = json!({
        "outcome": state, "cause": cause, "seconds": seconds,
        "cleanup": { "isolation": "none", "checkout": "none", "secrets": "none", "keychain": "none" },
    });
    if let Some(model) = model {
        result["model"] = model;
    }
    json!({ "id": id, "attempt": attempt, "state": state, "result": result })
}

fn api_key() -> Option<zeroize::Zeroizing<String>> {
    let mut bytes = zeroize::Zeroizing::new([0u8; 32]);
    getrandom::fill(bytes.as_mut()).ok()?;
    Some(zeroize::Zeroizing::new(super::fetch::hex(bytes.as_ref())))
}

fn os_word(os: WorkOs) -> &'static str {
    match os {
        WorkOs::Linux => "linux",
        WorkOs::Macos => "macos",
    }
}

fn architecture_word(architecture: WorkArchitecture) -> &'static str {
    match architecture {
        WorkArchitecture::X86_64 => "x86_64",
        WorkArchitecture::Aarch64 => "aarch64",
    }
}

impl Node {
    /// Ends the processes and the cgroups of each server of an earlier run, from their records
    /// (R-NEW-containment). The daemon calls it once when it starts, before any claim.
    ///
    /// # Errors
    /// An I/O error of the state folder.
    pub fn sweep(&self) -> std::io::Result<usize> {
        super::tree::sweep(&self.folders.state.join("servers"))
    }

    /// Seconds of the clock of this node.
    #[must_use]
    pub fn now(&self) -> u64 {
        self.started.elapsed().as_secs()
    }

    /// The claim of model work.
    #[must_use]
    pub fn claim(&self) -> Claim {
        Claim {
            device: self.device,
            switch: if self.switch_on {
                Switch::On
            } else {
                Switch::Off
            },
            types: BUILT.to_vec(),
            kinds: vec![WorkKind::Model],
            os: self.os,
            architecture: self.architecture,
            // Model items do not use the job slot of K10 (Section 3).
            free: Free {
                slots: 0,
                memory_bytes: 0,
                disk_bytes: 0,
            },
            holds: Vec::new(),
            unclean: false,
        }
    }

    fn server_for(
        &self,
        id: Uuid,
        release: &Release,
        settings: &ServerSettings,
    ) -> Result<Server, Cause> {
        if self.settings.sandbox {
            // TODO(L1-6 follow-up): the sandbox-exec profile (macOS) and Landlock or bubblewrap
            // (Linux) are not built; with the setting on, no start (Section 9).
            return Err(Cause::IsolationFailed);
        }
        releases::accepted(release)?;
        let entry = self.sources.find(
            release,
            os_word(self.os),
            architecture_word(self.architecture),
        )?;
        if !install::installed(&self.folders, release, entry)
            || release.runtime != Runtime::LlamaServer
        {
            return Err(Cause::RuntimeNotInstalled);
        }
        let program = self.folders.release(release).join(&entry.program);
        let bin = program.parent().unwrap_or(Path::new("/")).to_path_buf();
        let place = self.folders.server(id);
        let state = self.folders.state.clone();
        let launcher = self.launcher.clone();
        let settings = settings.clone();
        let ports = self.ports.clone();
        let launch = move || {
            let port = server::free_port(ports.clone())?;
            let key = api_key()?;
            let temporary = place.join("tmp");
            Some((
                Launch {
                    program: program.clone(),
                    arguments: command::llama_arguments(&settings, port),
                    environment: command::environment(&temporary, &bin, &key),
                    state: state.clone(),
                    temporary,
                    ring: place.join("output.log"),
                    key,
                    place: place.clone(),
                    server: id,
                    launcher: launcher.clone(),
                },
                port,
            ))
        };
        Ok(Server {
            runtime: release.runtime.as_str(),
            launch: Box::new(launch),
            running: None,
            record: daemon_core::model_work::supervision::Supervision::default(),
            state: ServerState::Starting,
            since: 0,
            restart_at: None,
        })
    }

    /// Does the step of `order`.
    ///
    /// # Errors
    /// The cause of Section 5 of the step.
    pub async fn run(&mut self, order: &Order) -> Result<Option<Value>, Cause> {
        if !BUILT.contains(&order.work_type) {
            return Err(Cause::TypeUnknown);
        }
        match &order.parameters {
            Parameters::Report => Ok(Some(state::model_state(
                &self.folders,
                &self.servers,
                &self.settings,
            ))),
            Parameters::Install(release) => {
                let place = Place {
                    folders: &self.folders,
                    sources: &self.sources,
                    os: os_word(self.os),
                    architecture: architecture_word(self.architecture),
                    reserve: self.reserve,
                };
                install::install(release, &place).await.map(|()| None)
            }
            Parameters::Start {
                server,
                release,
                settings,
            } => {
                super::folders::make_all(&self.folders).map_err(|_| Cause::LimitDisk)?;
                let started = self.server_for(*server, release, settings)?;
                let clock = self.started;
                self.servers
                    .start(*server, started, move || clock.elapsed().as_secs())
                    .await
                    .map(|()| None)
            }
            Parameters::Stop { server } => {
                if self.servers.stop(*server).await {
                    Ok(None)
                } else {
                    Err(Cause::CopyUnknown)
                }
            }
            _ => Err(Cause::TypeUnknown),
        }
    }

    /// One claim, and the report of the item that it got. `Ok(None)` when the hub had no item.
    ///
    /// # Errors
    /// The cause of the hub when the claim or the report fails.
    pub async fn cycle(
        &mut self,
        client: &HubClient,
        token: &DaemonToken,
    ) -> Result<Option<Value>, HubCause> {
        let item = match client.claim_work(token, &self.claim()).await {
            Answer::Accepted(item) => item,
            Answer::Refused(code) => return Err(HubCause::Status(code.status())),
            Answer::Failed(cause) => return Err(cause),
        };
        let Some(item) = item else { return Ok(None) };
        // TODO(L1-5): the memory share and the GPUs come from the device policy and the
        // inventory; until then no `memoryShare` and no `gpu` pass.
        let machine = ModelMachine {
            switch_on: self.switch_on,
            policy_permits: true,
            settings: &self.settings,
            limits: SettingLimits {
                memory_share: 1.0,
                gpus: &[],
            },
        };
        let start = Instant::now();
        let body = match order::check(&item, &machine) {
            Checked::Drop => return Ok(None),
            Checked::Refuse { id, attempt, cause } => report(id, attempt, Err(cause), 0),
            // TODO(L1-7, L1-8, L1-16): the types that this build does not do yet.
            Checked::Start(order) if !BUILT.contains(&order.work_type) => {
                report(order.id, order.attempt, Err(Cause::TypeUnknown), 0)
            }
            Checked::Start(order) => {
                let running =
                    json!({ "id": order.id, "attempt": order.attempt, "state": "running" });
                if let Answer::Failed(cause) = client.report_work(token, &running).await {
                    return Err(cause);
                }
                let outcome = self.run(&order).await;
                report(order.id, order.attempt, outcome, start.elapsed().as_secs())
            }
        };
        match client.report_work(token, &body).await {
            Answer::Failed(cause) => Err(cause),
            Answer::Accepted(_) | Answer::Refused(_) => Ok(Some(body)),
        }
    }
}
