//! The only SQL of the tables `agent_usage.salt`, `agent_usage.accounts` and
//! `agent_usage.account_limits` (migration 1021).

use chrono::{DateTime, Utc};
use daemon_core::agent_usage::Source;
use daemon_core::agent_usage::account::{Account, Limit};
use sqlx::{PgConnection, PgPool};

use super::Reported;
use crate::fleet::agent_usage::Error;
use crate::ids::DeviceId;

/// Replaces the report of `device` for the source of `account`: its key, plan and time, then
/// its limit windows.
pub(super) async fn write(
    conn: &mut PgConnection,
    device: DeviceId,
    account: &Account,
    now: DateTime<Utc>,
) -> Result<(), Error> {
    let source = account.source.as_str();
    sqlx::query(
        "INSERT INTO agent_usage.accounts (device_id, source, key, plan, reported_at) \
         VALUES ($1, $2, $3, $4, $5) \
         ON CONFLICT (device_id, source) DO UPDATE SET key = excluded.key, \
         plan = excluded.plan, reported_at = excluded.reported_at",
    )
    .bind(device)
    .bind(source)
    .bind(&account.key)
    .bind(account.plan.as_deref())
    .bind(now)
    .execute(&mut *conn)
    .await?;
    sqlx::query("DELETE FROM agent_usage.account_limits WHERE device_id = $1 AND source = $2")
        .bind(device)
        .bind(source)
        .execute(&mut *conn)
        .await?;
    let windows: Vec<&str> = account
        .limits
        .iter()
        .map(|one| one.window.as_str())
        .collect();
    let used: Vec<Option<i16>> = account
        .limits
        .iter()
        .map(|one| one.used_percent.map(i16::from))
        .collect();
    let resets: Vec<Option<DateTime<Utc>>> =
        account.limits.iter().map(|one| one.resets_at).collect();
    let reached: Vec<bool> = account.limits.iter().map(|one| one.reached).collect();
    sqlx::query(
        "INSERT INTO agent_usage.account_limits \
         (device_id, source, window_name, used_percent, resets_at, reached) \
         SELECT $1, $2, * FROM unnest($3::text[], $4::smallint[], $5::timestamptz[], $6::boolean[])",
    )
    .bind(device)
    .bind(source)
    .bind(&windows)
    .bind(&used)
    .bind(&resets)
    .bind(&reached)
    .execute(&mut *conn)
    .await?;
    Ok(())
}

/// The salt of the hub: `made` when none is stored yet, else the stored one.
pub(super) async fn salt(db: &PgPool, made: &[u8; 32]) -> Result<[u8; 32], Error> {
    sqlx::query("INSERT INTO agent_usage.salt (salt) VALUES ($1) ON CONFLICT (one) DO NOTHING")
        .bind(made.as_slice())
        .execute(db)
        .await?;
    let stored: Vec<u8> = sqlx::query_scalar("SELECT salt FROM agent_usage.salt")
        .fetch_one(db)
        .await?;
    stored.try_into().map_err(|_| Error::Unreadable)
}

type AccountRow = (DeviceId, String, String, Option<String>, DateTime<Utc>);
type LimitRow = (
    DeviceId,
    String,
    String,
    Option<i16>,
    Option<DateTime<Utc>>,
    bool,
);

/// Every report with its limit windows, in no set order.
pub(super) async fn read(db: &PgPool) -> Result<Vec<Reported>, Error> {
    let accounts: Vec<AccountRow> = sqlx::query_as(
        "SELECT device_id, source, key, plan, reported_at FROM agent_usage.accounts",
    )
    .fetch_all(db)
    .await?;
    let limits: Vec<LimitRow> = sqlx::query_as(
        "SELECT device_id, source, window_name, used_percent, resets_at, reached \
         FROM agent_usage.account_limits ORDER BY window_name COLLATE \"C\"",
    )
    .fetch_all(db)
    .await?;
    accounts
        .into_iter()
        .map(|(device, source, key, plan, reported_at)| {
            let limits = limits
                .iter()
                .filter(|row| row.0 == device && row.1 == source)
                .map(|(_, _, window, used, resets_at, reached)| Limit {
                    window: window.clone(),
                    used_percent: used.and_then(|used| u8::try_from(used).ok()),
                    resets_at: *resets_at,
                    reached: *reached,
                })
                .collect();
            Ok(Reported {
                device,
                account: Account {
                    source: Source::parse(&source).ok_or(Error::Unreadable)?,
                    key,
                    plan,
                    limits,
                },
                reported_at,
            })
        })
        .collect()
}
