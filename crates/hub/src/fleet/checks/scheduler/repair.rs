//! An ICMP check that never answered repairs its method once: after a Down beat, the hub
//! tries a TCP connect to each enabled connection port of the device and moves the check to
//! the first that answers, or warns that none does (CHK-013, R-064).
//!
//! The connects run under the scheduler's target rule (R-032) and before any row is locked;
//! the write re-checks the locked row, so an operator's edit that landed meanwhile wins.

mod rule;
#[cfg(test)]
mod tests;

use std::net::{IpAddr, Ipv4Addr};

use chrono::{DateTime, Utc};

use super::{
    attempt::{Context, warn},
    log::Fault,
};
use crate::fleet::{
    checks::{
        Due,
        probe::{ProbeEnv, ProbeTarget, probe},
        store::repairs::{self, Seen},
    },
    devices::{self, Device},
    events::{
        self, ActorKind, AuditAction, AuditTarget, ClientAddr, NewAudit, NewEvent, Operator,
        Severity, Source,
    },
};

use rule::Port;

/// Repairs the check if it is a candidate after a Down beat at `now`. A failed read or write
/// is logged and leaves the check untried, so a later Down beat tries again.
pub(super) async fn run(ctx: &Context, due: &Due, in_maintenance: bool, now: DateTime<Utc>) {
    // An inverted check that is Down got replies, so its method works; no read is needed.
    if due.settings.invert || in_maintenance {
        return;
    }
    let seen = match repairs::facts(&ctx.db, due.id).await {
        Ok(Some(seen)) => seen,
        Ok(None) => return,
        Err(error) => return warn(due, "the repair facts cannot be read", &error),
    };
    if !rule::is_candidate(&seen.facts, in_maintenance) {
        return;
    }
    let device = match devices::get(&ctx.db, due.device).await {
        Ok(device) => device,
        Err(error) => return warn(due, "the device of a repair cannot be read", &error),
    };
    let candidate = Candidate {
        due,
        seen: &seen,
        device: &device.name,
    };
    let written = match answering_port(ctx, &device, now).await {
        Some(port) => switch(ctx, &candidate, port, now).await,
        None => give_up(ctx, &candidate, now).await,
    };
    if let Err(error) = written {
        warn(due, "the repair cannot be written", &error);
    }
}

/// The first of the device's enabled ports that accepts a TCP connect at its host.
async fn answering_port(ctx: &Context, device: &Device, now: DateTime<Utc>) -> Option<Port> {
    let target = ProbeTarget {
        host: device.host.clone(),
    };
    let env = ProbeEnv::new(ctx.probing.policy.clone(), now);
    for port in rule::ports(device) {
        if probe(&target, &port.config(), None, &env).await.ok {
            return Some(port);
        }
    }
    None
}

/// The hub as the actor of its own repairs (D-4).
fn hub() -> (Operator, ClientAddr) {
    let operator = Operator::hub();
    (operator, ClientAddr(IpAddr::V4(Ipv4Addr::LOCALHOST)))
}

/// A check the rule chose, as it was read, on its device.
struct Candidate<'a> {
    due: &'a Due,
    seen: &'a Seen,
    device: &'a str,
}

impl Candidate<'_> {
    /// The check event of the repair, with the device name at this time.
    fn event(&self, severity: Severity, message: String) -> NewEvent {
        NewEvent {
            severity,
            source: Source::Check,
            device: Some((self.due.device, self.device.to_owned())),
            message,
            actor: ActorKind::Hub,
        }
    }
}

/// Moves the check to the port in one transaction with its audit entry and Info event;
/// returns whether the locked check was still the one decided on.
async fn switch(
    ctx: &Context,
    candidate: &Candidate<'_>,
    port: Port,
    now: DateTime<Utc>,
) -> Result<bool, Fault> {
    let Candidate { due, seen, device } = *candidate;
    let Some(before) = seen.facts.config.as_ref() else {
        return Ok(false);
    };
    let mut tx = ctx.db.begin().await?;
    // The device was read before the connect and is not checked again here: the next TCP
    // beat probes the device's host and port as they are then.
    if !repairs::lock_unchanged(&mut tx, due.id, seen).await? {
        return Ok(false);
    }
    let taken = repairs::names_beside(&mut tx, due.device, due.id).await?;
    let name = rule::repaired_name(port, &seen.name, &taken);
    let config = port.config();
    repairs::switch_method(&mut tx, due.id, (&name, &config), now).await?;
    let (operator, client) = hub();
    let audit = NewAudit {
        operator,
        action: AuditAction::Update,
        target: AuditTarget::Check(due.id),
        client,
        change: rule::audit_change((&seen.name, before), (&name, &config)),
    };
    events::audit(&mut tx, now, audit).await?;
    let event = candidate.event(Severity::Info, rule::repaired_message(device, port));
    events::record(&mut tx, &ctx.bus, now, event).await?;
    tx.commit().await?;
    Ok(true)
}

/// Marks the check as tried in one transaction with one Warning event; returns whether the
/// locked check was still the one decided on.
async fn give_up(
    ctx: &Context,
    candidate: &Candidate<'_>,
    now: DateTime<Utc>,
) -> Result<bool, Fault> {
    let Candidate { due, seen, device } = *candidate;
    let mut tx = ctx.db.begin().await?;
    if !repairs::lock_unchanged(&mut tx, due.id, seen).await? {
        return Ok(false);
    }
    repairs::mark_tried(&mut tx, due.id, now).await?;
    let event = candidate.event(Severity::Warning, rule::silent_message(device));
    events::record(&mut tx, &ctx.bus, now, event).await?;
    tx.commit().await?;
    Ok(true)
}
