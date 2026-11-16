//! The link GraphQL part: `Query.linkDevices`, `Mutation.createLinkInvite` and
//! `Mutation.revokeLinkDevice` (K1; FRN-009, LNK-003, LNK-004, OR-S6-3).
//!
//! Both mutations are for a login or browser token only, as `createApiKey` (D-23): the
//! backstop refuses an API key and a builder token before any resolver, and each resolver asks
//! again with `operator_only`. Each writes one audit entry and one event, never the invite
//! (LOG-005, LOG-007).

mod change;

use std::sync::Arc;

use async_graphql::{Context, Error as GraphQlError, Object, Result};
use chrono::{DateTime, Utc};

use super::{
    Error, LinkPath, Refusal,
    model::{ListedDevice, MadeInvite, Revoked},
    socket::Control,
    wire::Request,
};
use crate::{
    api_errors::{ErrorCode, FieldError, coded, internal, invalid, schema_dependency},
    config::Config,
    fleet::access::{Authenticated, operator_only},
    scalars::IpAddr,
    text::clean,
};

/// The longest device name the link takes (`link/devices/list.go`).
const NAME_CHARS: usize = 64;
/// Longer than any key text the link writes; a longer argument is not sent.
const KEY_CHARS: usize = 256;

/// A device the link permits (LNK-004).
pub struct LinkDevice(ListedDevice);

#[Object]
impl LinkDevice {
    /// The device's public key, as `revokeLinkDevice` takes it.
    async fn key(&self) -> &str {
        &self.0.key
    }

    /// The short form of the key that `anima-link join` prints on the device.
    async fn fingerprint(&self) -> String {
        clean(&self.0.fingerprint, NAME_CHARS)
    }

    async fn name(&self) -> String {
        clean(&self.0.name, NAME_CHARS)
    }

    /// The device's link address: the client address the hub records for it.
    async fn address(&self) -> IpAddr {
        IpAddr(self.0.address)
    }

    async fn path(&self) -> LinkPath {
        self.0.path
    }

    /// The relay's name when the path is `RELAYED`.
    async fn relay(&self) -> Option<String> {
        (self.0.relay.as_deref()).map(|relay| clean(relay, NAME_CHARS))
    }

    /// When the device's key stops being permitted; null when the link gives no time.
    async fn expires_at(&self) -> Option<DateTime<Utc>> {
        (self.0.expires > DateTime::UNIX_EPOCH).then_some(self.0.expires)
    }

    /// The streams open now.
    async fn streams(&self) -> i32 {
        saturated(self.0.streams)
    }
}

/// K1 `LinkInvite`. No `Debug` of the invite: `MadeInvite` prints none.
pub struct LinkInvite(MadeInvite);

#[Object]
impl LinkInvite {
    /// Shown once; it works once and expires (LNK-003). The hub keeps no copy.
    async fn invite(&self) -> &str {
        self.0.invite.expose()
    }

    async fn expires_at(&self) -> DateTime<Utc> {
        self.0.expires
    }
}

#[derive(Default)]
pub struct Query;

#[Object(name = "LinkQuery")]
impl Query {
    /// The devices the link permits, in the link's order.
    #[graphql(guard = "Authenticated")]
    async fn link_devices(&self, ctx: &Context<'_>) -> Result<Vec<LinkDevice>> {
        let devices: Vec<ListedDevice> = control_of(ctx)?
            .call(Request::DevicesList)
            .await
            .map_err(|error| to_graphql(&error, None))?;
        Ok(devices.into_iter().map(LinkDevice).collect())
    }
}

#[derive(Default)]
pub struct Mutation;

#[Object(name = "LinkMutation")]
impl Mutation {
    /// An invite for a new device named `name`. Login or browser token only (D-23).
    #[graphql(guard = "Authenticated")]
    async fn create_link_invite(&self, ctx: &Context<'_>, name: String) -> Result<LinkInvite> {
        operator_only(ctx, "createLinkInvite")?;
        let control = control_of(ctx)?;
        let length = name.chars().count();
        if length == 0 || length > NAME_CHARS || name.chars().any(char::is_control) {
            return Err(invalid(&[FieldError::new("name", NAME_RULE)]));
        }
        // The answer holds a secret: no cache may keep it.
        ctx.insert_http_header("cache-control", "no-store");
        let entry_name = name.clone();
        let call = async move {
            let request = Request::InvitesMake { name: &name };
            control.call::<MadeInvite>(request).await
        };
        let made = change::run(ctx, call, Some("name"), move |made| {
            change::invite_made(&entry_name, made)
        })
        .await?;
        Ok(LinkInvite(made))
    }

    /// Removes the device with this key from the link and closes its streams; returns how many
    /// it closed. Login or browser token only (D-23).
    #[graphql(guard = "Authenticated")]
    async fn revoke_link_device(&self, ctx: &Context<'_>, key: String) -> Result<i32> {
        operator_only(ctx, "revokeLinkDevice")?;
        let control = control_of(ctx)?;
        if key.is_empty() || key.chars().count() > KEY_CHARS {
            return Err(invalid(&[FieldError::new("key", KEY_RULE)]));
        }
        let devices: Vec<ListedDevice> =
            (control.call(Request::DevicesList).await).map_err(|error| to_graphql(&error, None))?;
        let device = (devices.into_iter())
            .find(|device| device.key == key)
            .ok_or_else(|| to_graphql(&Error::Refused(Refusal::DeviceUnknown), None))?;
        let call = async move {
            let request = Request::DevicesRevoke { key: &key };
            control.call::<Revoked>(request).await
        };
        let revoked = change::run(ctx, call, Some("key"), move |revoked| {
            change::device_revoked(&device, revoked)
        })
        .await?;
        Ok(saturated(revoked.streams_closed))
    }
}

const NAME_RULE: &str = "A device needs a name of 1 to 64 visible characters.";
const KEY_RULE: &str = "That is not the key of a device on the link (see linkDevices).";
const LINK_OFF: &str = "This hub has no device link (ANIMA_LINK_CONTROL is not set).";
const NO_ANSWER: &str = "The device link (anima-link) did not answer. Check that it runs.";
const EXPOSED: &str = "The hub does not use the link's control socket: another user owns it \
                       or can reach it. See the hub's log.";
const UNKNOWN: &str = "No device on the link has this key.";

/// The link's control socket; `UNAVAILABLE` when the hub has none.
fn control_of(ctx: &Context<'_>) -> Result<Control> {
    let config = schema_dependency::<Arc<Config>>(ctx)?;
    (config.link_control.clone())
        .map(Control::new)
        .ok_or_else(|| to_graphql(&Error::Off, None))
}

/// The K7 error of a failed link call. A refusal about the input is `VALIDATION` on `field`;
/// with no input field it is the hub's own mistake, so `INTERNAL`. The process log gets the
/// fault word and, for an exposed socket, the rule it breaks; never a reply (LOG-007).
fn to_graphql(error: &Error, field: Option<&'static str>) -> GraphQlError {
    match (error, field) {
        (Error::Off, _) => coded(LINK_OFF, ErrorCode::Unavailable),
        (
            Error::Exposed(_)
            | Error::NotRunning
            | Error::TimedOut
            | Error::Connection
            | Error::Unreadable,
            _,
        ) => unavailable(error),
        (Error::Refused(Refusal::DeviceUnknown), _) => coded(UNKNOWN, ErrorCode::NotFound),
        (Error::Refused(Refusal::BadName | Refusal::BadRequest), Some(field)) => {
            let rule = if field == "name" { NAME_RULE } else { KEY_RULE };
            invalid(&[FieldError::new(field, rule)])
        }
        (Error::Refused(refusal), _) => internal(Error::Refused(*refusal)),
    }
}

/// `UNAVAILABLE` for a link the hub cannot use, with the fault word in the process log and,
/// for an exposed socket, the rule it breaks.
fn unavailable(error: &Error) -> GraphQlError {
    let (rule, message) = match error {
        Error::Exposed(rule) => (*rule, EXPOSED),
        _ => ("", NO_ANSWER),
    };
    tracing::warn!(fault = error.fault(), rule, "link control call failed");
    coded(message, ErrorCode::Unavailable)
}

/// A count of the link as a GraphQL `Int`.
fn saturated(count: i64) -> i32 {
    i32::try_from(count.max(0)).unwrap_or(i32::MAX)
}
