//! Turns the parsed check inputs into domain values and checks the range of every number.
//! Pure: the new push token comes in as a parameter. Errors name the field, never its value
//! (LOG-007).

use async_graphql::ID;

use super::input::{
    AuthShape, ConfigShape, CreateShape, DnsShape, HttpShape, IcmpShape, TcpShape, TlsShape,
    UpdateShape,
};
use super::settings::RawSettings;
use crate::{
    checks::{
        CheckChange, CheckSettings, Definition, FieldError, NewCheck, PushToken,
        config::{CheckConfig, HttpCredentials, HttpInput, StatusRange, is_secret_header},
    },
    ids::{CheckId, DeviceId},
};

/// A configuration input as the domain takes it.
pub(super) struct ConfigForm {
    pub(super) definition: Definition,
    /// For a group: the checks that belong to it.
    pub(super) children: Option<Vec<CheckId>>,
    /// Whether the operator sent a non-empty secret (the audit entry says only that).
    pub(super) sealed: bool,
}

impl ConfigForm {
    fn plain(config: CheckConfig) -> Self {
        Self {
            definition: Definition::Config(config),
            children: None,
            sealed: false,
        }
    }
}

pub(super) fn number<T: TryFrom<i32>>(field: &'static str, value: i32) -> Result<T, FieldError> {
    T::try_from(value).map_err(|_| FieldError::new(field, "This number is out of range."))
}

fn or_default<T: TryFrom<i32>>(
    field: &'static str,
    value: Option<i32>,
    default: i32,
) -> Result<T, FieldError> {
    number(field, value.unwrap_or(default))
}

fn id<T: std::str::FromStr>(field: &'static str, id: &ID) -> Result<T, FieldError> {
    id.parse()
        .map_err(|_| FieldError::new(field, "That id is not valid."))
}

/// Records the error and gives `None`, so one pass reports every field in error.
pub(super) fn keep<T>(result: Result<T, FieldError>, errors: &mut Vec<FieldError>) -> Option<T> {
    result.map_err(|error| errors.push(error)).ok()
}

fn http(shape: HttpShape) -> Result<ConfigForm, FieldError> {
    let mut sealed = false;
    let mut headers = Vec::new();
    for header in shape.headers.unwrap_or_default() {
        let header = header.0;
        sealed |= is_secret_header(&header.name) && !header.value.0.expose().is_empty();
        headers.push((header.name, header.value.0));
    }
    let credentials = match shape.auth.map(|auth| auth.0) {
        None => HttpCredentials::None,
        Some(AuthShape::Basic(basic)) => {
            let basic = basic.0;
            sealed |= !basic.password.0.expose().is_empty();
            HttpCredentials::Basic {
                username: basic.username,
                password: basic.password.0,
            }
        }
        Some(AuthShape::BearerToken(token)) => {
            sealed |= !token.0.expose().is_empty();
            HttpCredentials::Bearer(token.0)
        }
    };
    let ranges = shape.accepted_status.unwrap_or_default();
    let accepted_status = ranges
        .into_iter()
        .map(|range| {
            Ok(StatusRange {
                from: number("acceptedStatus", range.from)?,
                to: number("acceptedStatus", range.to)?,
            })
        })
        .collect::<Result<_, FieldError>>()?;
    let input = HttpInput {
        url: shape.url,
        method: shape.method.unwrap_or_default().into(),
        headers,
        body: shape.body,
        max_redirects: or_default("maxRedirects", shape.max_redirects, 10)?,
        accepted_status,
        keyword: shape.keyword,
        json_query: shape.json_query,
        json_expected: shape.json_expected,
        credentials,
        verify_tls: shape.verify_tls.unwrap_or(true),
        timeout_ms: or_default("timeoutMs", shape.timeout_ms, 10_000)?,
    };
    Ok(ConfigForm {
        definition: Definition::Http(input),
        children: None,
        sealed,
    })
}

fn icmp(shape: &IcmpShape) -> Result<ConfigForm, FieldError> {
    Ok(ConfigForm::plain(CheckConfig::Icmp {
        timeout_ms: or_default("timeoutMs", shape.timeout_ms, 2000)?,
        ip_version: shape.ip_version.unwrap_or_default().into(),
    }))
}

fn tcp(shape: &TcpShape) -> Result<ConfigForm, FieldError> {
    Ok(ConfigForm::plain(CheckConfig::Tcp {
        port: number("port", shape.port)?,
        timeout_ms: or_default("timeoutMs", shape.timeout_ms, 5000)?,
    }))
}

fn tls(shape: TlsShape) -> Result<ConfigForm, FieldError> {
    Ok(ConfigForm::plain(CheckConfig::Tls {
        port: or_default("port", shape.port, 443)?,
        server_name: shape.server_name,
        timeout_ms: or_default("timeoutMs", shape.timeout_ms, 5000)?,
    }))
}

fn dns(shape: DnsShape) -> ConfigForm {
    ConfigForm::plain(CheckConfig::Dns {
        name: shape.name,
        record_type: shape.record_type.unwrap_or_default().into(),
        resolver: shape.resolver.map(|ip| ip.0),
        expected: shape.expected,
    })
}

/// The group's children, each once, in the order given.
fn children(ids: &[ID]) -> Result<Vec<CheckId>, FieldError> {
    let mut parsed: Vec<CheckId> = Vec::with_capacity(ids.len());
    for each in ids {
        let child = id("childIds", each)?;
        if !parsed.contains(&child) {
            parsed.push(child);
        }
    }
    Ok(parsed)
}

/// The configuration input as domain values. `token` is used only for a push configuration.
pub(super) fn config(shape: ConfigShape, token: PushToken) -> Result<ConfigForm, FieldError> {
    match shape {
        ConfigShape::Icmp(shape) => icmp(&shape),
        ConfigShape::Tcp(shape) => tcp(&shape),
        ConfigShape::Tls(shape) => tls(shape),
        ConfigShape::Http(shape) => http(shape.0),
        ConfigShape::Dns(shape) => Ok(dns(shape)),
        ConfigShape::Push(true) => Ok(ConfigForm {
            definition: Definition::Push(token),
            children: None,
            sealed: false,
        }),
        ConfigShape::Push(false) => Err(FieldError::new("config", "Send push: true.")),
        ConfigShape::Group(shape) => Ok(ConfigForm {
            definition: Definition::Config(CheckConfig::Group),
            children: Some(children(&shape.child_ids)?),
            sealed: false,
        }),
    }
}

/// A check to add, with what the mutation does besides the insert.
pub(super) struct CreateForm {
    pub(super) new: NewCheck,
    pub(super) children: Option<Vec<CheckId>>,
    pub(super) sealed: bool,
}

/// The create input as domain values; every field in error is reported.
pub(super) fn create(input: CreateShape, token: PushToken) -> Result<CreateForm, Vec<FieldError>> {
    let mut errors = Vec::new();
    let device = keep(id::<DeviceId>("deviceId", &input.device_id), &mut errors);
    let parent = input
        .parent_id
        .as_ref()
        .and_then(|parent| keep(id::<CheckId>("parentId", parent), &mut errors));
    let config = keep(config(input.config.0, token), &mut errors);
    let raw = RawSettings {
        interval_seconds: input.interval_seconds,
        retries: input.retries,
        retry_interval_seconds: input.retry_interval_seconds,
        degraded_threshold_ms: input.degraded_threshold_ms,
    };
    let given = raw.convert(&mut errors);
    let (Some(device), Some(config), true) = (device, config, errors.is_empty()) else {
        return Err(errors);
    };
    let base = CheckSettings::default();
    let settings = CheckSettings {
        interval_seconds: given.interval_seconds.unwrap_or(base.interval_seconds),
        retries: given.retries.unwrap_or(base.retries),
        retry_interval_seconds: given
            .retry_interval_seconds
            .unwrap_or(base.retry_interval_seconds),
        degraded_threshold_ms: given
            .degraded_threshold_ms
            .unwrap_or(base.degraded_threshold_ms),
        invert: input.invert.unwrap_or(base.invert),
    };
    Ok(CreateForm {
        new: NewCheck {
            device,
            parent,
            name: input.name,
            definition: config.definition,
            settings,
        },
        children: config.children,
        sealed: config.sealed,
    })
}

/// A change to apply, with what the mutation does besides the update.
pub(super) struct UpdateForm {
    pub(super) change: CheckChange,
    pub(super) children: Option<Vec<CheckId>>,
    pub(super) sealed: bool,
    /// The change gives the push check a new token.
    pub(super) rotated: bool,
}

impl UpdateForm {
    /// Whether the input names nothing to change.
    pub(super) fn is_empty(&self) -> bool {
        let change = &self.change;
        change.parent.is_none()
            && change.name.is_none()
            && change.definition.is_none()
            && change.interval_seconds.is_none()
            && change.retries.is_none()
            && change.retry_interval_seconds.is_none()
            && change.degraded_threshold_ms.is_none()
            && change.invert.is_none()
    }
}

#[expect(
    clippy::option_option,
    reason = "CheckChange.parent separates keeping the group from leaving it"
)]
fn parent(
    input: &async_graphql::MaybeUndefined<ID>,
    errors: &mut Vec<FieldError>,
) -> Option<Option<CheckId>> {
    use async_graphql::MaybeUndefined::{Null, Undefined, Value};
    match input {
        Undefined => None,
        Null => Some(None),
        Value(parent) => keep(id("parentId", parent), errors).map(Some),
    }
}

/// The update input as domain values; every field in error is reported.
pub(super) fn update(input: UpdateShape, token: PushToken) -> Result<UpdateForm, Vec<FieldError>> {
    let mut errors = Vec::new();
    let parent = parent(&input.parent_id, &mut errors);
    let config = input
        .config
        .and_then(|shape| keep(config(shape.0, token), &mut errors));
    let raw = RawSettings {
        interval_seconds: input.interval_seconds,
        retries: input.retries,
        retry_interval_seconds: input.retry_interval_seconds,
        degraded_threshold_ms: input.degraded_threshold_ms,
    };
    let given = raw.convert(&mut errors);
    if !errors.is_empty() {
        return Err(errors);
    }
    let (definition, children, sealed) = match config {
        Some(form) => (Some(form.definition), form.children, form.sealed),
        None => (None, None, false),
    };
    let rotated = matches!(definition, Some(Definition::Push(_)));
    Ok(UpdateForm {
        change: CheckChange {
            parent,
            name: input.name,
            definition,
            interval_seconds: given.interval_seconds,
            retries: given.retries,
            retry_interval_seconds: given.retry_interval_seconds,
            degraded_threshold_ms: given.degraded_threshold_ms,
            invert: input.invert,
        },
        children,
        sealed,
        rotated,
    })
}

/// The one id an `id` argument names.
pub(super) fn check_id(argument: &ID) -> Result<CheckId, FieldError> {
    id("id", argument)
}
