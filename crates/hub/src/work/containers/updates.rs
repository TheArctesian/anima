//! A newer image for a container (HOST-021): the digest of the image on the device against
//! the digest of the same tag at its registry.
//!
//! One fixed read-only command line gives the image that each container names and the digest
//! that Docker keeps for each tag. The hub then asks the registry of each tag for its digest
//! now ([`egress`]). It reads four public registries and sends no credential. It updates
//! nothing (ETH-004): the answer only says that a newer image exists.

pub(super) mod egress;
pub(super) mod registry;

use std::{collections::HashMap, time::Duration};

use futures_util::future::join_all;
use serde::Deserialize;

use self::{
    egress::{Egress, FetchError, is_digest},
    registry::{Named, Reference, named},
};
use super::{
    Error,
    docker::{failure, shown_name},
    model::{NotKnown, UpdateState},
};
use crate::fleet::ssh::{CommandOutput, FixedCommand};

/// What each container names as its image. No label is asked for.
const USERS_FORMAT: &str = concat!(
    r#"'{"kind":"container","id":{{json .ID}},"names":{{json .Names}},"#,
    r#""image":{{json .Image}}}'"#,
);

/// The digest that Docker keeps for each tag of each image.
const DIGESTS_FORMAT: &str = concat!(
    r#"'{"kind":"image","repository":{{json .Repository}},"tag":{{json .Tag}},"#,
    r#""digest":{{json .Digest}}}'"#,
);

/// The most different tags one read asks the registries for.
pub(super) const MOST_TAGS: usize = 40;
/// The longest the read of one tag may take: the token and the manifest.
const READ_LIMIT: Duration = Duration::from_secs(25);

/// Whether a newer image exists for one container.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImageUpdate {
    pub container_id: String,
    pub container: String,
    /// The image as the container names it.
    pub image: String,
    pub state: UpdateState,
    /// `Some` when the state is `NotKnown`.
    pub reason: Option<NotKnown>,
    /// The digest that the device has for the tag.
    pub local_digest: Option<String>,
    /// The digest that the registry gives for the tag now.
    pub registry_digest: Option<String>,
}

/// `docker ps` and `docker image ls --digests`, from static text only.
pub(super) fn command() -> FixedCommand {
    FixedCommand::new("docker")
        .arg("ps")
        .arg("-a")
        .arg("--no-trunc")
        .arg("--format")
        .arg(USERS_FORMAT)
        .arg("&&")
        .arg("docker")
        .arg("image")
        .arg("ls")
        .arg("--no-trunc")
        .arg("--digests")
        .arg("--format")
        .arg(DIGESTS_FORMAT)
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
enum Line {
    Container {
        id: String,
        names: String,
        image: String,
    },
    Image {
        repository: String,
        tag: String,
        digest: String,
    },
}

/// What the device says: the image each container names, and the digest of each tag.
pub(super) struct Found {
    /// `(identifier, name, image)` of each container, in Docker's order.
    containers: Vec<(String, String, String)>,
    /// The digests of each tag. Docker can keep more than one for a tag, for example when
    /// an index was published again with the same image.
    digests: HashMap<Reference, Vec<String>>,
}

/// Reads the answer of [`command`].
///
/// # Errors
/// `Unreadable` when the output was cut or a line is not what the formats write; `NoDocker`
/// or `DockerFailed` when a command did not exit 0.
pub(super) fn found(output: &CommandOutput) -> Result<Found, Error> {
    if output.cut {
        return Err(Error::Unreadable);
    }
    if output.exit_status != Some(0) {
        return Err(failure(output));
    }
    let text = std::str::from_utf8(&output.stdout).map_err(|_| Error::Unreadable)?;
    let mut read = Found {
        containers: Vec::new(),
        digests: HashMap::new(),
    };
    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        match serde_json::from_str(line).map_err(|_| Error::Unreadable)? {
            Line::Container { id, names, image } => {
                read.containers.push((id, shown_name(&names), image));
            }
            Line::Image {
                repository,
                tag,
                digest,
            } => {
                let place = named(&format!("{repository}:{tag}"));
                if let (Named::Tag(reference), true) = (place, is_digest(&digest)) {
                    read.digests.entry(reference).or_default().push(digest);
                }
            }
        }
    }
    Ok(read)
}

impl Found {
    /// The different tags to ask the registries for: those a container names and the device
    /// has a digest for, at most [`MOST_TAGS`], in the order of the containers.
    fn wanted(&self) -> Vec<Reference> {
        let mut wanted: Vec<Reference> = Vec::new();
        for (_, _, image) in &self.containers {
            if let Named::Tag(reference) = named(image) {
                let new = self.digests.contains_key(&reference) && !wanted.contains(&reference);
                if new && wanted.len() < MOST_TAGS {
                    wanted.push(reference);
                }
            }
        }
        wanted
    }

    /// The answer for each container, from what the registries gave.
    fn compared(self, read: &HashMap<Reference, Result<String, FetchError>>) -> Vec<ImageUpdate> {
        let Self {
            containers,
            digests,
        } = self;
        containers
            .into_iter()
            .map(|(container_id, container, image)| {
                let (state, reason, local, remote) = match named(&image) {
                    Named::Pinned => not_known(NotKnown::PinnedDigest, None),
                    Named::OtherRegistry => not_known(NotKnown::OtherRegistry, None),
                    Named::NoName => not_known(NotKnown::NoName, None),
                    Named::Tag(reference) => match digests.get(&reference) {
                        None => not_known(NotKnown::NoLocalDigest, None),
                        Some(local) => judged(local, read.get(&reference)),
                    },
                };
                ImageUpdate {
                    container_id,
                    container,
                    image,
                    state,
                    reason,
                    local_digest: local,
                    registry_digest: remote,
                }
            })
            .collect()
    }
}

type Verdict = (
    UpdateState,
    Option<NotKnown>,
    Option<String>,
    Option<String>,
);

fn not_known(reason: NotKnown, local: Option<String>) -> Verdict {
    (UpdateState::NotKnown, Some(reason), local, None)
}

/// The verdict for a tag the device has the digests `local` for: up to date when the
/// registry gives one of them.
fn judged(local: &[String], read: Option<&Result<String, FetchError>>) -> Verdict {
    let first = local.first().cloned();
    match read {
        Some(Ok(remote)) if local.contains(remote) => (
            UpdateState::UpToDate,
            None,
            Some(remote.clone()),
            Some(remote.clone()),
        ),
        Some(Ok(remote)) => (UpdateState::NewerImage, None, first, Some(remote.clone())),
        Some(Err(FetchError::NotThere)) => not_known(NotKnown::NotAtRegistry, first),
        Some(Err(FetchError::Failed)) | None => not_known(NotKnown::RegistryNotRead, first),
    }
}

/// Asks the registries for each wanted tag, all at the same time, and gives the answer for
/// each container. A read that does not end in [`READ_LIMIT`], the look-up of its host
/// included, is a registry that was not read; the other reads keep their answers.
pub(super) async fn compare(found: Found, egress: &Egress) -> Vec<ImageUpdate> {
    let wanted = found.wanted();
    let answers = join_all(wanted.iter().map(|reference| async move {
        tokio::time::timeout(READ_LIMIT, egress.digest(reference))
            .await
            .unwrap_or(Err(FetchError::Failed))
    }))
    .await;
    let read: HashMap<Reference, Result<String, FetchError>> =
        wanted.into_iter().zip(answers).collect();
    found.compared(&read)
}
