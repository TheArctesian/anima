//! The login throttle for failures outside sign-in, such as a `joinShare` miss (CR-16 b, D-8).

use chrono::{DateTime, Utc};
use sqlx::PgPool;

use crate::access::{
    ClientAddr, Error,
    store::{self, Outcome},
    throttle,
};

/// The username a failure from outside sign-in is stored under. It fails the operator username
/// check (`^[A-Za-z0-9._-]{1,64}$`), so no operator can have it: it never matches an account and
/// `operator set` never clears it by name (D-8, D-9, CR-16). A `login` typing this text as its
/// username is stored under the same text.
const OUTSIDE_SIGN_IN: &str = "(share join)";

/// Whether the login throttle turns away an attempt from `client` at `now`, by the rule and
/// the failures `login` reads (D-8). `joinShare` asks before it checks a share secret, so a
/// share miss and a wrong password count against the same address (CR-16).
///
/// The limit is not exact under concurrent calls: asking and then calling [`record_failure`]
/// is two steps, so guesses sent in parallel can all pass before any is recorded, and far more
/// than five get through in a burst. Do not rely on it to protect a guessable secret; `login`
/// records its attempt first and has no such gap.
///
/// # Errors
///
/// `Error::Database` if the attempts cannot be read.
pub async fn throttled(db: &PgPool, client: ClientAddr, now: DateTime<Utc>) -> Result<bool, Error> {
    let failures = store::failures(db, client.0, now, None).await?;
    Ok(throttle::refused(&failures, now))
}

/// Records a failed attempt from `client` at `now` that counts toward the login throttle like a
/// wrong password (D-8, CR-16). When the address is already throttled at `now` the attempt is
/// recorded as refused, so it does not extend the lockout. Like [`throttled`], the limit is not
/// exact under concurrent calls. `operator set` does not clear an address throttled only by
/// these failures, as they name no operator; the lockout ends on its own (D-9).
///
/// # Errors
///
/// `Error::Database` if the attempt cannot be recorded.
pub async fn record_failure(
    db: &PgPool,
    client: ClientAddr,
    now: DateTime<Utc>,
) -> Result<(), Error> {
    let refused = throttled(db, client, now).await?;
    let id = store::start_attempt(db, now, OUTSIDE_SIGN_IN, client.0).await?;
    if refused {
        store::finish_attempt(db, id, Outcome::Refused).await?;
    }
    Ok(())
}
