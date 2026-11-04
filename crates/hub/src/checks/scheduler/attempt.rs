//! One attempt of one due check: observe, evaluate, record the beat and publish it
//! (CHK-001, CHK-002, CHK-003, CHK-006).

use chrono::{DateTime, Utc};
use sqlx::PgPool;

use super::{Probing, schedule};
use crate::{
    checks::{
        self, Beat, Due, Error,
        config::CheckConfig,
        evaluate,
        probe::{Failure, ProbeEnv, ProbeResult, ProbeTarget, probe},
        status::maintained,
    },
    devices::{self, Device},
    events::{Bus, HeartbeatSummary, Signal},
    ids::{CheckId, DeviceId},
    maintenance,
    status::Status,
};

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
    /// The device name when a beat was recorded; `None` when nothing was written.
    pub(super) recorded: Option<String>,
}

/// Logs a failed read or write at this I/O boundary; the error text holds no SQL, URL,
/// header or secret (LOG-007).
pub(super) fn warn(due: &Due, what: &str, error: &dyn std::fmt::Display) {
    tracing::warn!(
        domain = "checks",
        check_id = %due.id,
        device_id = %due.device,
        %error,
        "{what}"
    );
}

/// Runs the attempt. Every failure is logged here and leaves the check due, so the next tick
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
    let Some(beat) = judge(ctx, &due, &device, now).await else {
        return finished;
    };
    if let Err(error) = record(&ctx.db, due.id, &beat).await {
        warn(&due, "the heartbeat cannot be recorded", &error);
        return finished;
    }
    ctx.bus.publish(Signal::Heartbeat {
        check: due.id,
        device: due.device,
        beat: HeartbeatSummary {
            at: beat.at,
            status: beat.status,
            response_time_ms: beat.response_time_ms,
        },
    });
    finished.recorded = Some(device.name);
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
            Err(Error::Credentials(_)) => {
                return Some(schedule::failed("the check secrets cannot be opened"));
            }
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

/// The beat for the result. A Down beat is recorded as Maintenance when the device's manual
/// flag or an active window covers the check, so uptime counts maintenance (CHK-006); the
/// stored state stays Down and the status rules show it as Maintenance while covered.
/// `None` when the windows cannot be read.
async fn judge(ctx: &Context, due: &Due, device: &Device, now: DateTime<Utc>) -> Option<Beat> {
    let result = observe(ctx, due, device, now).await?;
    let (state, status) = evaluate(&due.state, result.clone(), &due.settings, now);
    let covered = if status == Status::Down && !device.maintenance {
        let active =
            maintenance::active_for(&ctx.db, device.id, Some(due.id), &device.tags, now).await;
        match active {
            Ok(active) => active,
            Err(error) => {
                warn(due, "the maintenance windows cannot be read", &error);
                return None;
            }
        }
    } else {
        device.maintenance
    };
    Some(Beat::of(result, (state, maintained(status, covered)), now))
}

async fn record(db: &PgPool, check: CheckId, beat: &Beat) -> Result<(), Error> {
    let mut tx = db.begin().await?;
    checks::record_beat(&mut tx, check, beat).await?;
    tx.commit().await?;
    Ok(())
}
