//! The download of a runtime archive (Sections 4 and 6): HTTPS only, no proxy, the roots of
//! the program, at most 5 redirects and only to a host of the fixed list of that runtime, never
//! to an address written as an IP address; at most `sizeBytes` read, the SHA-256 computed as
//! the bytes arrive.
//!
//! Contract Section 4 asks for a child "fetch process" with no token. This build fetches in the
//! daemon process, with a client of its own that holds no token and no hub origin (reported as
//! a deviation of task L1-6).

use std::io::Write as _;
use std::time::Duration;

use daemon_core::model_work::cause::Cause;
use daemon_core::model_work::names::Runtime;
use reqwest::{Client, Url, redirect};
use sha2::{Digest as _, Sha256};

/// The hosts that a redirect of a release source may reach, for each runtime.
#[must_use]
pub const fn release_hosts(runtime: Runtime) -> &'static [&'static str] {
    match runtime {
        Runtime::LlamaServer => &[
            "github.com",
            "objects.githubusercontent.com",
            "release-assets.githubusercontent.com",
        ],
        Runtime::MlxLm | Runtime::Vllm => &[],
    }
}

/// Whether a redirect to `url` is permitted: `https`, a name (not an IP address) of `hosts`.
#[must_use]
pub fn redirect_permitted(url: &Url, hosts: &[&str]) -> bool {
    url.scheme() == "https"
        && url
            .host_str()
            .is_some_and(|host| host.parse::<std::net::IpAddr>().is_err() && hosts.contains(&host))
}

fn client(hosts: &'static [&'static str]) -> Result<Client, Cause> {
    let policy = redirect::Policy::custom(move |attempt| {
        if attempt.previous().len() >= 5 || !redirect_permitted(attempt.url(), hosts) {
            attempt.stop()
        } else {
            attempt.follow()
        }
    });
    Client::builder()
        .https_only(true)
        .no_proxy()
        .redirect(policy)
        .connect_timeout(Duration::from_secs(10))
        .read_timeout(Duration::from_secs(60))
        .build()
        .map_err(|_| Cause::SourceFailed)
}

/// Downloads `address` into `file` and gives the SHA-256 in lower-case hexadecimal.
///
/// # Errors
/// `source_failed` when the source does not answer 200; `size` when it sends more than
/// `max_bytes`; `limit_disk` when the file cannot be written.
pub async fn fetch(
    address: &str,
    hosts: &'static [&'static str],
    max_bytes: u64,
    file: &mut std::fs::File,
) -> Result<String, Cause> {
    let url = Url::parse(address).map_err(|_| Cause::RuntimeNotConfigured)?;
    let mut response = client(hosts)?
        .get(url)
        .send()
        .await
        .map_err(|_| Cause::SourceFailed)?;
    if response.status().as_u16() != 200 {
        return Err(Cause::SourceFailed);
    }
    let mut hash = Sha256::new();
    let mut read = 0u64;
    while let Some(chunk) = response.chunk().await.map_err(|_| Cause::SourceFailed)? {
        read += chunk.len() as u64;
        if read > max_bytes {
            return Err(Cause::Size);
        }
        hash.update(&chunk);
        file.write_all(&chunk).map_err(|_| Cause::LimitDisk)?;
    }
    Ok(hex(&hash.finalize()))
}

/// Lower-case hexadecimal of `bytes`.
#[must_use]
pub fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes
        .iter()
        .fold(String::with_capacity(bytes.len() * 2), |mut text, byte| {
            let _ = write!(text, "{byte:02x}");
            text
        })
}
