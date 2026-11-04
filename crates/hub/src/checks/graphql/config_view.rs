//! `CheckConfig` and the config types of K1, as the API returns them. No secret is part of
//! any of them: a secret header's value is null and the sign-in is only its kind (LOG-007,
//! SEC-003).

use async_graphql::{Context, Object, Result, SimpleObject, Union};

use super::{
    enums::{IpFamily, RecordType, RequestMethod, SignInKind},
    error::{schema_dependency, to_graphql},
};
use crate::{
    checks::{
        self, Check,
        config::{CheckConfig, HttpConfig, HttpHeader, StatusRange, is_secret_header},
    },
    ids::{CheckId, DeviceId},
    scalars::IpAddr,
};

#[derive(SimpleObject)]
#[graphql(name = "IcmpConfig")]
pub(super) struct IcmpView {
    timeout_ms: i32,
    ip_version: IpFamily,
}

#[derive(SimpleObject)]
#[graphql(name = "TcpConfig")]
pub(super) struct TcpView {
    port: i32,
    timeout_ms: i32,
}

#[derive(SimpleObject)]
#[graphql(name = "TlsConfig")]
pub(super) struct TlsView {
    port: i32,
    server_name: Option<String>,
    timeout_ms: i32,
}

#[derive(SimpleObject)]
#[graphql(name = "HttpHeader")]
pub(super) struct HeaderView {
    name: String,
    /// Null for Authorization, Proxy-Authorization and Cookie.
    value: Option<String>,
}

#[derive(SimpleObject)]
#[graphql(name = "StatusRange")]
pub(super) struct RangeView {
    from: i32,
    to: i32,
}

#[derive(SimpleObject)]
#[graphql(name = "HttpConfig")]
pub(super) struct HttpView {
    url: String,
    method: RequestMethod,
    headers: Vec<HeaderView>,
    body: Option<String>,
    max_redirects: i32,
    accepted_status: Vec<RangeView>,
    keyword: Option<String>,
    json_query: Option<String>,
    json_expected: Option<String>,
    auth_kind: SignInKind,
    verify_tls: bool,
    timeout_ms: i32,
}

#[derive(SimpleObject)]
#[graphql(name = "DnsConfig")]
pub(super) struct DnsView {
    name: String,
    record_type: RecordType,
    resolver: Option<IpAddr>,
    expected: Option<String>,
}

#[derive(SimpleObject)]
#[graphql(name = "PushConfig")]
pub(super) struct PushView {
    /// Path on the hub that the device calls. The token is part of the path.
    push_path: String,
}

pub(super) struct GroupView {
    group: CheckId,
    device: DeviceId,
}

#[Object(name = "GroupConfig")]
impl GroupView {
    async fn children(&self, ctx: &Context<'_>) -> Result<Vec<Check>> {
        let pool = schema_dependency::<sqlx::PgPool>(ctx)?;
        let siblings = checks::for_device(pool, self.device)
            .await
            .map_err(to_graphql)?;
        Ok(siblings
            .into_iter()
            .filter(|check| check.parent == Some(self.group))
            .collect())
    }
}

#[derive(Union)]
#[graphql(name = "CheckConfig")]
pub(super) enum ConfigView {
    Icmp(IcmpView),
    Tcp(TcpView),
    Tls(TlsView),
    Http(HttpView),
    Dns(DnsView),
    Push(PushView),
    Group(GroupView),
}

fn wide(value: impl Into<u32>) -> i32 {
    i32::try_from(value.into()).unwrap_or(i32::MAX)
}

/// The stored value of a secret header is already null; filtering by name again is defence in
/// depth, so a stored value that slipped in (an older version, a hand-edited row) never leaves.
fn header(header: &HttpHeader) -> HeaderView {
    HeaderView {
        name: header.name.clone(),
        value: header
            .value
            .clone()
            .filter(|_| !is_secret_header(&header.name)),
    }
}

fn range(range: StatusRange) -> RangeView {
    RangeView {
        from: wide(range.from),
        to: wide(range.to),
    }
}

fn http(config: &HttpConfig) -> HttpView {
    HttpView {
        url: config.url.clone(),
        method: config.method.into(),
        headers: config.headers.iter().map(header).collect(),
        body: config.body.clone(),
        max_redirects: wide(config.max_redirects),
        accepted_status: config.accepted_status.iter().copied().map(range).collect(),
        keyword: config.keyword.clone(),
        json_query: config.json_query.clone(),
        json_expected: config.json_expected.clone(),
        auth_kind: SignInKind::from(&config.auth),
        verify_tls: config.verify_tls,
        timeout_ms: wide(config.timeout_ms),
    }
}

impl ConfigView {
    /// The configuration of the check; a PUSH check shows its path.
    pub(super) fn of(check: &Check) -> Self {
        match &check.config {
            CheckConfig::Icmp {
                timeout_ms,
                ip_version,
            } => Self::Icmp(IcmpView {
                timeout_ms: wide(*timeout_ms),
                ip_version: (*ip_version).into(),
            }),
            CheckConfig::Tcp { port, timeout_ms } => Self::Tcp(TcpView {
                port: wide(*port),
                timeout_ms: wide(*timeout_ms),
            }),
            CheckConfig::Tls {
                port,
                server_name,
                timeout_ms,
            } => Self::Tls(TlsView {
                port: wide(*port),
                server_name: server_name.clone(),
                timeout_ms: wide(*timeout_ms),
            }),
            CheckConfig::Http(config) => Self::Http(http(config)),
            CheckConfig::Dns {
                name,
                record_type,
                resolver,
                expected,
            } => Self::Dns(DnsView {
                name: name.clone(),
                record_type: (*record_type).into(),
                resolver: resolver.map(IpAddr),
                expected: expected.clone(),
            }),
            CheckConfig::Push => Self::Push(PushView {
                push_path: check
                    .push_token
                    .as_ref()
                    .map_or_else(String::new, |token| format!("/push/{}", token.expose())),
            }),
            CheckConfig::Group => Self::Group(GroupView {
                group: check.id,
                device: check.device,
            }),
        }
    }
}
