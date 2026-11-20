//! The settings at the machine (contract K10 Section 6.4.2). Only the administrator of the
//! machine sets them; the applications program reads them and never writes them.
//!
//! The file is JSON with these members, each always there:
//!
//! ```json
//! {
//!   "version": 1,
//!   "manageApplications": null,
//!   "allContainers": false,
//!   "openToNetwork": false,
//!   "hubMayDeleteVolumes": false,
//!   "adoptedProjects": [],
//!   "protected": [],
//!   "registriesWithLogin": [],
//!   "limits": { "memoryBytes": null, "processes": 512, "processorBillionths": null, "stopSeconds": 30, "freeDiskBytes": 10737418240 },
//!   "labelPrefixes": []
//! }
//! ```
//!
//! `manageApplications` is `null` while the setting is off. While it is on, it is
//! `{ "engine": "system", "socket": "/run/docker.sock", "docker": "/usr/bin/docker",
//! "compose": "/usr/libexec/docker/cli-plugins/docker-compose", "acceptedUser": 1000,
//! "setOnAt": 1795000000 }`: `engine` is `system` or `user`; `acceptedUser` is the identifier
//! of the user of the daemon; `setOnAt` is the time, in seconds, when the administrator set
//! it on. An adopted project is `{ "name": "tracks", "folder": "/srv/tracks", "files":
//! ["/srv/tracks/compose.yaml"] }`. In `limits`, `null` is the value of the machine: a
//! quarter of its memory, each of its processors.

mod guard;
mod reading;

pub use guard::{Guarded, NotGuarded, guard};
pub use reading::{LARGEST_SETTINGS, SettingsFault, read};

/// The kind of the engine of a machine (K10 Section 6.4.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EngineKind {
    /// The real socket of the engine belongs to the identifier 0.
    System,
    /// The real socket of the engine belongs to a different identifier.
    User,
}

/// What goes with "Manage applications" while it is on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Managed {
    pub engine: EngineKind,
    /// The path of the Unix socket of the engine, as the administrator recorded it.
    pub socket: String,
    /// The path of the `docker` program.
    pub docker: String,
    /// The path of the Compose program.
    pub compose: String,
    /// The identifier of the one user whose daemon is accepted on the socket of the program.
    pub accepted_user: u32,
    /// When the administrator set the setting on, in seconds. A record of a pair of a hub
    /// that was made for a different time is not valid (Section 6.4.2).
    pub set_on_at: u64,
}

/// A Compose project that Anima did not install and may manage, with its recorded paths.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Adopted {
    pub name: String,
    /// The real path of the folder of the project, as recorded at the adoption.
    pub folder: String,
    /// The real path of each Compose file of the project, as recorded at the adoption.
    pub files: Vec<String>,
}

/// "Limits of a service": what Section 6.4.4 calls "its own limit". `None` is the value of
/// the machine.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    pub memory_bytes: Option<u64>,
    pub processes: u64,
    pub processor_billionths: Option<u64>,
    pub stop_seconds: u64,
    /// The free disk that a pull needs.
    pub free_disk_bytes: u64,
}

/// What the limits need to know of the machine.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Machine {
    pub memory_bytes: u64,
    pub processors: u64,
    /// The free disk where the engine keeps its images.
    pub free_disk_bytes: u64,
}

/// The limits of one service, each with a value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OwnLimits {
    pub memory_bytes: u64,
    pub processes: u64,
    pub processor_billionths: u64,
    pub stop_seconds: u64,
}

impl Limits {
    /// The limits after the install: a quarter of the memory of the machine, 512 processes,
    /// the processors of the machine, 30 seconds to stop, 10 GiB of free disk before a pull.
    #[must_use]
    pub const fn after_install() -> Self {
        Self {
            memory_bytes: None,
            processes: 512,
            processor_billionths: None,
            stop_seconds: 30,
            free_disk_bytes: 10 << 30,
        }
    }

    /// The limits of a service on `machine`. No limit is zero: zero is "no limit" to the
    /// engine.
    #[must_use]
    pub fn own(&self, machine: &Machine) -> OwnLimits {
        let quarter = (machine.memory_bytes / 4).max(1);
        let each = machine.processors.max(1).saturating_mul(1_000_000_000);
        OwnLimits {
            memory_bytes: self.memory_bytes.unwrap_or(quarter).max(1),
            processes: self.processes.max(1),
            processor_billionths: self.processor_billionths.unwrap_or(each).max(1),
            stop_seconds: self.stop_seconds.max(1),
        }
    }
}

/// The settings of one machine.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Settings {
    /// "Manage applications": `None` is off.
    pub managed: Option<Managed>,
    /// "All containers".
    pub all_containers: bool,
    /// "Open to the network".
    pub open_to_network: bool,
    /// "The hub may delete volumes".
    pub hub_may_delete_volumes: bool,
    /// "Adopted projects".
    pub adopted: Vec<Adopted>,
    /// "Protected": names of containers and of Compose projects that no item can change.
    pub protected: Vec<String>,
    /// "Registries with a login".
    pub registries: Vec<String>,
    /// "Limits of a service".
    pub limits: Limits,
    /// "Label prefixes".
    pub label_prefixes: Vec<String>,
}

impl Settings {
    /// The settings after the install: each setting is off and each list is empty.
    #[must_use]
    pub const fn after_install() -> Self {
        Self {
            managed: None,
            all_containers: false,
            open_to_network: false,
            hub_may_delete_volumes: false,
            adopted: Vec::new(),
            protected: Vec::new(),
            registries: Vec::new(),
            limits: Limits::after_install(),
            label_prefixes: Vec::new(),
        }
    }

    /// What the check of a Compose file needs of these settings.
    #[must_use]
    pub fn of_a_file(&self) -> compose_rules::Settings {
        compose_rules::Settings {
            open_to_network: self.open_to_network,
            label_prefixes: self.label_prefixes.clone(),
        }
    }

    /// True when `name` is in the list "Protected".
    #[must_use]
    pub fn protects(&self, name: &str) -> bool {
        self.protected.iter().any(|protected| protected == name)
    }

    /// The adopted project of the name `name`.
    #[must_use]
    pub fn adopted_project(&self, name: &str) -> Option<&Adopted> {
        self.adopted.iter().find(|adopted| adopted.name == name)
    }
}
