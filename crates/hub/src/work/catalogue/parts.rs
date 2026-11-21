//! The K1 objects of the parts of an entry: a port on the network, a stream, an optional part
//! and the publisher of a generated entry (task H39).
#![expect(
    clippy::unused_async_trait_impl,
    reason = "the async-graphql Enum derive generates async trait impls"
)]

use async_graphql::{Enum, Object};
use catalogue::entry::{Optional, Origin, Publisher};

/// A published port and its protocols: a port on the network (`CatalogueNetworkPort`) or a
/// stream that a route of task H40 can carry (`CatalogueStream`).
pub struct Port {
    pub port: u16,
    pub protocols: Vec<String>,
}

/// One port on the network that an entry needs.
pub struct NetworkPort(pub Port);

#[Object(name = "CatalogueNetworkPort")]
impl NetworkPort {
    async fn port(&self) -> u16 {
        self.0.port
    }

    /// `tcp` or `udp`.
    async fn protocols(&self) -> &[String] {
        &self.0.protocols
    }
}

/// One published port of an entry other than the port of its test.
pub struct Stream(pub Port);

#[Object(name = "CatalogueStream")]
impl Stream {
    /// The port in the container.
    async fn port(&self) -> u16 {
        self.0.port
    }

    /// `tcp`, `udp` or both.
    async fn protocols(&self) -> &[String] {
        &self.0.protocols
    }
}

/// The kind of an optional part (K1 `CatalogueOptionalPartKind`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum)]
#[graphql(name = "CatalogueOptionalPartKind")]
pub enum PartKind {
    /// A device of the machine below `/dev/`.
    Device,
}

/// A part of the application that is off.
pub struct OptionalPart<'a>(pub &'a Optional);

#[Object(name = "CatalogueOptionalPart")]
impl OptionalPart<'_> {
    async fn kind(&self) -> PartKind {
        match self.0 {
            Optional::Device { .. } => PartKind::Device,
        }
    }

    /// The path of the device, for example `/dev/dri`.
    async fn path(&self) -> &str {
        match self.0 {
            Optional::Device { path } => path,
        }
    }

    /// Whether the install turns the part on. Never today.
    async fn on(&self) -> bool {
        false
    }

    /// The part needs a setting of the machine: no rule of a Compose file permits a device.
    async fn needs_machine_setting(&self) -> bool {
        true
    }
}

/// The publisher of a generated entry.
pub struct GeneratedFrom<'a>(pub &'a Origin);

#[Object(name = "CatalogueOrigin")]
impl GeneratedFrom<'_> {
    /// The name of the publisher, for a person: `LinuxServer.io`.
    async fn publisher(&self) -> &'static str {
        match self.0.from {
            Publisher::Linuxserver => "LinuxServer.io",
        }
    }

    /// The day (`YYYY-MM-DD`) of the data of the publisher that the entry was made from.
    async fn read(&self) -> &str {
        &self.0.read
    }
}
