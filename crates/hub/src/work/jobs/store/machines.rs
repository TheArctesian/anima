//! What the hub knows of a machine for work, in schema `jobs`.

use chrono::{DateTime, Utc};
use daemon_core::wire::Switch;
use sqlx::PgConnection;

use super::{read_word, word};
use crate::{
    ids::DeviceId,
    work::jobs::{
        Error,
        model::{Fault, Machine},
    },
};

type MachineRow = (
    String,
    bool,
    bool,
    Option<DateTime<Utc>>,
    Option<DateTime<Utc>>,
);

/// What the hub knows of the machine, locked; `None` before its first claim.
pub(in crate::work::jobs) async fn machine_locked(
    conn: &mut PgConnection,
    device: DeviceId,
) -> Result<Option<Machine>, Error> {
    let row: Option<MachineRow> = sqlx::query_as(
        "SELECT switch, unclean, off_by_operator, off_until, first_on_at FROM jobs.machines \
         WHERE device_id = $1 FOR UPDATE",
    )
    .bind(device)
    .fetch_optional(conn)
    .await?;
    row.map(
        |(switch, unclean, off_by_operator, off_until, first_on_at)| {
            Ok(Machine {
                device,
                switch: read_word::<Switch>(&switch)?,
                unclean,
                off_by_operator,
                off_until,
                first_on_at,
            })
        },
    )
    .transpose()
}

/// Writes what the machine said in its claim at `now`.
pub(in crate::work::jobs) async fn save_machine(
    conn: &mut PgConnection,
    machine: &Machine,
    now: DateTime<Utc>,
) -> Result<(), Error> {
    sqlx::query(
        "INSERT INTO jobs.machines (device_id, switch, unclean, off_by_operator, off_until, \
         first_on_at, last_claim_at) VALUES ($1, $2, $3, $4, $5, $6, $7) \
         ON CONFLICT (device_id) DO UPDATE SET switch = $2, unclean = $3, off_by_operator = $4, \
         off_until = $5, first_on_at = $6, last_claim_at = $7",
    )
    .bind(machine.device)
    .bind(word(&machine.switch)?)
    .bind(machine.unclean)
    .bind(machine.off_by_operator)
    .bind(machine.off_until)
    .bind(machine.first_on_at)
    .bind(now)
    .execute(conn)
    .await?;
    Ok(())
}

/// Records that the machine refused or lost a job at `at`, and returns how many it refused or
/// lost since `since`. A machine with no row of its own (a device that was removed) counts 0.
pub(in crate::work::jobs) async fn add_fault(
    conn: &mut PgConnection,
    (device, fault): (DeviceId, Fault),
    at: DateTime<Utc>,
    since: DateTime<Utc>,
) -> Result<usize, Error> {
    let known: Option<(i64,)> = sqlx::query_as(
        "WITH added AS ( \
           INSERT INTO jobs.faults (device_id, at, kind) \
           SELECT $1, $2, $3 WHERE EXISTS (SELECT FROM devices.devices WHERE id = $1) \
           RETURNING 1 \
         ) SELECT (SELECT count(*) FROM added) + \
                  (SELECT count(*) FROM jobs.faults WHERE device_id = $1 AND at > $4)",
    )
    .bind(device)
    .bind(at)
    .bind(fault.as_str())
    .bind(since)
    .fetch_optional(conn)
    .await?;
    Ok(usize::try_from(known.map_or(0, |(faults,)| faults)).unwrap_or(usize::MAX))
}

/// Sets the end of the hour with no work of the machine.
pub(in crate::work::jobs) async fn set_off_until(
    conn: &mut PgConnection,
    device: DeviceId,
    until: DateTime<Utc>,
) -> Result<(), Error> {
    sqlx::query("UPDATE jobs.machines SET off_until = $2 WHERE device_id = $1")
        .bind(device)
        .bind(until)
        .execute(conn)
        .await?;
    Ok(())
}
