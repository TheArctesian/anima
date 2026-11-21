//! The real [`Resolver`] of the routes: the resolver of the system, with no cache and no hosts
//! file, so each read asks DNS again. "No such record" is an empty answer, not a failure. I/O
//! edge; nothing here logs.

use std::{net::IpAddr, time::Duration};

use async_trait::async_trait;
use hickory_resolver::{
    Resolver as Hickory, TokioResolver, config::ResolveHosts, proto::rr::RData,
    proto::rr::RecordType,
};

use super::proof::{Caa, LookupFailed, Resolver};

/// The longest one question may take.
const LIMIT: Duration = Duration::from_secs(5);

/// The resolver of the system.
pub struct SystemDns(TokioResolver);

impl SystemDns {
    /// # Errors
    /// [`LookupFailed`] when the system has no resolver configuration.
    pub fn new() -> Result<Self, LookupFailed> {
        let mut builder = Hickory::builder_tokio().map_err(|_| LookupFailed)?;
        let options = builder.options_mut();
        options.timeout = LIMIT;
        options.cache_size = 0;
        options.use_hosts_file = ResolveHosts::Never;
        builder.build().map(Self).map_err(|_| LookupFailed)
    }

    /// The data of each answer of `kind` for `name`; empty when the name has none.
    async fn ask(&self, name: &str, kind: RecordType) -> Result<Vec<RData>, LookupFailed> {
        let absolute = format!("{}.", name.trim_end_matches('.'));
        match self.0.lookup(absolute, kind).await {
            Ok(lookup) => Ok(lookup
                .answers()
                .iter()
                .filter(|record| record.data.record_type() == kind)
                .map(|record| record.data.clone())
                .collect()),
            Err(error) if error.is_no_records_found() => Ok(Vec::new()),
            Err(_) => Err(LookupFailed),
        }
    }
}

#[async_trait]
impl Resolver for SystemDns {
    async fn txt(&self, name: &str) -> Result<Vec<String>, LookupFailed> {
        Ok(self
            .ask(name, RecordType::TXT)
            .await?
            .into_iter()
            .filter_map(|data| match data {
                RData::TXT(txt) => Some(
                    txt.txt_data
                        .iter()
                        .map(|part| String::from_utf8_lossy(part).into_owned())
                        .collect(),
                ),
                _ => None,
            })
            .collect())
    }

    async fn addresses(&self, name: &str) -> Result<Vec<IpAddr>, LookupFailed> {
        let mut found = Vec::new();
        for kind in [RecordType::A, RecordType::AAAA] {
            for data in self.ask(name, kind).await? {
                match data {
                    RData::A(a) => found.push(IpAddr::V4(a.0)),
                    RData::AAAA(a) => found.push(IpAddr::V6(a.0)),
                    _ => {}
                }
            }
        }
        Ok(found)
    }

    async fn caa(&self, name: &str) -> Result<Vec<Caa>, LookupFailed> {
        Ok(self
            .ask(name, RecordType::CAA)
            .await?
            .into_iter()
            .filter_map(|data| match data {
                RData::CAA(caa) => Some(Caa {
                    tag: caa.tag,
                    value: String::from_utf8_lossy(&caa.value).into_owned(),
                }),
                _ => None,
            })
            .collect())
    }
}
