//! The end of the flow: the installed App's key goes to the one storing path, `establish`
//! (K-E1 Section 10a).
//!
//! The store is taken under the lock (`Finishing`), so the `/installed` callback and the poll
//! cannot both store. It runs in a task of its own, so a browser that leaves does not stop it,
//! and a [`Settle`] sends the attempt back to `APP_CREATED` if the store is dropped or panics.
//! The audit entry is made for the operator and the address captured at the start. An
//! unreachable mint or a database error goes back to `APP_CREATED` for the next poll; a refused
//! key ends the attempt with `KEY_REFUSED`.

use uuid::Uuid;

use super::{
    attempt::{Failure, Finished},
    attempts::{Attempts, Settle},
};
use crate::{
    engine::sources::{
        Error, Scheduling,
        connect::{Connection, Established, establish},
    },
    fleet::events::Trail,
};

/// Stores attempt `id` with installation `installation`, when it waits for Install and no
/// other caller stores it. `None` when it did not take the store or its task was lost;
/// otherwise how it went. The store goes on when the caller is dropped.
pub async fn finish(
    scheduling: &Scheduling,
    attempts: &Attempts,
    (id, installation): (Uuid, u64),
) -> Option<Finished> {
    let task = tokio::spawn(store(
        scheduling.clone(),
        attempts.clone(),
        (id, installation),
    ));
    task.await.ok().flatten()
}

/// The store itself, in its own task.
async fn store(
    scheduling: Scheduling,
    attempts: Attempts,
    (id, installation): (Uuid, u64),
) -> Option<Finished> {
    let now = scheduling.clock.now();
    let finish = attempts
        .step(Some(id), now, |held| held.begin_finish(now))
        .ok()?;
    let round = finish.round;
    // Dropped or panicked before the outcome is known: back to `APP_CREATED`.
    let mut settle = Settle::new(attempts, id, scheduling.clock.clone(), move |held, now| {
        held.finished(round, Finished::Again, now);
    });
    let trail = Trail::new(&scheduling.bus, (finish.by.operator, finish.by.client), now);
    let connection = Connection {
        label: finish.label,
        app_id: finish.app_id.to_string(),
        installation_id: installation.to_string(),
        private_key: finish.key,
    };
    let stored = establish(&scheduling, trail, connection).await;
    if let Err(error) = &stored {
        // Fixed words only: the error holds no key, reply or token (LOG-007).
        tracing::warn!(%error, "github connection could not be stored");
    }
    let outcome = outcome(&stored);
    settle.instead(move |held, now| held.finished(round, outcome, now));
    // The attempt is held in `Finishing` until the settle runs: nothing can replace it meanwhile.
    drop(settle);
    Some(outcome)
}

/// What the store's result means for the attempt: a refused key ends it, a fault of the hub or
/// an unreachable source waits for the next poll.
fn outcome(stored: &Result<Established, Error>) -> Finished {
    match stored {
        Ok(established) => Finished::Connected {
            account: established.id,
            consumer_on: established.consumer_on,
        },
        Err(Error::KeyRefused) => Finished::Failed(Failure::KeyRefused),
        Err(
            Error::Unreachable
            | Error::Database(_)
            | Error::Credentials(_)
            | Error::Extensions(_)
            | Error::Intake(_),
        ) => Finished::Again,
        Err(_) => Finished::Failed(Failure::ReplyUnusable),
    }
}
