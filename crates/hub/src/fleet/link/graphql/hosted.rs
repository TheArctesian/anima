//! The two calls of a hosted hub (hosted-mode.md HMD-037, R-129): `hostedDeviceInvite`, which
//! only the Anima.ing cloud can make, and `claimDeviceToken`, which only the device that joined
//! with that invite can make. Together they let a device join and sign in with no invite typed
//! and no password: the account is the only way in. The cloud never sees the device's token.
//!
//! Neither call writes an invite, a claim, a token or the cloud key to a log or an entry
//! (LOG-007).

use std::{path::Path, sync::Arc};

use async_graphql::{Context, Object, Result, SimpleObject};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq as _;

use super::{
    super::claims::{CLAIM_BYTES, ClaimHash, Claims, OpenClaim},
    ListedDevice, MadeInvite, NAME_CHARS, NAME_RULE, Request, control_of, to_graphql,
};
use crate::{
    api_errors::{ErrorCode, FieldError, coded, internal, invalid, pool, schema_dependency},
    clock::Clock,
    config::{Config, LINK_DEVICES},
    fleet::{
        access::{self, AuthPayload},
        events::{self, ActorKind, Bus, ClientAddr, NewEvent, Severity, Source},
    },
};

const NOT_HOSTED: &str = "This hub is not hosted by Anima.ing.";
const WRONG_KEY: &str = "The cloud key is wrong.";
/// How long the claim of a device that is already on the link works: as long as an invite.
const CLAIM_LIFE: chrono::TimeDelta = chrono::TimeDelta::minutes(15);
const WRONG_CLAIM: &str =
    "The claim is used, expired or wrong, or this device did not join with its invite.";

/// K1 `HostedDeviceInvite`: shown once to the cloud, which passes it to the device. No `Debug`.
#[derive(SimpleObject)]
pub struct HostedDeviceInvite {
    /// The link invite: works once and expires (LNK-003). Null when a device with this name is
    /// already on the link: the claim then gives that device a new token.
    invite: Option<String>,
    /// The one-time claim that the device that joined with `invite` trades for its token.
    claim: String,
    expires_at: DateTime<Utc>,
}

#[derive(Default)]
pub struct HostedMutation;

#[Object]
impl HostedMutation {
    /// An invite and a claim for the device named `name`, for the Anima.ing cloud only: it
    /// sends the key of `ANIMA_HOSTED_CLOUD_KEY_FILE`. A device already on the link gets a claim
    /// only. A new claim for a name ends the open claim of that name. Needs no token (R-129).
    async fn hosted_device_invite(
        &self,
        ctx: &Context<'_>,
        cloud_key: String,
        name: String,
    ) -> Result<HostedDeviceInvite> {
        check_cloud_key(&cloud_key_file(ctx)?, &cloud_key).await?;
        let length = name.chars().count();
        if length == 0 || length > NAME_CHARS || name.chars().any(char::is_control) {
            return Err(invalid(&[FieldError::new("name", NAME_RULE)]));
        }
        let (control, claims) = (control_of(ctx)?, schema_dependency::<Claims>(ctx)?);
        let now = schema_dependency::<Arc<dyn Clock>>(ctx)?.now();
        let devices: Vec<ListedDevice> =
            (control.call(Request::DevicesList).await).map_err(|error| to_graphql(&error, None))?;
        ctx.insert_http_header("cache-control", "no-store");
        let (invite, expires, message) = if devices.iter().any(|device| device.name == name) {
            let message = format!("The Anima.ing cloud asked a new token for the device {name}.");
            (None, now + CLAIM_LIFE, message)
        } else {
            let made: MadeInvite = (control.call(Request::InvitesMake { name: &name }).await)
                .map_err(|error| to_graphql(&error, Some("name")))?;
            let message = format!("The Anima.ing cloud made a link invite for the device {name}.");
            (Some(made.invite.expose().to_owned()), made.expires, message)
        };
        let claim =
            URL_SAFE_NO_PAD.encode(access::random_bytes::<CLAIM_BYTES>().map_err(internal)?);
        let open = OpenClaim {
            device_name: name,
            expires,
        };
        claims.open(ClaimHash::of(&claim), open, now);
        let event = NewEvent {
            severity: Severity::Info,
            source: Source::Access,
            device: None,
            message,
            actor: ActorKind::Hub,
        };
        let mut conn = pool(ctx)?.acquire().await.map_err(internal)?;
        let bus = schema_dependency::<Bus>(ctx)?;
        events::record(&mut conn, bus, now, event)
            .await
            .map_err(internal)?;
        Ok(HostedDeviceInvite {
            invite,
            claim,
            expires_at: expires,
        })
    }

    /// The token of the device that sends this, when it joined the link with the invite of
    /// `claim`. The claim works once. Needs no token (R-129).
    async fn claim_device_token(&self, ctx: &Context<'_>, claim: String) -> Result<AuthPayload> {
        cloud_key_file(ctx)?;
        let client = *schema_dependency::<ClientAddr>(ctx)?;
        let refused = || coded(WRONG_CLAIM, ErrorCode::Unauthenticated);
        let from_link = matches!(client.0.to_canonical(), std::net::IpAddr::V6(address) if LINK_DEVICES.contains(&address));
        if !from_link {
            return Err(refused());
        }
        let now = schema_dependency::<Arc<dyn Clock>>(ctx)?.now();
        let devices: Vec<ListedDevice> = (control_of(ctx)?.call(Request::DevicesList).await)
            .map_err(|error| to_graphql(&error, None))?;
        let sender = devices
            .into_iter()
            .find(|device| device.address == client.0);
        // The claim is spent before the device is compared, so a wrong sender cannot try it twice.
        let open = schema_dependency::<Claims>(ctx)?
            .use_claim(ClaimHash::of(&claim), now)
            .ok_or_else(refused)?;
        let sender = sender
            .filter(|device| device.name == open.device_name)
            .ok_or_else(refused)?;
        let (db, bus) = (pool(ctx)?, schema_dependency::<Bus>(ctx)?);
        ctx.insert_http_header("cache-control", "no-store");
        let device = access::HostedDevice {
            client,
            name: &sender.name,
            at: now,
        };
        access::sign_in_hosted(db, bus, device).await
    }
}

/// The cloud key file of this hub; `FORBIDDEN` on a hub that is not hosted.
fn cloud_key_file(ctx: &Context<'_>) -> Result<std::path::PathBuf> {
    (schema_dependency::<Arc<Config>>(ctx)?
        .hosted_cloud_key_file
        .clone())
    .ok_or_else(|| coded(NOT_HOSTED, ErrorCode::Forbidden))
}

/// Compares the hashes of the sent key and of the file's key in constant time. A file that
/// cannot be read is the hub's fault; an empty key never matches.
async fn check_cloud_key(file: &Path, sent: &str) -> Result<()> {
    let kept = tokio::fs::read_to_string(file).await.map_err(internal)?;
    let kept = kept.trim();
    let matches = !kept.is_empty()
        && bool::from(Sha256::digest(kept.as_bytes()).ct_eq(&Sha256::digest(sent.as_bytes())));
    if matches {
        Ok(())
    } else {
        Err(coded(WRONG_KEY, ErrorCode::Unauthenticated))
    }
}
