//! The SNMP configuration input as domain values: the secrets move into the input without a
//! copy, and the form records whether one was sent. Absent and empty secrets keep the
//! stored ones; the domain decides whether it may (D-5).

use super::{ConfigForm, or_default};
use crate::fleet::{
    checks::{
        Definition, FieldError,
        config::{SnmpConfig, SnmpCredentials, SnmpInput},
        graphql::input::{SecretText, SnmpShape},
    },
    credentials::Secret,
};

pub(super) fn snmp(shape: SnmpShape) -> Result<ConfigForm, FieldError> {
    let secret = |text: Option<SecretText>| text.map(|text| text.0);
    let credentials = SnmpCredentials {
        community: secret(shape.community),
        auth_password: secret(shape.auth_password),
        privacy_password: secret(shape.privacy_password),
    };
    let given = |secret: &Option<Secret>| secret.as_ref().is_some_and(|s| !s.expose().is_empty());
    let sealed = given(&credentials.community)
        || given(&credentials.auth_password)
        || given(&credentials.privacy_password);
    let config = SnmpConfig {
        port: or_default("port", shape.port, 161)?,
        version: shape.version.unwrap_or_default().into(),
        oid: shape.oid,
        expected: shape.expected,
        username: shape.username,
        timeout_ms: or_default("timeoutMs", shape.timeout_ms, 5000)?,
    };
    Ok(ConfigForm {
        definition: Definition::Snmp(SnmpInput {
            config,
            credentials,
        }),
        children: None,
        sealed,
    })
}
