//! One attempt of one due check: observe, record on the current state and publish
//! (CHK-001, CHK-002, CHK-003, CHK-006).
//!
//! Maintenance is decided when the beat is recorded, so a window that starts or ends shows
//! on the check's next beat; the status rules apply it to the stored state at once.

use chrono::{DateTime, TimeDelta, Utc};
use sqlx::PgPool;

use super::{
    repair,
    schedule::{self, Timing},
};
use crate::{
    checks::{
        self, Attempt, Beat, Due, Error,
        config::CheckConfig,
        probe::{Failure, ProbeEnv, ProbeResult, ProbeTarget, Reach, observe_listing, probe},
        store,
    },
    credentials::Vault,
    devices::{self, Device},
    events::{Bus, HeartbeatSummary, Signal},
    ids::{CheckId, DeviceId},
    maintenance,
    status::Status,
    targets::AddressPolicy,
};

/// What probes need: the vault that opens HTTP secrets and the addresses they may reach.
/// The policy always refuses loopback, link-local and multicast addresses; the
/// deny list only refuses more.
pub struct Probing {
    pub vault: Vault,
    pub policy: AddressPolicy,
    /// `host:port` of the Tailscale `LocalAPI` proxy; `None` when none is configured (D-4).
    pub localapi: Option<String>,
}

/// What the attempts share.
pub(super) struct Context {
    pub(super) db: PgPool,
    pub(super) bus: Bus,
    pub(super) probing: Probing,
}

/// What an attempt left behind, for the scheduler's bookkeeping.
#[derive(Debug)]
pub(super) struct Finished {
    pub(super) check: CheckId,
    pub(super) device: DeviceId,
    /// The check's new timing when a beat was written; `None` when nothing was.
    pub(super) recorded: Option<Timing>,
}

/// Logs a failed read or write about one check at this I/O boundary; the error text holds
/// no SQL, URL, header or secret (LOG-007).
pub(super) fn warn(due: &Due, what: &str, error: &dyn std::fmt::Display) {
    tracing::warn!(
        domain = "checks",
        check_id = %due.id,
        device_id = %due.device,
        %error,
        "{what}"
    );
}

/// What became of an attempt.
enum Outcome {
    /// The beat, and whether maintenance covered the check when it was recorded.
    Written(Beat, bool),
    /// Nothing to record: the check is paused, has a newer beat or was deleted.
    Skipped,
    /// A read or write failed; it was logged.
    Failed,
}

/// Runs the attempt. After a failed read or write the check is tried again only after its
/// retry interval, so a write that keeps failing neither probes on every tick nor starves
/// the other checks (CHK-001). A Down beat of an ICMP check may repair its method within
/// this attempt, so the permit covers the repair's connects (CHK-013).
pub(super) async fn run(ctx: &Context, due: Due, now: DateTime<Utc>) -> Finished {
    let mut finished = Finished {
        check: due.id,
        device: due.device,
        recorded: None,
    };
    match attempt(ctx, &due, now).await {
        Outcome::Written(beat, in_maintenance) => {
            ctx.bus.publish(Signal::Heartbeat {
                check: due.id,
                device: due.device,
                beat: HeartbeatSummary {
                    at: beat.at,
                    status: beat.status,
                    response_time_ms: beat.response_time_ms,
                    message: beat.message.clone(),
                },
            });
            finished.recorded = Some(Timing {
                next_due_at: beat.state.next_due_at,
                interval_seconds: due.settings.interval_seconds,
            });
            let icmp = matches!(due.config, Some(CheckConfig::Icmp { .. }));
            if icmp && beat.state.status == Status::Down {
                repair::run(ctx, &due, in_maintenance, now).await;
            }
        }
        Outcome::Skipped => {}
        Outcome::Failed => back_off(ctx, &due, now).await,
    }
    finished
}

async fn attempt(ctx: &Context, due: &Due, now: DateTime<Utc>) -> Outcome {
    let device = match devices::get(&ctx.db, due.device).await {
        Ok(device) => device,
        Err(error) => {
            warn(due, "the device of a due check cannot be read", &error);
            return Outcome::Failed;
        }
    };
    let Some(result) = observe(ctx, due, &device, now).await else {
        return Outcome::Failed;
    };
    // Read for every result, because the settings that decide Down are read only when the
    // beat is written.
    let in_maintenance = match in_maintenance(&ctx.db, &device, (due.id, due.parent), now).await {
        Ok(covered) => covered,
        Err(error) => {
            warn(due, "the maintenance windows cannot be read", &error);
            return Outcome::Failed;
        }
    };
    let attempt = Attempt {
        result: &result,
        in_maintenance,
        at: now,
    };
    match record(&ctx.db, due.id, &attempt).await {
        Ok(Some(beat)) => Outcome::Written(beat, in_maintenance),
        Ok(None) => Outcome::Skipped,
        Err(Error::NotFound(_)) => {
            tracing::debug!(domain = "checks", check_id = %due.id, "the check was deleted");
            Outcome::Skipped
        }
        Err(error) => {
            warn(due, "the heartbeat cannot be recorded", &error);
            Outcome::Failed
        }
    }
}

/// Moves the due time past the retry interval, unless a beat moved it meanwhile.
async fn back_off(ctx: &Context, due: &Due, now: DateTime<Utc>) {
    let retry = TimeDelta::seconds(i64::from(due.settings.retry_interval_seconds));
    let moved = store::set_next_due(&ctx.db, due.id, due.state.next_due_at, now + retry).await;
    if let Err(error) = moved {
        warn(due, "the retry time cannot be stored", &error);
    }
}

/// The raw result of the attempt, or `None` when a database read failed. PUSH checks are
/// not probed: being due means no push came (CHK-002).
async fn observe(
    ctx: &Context,
    due: &Due,
    device: &Device,
    now: DateTime<Utc>,
) -> Option<ProbeResult> {
    let target = ProbeTarget {
        host: device.host.clone(),
    };
    let env =
        ProbeEnv::new(ctx.probing.policy.clone(), now).with_localapi(ctx.probing.localapi.clone());
    match &due.config {
        None => Some(ProbeResult::failed(&Failure::BadConfig)),
        Some(CheckConfig::Push) => Some(schedule::missed_push()),
        Some(CheckConfig::Http(_)) => observe_http(ctx, due, &target, &env).await,
        Some(config @ (CheckConfig::Docker { .. } | CheckConfig::Systemd { .. })) => {
            let reach = Reach {
                db: &ctx.db,
                vault: &ctx.probing.vault,
                policy: &ctx.probing.policy,
            };
            Some(observe_listing(&reach, device, config, now).await)
        }
        Some(config) => Some(probe(&target, config, None, &env).await),
    }
}

/// Probes the HTTP configuration stored now, read with its secrets, never the one read at
/// tick time: the secrets go only where they were given for (SEC-005). Secrets that do not
/// open, or no longer match the configuration, fail the probe with a fixed message and send
/// nothing (CHK-001). `None` when the read failed.
async fn observe_http(
    ctx: &Context,
    due: &Due,
    target: &ProbeTarget,
    env: &ProbeEnv,
) -> Option<ProbeResult> {
    let setup = match checks::load_http_secrets(&ctx.db, &ctx.probing.vault, due.id).await {
        Ok(Some(setup)) => setup,
        Ok(None) | Err(Error::Unreadable(_)) => {
            return Some(ProbeResult::failed(&Failure::BadConfig));
        }
        Err(Error::Credentials(_)) => return Some(ProbeResult::failed(&Failure::Secrets)),
        Err(Error::SecretsUnbound) => {
            return Some(ProbeResult::failed(&Failure::SecretsUnbound));
        }
        Err(error) => {
            warn(due, "the check secrets cannot be read", &error);
            return None;
        }
    };
    let config = CheckConfig::Http(setup.config);
    Some(probe(target, &config, setup.secrets.as_ref(), env).await)
}

/// Whether maintenance covers the check: the device's manual flag, or an active window on
/// the device, the check, its group or a tag (CHK-006). The scheduler and the push route
/// both ask it, so every beat is recorded under the same cover. The pair is the check and
/// its group.
///
/// # Errors
/// `Database` when the windows cannot be read.
pub(in crate::checks) async fn in_maintenance(
    db: &PgPool,
    device: &Device,
    (check, parent): (CheckId, Option<CheckId>),
    now: DateTime<Utc>,
) -> Result<bool, maintenance::Error> {
    if device.maintenance {
        return Ok(true);
    }
    let active = maintenance::active_targets(db, now).await?;
    Ok([Some(check), parent]
        .into_iter()
        .flatten()
        .any(|check| active.covers(device.id, Some(check), &device.tags)))
}

async fn record(db: &PgPool, check: CheckId, attempt: &Attempt<'_>) -> Result<Option<Beat>, Error> {
    let mut tx = db.begin().await?;
    let beat = checks::record_attempt(&mut tx, check, attempt).await?;
    tx.commit().await?;
    Ok(beat)
}
