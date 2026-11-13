//! A start refused because the device's stored credential for the protocol was entered for
//! another host than the device's host now (SEC-011). `startSession` asks before `start`, so the
//! operator gets the refusal instead of a session that ends at once; the tunnels check again
//! when they open the credential, so a host changed after the start still sends nothing.
//!
//! The credential is opened only to read the host it is bound to, and dropped at once: nothing
//! leaves the hub. It stays stored, so the device opens sessions again if its host goes back.
//! No log or message here holds a host or a secret (LOG-007).

use chrono::{DateTime, Utc};
use sqlx::PgPool;

use crate::{
    credentials::{self, Vault},
    devices::Device,
    events::{self, Bus, NewEvent, Severity, Source},
    ids::DeviceId,
    sessions::{Conflict, Desktop, Error, Protocol, find_device},
};

/// Refuses a start of `protocol` on the device when its stored credential for that protocol
/// was entered for another host, with one Warning event at `now`. A device with no stored
/// credential is not refused here.
///
/// # Errors
/// `Conflict(CredentialMoved)` as above; `NotFound` for an unknown device; `Credentials`,
/// `Events` or `Database` when a dependency fails.
pub(in crate::sessions) async fn refuse_moved(
    (db, vault, bus): (&PgPool, &Vault, &Bus),
    device: DeviceId,
    protocol: Protocol,
    now: DateTime<Utc>,
) -> Result<(), Error> {
    let device = find_device(db, device).await?;
    let opened = match Desktop::of(protocol) {
        Some(desktop) => {
            let bound = (device.id, device.host.as_str());
            credentials::load_password(db, vault, bound, desktop.password())
                .await
                .map(drop)
        }
        None => credentials::load_ssh(db, vault, (device.id, &device.host))
            .await
            .map(drop),
    };
    match opened {
        Ok(()) => Ok(()),
        Err(credentials::Error::EnteredForOtherHost) => {
            let conflict = Conflict::CredentialMoved(protocol);
            record(db, bus, now, event(&device, conflict)).await?;
            Err(Error::Conflict(conflict))
        }
        Err(error) => Err(Error::Credentials(error)),
    }
}

/// The Warning event of a refused start, in the words of the refusal.
fn event(device: &Device, conflict: Conflict) -> NewEvent {
    NewEvent {
        severity: Severity::Warning,
        source: Source::Session,
        device: Some((device.id, device.name.clone())),
        message: conflict.to_string(),
    }
}

async fn record(db: &PgPool, bus: &Bus, now: DateTime<Utc>, event: NewEvent) -> Result<(), Error> {
    let mut tx = db.begin().await?;
    events::record(&mut tx, bus, now, event).await?;
    tx.commit().await?;
    Ok(())
}
