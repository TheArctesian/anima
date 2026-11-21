//! The install of an entry: present, and refused with a fixed reason until task H6 exists
//! (R-299). Before that reason, an entry of the class shell needs a person signed in
//! (OD-HOST-54) and a generated entry needs the digest of its image in the request
//! (OD-HOST-50, SF-NEW-revlsform). No machine is reached.
#![expect(
    clippy::unused_async_trait_impl,
    reason = "the async-graphql Enum derive generates async trait impls"
)]

use async_graphql::Enum;
use catalogue::entry::{Class, Mark};

use super::model::Listed;

/// Why the hub does not install an entry (K1 `CatalogueInstallRefusal`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum)]
#[graphql(name = "CatalogueInstallRefusal")]
pub enum Refusal {
    /// The catalogue lists no entry with this identifier.
    NotListed,
    /// The entry needs the socket of the engine; no such entry is installed (R-247).
    EngineSocket,
    /// The entry is a shell: only a person signed in to the client installs it, never an API
    /// key or an agent (OD-HOST-54).
    ShellNeedsPerson,
    /// The entry is generated from the data of a publisher and not tested by Anima: the
    /// request must name the digest of its image (OD-HOST-50).
    GeneratedNeedsDigest,
    /// The install needs task H6 of the hosted applications plan, which is not built.
    InstallNotBuilt,
}

impl Refusal {
    /// The fixed text of the refusal, for a person.
    #[must_use]
    pub const fn reason(self) -> &'static str {
        match self {
            Self::NotListed => "The catalogue lists no entry with this identifier.",
            Self::EngineSocket => {
                "This entry needs the socket of the container engine. Anima installs no \
                 such entry (R-247)."
            }
            Self::ShellNeedsPerson => {
                "A person who reaches this entry is root in its container. Only a person who \
                 is signed in to the client can install it, not an API key or an agent."
            }
            Self::GeneratedNeedsDigest => {
                "Anima did not test this entry: it is generated from the data of its \
                 publisher. To install it, a person confirms the digest of its image."
            }
            Self::InstallNotBuilt => {
                "The install needs task H6 of the hosted applications plan, which is not \
                 built. No machine was reached."
            }
        }
    }
}

/// The answer of the hub to the install of one entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Install {
    /// `None` when the hub would install the entry. Never `None` today.
    pub refusal: Option<Refusal>,
    /// The entry publishes a port on the network: the machine needs "Open to the network"
    /// (R-229).
    pub needs_open_to_network: bool,
}

/// Who asks for an install, and what the request confirms.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Ask<'a> {
    /// A person signed in to the client (a login or a browser token, not declared an agent);
    /// false for an API key and an agent.
    pub by_person: bool,
    /// The digest that the request names, `sha256:` and 64 hexadecimal digits.
    pub digest: Option<&'a str>,
}

/// True when `digest` is the digest of each image of `listed`.
fn digest_named(listed: &Listed, digest: Option<&str>) -> bool {
    let digests = listed.image_digests();
    digest.is_some_and(|named| !digests.is_empty() && digests.iter().all(|own| *own == named))
}

/// The first refusal that applies to the install of `listed` by `ask`.
fn refusal(listed: &Listed, ask: Ask<'_>) -> Refusal {
    let anima = &listed.entry.anima;
    if anima.marks.contains(&Mark::EngineSocket) {
        Refusal::EngineSocket
    } else if anima.class == Class::Shell && !ask.by_person {
        Refusal::ShellNeedsPerson
    } else if anima.origin.is_some() && !digest_named(listed, ask.digest) {
        Refusal::GeneratedNeedsDigest
    } else {
        Refusal::InstallNotBuilt
    }
}

/// The answer to the install of `listed` by `ask`; `listed` is `None` for an identifier that
/// is not listed. The first refusal that applies is the one given.
#[must_use]
pub fn install(listed: Option<&Listed>, ask: Ask<'_>) -> Install {
    let Some(listed) = listed else {
        return Install {
            refusal: Some(Refusal::NotListed),
            needs_open_to_network: false,
        };
    };
    let refusal = refusal(listed, ask);
    Install {
        refusal: Some(refusal),
        needs_open_to_network: !listed.ports_on_network().is_empty(),
    }
}
