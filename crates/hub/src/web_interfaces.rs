//! Web interfaces: the web addresses an operator keeps for a device, the HTTP checks offered
//! as new ones, and the certificate the operator accepted for an `https` one (spec 6.9,
//! WEB-001, WEB-002, WEB-008).
//!
//! Reads take the pool. Writes take the caller's connection so the GraphQL layer adds its audit
//! entry in the same transaction (LOG-005). Messages in [`Error`] hold no input value
//! (LOG-007).

pub mod graphql;
mod model;
mod rules;
mod store;
mod suggestion;
#[cfg(test)]
mod tests;

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use crate::{
    checks::{self, config::CheckConfig},
    ids::{CheckId, DeviceId, WebInterfaceId},
};

pub use model::{
    Acceptance, Accepted, FieldError, Fingerprint, Missing, OpenIn, Pin, Suggestion, Updated,
    WebInterface, WebInterfaceInput,
};

/// Why a web interface operation failed.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0} not found")]
    NotFound(Missing),
    #[error("input is not valid: {}", field_names(.0))]
    Validation(Vec<FieldError>),
    #[error("database error")]
    Database(#[from] sqlx::Error),
    /// A stored row is not in a form this version reads.
    #[error("a stored web interface cannot be read")]
    Unreadable,
    #[error("the checks cannot be read")]
    Checks(#[source] checks::Error),
}

/// The names of the fields in error, never their values.
fn field_names(errors: &[FieldError]) -> String {
    errors
        .iter()
        .map(|error| error.field)
        .collect::<Vec<_>>()
        .join(", ")
}

/// The interfaces of each device in creation order, in one query (API-005). A device with
/// none is absent.
///
/// # Errors
/// `Database` when the query fails.
pub async fn for_devices(
    db: &PgPool,
    devices: &[DeviceId],
) -> Result<HashMap<DeviceId, Vec<WebInterface>>, Error> {
    Ok(group(store::for_devices(db, devices).await?))
}

fn group(interfaces: Vec<WebInterface>) -> HashMap<DeviceId, Vec<WebInterface>> {
    let mut grouped: HashMap<DeviceId, Vec<WebInterface>> = HashMap::new();
    for interface in interfaces {
        grouped.entry(interface.device).or_default().push(interface);
    }
    grouped
}

/// The HTTP checks of each device offered as interfaces (D-7). A device with none is absent.
///
/// # Errors
/// `Checks` when the checks cannot be read; `Database` when a query fails.
pub async fn suggestions(
    db: &PgPool,
    devices: &[DeviceId],
) -> Result<HashMap<DeviceId, Vec<Suggestion>>, Error> {
    let http = http_urls(db, devices).await?;
    let all_checks: Vec<CheckId> = http.values().flatten().map(|(check, _)| *check).collect();
    let dismissed = store::dismissed_among(db, &all_checks).await?;
    let interfaces = for_devices(db, devices).await?;
    Ok(http
        .into_iter()
        .filter_map(|(device, urls)| {
            let existing = interfaces.get(&device).map_or(&[][..], Vec::as_slice);
            let offered = suggestion::filter(&urls, existing, &dismissed);
            (!offered.is_empty()).then_some((device, offered))
        })
        .collect())
}

/// The id and URL of each device's HTTP checks (D-1). The one place that reads checks: when
/// `checks::http_urls` exists, this is one call to it.
async fn http_urls(
    db: &PgPool,
    devices: &[DeviceId],
) -> Result<HashMap<DeviceId, Vec<(CheckId, String)>>, Error> {
    let found = checks::for_devices(db, devices)
        .await
        .map_err(Error::Checks)?;
    Ok(found
        .into_iter()
        .map(|(device, list)| {
            let urls = list
                .into_iter()
                .filter_map(|check| match check.config {
                    CheckConfig::Http(http) => Some((check.id, http.url)),
                    _ => None,
                })
                .collect();
            (device, urls)
        })
        .collect())
}

fn born(device: DeviceId, at: DateTime<Utc>) -> store::Born {
    store::Born {
        id: WebInterfaceId::from(Uuid::now_v7()),
        device,
        at,
    }
}

/// Adds an interface to a device.
///
/// # Errors
/// `Validation` (`url`, `label`) when the input breaks a rule; `NotFound` when there is no
/// such device.
pub async fn create(
    conn: &mut PgConnection,
    device: DeviceId,
    input: &WebInterfaceInput,
    now: DateTime<Utc>,
) -> Result<WebInterface, Error> {
    let new = rules::validate(input).map_err(Error::Validation)?;
    store::insert(conn, born(device, now), &new).await
}

/// Replaces an interface's fields. A new scheme, host or port drops its pin (D-6).
///
/// # Errors
/// `Validation` (`url`, `label`); `NotFound` when there is no such interface.
pub async fn update(
    conn: &mut PgConnection,
    id: WebInterfaceId,
    input: &WebInterfaceInput,
) -> Result<Updated, Error> {
    let new = rules::validate(input).map_err(Error::Validation)?;
    let (before, after) = store::update(conn, id, &new).await?;
    Ok(Updated { before, after })
}

/// Deletes an interface and its pin; returns the device it belonged to.
///
/// # Errors
/// `NotFound` when there is no such interface.
pub async fn delete(conn: &mut PgConnection, id: WebInterfaceId) -> Result<DeviceId, Error> {
    store::delete(conn, id).await
}

/// The URL of an HTTP check, and its device.
async fn http_check(db: &PgPool, id: CheckId) -> Result<(DeviceId, String), Error> {
    let check = checks::get(db, id).await.map_err(|error| match error {
        checks::Error::NotFound(checks::Missing::Check(id)) => Error::NotFound(Missing::Check(id)),
        other => Error::Checks(other),
    })?;
    match check.config {
        CheckConfig::Http(http) => Ok((check.device, http.url)),
        _ => Err(Error::NotFound(Missing::Check(id))),
    }
}

/// Makes an interface of an HTTP check's URL (`AnimaTab`, labelled with the host). When the
/// device already has an interface at that URL, returns it and creates nothing (D-7).
///
/// # Errors
/// `NotFound` when the check does not exist or is not an HTTP check; `Validation` (`url`)
/// when its URL breaks the URL rule.
pub async fn accept_suggestion(
    db: &PgPool,
    conn: &mut PgConnection,
    check: CheckId,
    now: DateTime<Utc>,
) -> Result<Accepted, Error> {
    let (device, url) = http_check(db, check).await?;
    let input = WebInterfaceInput {
        label: None,
        url,
        open_in: OpenIn::AnimaTab,
    };
    let new = rules::validate(&input).map_err(Error::Validation)?;
    let existing = store::for_devices(&mut *conn, &[device]).await?;
    if let Some(interface) = existing
        .into_iter()
        .find(|interface| suggestion::same_url(&interface.url, &new.url))
    {
        return Ok(Accepted {
            interface,
            created: false,
        });
    }
    let interface = store::insert(conn, born(device, now), &new).await?;
    Ok(Accepted {
        interface,
        created: true,
    })
}

/// Hides an HTTP check's suggestion; a second dismissal changes nothing. Returns the check's
/// device.
///
/// # Errors
/// `NotFound` when the check does not exist or is not an HTTP check.
pub async fn dismiss_suggestion(
    db: &PgPool,
    conn: &mut PgConnection,
    check: CheckId,
    now: DateTime<Utc>,
) -> Result<DeviceId, Error> {
    let (device, _) = http_check(db, check).await?;
    store::insert_dismissed(conn, check, now).await?;
    Ok(device)
}

/// Pins the certificate of an `https` interface, replacing an earlier pin (D-5). Returns the
/// interface with its new pin.
///
/// # Errors
/// `NotFound` when there is no such interface; `Validation` (`fingerprintSha256`) when the
/// fingerprint is badly formed or the interface is `http`.
pub async fn accept_certificate(
    conn: &mut PgConnection,
    id: WebInterfaceId,
    acceptance: Acceptance<'_>,
) -> Result<WebInterface, Error> {
    let interface = store::get(&mut *conn, id).await?;
    let fingerprint =
        rules::fingerprint(acceptance.fingerprint).map_err(|e| Error::Validation(vec![e]))?;
    if !rules::is_https(&interface.url) {
        return Err(Error::Validation(vec![FieldError::new(
            "fingerprintSha256",
            "Only an https interface has a certificate to accept.",
        )]));
    }
    let pin = Pin {
        fingerprint,
        at: acceptance.at,
    };
    store::set_pin(conn, id, &pin, acceptance.operator).await?;
    Ok(WebInterface {
        pin: Some(pin),
        ..interface
    })
}
