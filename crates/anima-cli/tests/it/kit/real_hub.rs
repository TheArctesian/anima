//! The real hub on a loopback port: the router, the permitted-networks check, the resolvers and
//! the stores that `anima-hub` serves, wired as its `main.rs` wires them, over the test's own
//! migrated database. Plain HTTP on loopback is the hub's own rule (NET-003); the permitted
//! networks are `127.0.0.0/8`, so the check runs on every request (NET-002).

use std::collections::BTreeMap;
use std::io::Write as _;
use std::net::SocketAddr;
use std::sync::Arc;

use anima_hub::{
    DRAIN_LIMIT, Shutdown, all_routes,
    clock::{Clock, SystemClock},
    config,
    fleet::{
        access::{self, Password, listen},
        builds,
        credentials::Vault,
        events::Bus,
        metrics, sessions,
        targets::AddressPolicy,
        vpn,
    },
    schema::Dependencies,
    serve_until,
};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde_json::{Value, json};
use sqlx::PgPool;
use tempfile::NamedTempFile;
use tokio::sync::oneshot;

use super::totp;
use crate::kit::Anima;

pub const USERNAME: &str = "operator";
pub const PASSWORD: &str = "correct horse battery staple";

/// A served hub; it stops when dropped.
pub struct RealHub {
    pub origin: String,
    pub pool: PgPool,
    vault_key: NamedTempFile,
    stop: Option<oneshot::Sender<()>>,
}

impl Drop for RealHub {
    fn drop(&mut self) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
    }
}

/// A key file with a throwaway key (SEC-004); never a real key.
fn vault_key() -> NamedTempFile {
    let mut file = NamedTempFile::new().expect("a key file");
    writeln!(file, "{}", STANDARD.encode([7_u8; 32])).expect("the key written");
    file
}

/// What `main.rs` builds before it serves, with the permitted networks `127.0.0.0/8` stored,
/// the system clock, and the operator set.
async fn dependencies(pool: &PgPool, vault: Vault) -> Dependencies {
    let mut env = BTreeMap::new();
    let networks = (
        "ANIMA_PERMITTED_NETWORKS".to_owned(),
        "127.0.0.0/8".to_owned(),
    );
    env.insert(networks.0, networks.1);
    let config = Arc::new(config::from_env(&env).expect("the configuration"));
    let clock: Arc<dyn Clock> = Arc::new(SystemClock);
    let now = clock.now();
    let password = Password::new(PASSWORD.to_owned());
    access::set_operator(pool, USERNAME, password, now)
        .await
        .expect("the operator");
    access::permit_networks(pool, &config.permitted_networks)
        .await
        .expect("the permitted networks");
    // The SSH test device listens on loopback, which the hub otherwise refuses (R-030);
    // these are the hub's own test switches (`test-support`), never a served hub's.
    let policy = AddressPolicy::new(config.probe_deny.clone()).allowing_loopback();
    let (live, _) = sessions::open(pool, now).await.expect("the sessions");
    Dependencies {
        vpn_broker: None,
        config,
        clock,
        pool: pool.clone(),
        bus: Bus::new(1024),
        vault,
        live: live.reaching_loopback_for_tests().denying(policy.clone()),
        policy,
        metrics_feed: metrics::Feed::new(),
        build_updates: builds::Updates::new(),
        bucket: None,
        vpn_feed: vpn::Feed::new(),
    }
}

impl RealHub {
    /// Serves the hub over `pool` (migrated by `sqlx::test`) with the operator set.
    pub async fn start(pool: PgPool) -> Self {
        let vault_key = vault_key();
        let vault = Vault::from_key_file(vault_key.path()).expect("the vault");
        let dependencies = dependencies(&pool, vault).await;
        let clock = dependencies.clock.clone();
        let listen_on: SocketAddr = "127.0.0.1:0".parse().expect("an address");
        let listening = listen::listen(listen_on, None, clock.clone())
            .await
            .expect("the hub listens");
        let addr = listening.local_addr().expect("the bound address");
        let live = dependencies.live.clone();
        let edge = access::Edge::new(pool.clone(), dependencies.bus.clone(), clock.clone());
        let app = all_routes(dependencies, edge.clone());
        let shutdown = Shutdown {
            db: pool.clone(),
            live,
            clock,
            drain_limit: DRAIN_LIMIT,
        };
        let (stop, stopped) = oneshot::channel::<()>();
        tokio::spawn(serve_until(
            listening.admitting(edge),
            app,
            shutdown,
            async move {
                let _ = stopped.await;
            },
        ));
        Self {
            origin: format!("http://{addr}"),
            pool,
            vault_key,
            stop: Some(stop),
        }
    }

    /// Gives the operator `password` instead of [`PASSWORD`].
    pub async fn set_password(&self, password: &str) {
        let now = SystemClock.now();
        access::set_operator(
            &self.pool,
            USERNAME,
            Password::new(password.to_owned()),
            now,
        )
        .await
        .expect("the new password");
    }

    /// Turns on TOTP for the operator; returns the seed the authenticator app would hold.
    pub async fn enrol_totp(&self) -> Vec<u8> {
        let vault = Vault::from_key_file(self.vault_key.path()).expect("the vault");
        let enrolment = access::enrol_totp(&self.pool, &vault, USERNAME)
            .await
            .expect("TOTP enrolled");
        totp::secret_of(enrolment.uri())
    }

    /// An `anima` for this hub: its own home folder, `ANIMA_HUB` set to this hub.
    pub fn anima(&self) -> Anima {
        Anima::new().with("ANIMA_HUB", &self.origin)
    }

    /// Posts `query` with `variables` as the hub's own client would; the whole answer.
    pub async fn graphql(&self, token: Option<&str>, query: &str, variables: Value) -> Value {
        let mut request = reqwest::Client::new()
            .post(format!("{}/graphql", self.origin))
            .header("content-type", "application/json")
            .body(json!({ "query": query, "variables": variables }).to_string());
        if let Some(token) = token {
            request = request.header("authorization", format!("Bearer {token}"));
        }
        let response = request.send().await.expect("the hub answers");
        let text = response.text().await.expect("an answer body");
        serde_json::from_str(&text).unwrap_or_else(|_| panic!("not JSON: {text}"))
    }

    /// `data` of an answer that must have no errors.
    pub async fn data(&self, token: &str, query: &str, variables: Value) -> Value {
        let answer = self.graphql(Some(token), query, variables).await;
        assert!(answer.get("errors").is_none(), "{answer}");
        answer["data"].clone()
    }

    /// A login token for the operator without TOTP, for reading the hub's state back.
    pub async fn login_token(&self) -> String {
        let query = "mutation($input: LoginInput!) { login(input: $input) { token } }";
        let input = json!({ "input": { "username": USERNAME, "password": PASSWORD } });
        let answer = self.graphql(None, query, input).await;
        answer["data"]["login"]["token"]
            .as_str()
            .unwrap_or_else(|| panic!("login failed: {answer}"))
            .to_owned()
    }
}
