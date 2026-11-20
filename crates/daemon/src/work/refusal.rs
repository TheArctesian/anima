//! The report of an item that the machine refused (K10 Sections 5 and 7): the state `refused`
//! with a fixed cause. Nothing was made for the item, so nothing was deleted.

use daemon_core::wire::{Cleanup, WorkReport, WorkResult, WorkState};
use uuid::Uuid;

/// Why the machine does not start an item: the fixed causes of K10 Section 5 for a job.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Cause {
    TypeUnknown,
    KindNotPermitted,
    MemberUnknown,
    SecretUnknown,
    SwitchOff,
    Policy,
    NoIsolation,
    Unclean,
    RepositoryNotPermitted,
    RunnerTooOld,
}

impl Cause {
    /// The cause as the report spells it.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::TypeUnknown => "type_unknown",
            Self::KindNotPermitted => "kind_not_permitted",
            Self::MemberUnknown => "member_unknown",
            Self::SecretUnknown => "secret_unknown",
            Self::SwitchOff => "switch_off",
            Self::Policy => "policy",
            Self::NoIsolation => "no_isolation",
            Self::Unclean => "unclean",
            Self::RepositoryNotPermitted => "repository_not_permitted",
            Self::RunnerTooOld => "runner_too_old",
        }
    }
}

/// The report that refuses attempt `attempt` of item `id` for `cause`.
#[must_use]
pub fn report(id: Uuid, attempt: u32, cause: Cause) -> WorkReport {
    let none = || "none".to_owned();
    WorkReport {
        id,
        attempt,
        state: WorkState::Refused,
        log: None,
        result: Some(WorkResult {
            outcome: "refused".to_owned(),
            cause: Some(cause.as_str().to_owned()),
            seconds: 0,
            cleanup: Cleanup {
                isolation: none(),
                checkout: none(),
                secrets: none(),
                keychain: none(),
            },
        }),
    }
}
