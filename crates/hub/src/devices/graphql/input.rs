//! The inputs of the device mutations (K1 `CreateDeviceInput`, `UpdateDeviceInput`,
//! `SshSettingsInput`). Each converts to the form types the pure rules check.

use async_graphql::{ID, InputObject, Result};

use crate::{
    devices::{
        FieldError, OperatingSystem, SshAuth,
        model::{DeviceChangeInput, NewDeviceInput, SshInput},
        rules::DEFAULT_KIND,
    },
    ids::{DeviceId, SiteId},
};

use super::error::invalid;

#[derive(InputObject)]
#[graphql(name = "SshSettingsInput")]
pub struct SshSettingsInput {
    enabled: bool,
    #[graphql(default_with = "Some(22)")]
    port: Option<i32>,
    username: String,
    auth: SshAuth,
}

impl SshSettingsInput {
    /// The SSH port as sent, K1's default 22 when left out, unchecked. For
    /// `ConnectionTestInput.ssh` (WP-10, R-011), which reuses this K1 type.
    #[must_use]
    pub const fn port(&self) -> Option<i32> {
        self.port
    }
}

impl From<SshSettingsInput> for SshInput {
    fn from(input: SshSettingsInput) -> Self {
        Self {
            enabled: input.enabled,
            port: input.port,
            username: input.username,
            auth: input.auth,
        }
    }
}

#[derive(InputObject)]
pub(super) struct CreateDeviceInput {
    site_id: ID,
    name: String,
    host: String,
    #[graphql(default_with = r#"Some("server".to_owned())"#)]
    kind: Option<String>,
    #[graphql(default_with = "Some(OperatingSystem::Other)")]
    os: Option<OperatingSystem>,
    #[graphql(default_with = "Some(Vec::<String>::new())")]
    tags: Option<Vec<String>>,
    ssh: Option<SshSettingsInput>,
    /// When true (the default), the hub adds one ICMP check (spec 6.2).
    #[graphql(default_with = "Some(true)")]
    add_default_check: Option<bool>,
}

#[derive(InputObject)]
pub(super) struct UpdateDeviceInput {
    site_id: Option<ID>,
    name: Option<String>,
    host: Option<String>,
    kind: Option<String>,
    os: Option<OperatingSystem>,
    tags: Option<Vec<String>>,
    maintenance: Option<bool>,
    ssh: Option<SshSettingsInput>,
}

fn site_id(id: &ID) -> Result<SiteId, FieldError> {
    id.parse()
        .map_err(|_| FieldError::new("siteId", "That site id is not valid."))
}

/// The id in a mutation argument.
pub(super) fn device_id(id: &ID) -> Result<DeviceId, FieldError> {
    id.parse()
        .map_err(|_| FieldError::new("id", "That device id is not valid."))
}

impl CreateDeviceInput {
    /// Whether to add the default check; true unless the operator sent false.
    pub(super) fn add_default_check(&self) -> bool {
        self.add_default_check.unwrap_or(true)
    }

    /// The form to validate, and the malformed `siteId` if there is one. A malformed id is
    /// replaced by the nil id so the other fields are still checked and reported together.
    pub(super) fn into_form(self) -> (NewDeviceInput, Option<FieldError>) {
        let (site, bad_site) = match site_id(&self.site_id) {
            Ok(site) => (site, None),
            Err(error) => (SiteId::from(uuid::Uuid::nil()), Some(error)),
        };
        let form = NewDeviceInput {
            site,
            name: self.name,
            host: self.host,
            kind: self.kind.unwrap_or_else(|| DEFAULT_KIND.to_owned()),
            os: self.os.unwrap_or(OperatingSystem::Other),
            tags: self.tags.unwrap_or_default(),
            ssh: self.ssh.map(SshInput::from),
        };
        (form, bad_site)
    }
}

impl UpdateDeviceInput {
    /// The form to validate, and the malformed `siteId` if there is one.
    pub(super) fn into_form(self) -> (DeviceChangeInput, Option<FieldError>) {
        let (site, bad_site) = match self.site_id.as_ref().map(site_id).transpose() {
            Ok(site) => (site, None),
            Err(error) => (None, Some(error)),
        };
        let form = DeviceChangeInput {
            site,
            name: self.name,
            host: self.host,
            kind: self.kind,
            os: self.os,
            tags: self.tags,
            maintenance: self.maintenance,
            ssh: self.ssh.map(SshInput::from),
        };
        (form, bad_site)
    }
}

/// The value when the rules and the id parsing found nothing wrong; otherwise one `VALIDATION`
/// error that lists every field at once.
pub(super) fn checked<T>(
    rules: Result<T, Vec<FieldError>>,
    bad_ids: impl IntoIterator<Item = FieldError>,
) -> Result<T> {
    let mut fields: Vec<_> = bad_ids.into_iter().collect();
    match rules {
        Ok(value) if fields.is_empty() => Ok(value),
        rules => {
            fields.extend(rules.err().into_iter().flatten());
            Err(invalid(&fields))
        }
    }
}
