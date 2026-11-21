//! The install of an entry: present, and refused with a fixed reason until task H6 exists
//! (R-299). Before that reason, an entry of the class shell needs a person signed in to the
//! Anima app in a browser (OD-HOST-54), and a generated entry needs the digest of its image in
//! the request and, from anything but that app, a passed test on a machine with that image
//! (OD-HOST-50, SF-252). No machine is reached.
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
    /// The entry is a shell: only a person signed in to the Anima app in a browser installs it,
    /// never the `anima` command, an API key or an agent (OD-HOST-54, SF-252).
    ShellNeedsPerson,
    /// The entry is generated from the data of a publisher and not tested by Anima: the
    /// request must name the digest of its image (OD-HOST-50).
    GeneratedNeedsDigest,
    /// The entry is generated and no test on a machine passed with its image: only a person
    /// in the Anima app in a browser installs it (SF-252).
    GeneratedNotTested,
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
                "A person who reaches this entry is root in its container. Install it from \
                 the Anima app in a web browser. The anima command, an API key and an agent \
                 cannot install it."
            }
            Self::GeneratedNeedsDigest => {
                "Anima did not test this entry: it is generated from the data of its \
                 publisher. To install it, a person confirms the digest of its image."
            }
            Self::GeneratedNotTested => {
                "Anima did not test this entry on a machine with this image. Install it from \
                 the Anima app in a web browser, or wait for a test that passes."
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
    /// A person signed in to the Anima app in a browser: a browser token, not declared an
    /// agent. False for a login token (the `anima` command and the desktop app), an API key
    /// and an agent, because the header that names an agent is the claim of the client.
    pub in_browser: bool,
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
    } else if anima.class == Class::Shell && !ask.in_browser {
        Refusal::ShellNeedsPerson
    } else if anima.origin.is_some() && !ask.in_browser && !listed.tested_with_its_images() {
        Refusal::GeneratedNotTested
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
