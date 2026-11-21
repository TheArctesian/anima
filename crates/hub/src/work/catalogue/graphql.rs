//! The K1 roots of the catalogue (HOST-047, task H38): `catalogueEntries`, `catalogueEntry`
//! and `installCatalogueEntry`. Each is open to an API key: a key and an agent may ask for the
//! install of an entry of the tested catalogue (R-234). The install is refused with a fixed
//! reason, reaches no machine and keeps nothing.

use async_graphql::{ID, Object, Result};

use super::{EntryKind, Install, Listed, MarkKind, Refusal, entries, entry, install};
use crate::{
    api_errors::{FieldError, invalid},
    fleet::access::Authenticated,
    ids::DeviceId,
};

#[derive(Default)]
pub struct Query;

#[derive(Default)]
pub struct Mutation;

/// Merged into the root `Query`.
#[Object(name = "CatalogueQuery")]
impl Query {
    /// Each entry of the catalogue that passed the static test, in the order of the
    /// identifiers.
    #[graphql(guard = "Authenticated")]
    async fn catalogue_entries(&self) -> &'static [Listed] {
        entries()
    }

    /// The listed entry with this identifier; null when none is listed.
    #[graphql(guard = "Authenticated")]
    async fn catalogue_entry(&self, id: String) -> Option<&'static Listed> {
        entry(&id)
    }
}

/// Merged into the root `Mutation`.
#[Object(name = "CatalogueMutation")]
impl Mutation {
    /// Asks for the install of an entry on a device. Refused today with a fixed reason: the
    /// install needs task H6. No machine is reached and nothing is kept.
    #[graphql(guard = "Authenticated")]
    #[expect(
        clippy::unused_async,
        clippy::unused_async_trait_impl,
        reason = "async-graphql resolvers must be async; this one can fail"
    )]
    async fn install_catalogue_entry(&self, entry_id: String, device_id: ID) -> Result<Install> {
        device_id
            .parse::<DeviceId>()
            .map_err(|_| invalid(&[FieldError::new("deviceId", "That device id is not valid.")]))?;
        Ok(install(entry(&entry_id)))
    }
}

/// One port on the network that an entry needs.
struct NetworkPort {
    port: u16,
    protocols: Vec<String>,
}

#[Object(name = "CatalogueNetworkPort")]
impl NetworkPort {
    async fn port(&self) -> u16 {
        self.port
    }

    /// `tcp` or `udp`.
    async fn protocols(&self) -> &[String] {
        &self.protocols
    }
}

#[Object(name = "CatalogueInstall")]
impl Install {
    /// Why the hub does not install the entry; null when it would.
    async fn refusal(&self) -> Option<Refusal> {
        self.refusal
    }

    /// The fixed text of the refusal, for a person.
    async fn reason(&self) -> Option<&'static str> {
        self.refusal.map(Refusal::reason)
    }

    /// The entry publishes a port on the network: the machine needs "Open to the network".
    async fn needs_open_to_network(&self) -> bool {
        self.needs_open_to_network
    }
}

#[Object(name = "CatalogueEntry")]
impl Listed {
    async fn id(&self) -> &str {
        &self.entry.id
    }

    async fn name(&self) -> &str {
        &self.entry.name
    }

    async fn description(&self) -> &str {
        &self.entry.description
    }

    /// The version of the application.
    async fn version(&self) -> &str {
        &self.entry.version
    }

    async fn author(&self) -> &str {
        &self.entry.author
    }

    async fn tags(&self) -> &[String] {
        &self.entry.tags
    }

    async fn kind(&self) -> EntryKind {
        self.entry.anima.kind.into()
    }

    /// An `https://` address of the icon, or null.
    async fn icon_url(&self) -> Option<&str> {
        self.entry.icon_url.as_deref()
    }

    async fn documentation_url(&self) -> &str {
        &self.entry.documentation_url
    }

    /// What the entry needs beyond the default rules of a Compose file.
    async fn marks(&self) -> Vec<MarkKind> {
        self.entry.anima.marks.iter().map(MarkKind::from).collect()
    }

    /// The ports on the network of the `NETWORK_PORT` marks.
    async fn network_ports(&self) -> Vec<NetworkPort> {
        self.ports_on_network()
            .into_iter()
            .map(|(port, protocols)| NetworkPort {
                port,
                protocols: protocols
                    .iter()
                    .map(|protocol| format!("{protocol:?}").to_lowercase())
                    .collect(),
            })
            .collect()
    }

    /// The day (`YYYY-MM-DD`) of the last test on a machine, which passed; null: not tested on
    /// a machine.
    async fn last_machine_test(&self) -> Option<&str> {
        self.machine_test_day()
    }

    /// The day (`YYYY-MM-DD`) of the last pin of the image digests.
    async fn pinned(&self) -> &str {
        &self.entry.anima.pinned
    }

    /// What the hub answers to an install of this entry now.
    async fn install(&self) -> Install {
        install(Some(self))
    }

    /// The text of the Compose file. It holds no secret.
    async fn compose_file(&self) -> &str {
        &self.compose
    }

    /// The names of the variables of the environment file, never a value.
    async fn environment_variables(&self) -> &[String] {
        &self.variables
    }
}
