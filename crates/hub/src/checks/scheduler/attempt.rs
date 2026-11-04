//! One attempt of one due check: observe, record on the current state and publish
//! (CHK-001, CHK-002, CHK-003, CHK-006).
//!
//! Maintenance is decided when the beat is recorded, so a window that starts or ends shows
//! on the check's next beat; the status rules apply it to the stored state at once.

use chrono::{DateTime, Utc};
use sqlx::PgPool;

use super::schedule::{self, Timing};
use crate::{
    checks::{
        self, Attempt, Beat, Due, Error,
        config::CheckConfig,
        probe::{AddressPolicy, Failure, ProbeEnv, ProbeResult, ProbeTarget, probe},
    },
    credentials::Vault,
    devices::{self, Device},
    events::{Bus, HeartbeatSummary, Signal},
    ids::{CheckId, DeviceId},
    maintenance,
};

/// What probes need: the vault that opens HTTP secrets and the addresses they may reach
/// (R-030). The policy always refuses loopback, link-local and multicast addresses; the
/// deny list only refuses more (D-8).
pub struct Probing {
    pub vault: Vault,
    pub policy: AddressPolicy,
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

/// Runs the attempt. Every failure is logged here and leaves the check due, so a later tick
/// tries again.
pub(super) async fn run(ctx: &Context, due: Due, now: DateTime<Utc>) -> Finished {
    let mut finished = Finished {
        check: due.id,
        device: due.device,
        recorded: None,
    };
    let device = match devices::get(&ctx.db, due.device).await {
        Ok(device) => device,
        Err(error) => {
            warn(&due, "the device of a due check cannot be read", &error);
            return finished;
        }
    };
    let Some(result) = observe(ctx, &due, &device, now).await else {
        return finished;
    };
    let Some(covered) = covered(ctx, &due, &device, (&result, now)).await else {
        return finished;
    };
    let attempt = Attempt {
        result: &result,
        settings: &due.settings,
        covered,
        at: now,
    };
    let beat = match record(&ctx.db, due.id, &attempt).await {
        Ok(Some(beat)) => beat,
        Ok(None) => return finished,
        Err(error) => {
            warn(&due, "the heartbeat cannot be recorded", &error);
            return finished;
        }
    };
    ctx.bus.publish(Signal::Heartbeat {
        check: due.id,
        device: due.device,
        beat: HeartbeatSummary {
            at: beat.at,
            status: beat.status,
            response_time_ms: beat.response_time_ms,
        },
    });
    finished.recorded = Some(Timing {
        next_due_at: beat.state.next_due_at,
        interval_seconds: due.settings.interval_seconds,
    });
    finished
}

/// The raw result of the attempt, or `None` when a database read failed. PUSH checks are
/// not probed: being due means no push came (CHK-002).
async fn observe(
    ctx: &Context,
    due: &Due,
    device: &Device,
    now: DateTime<Utc>,
) -> Option<ProbeResult> {
    let config = match &due.config {
        None => return Some(ProbeResult::failed(&Failure::BadConfig)),
        Some(CheckConfig::Push) => return Some(schedule::missed_push()),
        Some(config) => config,
    };
    let secrets = if matches!(config, CheckConfig::Http(_)) {
        match checks::load_http_secrets(&ctx.db, &ctx.probing.vault, due.id).await {
            Ok(secrets) => secrets,
            Err(Error::Credentials(_)) => return Some(ProbeResult::failed(&Failure::Secrets)),
            Err(error) => {
                warn(due, "the check secrets cannot be read", &error);
                return None;
            }
        }
    } else {
        None
    };
    let target = ProbeTarget {
        host: device.host.clone(),
    };
    let env = ProbeEnv::new(ctx.probing.policy.clone(), now);
    Some(probe(&target, config, secrets.as_ref(), &env).await)
}

/// Whether maintenance covers the check: the device's manual flag, or an active window on
/// the device, the check, its group or a tag (CHK-006). Windows are read only for a failed
/// result, the only kind that can be Down. `None` when the windows cannot be read.
async fn covered(
    ctx: &Context,
    due: &Due,
    device: &Device,
    (result, now): (&ProbeResult, DateTime<Utc>),
) -> Option<bool> {
    let failed = result.ok == due.settings.invert;
    if device.maintenance || !failed {
        return Some(device.maintenance);
    }
    match maintenance::active_targets(&ctx.db, now).await {
        Ok(active) => Some(
            [Some(due.id), due.parent]
                .into_iter()
                .flatten()
                .any(|check| active.covers(device.id, Some(check), &device.tags)),
        ),
        Err(error) => {
            warn(due, "the maintenance windows cannot be read", &error);
            None
        }
    }
}

async fn record(db: &PgPool, check: CheckId, attempt: &Attempt<'_>) -> Result<Option<Beat>, Error> {
    let mut tx = db.begin().await?;
    let beat = checks::record_attempt(&mut tx, check, attempt).await?;
    tx.commit().await?;
    Ok(beat)
}
