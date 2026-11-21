//! The install of an entry: present, and refused with a fixed reason until task H6 exists
//! (R-NEW-buildnow). No machine is reached.
#![expect(
    clippy::unused_async_trait_impl,
    reason = "the async-graphql Enum derive generates async trait impls"
)]

use async_graphql::Enum;
use catalogue::entry::Mark;

use super::model::Listed;

/// Why the hub does not install an entry (K1 `CatalogueInstallRefusal`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum)]
#[graphql(name = "CatalogueInstallRefusal")]
pub enum Refusal {
    /// The catalogue lists no entry with this identifier.
    NotListed,
    /// The entry needs the socket of the engine; no such entry is installed (R-247).
    EngineSocket,
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

/// The answer to the install of `listed`, which is `None` for an identifier that is not
/// listed. The first refusal that applies is the one given.
#[must_use]
pub fn install(listed: Option<&Listed>) -> Install {
    let Some(listed) = listed else {
        return Install {
            refusal: Some(Refusal::NotListed),
            needs_open_to_network: false,
        };
    };
    let marks = &listed.entry.anima.marks;
    let refusal = if marks.contains(&Mark::EngineSocket) {
        Refusal::EngineSocket
    } else {
        Refusal::InstallNotBuilt
    };
    Install {
        refusal: Some(refusal),
        needs_open_to_network: !listed.ports_on_network().is_empty(),
    }
}
