//! The device's SSH server on loopback: it presents its host key and refuses every sign-in,
//! which is all the tunnel's probe asks of it (R-034). Serves until the test ends.

use std::sync::Arc;

use russh::{
    keys::{PrivateKey, ssh_key::private::Ed25519Keypair},
    server,
};
use tokio::net::TcpListener;

use crate::sessions::PublicKey;

/// The SSH server's port and the host key it presents.
pub(super) struct SshHost {
    pub(super) port: u16,
    pub(super) key: PublicKey,
}

struct KeyOnly;

impl server::Handler for KeyOnly {
    type Error = russh::Error;
}

/// Starts a server presenting the ed25519 key made from `seed`.
pub(super) async fn ssh_host(seed: u8) -> SshHost {
    let host_key = PrivateKey::from(Ed25519Keypair::from_seed(&[seed; 32]));
    let key = PublicKey::from_blob(host_key.public_key().to_bytes().unwrap()).unwrap();
    let config = Arc::new(server::Config {
        keys: vec![host_key],
        ..server::Config::default()
    });
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            let config = Arc::clone(&config);
            tokio::spawn(async move {
                if let Ok(session) = server::run_stream(config, stream, KeyOnly).await {
                    let _ended = session.await;
                }
            });
        }
    });
    SshHost { port, key }
}
