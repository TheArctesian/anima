//! Certificate reload without a restart (R-021 H-2, DNS-002): once an hour the hub looks at
//! the modification time and size of both files and, when either changed, loads them and
//! swaps the served certificate. A load that fails keeps the old certificate.
//!
//! [`Reload`] is the pure core: the caller passes the time and the files' state.

use std::{fs, io, sync::Arc, time::Duration, time::SystemTime};

use chrono::{DateTime, TimeDelta, Utc};
use tokio_rustls::rustls::{crypto::CryptoProvider, sign::CertifiedKey};

use super::certificate::{self, Current, Fault, TlsFiles};
use crate::clock::Clock;

/// How often the files are looked at.
pub(super) const CHECK_EVERY: TimeDelta = TimeDelta::hours(1);

/// What a file looked like: changing the content changes at least one of these in practice.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Stamp {
    pub modified: Option<SystemTime>,
    pub len: u64,
}

/// The state of the certificate file and the key file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Stamps {
    pub cert: Stamp,
    pub key: Stamp,
}

/// Reads the state of both files.
pub(super) fn stamps(files: &TlsFiles) -> Result<Stamps, Fault> {
    let stamp = |path, fault: fn(io::ErrorKind) -> Fault| {
        let meta = fs::metadata(path).map_err(|error| fault(error.kind()))?;
        Ok(Stamp {
            modified: meta.modified().ok(),
            len: meta.len(),
        })
    };
    Ok(Stamps {
        cert: stamp(&files.cert, Fault::CertificateFile)?,
        key: stamp(&files.key, Fault::KeyFile)?,
    })
}

/// When the served certificate's files were loaded, and when to look at them next.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Reload {
    loaded: Stamps,
    next: DateTime<Utc>,
}

impl Reload {
    /// After the files in state `loaded` were loaded at `now`.
    pub(super) fn new(loaded: Stamps, now: DateTime<Utc>) -> Self {
        Self {
            loaded,
            next: now + CHECK_EVERY,
        }
    }

    /// How long to sleep at `now` before the next look; never more than [`CHECK_EVERY`],
    /// so a clock that went back does not stop the checks.
    pub(super) fn wait(&self, now: DateTime<Utc>) -> Duration {
        (self.next - now)
            .clamp(TimeDelta::zero(), CHECK_EVERY)
            .to_std()
            .unwrap_or(Duration::ZERO)
    }

    /// Whether a look is due at `now`; if so, schedules the next one.
    pub(super) fn begin(&mut self, now: DateTime<Utc>) -> bool {
        if now < self.next {
            return false;
        }
        self.next = now + CHECK_EVERY;
        true
    }

    /// Whether files in state `seen` must be loaded.
    pub(super) fn changed(&self, seen: Stamps) -> bool {
        seen != self.loaded
    }

    /// Notes that the files in state `seen` are now served.
    pub(super) fn loaded(&mut self, seen: Stamps) {
        self.loaded = seen;
    }
}

/// What one look at the files did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Outcome {
    NotDue,
    Unchanged,
    Swapped,
    /// The files changed but did not load; the old certificate is still served.
    Kept(Fault),
}

/// The served certificate, the files it comes from and the provider that loads them.
pub(super) struct Served {
    pub files: TlsFiles,
    pub current: Arc<Current>,
    pub provider: Arc<CryptoProvider>,
}

/// Looks at the files at `now` if a look is due, and swaps the served certificate when they
/// changed and load. The state is read before the content, so a change during the read is
/// seen at the next look.
pub(super) async fn check(reload: &mut Reload, now: DateTime<Utc>, served: &Served) -> Outcome {
    if !reload.begin(now) {
        return Outcome::NotDue;
    }
    let seen = match blocking(&served.files, stamps).await {
        Ok(seen) => seen,
        Err(fault) => return Outcome::Kept(fault),
    };
    if !reload.changed(seen) {
        return Outcome::Unchanged;
    }
    match load(&served.files, &served.provider).await {
        Ok(key) => {
            served.current.swap(key);
            reload.loaded(seen);
            Outcome::Swapped
        }
        Err(fault) => Outcome::Kept(fault),
    }
}

/// Reads and checks the certificate and key on the blocking pool.
pub(super) async fn load(
    files: &TlsFiles,
    provider: &Arc<CryptoProvider>,
) -> Result<CertifiedKey, Fault> {
    let provider = Arc::clone(provider);
    blocking(files, move |files| {
        certificate::certified_key(&certificate::read(files)?, &provider)
    })
    .await
}

/// Runs `work` on the files on the blocking pool.
pub(super) async fn blocking<T: Send + 'static>(
    files: &TlsFiles,
    work: impl FnOnce(&TlsFiles) -> Result<T, Fault> + Send + 'static,
) -> Result<T, Fault> {
    let files = files.clone();
    tokio::task::spawn_blocking(move || work(&files))
        .await
        .unwrap_or(Err(Fault::Interrupted))
}

/// Looks at the files every [`CHECK_EVERY`] for as long as the listener lives.
pub(super) async fn run(mut reload: Reload, served: Served, clock: Arc<dyn Clock>) {
    loop {
        tokio::time::sleep(reload.wait(clock.now())).await;
        log(check(&mut reload, clock.now(), &served).await);
    }
}

/// Logs a swap, and a kept certificate by fault kind only (LOG-007).
fn log(outcome: Outcome) {
    match outcome {
        Outcome::Swapped => tracing::info!(domain = "access", "TLS certificate reloaded"),
        Outcome::Kept(fault) => log_kept(fault),
        Outcome::NotDue | Outcome::Unchanged => {}
    }
}

fn log_kept(fault: Fault) {
    tracing::warn!(
        domain = "access",
        fault = fault.kind(),
        "TLS certificate reload failed; the previous certificate is still served"
    );
}
