//! The reason of a placement as one line that a person reads (ROU-004, R-259), for example
//! "studio: fits, 22 GB of memory and 180 GB of disk free; laptop refused: 2 GB of memory free,
//! the job needs 8 GB". The line is data: it holds names of machines (MCP-008).

use chrono::{DateTime, Utc};

use super::{
    model::Job,
    placement::{Unfit, Verdict},
};
use crate::ids::DeviceId;

/// The last decision of the placement for one job and one machine.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Decision {
    pub device: DeviceId,
    /// The name of the machine when it asked.
    pub machine: String,
    pub verdict: Verdict,
    /// What the machine said that it has free.
    pub memory_bytes: u64,
    pub disk_bytes: u64,
    pub at: DateTime<Utc>,
}

const GB: u64 = 1 << 30;
const MB: u64 = 1 << 20;

/// A count of bytes in whole GB, or in whole MB below 1 GB.
fn size(bytes: u64) -> String {
    if bytes >= GB {
        format!("{} GB", bytes / GB)
    } else {
        format!("{} MB", bytes / MB)
    }
}

fn spelled<T: serde::Serialize>(word: &T) -> String {
    match serde_json::to_value(word) {
        Ok(serde_json::Value::String(text)) => text,
        _ => "unknown".to_owned(),
    }
}

/// Why a machine does not get the job, without its name.
fn refusal(job: &Job, decision: &Decision) -> String {
    let asked = &job.asked;
    match decision.verdict {
        Verdict::Fits => String::new(),
        Verdict::Type => format!("it takes no work of the type {}", spelled(&asked.work_type)),
        Verdict::Kind => format!("it runs no {} job", spelled(&asked.kind)),
        Verdict::Os => "it has a different operating system".to_owned(),
        Verdict::Architecture => "it has a different processor architecture".to_owned(),
        Verdict::Memory => format!(
            "{} of memory free, the job needs {}",
            size(decision.memory_bytes),
            size(asked.limits.memory_bytes)
        ),
        Verdict::Disk => format!(
            "{} of disk free, the job needs {}",
            size(decision.disk_bytes),
            size(asked.limits.disk_bytes)
        ),
        Verdict::RefusedBefore => "it refused this job before".to_owned(),
        Verdict::NamesOther => "the job names a different machine".to_owned(),
        Verdict::Excluded => "the operator excluded it from work that names no machine".to_owned(),
        Verdict::WaitsForBetter => {
            "a machine with more free memory asked a short time before".to_owned()
        }
    }
}

/// The one line for `job` from the last decision of each machine: the machines that fit first,
/// then each machine that was refused with its reason.
#[must_use]
pub fn line(job: &Job, decisions: &[Decision]) -> String {
    if decisions.is_empty() {
        return "no machine has asked for this job".to_owned();
    }
    let (fits, refused): (Vec<_>, Vec<_>) = decisions
        .iter()
        .partition(|decision| decision.verdict == Verdict::Fits);
    let fits = fits.into_iter().map(|decision| {
        format!(
            "{}: fits, {} of memory and {} of disk free",
            decision.machine,
            size(decision.memory_bytes),
            size(decision.disk_bytes)
        )
    });
    let refused = refused
        .into_iter()
        .map(|decision| format!("{} refused: {}", decision.machine, refusal(job, decision)));
    fits.chain(refused).collect::<Vec<_>>().join("; ")
}

/// Why the machine that a job names can never run it, as one sentence.
#[must_use]
pub fn unfit(job: &Job, why: Unfit) -> String {
    let asked = &job.asked;
    match why {
        Unfit::NeverAsked => "the machine never asked this hub for work".to_owned(),
        Unfit::TakesNoWork => "the machine takes no work".to_owned(),
        Unfit::Requirement(Verdict::Type) => {
            format!(
                "the machine takes no work of the type {}",
                spelled(&asked.work_type)
            )
        }
        Unfit::Requirement(Verdict::Kind) => {
            format!("the machine runs no {} job", spelled(&asked.kind))
        }
        Unfit::Requirement(Verdict::Os) => {
            "the machine has a different operating system".to_owned()
        }
        Unfit::Requirement(_) => "the machine has a different processor architecture".to_owned(),
    }
}
