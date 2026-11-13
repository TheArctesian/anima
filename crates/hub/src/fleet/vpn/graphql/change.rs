//! One broker change with its audit entry and event, run to the end in a task of its own: a
//! client that goes away while the broker works still leaves the audit entry of the change the
//! broker made (LOG-005, R-055). The value comes back only to the resolver that asked for it.

use std::{future::Future, sync::Arc};

use async_graphql::{Context, Result};
use sqlx::PgPool;

use super::mutations::broker_error;
use crate::{
    api_errors::{commit, internal, pool, schema_dependency},
    clock::Clock,
    fleet::{
        events::{Bus, ClientAddr, Operator, Trail, TrailEntry},
        vpn::Error,
    },
};

/// Runs `call`, then writes `entry_of(&value)` in its own transaction, both in a spawned task,
/// so dropping the request does not stop either. A failed call is the K7 error on `field`; a
/// failed write is `INTERNAL`, and the broker's change stays in the broker's log.
pub(super) async fn broker_change<T: Send + 'static>(
    ctx: &Context<'_>,
    call: impl Future<Output = Result<T, Error>> + Send + 'static,
    field: Option<&'static str>,
    entry_of: impl FnOnce(&T) -> TrailEntry + Send + 'static,
) -> Result<T> {
    let pool = pool(ctx)?.clone();
    let bus = schema_dependency::<Bus>(ctx)?.clone();
    let who = (
        schema_dependency::<Operator>(ctx)?.clone(),
        *schema_dependency::<ClientAddr>(ctx)?,
    );
    let now = schema_dependency::<Arc<dyn Clock>>(ctx)?.now();
    let change = tokio::spawn(async move {
        let value = call.await?;
        let written = write(&pool, Trail::new(&bus, who, now), entry_of(&value)).await;
        Ok::<_, Error>((value, written))
    });
    let (value, written) = change
        .await
        .map_err(internal)?
        .map_err(|error| broker_error(error, field))?;
    written?;
    Ok(value)
}

/// Writes `entry` in its own transaction.
async fn write(pool: &PgPool, trail: Trail<'_>, entry: TrailEntry) -> Result<()> {
    let mut tx = pool.begin().await.map_err(internal)?;
    trail.write(&mut tx, entry).await.map_err(internal)?;
    commit(tx).await
}
