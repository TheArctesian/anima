//! DNS probe: asks one resolver for a record and compares the answer with what is expected.
//!
//! Only answers of the asked record type count. `expected` is compared without case and
//! without a trailing dot, against each answer: A and AAAA by address; CNAME, NS and PTR by
//! name; MX by the exchange or by `preference exchange`; SRV by the target only; TXT by its
//! character strings joined, compared case-insensitively. Any answer matching is a success.

use std::{
    net::IpAddr,
    time::{Duration, Instant},
};

use hickory_resolver::{
    Resolver, TokioResolver,
    config::{NameServerConfig, ResolveHosts, ResolverConfig},
    net::runtime::TokioRuntimeProvider,
    proto::rr::{RData, RecordType},
};

use super::{
    ProbeEnv,
    outcome::{Failure, ProbeResult},
};
use crate::checks::config::DnsRecordType;

/// One DNS question and, optionally, the answer that is expected.
pub(super) struct DnsQuery<'a> {
    pub name: &'a str,
    pub record_type: DnsRecordType,
    pub resolver: Option<IpAddr>,
    pub expected: Option<&'a str>,
}

const fn record_type(kind: DnsRecordType) -> RecordType {
    match kind {
        DnsRecordType::A => RecordType::A,
        DnsRecordType::Aaaa => RecordType::AAAA,
        DnsRecordType::Cname => RecordType::CNAME,
        DnsRecordType::Mx => RecordType::MX,
        DnsRecordType::Ns => RecordType::NS,
        DnsRecordType::Txt => RecordType::TXT,
        DnsRecordType::Srv => RecordType::SRV,
        DnsRecordType::Ptr => RecordType::PTR,
    }
}

/// Lower case, no trailing dot: the form answers are compared in.
fn normal(text: &str) -> String {
    text.trim_end_matches('.').to_ascii_lowercase()
}

/// The texts an answer can be written as: an MX or SRV answer also matches its bare target.
fn spellings(data: &RData) -> Vec<String> {
    match data {
        RData::A(a) => vec![a.0.to_string()],
        RData::AAAA(a) => vec![a.0.to_string()],
        RData::CNAME(n) => vec![n.0.to_string()],
        RData::NS(n) => vec![n.0.to_string()],
        RData::PTR(n) => vec![n.0.to_string()],
        RData::MX(mx) => vec![
            mx.exchange.to_string(),
            format!("{} {}", mx.preference, mx.exchange),
        ],
        RData::SRV(srv) => vec![srv.target.to_string()],
        RData::TXT(txt) => {
            let joined: Vec<u8> = txt
                .txt_data
                .iter()
                .flat_map(|part| part.iter().copied())
                .collect();
            vec![String::from_utf8_lossy(&joined).into_owned()]
        }
        _ => Vec::new(),
    }
    .iter()
    .map(|text| normal(text))
    .collect()
}

fn resolver(
    server: Option<IpAddr>,
    limit: Duration,
    env: &ProbeEnv,
) -> Result<TokioResolver, Failure> {
    let mut builder = match server {
        Some(ip) => {
            if !env.policy.permits(ip) {
                return Err(Failure::NotPermitted);
            }
            let mut name_server = NameServerConfig::udp_and_tcp(ip);
            if let Some(port) = env.overrides.dns_port {
                for connection in &mut name_server.connections {
                    connection.port = port;
                }
            }
            let config = ResolverConfig::from_name_servers(vec![name_server]);
            Resolver::builder_with_config(config, TokioRuntimeProvider::default())
        }
        None => Resolver::builder_tokio().map_err(|_| Failure::DnsLookup)?,
    };
    let options = builder.options_mut();
    options.attempts = 1;
    options.timeout = limit;
    options.cache_size = 0;
    options.use_hosts_file = ResolveHosts::Never;
    builder.build().map_err(|_| Failure::DnsLookup)
}

pub(super) async fn probe(query: &DnsQuery<'_>, limit: Duration, env: &ProbeEnv) -> ProbeResult {
    match ask(query, limit, env).await {
        Ok(elapsed) => ProbeResult::up(elapsed),
        Err(failure) => ProbeResult::failed(&failure),
    }
}

async fn ask(query: &DnsQuery<'_>, limit: Duration, env: &ProbeEnv) -> Result<Duration, Failure> {
    let resolver = resolver(query.resolver, limit, env)?;
    let kind = record_type(query.record_type);
    // A trailing dot makes the name absolute: no search domain is appended.
    let name = format!("{}.", query.name.trim_end_matches('.'));
    let started = Instant::now();
    let lookup = resolver
        .lookup(name, kind)
        .await
        .map_err(|_| Failure::DnsLookup)?;
    let elapsed = started.elapsed();
    let answers: Vec<_> = lookup
        .answers()
        .iter()
        .filter(|r| r.data.record_type() == kind)
        .collect();
    if answers.is_empty() {
        return Err(Failure::DnsLookup);
    }
    let Some(expected) = query.expected.map(normal) else {
        return Ok(elapsed);
    };
    if answers
        .iter()
        .any(|record| spellings(&record.data).contains(&expected))
    {
        Ok(elapsed)
    } else {
        Err(Failure::DnsMismatch)
    }
}
