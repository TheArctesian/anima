//! A throwaway release channel: a key pair made in memory, signed builds, and a loopback HTTP
//! server that answers like `releases.anima.ing` (200 for a file it has, 404 for any other).

use std::collections::HashMap;
use std::io::Cursor;
use std::sync::{Arc, Mutex};

use base64::{Engine as _, engine::general_purpose::STANDARD};
use minisign::KeyPair;
use tauri::test::{mock_builder, mock_context, noop_assets};
use tauri_plugin_updater::{Update, UpdaterExt};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

const CONFIG: &str = include_str!("../../tauri.conf.json");
const ORIGIN: &str = "https://releases.anima.ing";

/// This computer's updater target, as `tauri-plugin-updater` names it: `linux-x86_64`.
fn target() -> String {
    let os = if cfg!(target_os = "macos") {
        "darwin"
    } else {
        std::env::consts::OS
    };
    format!("{os}-{}", std::env::consts::ARCH)
}

/// Where the app must ask for this computer's feed.
pub fn feed_path() -> String {
    format!("/{}/latest.json", target())
}

/// The updater block of the shipped `tauri.conf.json`.
fn shipped_updater() -> serde_json::Value {
    let config: serde_json::Value = serde_json::from_str(CONFIG).unwrap();
    config["plugins"]["updater"].clone()
}

/// The one endpoint the shipped configuration lists.
pub fn shipped_endpoint() -> String {
    let updater = shipped_updater();
    let endpoints = updater["endpoints"].as_array().unwrap();
    assert_eq!(endpoints.len(), 1, "one feed, no other");
    endpoints[0].as_str().unwrap().to_owned()
}

/// A key pair standing in for the operator's; it signs builds and the app trusts its public half.
pub struct Channel(KeyPair);

/// A built file: the bytes the server holds, the version the feed announces, and the signature
/// text it lists.
pub struct Build {
    bytes: Vec<u8>,
    announced: String,
    signature: String,
}

/// How a build is listed in the feed: under this computer's target or under another.
pub struct Listing<'a> {
    key: String,
    build: &'a Build,
}

impl<'a> Listing<'a> {
    pub fn here(build: &'a Build) -> Self {
        Self {
            key: target(),
            build,
        }
    }

    pub fn elsewhere(build: &'a Build) -> Self {
        Self {
            key: "plan9-mips".to_owned(),
            build,
        }
    }
}

impl Channel {
    pub fn new() -> Self {
        Self(KeyPair::generate_unencrypted_keypair().unwrap())
    }

    fn sign(&self, bytes: &[u8], version: Option<&str>) -> String {
        let named = version.map_or_else(String::new, |version| format!("\tversion:{version}"));
        let comment = format!("timestamp:1700000000\tfile:Anima.AppImage{named}");
        let signature = minisign::sign(
            Some(&self.0.pk),
            &self.0.sk,
            Cursor::new(bytes),
            Some(&comment),
            None,
        )
        .unwrap();
        STANDARD.encode(signature.into_string())
    }

    /// A build of `version`, signed for `version`.
    pub fn build(&self, version: &str) -> Build {
        self.build_signed_for(version, version)
    }

    /// A build announced as `announced` whose signature names `signed_for`.
    pub fn build_signed_for(&self, announced: &str, signed_for: &str) -> Build {
        let bytes = format!("AppImage of Anima {signed_for}").into_bytes();
        Build {
            signature: self.sign(&bytes, Some(signed_for)),
            bytes,
            announced: announced.to_owned(),
        }
    }

    /// A build whose signature names no version, as older Tauri CLIs made them.
    pub fn build_unversioned(&self, announced: &str) -> Build {
        let bytes = format!("AppImage of Anima {announced}").into_bytes();
        Build {
            signature: self.sign(&bytes, None),
            bytes,
            announced: announced.to_owned(),
        }
    }

    /// A build whose bytes were changed after it was signed.
    pub fn build_tampered(&self, version: &str) -> Build {
        let mut build = self.build(version);
        build.bytes.extend_from_slice(b" and then some");
        build
    }

    /// The `plugins.updater.pubkey` text: base64 of the public key file.
    fn pubkey(&self) -> String {
        STANDARD.encode(self.0.pk.to_box().unwrap().into_string())
    }
}

impl Build {
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

type Routes = HashMap<String, Vec<u8>>;

/// A loopback server and the app configured to trust one channel and read from it.
pub struct Served {
    port: u16,
    pubkey: String,
    asked: Arc<Mutex<Vec<String>>>,
}

impl Served {
    /// Serves `listing` (or nothing, so every path answers 404) for an app that trusts `channel`.
    pub async fn start(channel: &Channel, listing: Option<Listing<'_>>) -> Self {
        let socket = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = socket.local_addr().unwrap().port();
        let mut routes = Routes::new();
        if let Some(Listing { key, build }) = listing {
            let file = format!("/{}/{}/Anima.AppImage", target(), build.announced);
            let feed = serde_json::json!({
                "version": build.announced,
                "pub_date": "2026-11-10T10:00:00Z",
                "platforms": { key: {
                    "signature": build.signature,
                    "url": format!("http://127.0.0.1:{port}{file}"),
                } },
            });
            routes.insert(feed_path(), serde_json::to_vec(&feed).unwrap());
            routes.insert(file, build.bytes.clone());
        }
        let asked = Arc::new(Mutex::new(Vec::new()));
        tokio::spawn(answer(socket, routes, Arc::clone(&asked)));
        Self {
            port,
            pubkey: channel.pubkey(),
            asked,
        }
    }

    /// The paths the server was asked for, in order.
    pub fn requested(&self) -> Vec<String> {
        self.asked.lock().unwrap().clone()
    }

    /// One update check by the real plugin, configured from the shipped block with the key and the
    /// origin swapped for this server's.
    pub async fn check(&self) -> tauri_plugin_updater::Result<Option<Update>> {
        let mut updater = shipped_updater();
        let endpoint =
            shipped_endpoint().replace(ORIGIN, &format!("http://127.0.0.1:{}", self.port));
        updater["pubkey"] = self.pubkey.clone().into();
        updater["endpoints"] = serde_json::json!([endpoint]);
        let mut context = mock_context(noop_assets());
        context
            .config_mut()
            .plugins
            .0
            .insert("updater".to_owned(), updater);
        let app = mock_builder()
            .plugin(tauri_plugin_updater::Builder::new().build())
            .build(context)
            .unwrap();
        app.handle().updater().unwrap().check().await
    }
}

/// Answers each connection with the route's bytes (200) or 404, and keeps the path asked for.
async fn answer(socket: TcpListener, routes: Routes, asked: Arc<Mutex<Vec<String>>>) {
    loop {
        let (mut stream, _) = socket.accept().await.unwrap();
        let mut head = vec![0_u8; 4096];
        let read = stream.read(&mut head).await.unwrap();
        let request = String::from_utf8_lossy(&head[..read]).into_owned();
        let path = request
            .split_whitespace()
            .nth(1)
            .unwrap_or_default()
            .to_owned();
        let reply = match routes.get(&path) {
            Some(body) => [
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                )
                .into_bytes(),
                body.clone(),
            ]
            .concat(),
            None => {
                b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_vec()
            }
        };
        asked.lock().unwrap().push(path);
        stream.write_all(&reply).await.unwrap();
        stream.shutdown().await.unwrap();
    }
}
