//! The K1 enums of the checks API. The configuration enums of the domain stay free of
//! GraphQL; these mirror them, and the two directions are `From` conversions.
#![expect(
    clippy::unused_async_trait_impl,
    reason = "the async-graphql Enum derive generates async trait impls"
)]

use async_graphql::Enum;

use crate::checks::config::{CheckType, DnsRecordType, HttpAuth, HttpMethod, IpVersion};

macro_rules! mirror {
    ($domain:ty => $graphql:ident { $($variant:ident),+ $(,)? }) => {
        impl From<$domain> for $graphql {
            fn from(value: $domain) -> Self {
                match value { $(<$domain>::$variant => Self::$variant),+ }
            }
        }

        impl From<$graphql> for $domain {
            fn from(value: $graphql) -> Self {
                match value { $($graphql::$variant => Self::$variant),+ }
            }
        }
    };
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum)]
#[graphql(name = "CheckType")]
pub(super) enum CheckKind {
    Icmp,
    Tcp,
    Tls,
    Http,
    Dns,
    Push,
    Group,
    Docker,
    Systemd,
    TailscalePing,
}
mirror!(CheckType => CheckKind { Icmp, Tcp, Tls, Http, Dns, Push, Group, Docker, Systemd, TailscalePing });

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Enum)]
#[graphql(name = "IpVersion")]
pub(super) enum IpFamily {
    #[default]
    V4,
    V6,
}
mirror!(IpVersion => IpFamily { V4, V6 });

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Enum)]
#[graphql(name = "HttpMethod")]
pub(super) enum RequestMethod {
    #[default]
    Get,
    Head,
    Post,
    Put,
    Patch,
    Delete,
    Options,
}
mirror!(HttpMethod => RequestMethod { Get, Head, Post, Put, Patch, Delete, Options });

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Enum)]
#[graphql(name = "DnsRecordType")]
pub(super) enum RecordType {
    #[default]
    A,
    Aaaa,
    Cname,
    Mx,
    Ns,
    Txt,
    Srv,
    Ptr,
}
mirror!(DnsRecordType => RecordType { A, Aaaa, Cname, Mx, Ns, Txt, Srv, Ptr });

/// How an HTTP check signs in; the secret half is never part of the API.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum)]
#[graphql(name = "HttpAuthKind")]
pub(super) enum SignInKind {
    None,
    Basic,
    Bearer,
}

impl From<&HttpAuth> for SignInKind {
    fn from(auth: &HttpAuth) -> Self {
        match auth {
            HttpAuth::None => Self::None,
            HttpAuth::Basic { .. } => Self::Basic,
            HttpAuth::Bearer => Self::Bearer,
        }
    }
}
