//! The secrets of a check write: the stored ones a change keeps, and the sealed bytes the
//! write stores (SEC-003, SEC-005).

use super::{
    config::CheckConfig,
    model::{Definition, FieldError},
    rules::{self, Prepared, Sealable, Stored},
    store,
};
use crate::{credentials::Vault, ids::CheckId};

/// The sealed bytes of the prepared secrets, bound to the check and to the configuration
/// they go with; `None` when there are none.
pub(super) fn sealed(vault: &Vault, id: CheckId, prepared: &Prepared) -> Option<Vec<u8>> {
    match (&prepared.secrets, &prepared.config) {
        (Some(Sealable::Http(secrets)), CheckConfig::Http(config)) => {
            Some(store::seal(vault, id, config, secrets))
        }
        (Some(Sealable::Snmp(secrets)), CheckConfig::Snmp(config)) => {
            Some(store::seal_snmp(vault, id, config, secrets))
        }
        _ => None,
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
