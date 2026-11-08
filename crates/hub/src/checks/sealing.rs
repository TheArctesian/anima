//! The secrets of a check write: the stored ones a change keeps, and the sealed bytes the
//! write stores (SEC-003, SEC-005).

use super::{
    Error,
    config::CheckConfig,
    model::{Definition, FieldError},
    rules::{self, Prepared, Sealable, Stored},
    store,
};
use crate::{credentials::Vault, ids::CheckId};

/// The sealed bytes of the prepared secrets, bound to the check and to the configuration
/// they go with; `None` when there are none.
///
/// # Errors
/// `Validation` (`url`) when the HTTP URL has no origin to bind the secrets to;
/// `Credentials` (`Random`) when the OS random source fails.
pub(super) fn sealed(
    vault: &Vault,
    id: CheckId,
    prepared: &Prepared,
) -> Result<Option<Vec<u8>>, Error> {
    match (&prepared.secrets, &prepared.config) {
        (Some(Sealable::Http(secrets)), CheckConfig::Http(config)) => {
            store::seal_http(vault, id, config, secrets).map(Some)
        }
        (Some(Sealable::Snmp(secrets)), CheckConfig::Snmp(config)) => {
            store::seal_snmp(vault, id, config, secrets).map(Some)
        }
        _ => Ok(None),
    }
}

fn unopened() -> FieldError {
    FieldError::new(
        "config",
        "The stored secrets cannot be opened. Enter every secret again.",
    )
}

/// Prepares a new definition of the same type, filling empty secrets from the stored ones.
/// Stored secrets that do not open for this check give a field error that asks for every
/// secret again.
pub(super) fn prepare_change(
    vault: &Vault,
    id: CheckId,
    current: store::Current,
    definition: Definition,
) -> Result<Prepared, FieldError> {
    if definition.kind().as_sql() != current.kind {
        return Err(FieldError::new(
            "config",
            "A check keeps its type. Delete it and add a new check instead.",
        ));
    }
    let stored = match &definition {
        Definition::Http(input) if rules::needs_stored(input) => current
            .http(vault, id)
            .map_err(|_| unopened())?
            .map_or(Stored::Nothing, Stored::Http),
        Definition::Snmp(input) if rules::snmp_needs_stored(input) => current
            .snmp(vault, id)
            .map_err(|_| unopened())?
            .map_or(Stored::Nothing, Stored::Snmp),
        _ => Stored::Nothing,
    };
    rules::prepare(definition, stored)
}
