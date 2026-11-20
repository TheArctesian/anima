//! What the hub records of a job (K10 Section 10): an audit entry for each job, each attempt
//! with its device, each stop and each final state, and the events of a machine. A log part is
//! cleaned here before it is stored (K10 Section 7, LOG-007).

use std::net::{IpAddr, Ipv4Addr};

use chrono::{DateTime, Utc};
use daemon_core::wire::StopReason;
use serde_json::json;
use sqlx::PgConnection;

use super::{
    Asker, Error,
    model::Job,
    reasons::{self, Decision},
};
use crate::{
    fleet::events::{
        self, ActorKind, AuditAction, AuditTarget, Bus, ClientAddr, NewAudit, NewEvent, Operator,
        Severity, Source,
    },
    ids::DeviceId,
    text,
};

/// The address of an entry that the hub writes with no request: the sweep of the leases.
pub(super) const HUB_ADDRESS: ClientAddr = ClientAddr(IpAddr::V4(Ipv4Addr::LOCALHOST));

/// The most bytes of one log line (K10 Section 7).
const MAX_LINE_BYTES: usize = 4096;

async fn audit(
    conn: &mut PgConnection,
    job: &Job,
    (action, change): (AuditAction, serde_json::Value),
    ((operator, client), now): (Asker, DateTime<Utc>),
) -> Result<(), Error> {
    let entry = NewAudit {
        operator,
        action,
        target: AuditTarget::Job(job.id),
        client,
        change,
    };
    events::audit(conn, now, entry).await?;
    Ok(())
}

fn described(job: &Job) -> serde_json::Value {
    let asked = &job.asked;
    json!({
        "type": asked.work_type,
        "kind": asked.kind,
        "project": asked.project,
        "repository": format!("{}/{}", asked.repository.owner, asked.repository.name),
        "secrets": asked.secrets,
    })
}

/// A job came into the queue.
pub(super) async fn queued(
    conn: &mut PgConnection,
    job: &Job,
    asker: Asker,
    now: DateTime<Utc>,
) -> Result<(), Error> {
    audit(
        conn,
        job,
        (AuditAction::Create, described(job)),
        (asker, now),
    )
    .await
}

/// A machine got the job: one attempt started. The entry is the record of the placement
/// decision (ROU-005): what the job asked of a machine, the reason as one line, and each
/// machine that was refused with its reason. It holds codes, numbers and names of machines;
/// no secret and no parameter of the job.
pub(super) async fn attempt_started(
    conn: &mut PgConnection,
    job: &Job,
    (device, decisions): (DeviceId, &[Decision]),
    (client, now): (ClientAddr, DateTime<Utc>),
) -> Result<(), Error> {
    let wanted = &job.asked;
    let refused: Vec<_> = decisions
        .iter()
        .filter(|decision| decision.device != device)
        .map(|decision| {
            json!({
                "device": decision.device,
                "machine": decision.machine,
                "why": decision.verdict.as_str(),
                "freeMemoryBytes": decision.memory_bytes,
                "freeDiskBytes": decision.disk_bytes,
                "at": decision.at,
            })
        })
        .collect();
    let change = json!({
        "attempt": job.attempt,
        "device": device,
        "asked": {
            "type": wanted.work_type, "kind": wanted.kind, "os": wanted.os,
            "architecture": wanted.architecture, "memoryBytes": wanted.limits.memory_bytes,
            "diskBytes": wanted.limits.disk_bytes, "machine": wanted.machine,
        },
        "reason": reasons::line(job, decisions),
        "refused": refused,
    });
    let asker = (Operator::hub(), client);
    audit(conn, job, (AuditAction::Start, change), (asker, now)).await
}

/// An attempt ended: `how` is the state that the machine reported or the fixed word of the hub.
pub(super) async fn attempt_ended(
    conn: &mut PgConnection,
    job: &Job,
    (device, how): (DeviceId, &str),
    (client, now): (ClientAddr, DateTime<Utc>),
) -> Result<(), Error> {
    let change = json!({
        "attempt": job.attempt,
        "device": device,
        "how": how,
        "state": job.state.as_str(),
        "reason": job.reason,
        "lost": job.lost,
    });
    let asker = (Operator::hub(), client);
    audit(conn, job, (AuditAction::End, change), (asker, now)).await
}

/// The operator pinned a machine for a project, removed the pin, or changed the exclusion of a
/// machine (ROU-006).
pub(super) async fn placement_set(
    conn: &mut PgConnection,
    (device, change): (DeviceId, serde_json::Value),
    ((operator, client), now): (Asker, DateTime<Utc>),
) -> Result<(), Error> {
    let entry = NewAudit {
        operator,
        action: AuditAction::Update,
        target: AuditTarget::Device(device),
        client,
        change,
    };
    events::audit(conn, now, entry).await?;
    Ok(())
}

/// A stop was asked for.
pub(super) async fn stop_asked(
    conn: &mut PgConnection,
    job: &Job,
    reason: StopReason,
    (asker, now): (Asker, DateTime<Utc>),
) -> Result<(), Error> {
    let change = json!({ "stop": reason.as_str(), "state": job.state.as_str() });
    audit(conn, job, (AuditAction::Cancel, change), (asker, now)).await
}

async fn event(
    conn: &mut PgConnection,
    bus: &Bus,
    (severity, device, message): (Severity, Option<(DeviceId, String)>, String),
    now: DateTime<Utc>,
) -> Result<(), Error> {
    let event = NewEvent {
        severity,
        source: Source::Hub,
        device,
        message,
        actor: ActorKind::Hub,
    };
    events::record(conn, bus, now, event).await?;
    Ok(())
}

/// The first claim of a machine with its switch on (USE-008).
pub(super) async fn first_claim(
    conn: &mut PgConnection,
    bus: &Bus,
    (device, name): (DeviceId, &str),
    now: DateTime<Utc>,
) -> Result<(), Error> {
    let message = format!("{name} takes work from this hub now.");
    let device = Some((device, name.to_owned()));
    event(conn, bus, (Severity::Info, device, message), now).await
}

/// The hub gives a machine no work for 1 hour (K10 Section 11).
pub(super) async fn set_off(
    conn: &mut PgConnection,
    bus: &Bus,
    device: DeviceId,
    now: DateTime<Utc>,
) -> Result<(), Error> {
    let message = format!(
        "The machine {device} refused or lost 5 jobs in 1 hour. It gets no work for 1 hour."
    );
    event(conn, bus, (Severity::Warning, None, message), now).await
}

/// A log part as the hub stores it: each line redacted again, without a control character but
/// the tab, without a character that hides or reorders text, and cut at 4,096 bytes on a
/// character boundary (K10 Section 7; BLD-049, LOG-007, SF-010). The text is data of a job,
/// never an instruction.
pub(super) fn clean_log(part: &str) -> String {
    let mut cleaned = String::with_capacity(part.len());
    for line in part.split_inclusive('\n') {
        let (body, end) = line
            .strip_suffix('\n')
            .map_or((line, ""), |body| (body, "\n"));
        let visible: String = body
            .chars()
            .filter(|&c| c == '\t' || (!c.is_control() && !text::is_invisible(c)))
            .collect();
        let redacted = redaction::redact_text(&visible);
        let mut cut = redacted.len().min(MAX_LINE_BYTES);
        while !redacted.is_char_boundary(cut) {
            cut -= 1;
        }
        cleaned.push_str(&redacted[..cut]);
        cleaned.push_str(end);
    }
    cleaned
}
