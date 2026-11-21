//! A listed entry and the K1 enums of its kind and its marks.
#![expect(
    clippy::unused_async_trait_impl,
    reason = "the async-graphql Enum derive generates async trait impls"
)]

use async_graphql::Enum;
use catalogue::entry::{Entry, Kind, MachineTest, Mark, Protocol};

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
            MachineTest::Passed { date } => Some(date),
            MachineTest::NotTested | MachineTest::Failed { .. } => None,
        }
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
