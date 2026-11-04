//! The inputs of the check mutations as K1 declares them (`CreateCheckInput`,
//! `UpdateCheckInput` and the configuration inputs below them). They only describe the
//! shape; `form` turns them into domain values and checks the numbers.
//!
//! Every input that can carry an HTTP secret parses through `Opaque` (LOG-007).

use async_graphql::{ID, InputObject, MaybeUndefined, OneofObject};

use super::{
    enums::{IpFamily, RecordType, RequestMethod},
    opaque::{Opaque, SecretText},
};
use crate::scalars::IpAddr;

#[derive(InputObject)]
#[graphql(name = "IcmpConfigInput")]
pub(super) struct IcmpShape {
    #[graphql(default_with = "Some(2000)")]
    pub(super) timeout_ms: Option<i32>,
    #[graphql(default_with = "Some(IpFamily::V4)")]
    pub(super) ip_version: Option<IpFamily>,
}

#[derive(InputObject)]
#[graphql(name = "TcpConfigInput")]
pub(super) struct TcpShape {
    pub(super) port: i32,
    #[graphql(default_with = "Some(5000)")]
    pub(super) timeout_ms: Option<i32>,
}

#[derive(InputObject)]
#[graphql(name = "TlsConfigInput")]
pub(super) struct TlsShape {
    #[graphql(default_with = "Some(443)")]
    pub(super) port: Option<i32>,
    pub(super) server_name: Option<String>,
    #[graphql(default_with = "Some(5000)")]
    pub(super) timeout_ms: Option<i32>,
}

#[derive(InputObject)]
#[graphql(name = "HttpHeaderInput")]
pub(super) struct HeaderShape {
    pub(super) name: String,
    pub(super) value: SecretText,
}

#[derive(InputObject)]
#[graphql(name = "StatusRangeInput")]
pub(super) struct StatusRangeShape {
    pub(super) from: i32,
    pub(super) to: i32,
}

#[derive(InputObject)]
#[graphql(name = "BasicAuthInput")]
pub(super) struct BasicShape {
    pub(super) username: String,
    pub(super) password: SecretText,
}

#[derive(OneofObject)]
#[graphql(name = "HttpAuthInput")]
pub(super) enum AuthShape {
    Basic(Opaque<BasicShape>),
    BearerToken(SecretText),
}

#[derive(InputObject)]
#[graphql(name = "HttpConfigInput")]
pub(super) struct HttpShape {
    pub(super) url: String,
    #[graphql(default_with = "Some(RequestMethod::Get)")]
    pub(super) method: Option<RequestMethod>,
    #[graphql(default_with = "Some(Vec::new())")]
    pub(super) headers: Option<Vec<Opaque<HeaderShape>>>,
    pub(super) body: Option<String>,
    #[graphql(default_with = "Some(10)")]
    pub(super) max_redirects: Option<i32>,
    #[graphql(default_with = "Some(vec![StatusRangeShape { from: 200, to: 299 }])")]
    pub(super) accepted_status: Option<Vec<StatusRangeShape>>,
    pub(super) keyword: Option<String>,
    pub(super) json_query: Option<String>,
    pub(super) json_expected: Option<String>,
    pub(super) auth: Option<Opaque<AuthShape>>,
    #[graphql(default_with = "Some(true)")]
    pub(super) verify_tls: Option<bool>,
    #[graphql(default_with = "Some(10000)")]
    pub(super) timeout_ms: Option<i32>,
}

#[derive(InputObject)]
#[graphql(name = "DnsConfigInput")]
pub(super) struct DnsShape {
    pub(super) name: String,
    #[graphql(default_with = "Some(RecordType::A)")]
    pub(super) record_type: Option<RecordType>,
    pub(super) resolver: Option<IpAddr>,
    pub(super) expected: Option<String>,
}

#[derive(InputObject)]
#[graphql(name = "GroupConfigInput")]
pub(super) struct GroupShape {
    pub(super) child_ids: Vec<ID>,
}

#[derive(OneofObject)]
#[graphql(name = "CheckConfigInput")]
pub(super) enum ConfigShape {
    Icmp(IcmpShape),
    Tcp(TcpShape),
    Tls(TlsShape),
    Http(Opaque<HttpShape>),
    Dns(DnsShape),
    Push(bool),
    Group(GroupShape),
}

#[derive(InputObject)]
#[graphql(name = "CreateCheckInput")]
pub(super) struct CreateShape {
    pub(super) device_id: ID,
    pub(super) parent_id: Option<ID>,
    pub(super) name: String,
    pub(super) config: Opaque<ConfigShape>,
    #[graphql(default_with = "Some(60)")]
    pub(super) interval_seconds: Option<i32>,
    #[graphql(default_with = "Some(3)")]
    pub(super) retries: Option<i32>,
    #[graphql(default_with = "Some(20)")]
    pub(super) retry_interval_seconds: Option<i32>,
    #[graphql(default_with = "Some(1000)")]
    pub(super) degraded_threshold_ms: Option<i32>,
    #[graphql(default_with = "Some(false)")]
    pub(super) invert: Option<bool>,
}

#[derive(InputObject)]
#[graphql(name = "UpdateCheckInput")]
pub(super) struct UpdateShape {
    /// Absent keeps the group; null removes the check from its group.
    pub(super) parent_id: MaybeUndefined<ID>,
    pub(super) name: Option<String>,
    pub(super) config: Option<Opaque<ConfigShape>>,
    pub(super) interval_seconds: Option<i32>,
    pub(super) retries: Option<i32>,
    pub(super) retry_interval_seconds: Option<i32>,
    pub(super) degraded_threshold_ms: Option<i32>,
    pub(super) invert: Option<bool>,
}
