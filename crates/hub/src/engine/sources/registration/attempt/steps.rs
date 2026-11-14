//! What the steps of an attempt take and give: who started it and what they asked for, the
//! refusals, and the hand-over to the store at the end.

use zeroize::Zeroizing;

use crate::{
    fleet::events::{ClientAddr, Operator},
    ids::SourceAccountId,
};

use super::Failure;

/// Why a step was refused; the attempt is unchanged. Fixed words (LOG-007).
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum Refusal {
    #[error("no attempt with this id is open")]
    NotFound,
    #[error("the attempt is not in a phase that allows this")]
    Conflict,
    #[error("an App is made and waits for Install")]
    AppWaiting,
    #[error("the link was already used or has ended")]
    Ticket,
    #[error("this browser is not the one bound to the attempt")]
    Unbound,
    #[error("the state does not match")]
    State,
}

/// Who started the attempt and from where, captured at the start for every audit entry and
/// for the store at the end.
#[derive(Clone)]
pub struct Starter {
    pub operator: Operator,
    pub client: ClientAddr,
}

/// What the operator asked for at the start.
pub struct Request {
    pub label: Option<String>,
    pub organisation: Option<String>,
    /// The App's name, `anima-` and ten characters.
    pub name: String,
    pub by: Starter,
}

/// What a hit on `/created` may do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Exchange {
    /// The state matched and is now spent: convert the code.
    Fresh,
    /// The App is already made and the spent state came back: convert nothing, send the
    /// browser to Install again with the same state.
    Made,
    /// The App is already made and the spent state did not come back (a cross-site hit, say):
    /// convert nothing, change nothing, show the waiting page.
    Waiting,
}

/// What the store at the end needs: taken under the lock with the stage `Finishing`.
pub struct Finish {
    pub label: String,
    pub app_id: u64,
    pub key: Zeroizing<String>,
    pub by: Starter,
    /// Which store this is: its outcome counts for this round only.
    pub round: u32,
}

/// How the store at the end went.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Finished {
    Connected {
        account: SourceAccountId,
        consumer_on: bool,
    },
    /// Unreachable or a database error: back to `AppCreated` for the next poll.
    Again,
    Failed(Failure),
}
