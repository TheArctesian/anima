//! A listed entry and the K1 enums of its kind and its marks.
#![expect(
    clippy::unused_async_trait_impl,
    reason = "the async-graphql Enum derive generates async trait impls"
)]

use async_graphql::Enum;
use catalogue::entry::{Class, Entry, Kind, MachineTest, Mark, Protocol, Reviewed};

/// One entry that passed the static test, with what the hub shows of its files.
#[derive(Clone, Debug)]
pub struct Listed {
    pub entry: Entry,
    /// The text of the Compose file. It holds no secret: each secret is a variable that the
    /// install makes (Section 13.6 of the proposal).
    pub compose: String,
    /// The names of the variables of the example environment file, in its order. Never a value.
    pub variables: Vec<String>,
}

impl Listed {
    /// The day of the last test on a machine that passed; `None`: not tested on a machine. An
    /// entry whose last test failed is not listed.
    #[must_use]
    pub fn machine_test_day(&self) -> Option<&str> {
        match &self.entry.anima.last_test {
            MachineTest::Passed { date, .. } => Some(date),
            MachineTest::NotTested | MachineTest::Failed { .. } => None,
        }
    }

    /// True when the last test on a machine passed with the images that the entry gives now,
    /// and no other (SF-252).
    #[must_use]
    pub fn tested_with_its_images(&self) -> bool {
        let MachineTest::Passed { digests, .. } = &self.entry.anima.last_test else {
            return false;
        };
        let own = self.image_digests();
        !own.is_empty()
            && own.len() == digests.len()
            && own.iter().zip(digests).all(|(a, b)| *a == b)
    }

    /// The person who reviewed a generated entry, and the day (SF-255); `None`
    /// when no person did, or for a tested entry.
    #[must_use]
    pub fn read_by(&self) -> Option<(&str, &str)> {
        match &self.entry.anima.reviewed {
            Some(Reviewed::Reviewed { by, on }) => Some((by, on)),
            Some(Reviewed::NotReviewed) | None => None,
        }
    }

    /// The digest of each image, `sha256:` and 64 hexadecimal digits, in the order of the entry.
    #[must_use]
    pub fn image_digests(&self) -> Vec<&str> {
        (self.entry.anima.images.iter())
            .filter_map(|pinned| pinned.image.split_once('@').map(|(_, digest)| digest))
            .collect()
    }

    /// The ports on the network that the `network-port` marks name: the machine needs "Open to
    /// the network" (R-229).
    #[must_use]
    pub fn ports_on_network(&self) -> Vec<(u16, Vec<Protocol>)> {
        self.entry
            .anima
            .marks
            .iter()
            .filter_map(|mark| match mark {
                Mark::NetworkPort { port, protocols } => Some((*port, protocols.clone())),
                _ => None,
            })
            .collect()
    }
}

/// What an entry installs (K1 `CatalogueEntryKind`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum)]
#[graphql(name = "CatalogueEntryKind")]
pub enum EntryKind {
    Application,
    /// A database server for the applications of the machine (R-216, task H24).
    Database,
}

impl From<Kind> for EntryKind {
    fn from(kind: Kind) -> Self {
        match kind {
            Kind::Application => Self::Application,
            Kind::Database => Self::Database,
        }
    }
}

/// A need of an entry beyond the default rules of a Compose file (K1 `CatalogueMarkKind`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum)]
#[graphql(name = "CatalogueMarkKind")]
pub enum MarkKind {
    /// A port on each address of the machine: the machine needs "Open to the network".
    NetworkPort,
    /// The socket of the engine (R-247): never installable.
    EngineSocket,
    HostNetwork,
    Device,
}

impl From<&Mark> for MarkKind {
    fn from(mark: &Mark) -> Self {
        match mark {
            Mark::NetworkPort { .. } => Self::NetworkPort,
            Mark::EngineSocket => Self::EngineSocket,
            Mark::HostNetwork => Self::HostNetwork,
            Mark::Device => Self::Device,
        }
    }
}

/// What a person may do with an entry (K1 `CatalogueEntryClass`, OD-HOST-54).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum)]
#[graphql(name = "CatalogueEntryClass")]
pub enum EntryClass {
    Application,
    /// Who reaches it is root in a container: it needs a credential that the machine makes,
    /// only a person installs it and it gets no public route.
    Shell,
}

impl From<Class> for EntryClass {
    fn from(class: Class) -> Self {
        match class {
            Class::Application => Self::Application,
            Class::Shell => Self::Shell,
        }
    }
}

/// The tier of an entry in the catalogue (K1 `CatalogueTier`, OD-HOST-50). The third tier is
/// installed at the machine and is not an entry of the catalogue.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum)]
#[graphql(name = "CatalogueTier")]
pub enum Tier {
    /// Tier 1: written and checked by the Anima project.
    Tested,
    /// Tier 2: generated from the data of a publisher and not tested by Anima.
    Generated,
}

impl Listed {
    /// The tier: generated when the entry names a publisher, tested otherwise.
    #[must_use]
    pub const fn tier_of_origin(&self) -> Tier {
        if self.entry.anima.origin.is_some() {
            Tier::Generated
        } else {
            Tier::Tested
        }
    }
}

/// The names of `protocols` as K1 writes them: `tcp`, `udp`.
#[must_use]
pub fn protocol_names(protocols: &[Protocol]) -> Vec<String> {
    protocols
        .iter()
        .map(|protocol| format!("{protocol:?}").to_lowercase())
        .collect()
}
