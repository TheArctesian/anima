//! The tool inventory of each device: replaced whole when a daemon batch brings one
//! (CAP-020, CAP-029).

use chrono::{DateTime, Utc};
use daemon_core::inventory::{Inventory, Reason, Tool};
use sqlx::{PgConnection, PgPool};

use super::{record::device_missing, rows::InventoryRow};
use crate::{
    fleet::metrics::{Error, capabilities::Capabilities},
    ids::DeviceId,
};

/// Every stored inventory, read in one statement so its tools and targets belong to it. The
/// tools of a name this version does not know are left out.
pub(in crate::fleet::metrics) async fn inventories(
    db: &PgPool,
) -> Result<Vec<Capabilities>, Error> {
    let rows: Vec<InventoryRow> = sqlx::query_as(
        "SELECT i.device_id, i.architecture, i.os, i.os_version, i.found_at, \
           COALESCE((SELECT array_agg(t.name ORDER BY t.name COLLATE \"C\") \
                     FROM metrics.tools t WHERE t.device_id = i.device_id), '{}') AS tool_names, \
           COALESCE((SELECT array_agg(t.version ORDER BY t.name COLLATE \"C\") \
                     FROM metrics.tools t WHERE t.device_id = i.device_id), '{}') AS tool_versions, \
           COALESCE((SELECT array_agg(t.reason ORDER BY t.name COLLATE \"C\") \
                     FROM metrics.tools t WHERE t.device_id = i.device_id), '{}') AS tool_reasons, \
           COALESCE((SELECT array_agg(b.target ORDER BY b.target COLLATE \"C\") \
                     FROM metrics.build_targets b WHERE b.device_id = i.device_id), '{}') \
             AS build_targets \
         FROM metrics.inventories i",
    )
    .fetch_all(db)
    .await?;
    rows.into_iter().map(Capabilities::try_from).collect()
}

/// Replaces the stored inventory of `device` with `inventory`, found at `found_at`, in the
/// caller's transaction: three inserts whatever the number of tools and targets.
pub(in crate::fleet::metrics) async fn write_inventory(
    conn: &mut PgConnection,
    device: DeviceId,
    inventory: &Inventory,
    found_at: DateTime<Utc>,
) -> Result<(), Error> {
    sqlx::query("DELETE FROM metrics.inventories WHERE device_id = $1")
        .bind(device)
        .execute(&mut *conn)
        .await?;
    sqlx::query(
        "INSERT INTO metrics.inventories (device_id, architecture, os, os_version, found_at) \
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(device)
    .bind(inventory.architecture.as_str())
    .bind(inventory.os.as_str())
    .bind(&inventory.os_version)
    .bind(found_at)
    .execute(&mut *conn)
    .await
    .map_err(device_missing)?;
    let names: Vec<&str> = inventory
        .tools
        .iter()
        .map(|tool| tool.name.as_str())
        .collect();
    let versions: Vec<Option<&str>> = inventory.tools.iter().map(Tool::version).collect();
    let reasons: Vec<Option<&str>> = inventory
        .tools
        .iter()
        .map(|tool| tool.reason().map(Reason::as_str))
        .collect();
    sqlx::query(
        "INSERT INTO metrics.tools (device_id, name, version, reason) \
         SELECT $1, name, version, reason \
         FROM UNNEST($2::text[], $3::text[], $4::text[]) AS t (name, version, reason)",
    )
    .bind(device)
    .bind(&names)
    .bind(&versions)
    .bind(&reasons)
    .execute(&mut *conn)
    .await?;
    sqlx::query(
        "INSERT INTO metrics.build_targets (device_id, target) \
         SELECT $1, target FROM UNNEST($2::text[]) AS b (target)",
    )
    .bind(device)
    .bind(&inventory.build_targets)
    .execute(&mut *conn)
    .await?;
    Ok(())
}
