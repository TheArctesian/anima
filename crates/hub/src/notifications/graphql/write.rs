//! The three channel mutations. Each checks the input, seals the secrets and writes the change,
//! one audit entry and one event in one transaction: if any step fails, nothing is stored
//! (API-003, LOG-005, NOT-002, SEC-003).

use async_graphql::{Context, ID, Result};
use chrono::{DateTime, DurationRound, TimeDelta, Utc};
use sqlx::PgConnection;
use uuid::Uuid;

use super::{form, input::ChannelShape, node::ChannelNode, to_graphql, trail};
use crate::{
    api_errors::{
        ErrorCode, FieldError, begin, coded, commit, internal, invalid, schema_dependency,
    },
    credentials::Vault,
    events::Trail,
    ids::ChannelId,
    notifications::{
        Channel, ChannelFields,
        channels::{self, Prepared, Settings, Touched},
        sealing, store,
    },
};

/// The channel `id` names, or the operator's `id` error.
fn channel_id(id: &ID) -> Result<ChannelId> {
    id.parse()
        .map_err(|_| invalid(&[FieldError::new("id", "That channel id is not valid.")]))
}

/// The time as the database keeps it: `timestamptz` holds microseconds, and the store
/// truncates to them.
fn stored_time(now: DateTime<Utc>) -> DateTime<Utc> {
    now.duration_trunc(TimeDelta::microseconds(1))
        .unwrap_or(now)
}

/// The fields to store for a prepared channel, its secrets sealed for channel `id`; and the
/// settings and touched secret fields, for the answer and the audit entry.
fn sealed(
    vault: &Vault,
    id: ChannelId,
    prepared: Prepared,
) -> Result<(ChannelFields, Settings, Touched)> {
    let secrets = prepared
        .secrets
        .as_ref()
        .map(|secrets| sealing::seal(vault, id, &prepared.settings, secrets))
        .transpose()
        .map_err(internal)?;
    let (title_template, body_template) = prepared
        .template
        .map(|template| (template.title, template.body))
        .unzip();
    let fields = ChannelFields {
        name: prepared.name,
        kind: prepared.settings.kind(),
        on_by_default: prepared.on_by_default,
        settings: prepared.settings.to_stored(),
        secrets,
        title_template,
        body_template,
    };
    Ok((fields, prepared.settings, prepared.touched))
}

/// The channel `id`, locked until the transaction ends; `NOT_FOUND` when there is none.
async fn locked(conn: &mut PgConnection, id: ChannelId) -> Result<ChannelNode> {
    let channel = store::channel_for_update(conn, id)
        .await
        .map_err(to_graphql)?
        .ok_or_else(|| coded("No such channel.", ErrorCode::NotFound))?;
    ChannelNode::try_from(channel).map_err(internal)
}

pub(super) async fn create(ctx: &Context<'_>, input: ChannelShape) -> Result<ChannelNode> {
    let prepared = channels::prepare_new(form::draft(input)).map_err(|errors| invalid(&errors))?;
    let vault = schema_dependency::<Vault>(ctx)?;
    let id = ChannelId::from(Uuid::now_v7());
    let (fields, settings, touched) = sealed(vault, id, prepared)?;
    let trail = Trail::from_context(ctx)?;
    let now = stored_time(trail.now);
    let mut tx = begin(ctx).await?;
    store::insert_channel(&mut tx, id, &fields, now)
        .await
        .map_err(to_graphql)?;
    let channel = Channel {
        id,
        fields,
        created_at: now,
        updated_at: now,
    };
    let node = ChannelNode { channel, settings };
    trail
        .write(&mut tx, trail::created(&node, &touched))
        .await
        .map_err(internal)?;
    commit(tx).await?;
    Ok(node)
}

/// The stored secrets are read under the row lock, so the rules judge the secrets the update
/// replaces (CR-14).
pub(super) async fn update(ctx: &Context<'_>, id: &ID, input: ChannelShape) -> Result<ChannelNode> {
    let id = channel_id(id)?;
    let draft = form::draft(input);
    let vault = schema_dependency::<Vault>(ctx)?;
    let trail = Trail::from_context(ctx)?;
    let now = stored_time(trail.now);
    let mut tx = begin(ctx).await?;
    let before = locked(&mut tx, id).await?;
    let stored = sealing::kept(
        vault,
        id,
        &before.settings,
        before.channel.fields.secrets.as_deref(),
    );
    let prepared = channels::prepare_change(&before.settings, stored, draft)
        .map_err(|errors| invalid(&errors))?;
    let (fields, settings, touched) = sealed(vault, id, prepared)?;
    store::update_channel(&mut tx, id, &fields, now)
        .await
        .map_err(to_graphql)?;
    let channel = Channel {
        id,
        fields,
        created_at: before.channel.created_at,
        updated_at: now,
    };
    let after = ChannelNode { channel, settings };
    trail
        .write(&mut tx, trail::updated(&before, &after, &touched))
        .await
        .map_err(internal)?;
    commit(tx).await?;
    Ok(after)
}

/// The channel's deliveries stay in the log under the name they were sent with (NOT-004).
pub(super) async fn delete(ctx: &Context<'_>, id: &ID) -> Result<ID> {
    let channel_id = channel_id(id)?;
    let trail = Trail::from_context(ctx)?;
    let mut tx = begin(ctx).await?;
    let before = locked(&mut tx, channel_id).await?;
    store::delete_channel(&mut tx, channel_id)
        .await
        .map_err(to_graphql)?;
    trail
        .write(&mut tx, trail::deleted(&before))
        .await
        .map_err(internal)?;
    commit(tx).await?;
    Ok(ID(channel_id.to_string()))
}
