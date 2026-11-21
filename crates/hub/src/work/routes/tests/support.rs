//! The doubles and the values that the tests of the routes share.

use std::{
    collections::HashMap,
    net::{IpAddr, SocketAddr},
    sync::Mutex,
};

use async_trait::async_trait;
use chrono::{DateTime, TimeDelta, Utc};

use crate::{
    fleet::{credentials::Secret, targets::AddressPolicy},
    ids::{ApplicationId, DeviceId},
    work::routes::{
        NewRoute, Settings,
        certificates::{IssueError, Issued, Issuer},
        names::{NameRules, RouteName},
        proof::{Caa, LookupFailed, ProofRule, Resolver},
    },
};

pub const HUB: &str = "hub.anima.ing";
/// `docker-host` of the devices fixture: its host is `10.10.0.21`.
pub const DOCKER_HOST: &str = "0190a000-0000-7000-8002-000000000002";
pub const APPLICATION: &str = "0190a000-0000-7000-8009-000000000001";

pub fn now() -> DateTime<Utc> {
    "2026-11-20T09:00:00Z".parse().unwrap()
}

pub fn rules() -> NameRules {
    NameRules {
        hub: HUB.to_owned(),
        cookie_domains: vec!["anima.ing".to_owned()],
    }
}

pub fn name(text: &str) -> RouteName {
    RouteName::parse(text, &rules()).unwrap()
}

pub fn settings() -> Settings {
    Settings {
        names: rules(),
        proof: ProofRule::Token {
            hub: HUB.to_owned(),
        },
        policy: AddressPolicy::default(),
        hub_listeners: vec!["10.10.0.99:8443".parse().unwrap()],
    }
}

pub fn device(id: &str) -> DeviceId {
    id.parse().unwrap()
}

pub fn application() -> ApplicationId {
    APPLICATION.parse().unwrap()
}

pub fn new_route(name: &str, port: i64) -> NewRoute {
    NewRoute {
        name: name.to_owned(),
        application: application(),
        device: device(DOCKER_HOST),
        port,
    }
}

/// A DNS that answers from its tables; a name in `failing` fails each question.
#[derive(Default)]
pub struct FakeDns {
    pub txt: Mutex<HashMap<String, Vec<String>>>,
    pub addresses: HashMap<String, Vec<IpAddr>>,
    pub caa: HashMap<String, Vec<Caa>>,
    pub failing: Mutex<Vec<String>>,
}

impl FakeDns {
    fn fails(&self, name: &str) -> Result<(), LookupFailed> {
        if self
            .failing
            .lock()
            .unwrap()
            .iter()
            .any(|failing| failing == name)
        {
            return Err(LookupFailed);
        }
        Ok(())
    }

    pub fn set_txt(&self, name: &str, texts: &[String]) {
        self.txt
            .lock()
            .unwrap()
            .insert(name.to_owned(), texts.to_vec());
    }

    pub fn fail(&self, name: &str) {
        self.failing.lock().unwrap().push(name.to_owned());
    }
}

#[async_trait]
impl Resolver for FakeDns {
    async fn txt(&self, name: &str) -> Result<Vec<String>, LookupFailed> {
        self.fails(name)?;
        Ok(self
            .txt
            .lock()
            .unwrap()
            .get(name)
            .cloned()
            .unwrap_or_default())
    }

    async fn addresses(&self, name: &str) -> Result<Vec<IpAddr>, LookupFailed> {
        self.fails(name)?;
        Ok(self.addresses.get(name).cloned().unwrap_or_default())
    }

    async fn caa(&self, name: &str) -> Result<Vec<Caa>, LookupFailed> {
        self.fails(name)?;
        Ok(self.caa.get(name).cloned().unwrap_or_default())
    }
}

pub const KEY_PEM: &str =
    "-----BEGIN PRIVATE KEY-----\nnot a real key\n-----END PRIVATE KEY-----\n";

/// An issuer that gives a fixed certificate valid for 90 days, or refuses each order.
pub struct FakeIssuer {
    pub refuse: bool,
}

#[async_trait]
impl Issuer for FakeIssuer {
    async fn issue(&self, _name: &RouteName) -> Result<Issued, IssueError> {
        if self.refuse {
            return Err(IssueError::Refused);
        }
        Ok(Issued {
            chain_pem: "-----BEGIN CERTIFICATE-----\nchain\n-----END CERTIFICATE-----\n".to_owned(),
            key_pem: Secret::new(KEY_PEM.as_bytes().to_vec()),
            not_after: now() + TimeDelta::days(90),
        })
    }
}

pub fn socket(text: &str) -> SocketAddr {
    text.parse().unwrap()
}
