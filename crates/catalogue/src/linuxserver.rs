//! The general recipe form of the images of LinuxServer.io (task H39, HOST-049, Section 15.4 of
//! the proposal): the recipe of Section 13 for one image, made from the `config` of the fleet
//! list of the publisher, or the reason that the image cannot fit the rules of a file from the
//! hub. A recipe made here is tier 2, "generated, not tested". Nothing here reaches a network:
//! the caller reads the fleet list and the digest.

mod assess;
mod entry;
mod plan;
mod update;
mod write;

pub use assess::{Assessment, assess};
pub use update::{Change, reviewed, tested, update};

use serde::Deserialize;

use crate::{
    Finding, Recipe, check,
    entry::{MachineTest, Reviewed},
};

/// The fleet list cannot be read.
#[derive(Debug, thiserror::Error)]
#[error("the fleet list cannot be read")]
pub struct FleetError(#[source] serde_json::Error);

#[derive(Deserialize)]
struct Fleet {
    last_updated: String,
    #[serde(rename = "data")]
    body: Body,
}

#[derive(Deserialize)]
struct Body {
    repositories: Repositories,
}

#[derive(Deserialize)]
struct Repositories {
    linuxserver: Vec<Image>,
}

/// The fleet list of the publisher: the day of its data, `YYYY-MM-DD`, and each image.
#[derive(Clone, Debug)]
pub struct List {
    pub read: String,
    pub images: Vec<Image>,
}

/// The fleet list in the form of `https://api.linuxserver.io/api/v1/images?include_config=true`.
///
/// # Errors
/// [`FleetError`] when the text is not that form.
pub fn list(text: &str) -> Result<List, FleetError> {
    let fleet: Fleet = serde_json::from_str(text).map_err(FleetError)?;
    Ok(List {
        read: fleet.last_updated.chars().take(10).collect(),
        images: fleet.body.repositories.linuxserver,
    })
}

/// One image of the fleet list. A key that is not here is not read.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct Image {
    pub name: String,
    pub description: String,
    /// The tag of the last build, for example `7.0.3.10918-ls328`.
    pub version: String,
    /// The categories, separated by `,`.
    pub category: String,
    pub github_url: String,
    pub project_logo: String,
    pub deprecated: bool,
    pub config: Config,
}

/// What the image needs, as the publisher describes it. Each part has a mark "optional".
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct Config {
    pub env_vars: Vec<Variable>,
    pub volumes: Vec<Volume>,
    pub ports: Vec<Port>,
    pub devices: Vec<Device>,
    pub caps: Vec<Capability>,
    pub security_opt: Vec<SecurityOption>,
    pub custom: Vec<Custom>,
    /// `host` for the network of the machine.
    pub networking: Option<String>,
    pub privileged: bool,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct Variable {
    pub name: String,
    /// The example value of the publisher.
    pub value: String,
    pub optional: bool,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct Volume {
    /// The path in the container.
    pub path: String,
    /// The path on the machine: a placeholder `/path/to/...`, or a real path of the machine.
    pub host_path: String,
    pub optional: bool,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct Port {
    /// The port on the machine.
    pub external: String,
    /// The port in the container, with `/udp` or `/tcp` after it or nothing.
    pub internal: String,
    pub optional: bool,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct Device {
    pub path: String,
    pub optional: bool,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct Capability {
    pub cap_add: String,
    pub optional: bool,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct SecurityOption {
    pub compose_var: String,
    pub optional: bool,
}

/// A key of Compose that the list gives by its name: `shm_size`, `sysctls`, `stop_grace_period`.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct Custom {
    pub name_compose: String,
    pub optional: bool,
}

/// The digest of the image as the registry gave it, and the days of the reads.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pin {
    /// The tag that was read, for example `7.0.3.10918-ls328`.
    pub tag: String,
    /// `sha256:` and 64 hexadecimal digits: the digest of the index of all platforms.
    pub digest: String,
    /// The day the registry was read, `YYYY-MM-DD`.
    pub resolved: String,
    /// The day of the data of the fleet list, `YYYY-MM-DD`.
    pub read: String,
}

/// What the catalogue holds already for the recipe that the form makes.
#[derive(Clone, Copy, Debug, Default)]
pub struct Existing<'a> {
    /// The review state of the recipe that exists (SF-255), which the form keeps.
    pub reviewed: Option<&'a Reviewed>,
    /// The last test on a machine of the recipe that exists. The form keeps a pass only for
    /// the same digest.
    pub tested: Option<&'a MachineTest>,
    /// The ports of the machine that the other recipes of the catalogue publish.
    pub taken: &'a [u16],
}

/// Why an image cannot be a recipe from the hub. Such an image is tier 3: it is installed at
/// the machine and adopted (R-228).
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum Unfit {
    #[error("the publisher gives no configuration")]
    NoConfig,
    #[error("the image needs `privileged`")]
    Privileged,
    #[error("the image needs the network of the machine")]
    HostNetwork,
    #[error("the image needs the capability {0}")]
    Capability(String),
    #[error("the image needs a `security_opt` that the rules refuse")]
    SecurityOption,
    #[error("the image needs a device of the machine")]
    Device,
    #[error("the image needs `shm_size`, which is not a key of the rules yet (OD-HOST-51)")]
    SharedMemory,
    #[error("the image needs `sysctls`")]
    KernelSetting,
    #[error("the image needs `DOCKER_MODS` or a package install at the start")]
    ImageMods,
    #[error("the image needs a `VERSION` other than `docker`")]
    VersionNotFixed,
    #[error("the image needs the socket of the engine (R-247)")]
    EngineSocket,
    #[error("the image needs a path of the machine")]
    HostPath,
    #[error("the image publishes no TCP port that a test can reach")]
    NoPort,
    #[error("a port of the image has a form that the rules refuse")]
    PortNotPermitted,
    #[error("a variable needs a value that only a person or a different service knows")]
    ValueNeeded,
    #[error("a variable has a name or a value that an environment file cannot hold")]
    VariableNotPermitted,
    #[error("a volume has a path that the rules refuse")]
    VolumeNotPermitted,
    #[error("the name of the image is not a name of the rules")]
    NameNotPermitted,
    #[error("the image is a shell with no credential that the install can make (OD-HOST-54)")]
    ShellWithoutCredential,
    #[error("the static test refuses the recipe")]
    Refused(Vec<Finding>),
}

impl Unfit {
    /// The reason as the report writes it.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::NoConfig => "no_config",
            Self::Privileged => "privileged",
            Self::HostNetwork => "host_network",
            Self::Capability(_) => "capability",
            Self::SecurityOption => "security_option",
            Self::Device => "device",
            Self::SharedMemory => "shared_memory",
            Self::KernelSetting => "kernel_setting",
            Self::ImageMods => "image_mods",
            Self::VersionNotFixed => "version_not_fixed",
            Self::EngineSocket => "engine_socket",
            Self::HostPath => "host_path",
            Self::NoPort => "no_port",
            Self::PortNotPermitted => "port_not_permitted",
            Self::ValueNeeded => "value_needed",
            Self::VariableNotPermitted => "variable_not_permitted",
            Self::VolumeNotPermitted => "volume_not_permitted",
            Self::NameNotPermitted => "name_not_permitted",
            Self::ShellWithoutCredential => "shell_without_credential",
            Self::Refused(_) => "refused",
        }
    }
}

/// The recipe of `image`, pinned by `pin`, which passes the static test; or why it cannot fit.
///
/// The recipe has only the parts that are not optional, with three exceptions: each volume of
/// a placeholder path, the device of hardware decoding as an optional part that is off, and the
/// credential of a shell. `PUID` and `PGID` are `1000`; each service refuses new rights and
/// drops each capability but the five of the init (`init-capabilities`); a port of the machine
/// below 1024 moves up by 8000; each port is on `127.0.0.1` with its protocol.
///
/// A port of the machine that a different recipe of the catalogue publishes moves to the next
/// free port, so that two entries can run on one machine. The entry keeps the review state of
/// the recipe that exists, else it is `not-reviewed` (SF-255). It keeps a passed
/// test on a machine only when the test ran with the same digest; else it is `not-tested`.
///
/// # Errors
/// The [`Unfit`] reason of the first need that the rules refuse.
pub fn recipe(image: &Image, pin: &Pin, existing: Existing<'_>) -> Result<Recipe, Unfit> {
    let plan = plan::plan(image, existing.taken)?;
    let made = write::recipe(image, &plan, pin, existing);
    let findings = check(&made);
    if findings.is_empty() {
        Ok(made)
    } else {
        Err(Unfit::Refused(findings))
    }
}
