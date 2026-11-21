//! The K1 roots of the catalogue (HOST-047, task H38): `catalogueEntries`, `catalogueEntry`
//! and `installCatalogueEntry`. Each is open to an API key: a key and an agent may ask for the
//! install of an entry of the tested catalogue (R-234), but not of a shell (OD-HOST-54) or of a
//! generated entry with no passed test on a machine. The install is refused with a fixed
//! reason, reaches no machine and keeps nothing.

use async_graphql::{Context, ID, Object, Result};

use super::model::{EntryClass, Tier, protocol_names};
use super::parts::{GeneratedFrom, NetworkPort, OptionalPart, Port, Stream};
use super::{Ask, EntryKind, Install, Listed, MarkKind, Refusal, entries, entry, install};
use crate::{
    api_errors::{FieldError, invalid},
    fleet::access::{Authenticated, Identity},
    ids::DeviceId,
};

/// True when the request is a person signed in to the Anima app in a browser (OD-HOST-54).
fn in_browser(ctx: &Context<'_>) -> bool {
    ctx.data_opt::<Identity>()
        .is_some_and(Identity::is_person_in_browser)
}

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
    /// install needs task H6. Before it, a shell needs a person in the Anima app in a browser,
    /// and a generated entry needs `digest`, the digest of its image, and, from anything but
    /// that app, a passed test on a machine with that image. No machine is reached and nothing
    /// is kept.
    #[graphql(guard = "Authenticated")]
    #[expect(
        clippy::unused_async,
        clippy::unused_async_trait_impl,
        reason = "async-graphql resolvers must be async; this one can fail"
    )]
    #[expect(
        clippy::too_many_arguments,
        reason = "K1 fixes the argument list of Mutation.installCatalogueEntry"
    )]
    async fn install_catalogue_entry(
        &self,
        ctx: &Context<'_>,
        entry_id: String,
        device_id: ID,
        digest: Option<String>,
    ) -> Result<Install> {
        device_id
            .parse::<DeviceId>()
            .map_err(|_| invalid(&[FieldError::new("deviceId", "That device id is not valid.")]))?;
        let ask = Ask {
            in_browser: in_browser(ctx),
            digest: digest.as_deref(),
        };
        Ok(install(entry(&entry_id), ask))
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
            .map(|(port, protocols)| {
                NetworkPort(Port {
                    port,
                    protocols: protocol_names(&protocols),
                })
            })
            .collect()
    }

    /// What a person may do with the entry: an application, or a shell (OD-HOST-54).
    async fn class(&self) -> EntryClass {
        self.entry.anima.class.into()
    }

    /// Tested by Anima (tier 1) or generated from the data of a publisher (tier 2).
    async fn tier(&self) -> Tier {
        self.tier_of_origin()
    }

    /// The publisher of a generated entry; null for a tested entry.
    async fn origin(&self) -> Option<GeneratedFrom<'_>> {
        self.entry.anima.origin.as_ref().map(GeneratedFrom)
    }

    /// The person who reviewed the generated entry; null when no person did, as for a tested
    /// entry (SF-255).
    async fn reviewed_by(&self) -> Option<&str> {
        self.read_by().map(|(by, _)| by)
    }

    /// The day (`YYYY-MM-DD`) of that review; null when no person reviewed the entry.
    async fn reviewed_on(&self) -> Option<&str> {
        self.read_by().map(|(_, on)| on)
    }

    /// The published ports other than the port of the test.
    async fn streams(&self) -> Vec<Stream> {
        (self.entry.anima.streams.iter())
            .map(|stream| {
                Stream(Port {
                    port: stream.port,
                    protocols: protocol_names(&stream.protocols),
                })
            })
            .collect()
    }

    /// The parts of the application that are off.
    async fn optional_parts(&self) -> Vec<OptionalPart<'_>> {
        self.entry.anima.optional.iter().map(OptionalPart).collect()
    }

    /// The digest of each image (`sha256:` and 64 hexadecimal digits).
    async fn digests(&self) -> Vec<&str> {
        self.image_digests()
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

    /// What the hub answers to an install of this entry by this request, with no digest.
    async fn install(&self, ctx: &Context<'_>) -> Install {
        let ask = Ask {
            in_browser: in_browser(ctx),
            digest: None,
        };
        install(Some(self), ask)
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
