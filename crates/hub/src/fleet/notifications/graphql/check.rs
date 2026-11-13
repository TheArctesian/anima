//! `Mutation.setCheckNotifications` and the `CheckNotifications` type (NOT-003, NOT-005, D-22).
//! The mutation replaces the check's channels and resend setting, writes one audit entry on the
//! check and one event, all in one transaction (API-003, LOG-005). The `Check.notifications`
//! field that returns this type lives with `checks`.

use async_graphql::{Context, ID, InputObject, Object, Result};
use serde_json::json;

use super::{node::ChannelNode, readable, to_graphql};
use crate::{
    api_errors::{ErrorCode, FieldError, begin, coded, commit, internal, invalid, pool},
    fleet::{
        events::{AuditAction, AuditTarget, Severity, Source, Trail, TrailEntry},
        notifications::{CheckNotifications, CheckSettings, Error, for_checks, store},
    },
    ids::{ChannelId, CheckId},
};

/// The most Down beats between two notifications (K2 `check_settings_resend_check`).
const RESEND_MAX: i32 = 1000;

#[Object(name = "CheckNotifications")]
impl CheckNotifications {
    /// The channels the check notifies, by name. A channel whose stored settings cannot be
    /// read is left out, as in `notificationChannels`.
    async fn channels(&self) -> Vec<ChannelNode> {
        self.channels.iter().cloned().filter_map(readable).collect()
    }

    /// Send again after each N Down heartbeats; 0: never (NOT-005).
    async fn resend_every_beats(&self) -> i32 {
        i32::from(self.resend_every_beats)
    }
}

#[derive(InputObject)]
#[graphql(name = "CheckNotificationsInput")]
pub(super) struct CheckInput {
    channel_ids: Vec<ID>,
    #[graphql(default_with = "Some(0)")]
    resend_every_beats: Option<i32>,
}

impl CheckInput {
    /// The channels without repeats, and the settings; or every field to fix.
    fn parse(&self) -> Result<(Vec<ChannelId>, CheckSettings), Vec<FieldError>> {
        let mut errors = Vec::new();
        let mut channels: Vec<ChannelId> = Vec::new();
        for id in &self.channel_ids {
            match id.parse::<ChannelId>() {
                Ok(id) if !channels.contains(&id) => channels.push(id),
                Ok(_) => {}
                Err(_) => {
                    errors.push(FieldError::new(
                        "channelIds",
                        "One of those channel ids is not valid.",
                    ));
                    break;
                }
            }
        }
        let resend = self.resend_every_beats.unwrap_or(0);
        let beats = u16::try_from(resend)
            .ok()
            .filter(|beats| i32::from(*beats) <= RESEND_MAX);
        if beats.is_none() {
            errors.push(FieldError::new(
                "resendEveryBeats",
                format!("Use 0 (never) or a number up to {RESEND_MAX}."),
            ));
        }
        match beats {
            Some(resend_every_beats) if errors.is_empty() => {
                Ok((channels, CheckSettings { resend_every_beats }))
            }
            _ => Err(errors),
        }
    }
}

/// An unknown check is `NOT_FOUND`; an unknown channel in the input is the operator's to fix.
fn refusal(error: Error) -> async_graphql::Error {
    match error {
        Error::UnknownCheck => coded("No such check.", ErrorCode::NotFound),
        Error::UnknownChannel => {
            invalid(&[FieldError::new("channelIds", "Choose channels that exist.")])
        }
        other => to_graphql(other),
    }
}

pub(super) async fn set(
    ctx: &Context<'_>,
    check_id: &ID,
    input: &CheckInput,
) -> Result<CheckNotifications> {
    let check: CheckId = check_id
        .parse()
        .map_err(|_| invalid(&[FieldError::new("checkId", "That check id is not valid.")]))?;
    let (channels, settings) = input.parse().map_err(|errors| invalid(&errors))?;
    let trail = Trail::from_context(ctx)?;
    let mut tx = begin(ctx).await?;
    store::set_check_channels(&mut tx, check, &channels)
        .await
        .map_err(refusal)?;
    store::write_check_settings(&mut tx, check, settings)
        .await
        .map_err(refusal)?;
    let entry = TrailEntry {
        source: Source::Notifications,
        severity: Severity::Info,
        device: None,
        message: "The notifications of a check were changed.".to_owned(),
        action: AuditAction::Update,
        target: AuditTarget::Check(check),
        change: json!({
            "channelIds": channels.iter().map(ToString::to_string).collect::<Vec<_>>(),
            "resendEveryBeats": settings.resend_every_beats,
        }),
    };
    trail.write(&mut tx, entry).await.map_err(internal)?;
    commit(tx).await?;
    for_checks(pool(ctx)?, &[check])
        .await
        .map_err(to_graphql)?
        .remove(&check)
        .ok_or_else(|| internal(Error::UnknownCheck))
}
