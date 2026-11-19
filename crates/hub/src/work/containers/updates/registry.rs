//! The name of an image as a place at a public registry (HOST-021): pure reading, no network.
//!
//! The name comes from the device. The hub reads only the four registries of [`Registry`],
//! and a repository or a tag reaches a URL only when each character of it is one that a
//! registry permits, so no text of a device can change the host or the path of a request.

/// The public registries the hub reads. A different host has no answer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(in crate::work::containers) enum Registry {
    DockerHub,
    GitHub,
    Quay,
    Microsoft,
}

impl Registry {
    /// The registry an image name means by this host.
    fn of(host: &str) -> Option<Self> {
        match host {
            "docker.io" | "index.docker.io" | "registry-1.docker.io" => Some(Self::DockerHub),
            "ghcr.io" => Some(Self::GitHub),
            "quay.io" => Some(Self::Quay),
            "mcr.microsoft.com" => Some(Self::Microsoft),
            _ => None,
        }
    }

    /// The host that serves the manifests.
    pub(super) const fn host(self) -> &'static str {
        match self {
            Self::DockerHub => "registry-1.docker.io",
            Self::GitHub => "ghcr.io",
            Self::Quay => "quay.io",
            Self::Microsoft => "mcr.microsoft.com",
        }
    }

    /// Where the registry gives a token to a reader with no account, as `(host, path,
    /// service)`; `None` when it asks for none.
    pub(super) const fn token(self) -> Option<(&'static str, &'static str, &'static str)> {
        match self {
            Self::DockerHub => Some(("auth.docker.io", "/token", "registry.docker.io")),
            Self::GitHub => Some(("ghcr.io", "/token", "ghcr.io")),
            Self::Quay | Self::Microsoft => None,
        }
    }
}

/// One tag of one repository at a registry the hub reads.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(in crate::work::containers) struct Reference {
    pub(in crate::work::containers) registry: Registry,
    /// With `library/` for an official image of Docker Hub.
    pub(in crate::work::containers) repository: String,
    pub(in crate::work::containers) tag: String,
}

/// What the name of an image says of where its newer versions are.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::work::containers) enum Named {
    Tag(Reference),
    /// The name holds a digest: the container asked for that image and no newer one.
    Pinned,
    /// A registry the hub does not read.
    OtherRegistry,
    /// Not a name of an image a registry would accept, or an identifier.
    NoName,
}

/// The most characters of a repository and of a tag that the hub puts in a URL.
const LONGEST_REPOSITORY: usize = 255;
const LONGEST_TAG: usize = 128;

fn repository_ok(text: &str) -> bool {
    let part_ok = |part: &str| {
        !part.is_empty()
            && part.bytes().all(|byte| {
                byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"._-".contains(&byte)
            })
            && part
                .bytes()
                .next()
                .is_some_and(|byte| byte.is_ascii_alphanumeric())
    };
    text.len() <= LONGEST_REPOSITORY && text.split('/').all(part_ok)
}

fn tag_ok(text: &str) -> bool {
    let first_ok = text
        .bytes()
        .next()
        .is_some_and(|byte| byte.is_ascii_alphanumeric() || byte == b'_');
    first_ok
        && text.len() <= LONGEST_TAG
        && text
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._-".contains(&byte))
}

/// True for what Docker shows when a container has no name for its image: `sha256:` and
/// digits, or 12 or more hex digits alone.
fn is_identifier(image: &str) -> bool {
    let digits = image.strip_prefix("sha256:");
    let hex = |text: &str| text.bytes().all(|byte| byte.is_ascii_hexdigit());
    digits.is_some() || (image.len() >= 12 && hex(image))
}

/// Reads an image name as Docker does: `[host[:port]/]repository[:tag][@digest]`. The first
/// part is a host when it has a dot or a colon or is `localhost`; with no host the registry
/// is Docker Hub, and a Docker Hub repository of one part is in `library/`. No tag is the
/// tag `latest`.
pub(in crate::work::containers) fn named(image: &str) -> Named {
    if image.contains('@') {
        return Named::Pinned;
    }
    if is_identifier(image) {
        return Named::NoName;
    }
    let (host, rest) = match image.split_once('/') {
        Some((first, rest))
            if first.contains('.') || first.contains(':') || first == "localhost" =>
        {
            (first, rest)
        }
        _ => ("docker.io", image),
    };
    let Some(registry) = Registry::of(host) else {
        return Named::OtherRegistry;
    };
    let last = rest.rsplit('/').next().unwrap_or_default();
    let (repository, tag) = match last.rsplit_once(':') {
        Some((_, tag)) => (&rest[..rest.len() - tag.len() - 1], tag),
        None => (rest, "latest"),
    };
    if !repository_ok(repository) || !tag_ok(tag) {
        return Named::NoName;
    }
    let official = registry == Registry::DockerHub && !repository.contains('/');
    Named::Tag(Reference {
        registry,
        repository: if official {
            format!("library/{repository}")
        } else {
            repository.to_owned()
        },
        tag: tag.to_owned(),
    })
}
