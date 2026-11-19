//! The images, the volumes and the networks of a device, read-only (HOST-020): the two
//! `docker` command lines and the pure reading of their output.
//!
//! Each command line writes one JSON object on a line, with a `kind` that says what the line
//! describes. Docker counts the containers that use an image or a volume; the names of those
//! containers come from the list of the containers in the same answer. A size is the text
//! that Docker writes (`13.6MB`): Docker gives no exact number here.

use serde::Deserialize;

use super::{Error, docker::failure};
use crate::fleet::ssh::{CommandOutput, FixedCommand};

/// The images and the volumes with their sizes, from `docker system df -v`.
const DETAILED_FORMAT: &str = concat!(
    r#"'{{range .Images}}{"kind":"image","id":{{json .ID}},"repository":{{json .Repository}},"#,
    r#""tag":{{json .Tag}},"size":{{json .Size}},"shared":{{json .SharedSize}},"#,
    r#""unique":{{json .UniqueSize}},"count":{{json .Containers}},"#,
    r#""created":{{json .CreatedAt}}}{{println}}{{end}}"#,
    r#"{{range .Volumes}}{"kind":"volume","name":{{json .Name}},"driver":{{json .Driver}},"#,
    r#""size":{{json .Size}},"count":{{json .Links}}}{{println}}{{end}}'"#,
);

/// One line for each type of `docker system df`, with the space that a clean-up can free.
const USAGE_FORMAT: &str = concat!(
    r#"'{"kind":"usage","type":{{json .Type}},"total":{{json .TotalCount}},"#,
    r#""active":{{json .Active}},"size":{{json .Size}},"reclaimable":{{json .Reclaimable}}}'"#,
);

/// What each container uses: its image, its mounts and its networks. No label is asked for.
const USERS_FORMAT: &str = concat!(
    r#"'{"kind":"container","names":{{json .Names}},"image":{{json .Image}},"#,
    r#""mounts":{{json .Mounts}},"networks":{{json .Networks}}}'"#,
);

const NETWORKS_FORMAT: &str = concat!(
    r#"'{"kind":"network","id":{{json .ID}},"name":{{json .Name}},"driver":{{json .Driver}},"#,
    r#""scope":{{json .Scope}},"internal":{{json .Internal}}}'"#,
);

/// The image tag and the repository that Docker writes for an image with none.
const NONE: &str = "<none>";

/// One image of a device.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Image {
    /// `sha256:` and 64 digits.
    pub id: String,
    /// `None` for an image with no name.
    pub repository: Option<String>,
    pub tag: Option<String>,
    /// The size as Docker writes it, for example `13.6MB`.
    pub size: String,
    /// The part of the size that other images share.
    pub shared_size: String,
    /// The part that a removal of this image frees.
    pub unique_size: String,
    /// The time as Docker writes it.
    pub created: String,
    /// How many containers use the image, as Docker counts them.
    pub container_count: u32,
    /// The names of the containers that name this image.
    pub containers: Vec<String>,
}

/// One volume of a device.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Volume {
    pub name: String,
    pub driver: String,
    pub size: String,
    /// How many containers use the volume, as Docker counts them.
    pub container_count: u32,
    pub containers: Vec<String>,
}

/// The space of one type of thing that Docker keeps, and what a clean-up can free.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiskUsage {
    /// `Images`, `Containers`, `Local Volumes` or `Build Cache`, as Docker writes it.
    pub kind: String,
    pub total_count: u32,
    pub active_count: u32,
    pub size: String,
    /// The space that a clean-up can free, for example `49.63MB (10%)`.
    pub reclaimable: String,
}

/// The images and the volumes of a device, with the space each type uses.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Storage {
    pub images: Vec<Image>,
    pub volumes: Vec<Volume>,
    pub usage: Vec<DiskUsage>,
}

/// One network of Docker on a device.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Network {
    pub id: String,
    pub name: String,
    pub driver: String,
    pub scope: String,
    /// The network has no route to outside the device.
    pub internal: bool,
    pub containers: Vec<String>,
}

/// Adds `&& docker ps` for what each container uses.
fn and_users(command: FixedCommand) -> FixedCommand {
    command
        .arg("&&")
        .arg("docker")
        .arg("ps")
        .arg("-a")
        .arg("--no-trunc")
        .arg("--format")
        .arg(USERS_FORMAT)
}

/// `docker system df`, twice, and the list of the containers, from static text only.
pub(super) fn storage_command() -> FixedCommand {
    let sizes = FixedCommand::new("docker")
        .arg("system")
        .arg("df")
        .arg("-v")
        .arg("--format")
        .arg(DETAILED_FORMAT)
        .arg("&&")
        .arg("docker")
        .arg("system")
        .arg("df")
        .arg("--format")
        .arg(USAGE_FORMAT);
    and_users(sizes)
}

/// `docker network ls` and the list of the containers, from static text only.
pub(super) fn networks_command() -> FixedCommand {
    let networks = FixedCommand::new("docker")
        .arg("network")
        .arg("ls")
        .arg("--no-trunc")
        .arg("--format")
        .arg(NETWORKS_FORMAT);
    and_users(networks)
}

/// One line of either command line. Docker writes each number and each truth value as text.
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
enum Line {
    Image(ImageLine),
    Volume(VolumeLine),
    Usage(UsageLine),
    Network(NetworkLine),
    Container(User),
}

#[derive(Deserialize)]
struct ImageLine {
    id: String,
    repository: String,
    tag: String,
    size: String,
    shared: String,
    unique: String,
    count: String,
    created: String,
}

impl ImageLine {
    fn image(self) -> Result<Image, Error> {
        Ok(Image {
            id: self.id,
            repository: named(self.repository),
            tag: named(self.tag),
            size: self.size,
            shared_size: self.shared,
            unique_size: self.unique,
            created: self.created,
            container_count: count(&self.count)?,
            containers: Vec::new(),
        })
    }
}

#[derive(Deserialize)]
struct VolumeLine {
    name: String,
    driver: String,
    size: String,
    count: String,
}

impl VolumeLine {
    fn volume(self) -> Result<Volume, Error> {
        Ok(Volume {
            name: self.name,
            driver: self.driver,
            size: self.size,
            container_count: count(&self.count)?,
            containers: Vec::new(),
        })
    }
}

#[derive(Deserialize)]
struct UsageLine {
    #[serde(rename = "type")]
    kind: String,
    total: String,
    active: String,
    size: String,
    reclaimable: String,
}

impl UsageLine {
    fn usage(self) -> Result<DiskUsage, Error> {
        Ok(DiskUsage {
            kind: self.kind,
            total_count: count(&self.total)?,
            active_count: count(&self.active)?,
            size: self.size,
            reclaimable: self.reclaimable,
        })
    }
}

#[derive(Deserialize)]
struct NetworkLine {
    id: String,
    name: String,
    driver: String,
    scope: String,
    internal: String,
}

impl NetworkLine {
    fn network(self) -> Network {
        Network {
            id: self.id,
            name: self.name,
            driver: self.driver,
            scope: self.scope,
            internal: self.internal == "true",
            containers: Vec::new(),
        }
    }
}

/// What one container uses.
#[derive(Deserialize)]
struct User {
    names: String,
    image: String,
    mounts: String,
    networks: String,
}

impl User {
    fn name(&self) -> String {
        super::docker::shown_name(&self.names)
    }

    fn lists(text: &str, wanted: &str) -> bool {
        text.split(',').any(|name| name.trim() == wanted)
    }

    /// Whether the container names this image: by `repository:tag`, by the repository alone
    /// for the tag `latest`, or by 12 or more digits of the identifier.
    fn runs(&self, image: &Image) -> bool {
        let named = self.image.split('@').next().unwrap_or_default();
        let digits = image.id.trim_start_matches("sha256:");
        let by_id = named.trim_start_matches("sha256:");
        if by_id.len() >= 12 && digits.starts_with(by_id) {
            return true;
        }
        match (&image.repository, &image.tag) {
            (Some(repository), Some(tag)) => {
                named == format!("{repository}:{tag}") || (tag == "latest" && named == repository)
            }
            _ => false,
        }
    }
}

/// The lines of an answer that exited 0.
fn lines(output: &CommandOutput) -> Result<Vec<Line>, Error> {
    if output.cut {
        return Err(Error::Unreadable);
    }
    if output.exit_status != Some(0) {
        return Err(failure(output));
    }
    let text = std::str::from_utf8(&output.stdout).map_err(|_| Error::Unreadable)?;
    text.lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).map_err(|_| Error::Unreadable))
        .collect()
}

fn count(text: &str) -> Result<u32, Error> {
    text.parse().map_err(|_| Error::Unreadable)
}

fn named(text: String) -> Option<String> {
    Some(text).filter(|text| !text.is_empty() && text != NONE)
}

fn using(users: &[User], wanted: impl Fn(&User) -> bool) -> Vec<String> {
    users
        .iter()
        .filter(|user| wanted(user))
        .map(User::name)
        .collect()
}

/// The images, the volumes and the space of each type.
///
/// # Errors
/// `Unreadable` when the output was cut or a line is not what the formats write; `NoDocker`
/// or `DockerFailed` when a command did not exit 0.
pub(super) fn storage(output: &CommandOutput) -> Result<Storage, Error> {
    let mut read = Storage {
        images: Vec::new(),
        volumes: Vec::new(),
        usage: Vec::new(),
    };
    let mut users = Vec::new();
    for line in lines(output)? {
        match line {
            Line::Image(line) => read.images.push(line.image()?),
            Line::Volume(line) => read.volumes.push(line.volume()?),
            Line::Usage(line) => read.usage.push(line.usage()?),
            Line::Container(user) => users.push(user),
            Line::Network(_) => return Err(Error::Unreadable),
        }
    }
    for image in &mut read.images {
        image.containers = using(&users, |user| user.runs(image));
    }
    for volume in &mut read.volumes {
        volume.containers = using(&users, |user| User::lists(&user.mounts, &volume.name));
    }
    Ok(read)
}

/// The networks, each with the containers on it.
///
/// # Errors
/// As [`storage`].
pub(super) fn networks(output: &CommandOutput) -> Result<Vec<Network>, Error> {
    let mut networks = Vec::new();
    let mut users = Vec::new();
    for line in lines(output)? {
        match line {
            Line::Network(line) => networks.push(line.network()),
            Line::Container(user) => users.push(user),
            _ => return Err(Error::Unreadable),
        }
    }
    for network in &mut networks {
        network.containers = using(&users, |user| User::lists(&user.networks, &network.name));
    }
    Ok(networks)
}
