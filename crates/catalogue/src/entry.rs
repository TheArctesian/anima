//! The entry file of a recipe (`entry.json`): the fields of the template registry of Arcane at
//! the top level (HOST-025), so that a list made for Arcane reads the same entry, and the fields
//! of Anima below the key `anima` (HOST-047). A key that is not here makes the file unreadable.

use serde::Deserialize;

/// One entry file.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub author: String,
    pub tags: Vec<String>,
    pub compose_url: String,
    pub env_url: String,
    pub documentation_url: String,
    pub icon_url: Option<String>,
    pub anima: Anima,
}

/// The fields that Anima adds to an entry of Arcane.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Anima {
    /// What the entry installs: an application, or a database server (R-216, task H24).
    #[serde(default)]
    pub kind: Kind,
    /// For an entry of kind `database`: the server that it installs.
    pub server: Option<Server>,
    /// For an application: the database that it needs on a server of the machine.
    pub database: Option<Need>,
    /// The image of each service, by digest (R-233).
    pub images: Vec<Pinned>,
    /// What the recipe needs beyond the default rules. Empty for most recipes.
    #[serde(default)]
    pub marks: Vec<Mark>,
    /// The exceptions of a fixed list to the rights that each service drops. Empty for most
    /// recipes.
    #[serde(default)]
    pub exceptions: Vec<Exception>,
    /// Each top-level volume of the Compose file, and how a copy of it is made.
    pub volumes: Vec<Kept>,
    /// The variables of the environment file that the install makes. The example file has
    /// no value for them.
    #[serde(default)]
    pub generated: Vec<String>,
    /// The test that a machine runs after the install (task H37).
    pub test: HealthTest,
    /// The day of the last pin of the digests, `YYYY-MM-DD`.
    pub pinned: String,
    /// The last test on a machine.
    pub last_test: MachineTest,
}

/// What an entry installs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    #[default]
    Application,
    /// A database server that applications of the machine use (Section 14 of the proposal).
    Database,
}

/// The engine of a database server.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Engine {
    Postgresql,
}

/// The server that an entry of kind `database` installs.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Server {
    pub engine: Engine,
    /// The major version of the image.
    pub major: u16,
    /// The extensions that the image has and that an application may name, for example
    /// `pg_trgm` or `postgis`.
    #[serde(default)]
    pub extensions: Vec<String>,
}

/// The database that an application needs: the engine, the lowest major version, the
/// extensions, and the names of the variables in which it wants the connection values.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Need {
    pub engine: Engine,
    pub lowest_major: u16,
    #[serde(default)]
    pub extensions: Vec<String>,
    pub variables: Variables,
}

/// The name of the variable for each connection value that the application wants. At least
/// one is given. The install writes each one, so each is also in `generated`.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Variables {
    /// The whole connection address, `postgresql://user:password@host:port/database`.
    pub url: Option<String>,
    pub host: Option<String>,
    pub port: Option<String>,
    pub name: Option<String>,
    pub user: Option<String>,
    pub password: Option<String>,
}

impl Variables {
    /// Each variable that is given.
    #[must_use]
    pub fn named(&self) -> Vec<&str> {
        [
            &self.url,
            &self.host,
            &self.port,
            &self.name,
            &self.user,
            &self.password,
        ]
        .into_iter()
        .filter_map(Option::as_deref)
        .collect()
    }
}

impl Need {
    /// True when `server` has the engine, a major version from the lowest one, and each
    /// extension that this need names.
    #[must_use]
    pub fn met_by(&self, server: &Server) -> bool {
        self.engine == server.engine
            && server.major >= self.lowest_major
            && self
                .extensions
                .iter()
                .all(|extension| server.extensions.contains(extension))
    }
}

/// The image of one service.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pinned {
    pub service: String,
    /// `repository@sha256:<64 hex digits>`.
    pub image: String,
    /// The tag that was read from the registry to get the digest. Only for people.
    pub tag: String,
    /// The day the tag was read, `YYYY-MM-DD`.
    pub resolved: String,
}

/// A need of a recipe that the default rules of a Compose file do not permit.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(tag = "mark", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Mark {
    /// A port on each address of the machine: the machine needs "Open to the network" (R-229).
    NetworkPort { port: u16, protocols: Vec<Protocol> },
    /// The socket of the engine (R-247). Refused until the exception exists.
    EngineSocket,
    /// The network of the machine itself, not a network of the project.
    HostNetwork,
    /// A device of the machine, for example a USB stick or a graphics processor.
    Device,
}

/// An exception to "each service drops each capability" (SF-224).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Exception {
    /// An init that starts as root, gives the files to a different user and changes to it:
    /// the s6 init of the images of `LinuxServer`, `gosu` or `su-exec`. It keeps exactly `CHOWN`,
    /// `DAC_OVERRIDE`, `FOWNER`, `SETGID` and `SETUID`.
    InitCapabilities,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Protocol {
    Tcp,
    Udp,
}

/// One volume and the kind of copy that it needs.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Kept {
    pub volume: String,
    pub copy: Copy,
}

/// How a copy of a volume is made (HOST-042).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Copy {
    /// Plain files: a copy of the files is good at any time.
    Files,
    /// An SQLite database: a copy needs the backup function of SQLite or a stopped service.
    Sqlite,
    /// A PostgreSQL server: a copy needs `pg_dump` or a copy of the stopped server.
    Postgresql,
    /// A cache that the application makes again: no copy is needed.
    Cache,
}

/// What the test of a machine reaches.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Reach {
    /// An HTTP request to a path.
    Http,
    /// A TCP connection only.
    Tcp,
}

/// The test that a machine runs: one port of one service, and the answer that counts as
/// healthy, inside a time limit. Data only: task H37 runs it.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HealthTest {
    pub service: String,
    /// The port in the container, which a `ports` entry of the service publishes.
    pub port: u16,
    pub reach: Reach,
    /// For HTTP: the path, which starts with `/`.
    pub path: Option<String>,
    /// For HTTP: each status that counts as healthy.
    #[serde(default)]
    pub statuses: Vec<u16>,
    /// For HTTP: a text that the body of a healthy answer contains.
    pub body: Option<String>,
    /// The most seconds from the start of the install to a healthy answer.
    pub limit_seconds: u32,
}

/// The result of the last test on a machine.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(tag = "result", rename_all = "kebab-case", deny_unknown_fields)]
pub enum MachineTest {
    NotTested,
    Passed { date: String },
    Failed { date: String },
}

/// The name of a mark as the entry file writes it.
#[must_use]
pub const fn mark_name(mark: &Mark) -> &'static str {
    match mark {
        Mark::NetworkPort { .. } => "network-port",
        Mark::EngineSocket => "engine-socket",
        Mark::HostNetwork => "host-network",
        Mark::Device => "device",
    }
}
