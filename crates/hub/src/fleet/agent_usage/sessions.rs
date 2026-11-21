//! The pure part of the single sessions (AIS-025, AIS-026): the tokens of a session summed
//! over its models with the estimate of their cost (AIS-021), and whether its last use is
//! recent by the clock of the hub.

use chrono::{DateTime, TimeDelta, Utc};

use super::{
    model::{Session, StoredSession, TokenSums},
    prices,
};

/// A session is recent when its last use is at most this long before the clock of the hub:
/// two count periods of the daemon, which counts every 15 minutes (AIS-026).
pub const RECENT: TimeDelta = TimeDelta::minutes(30);

/// `stored` with its tokens summed and priced: the estimate of each model with a known price,
/// and the tokens of the models with none (AIS-021). Null estimate when no token has a price.
#[must_use]
pub fn priced(stored: StoredSession) -> Session {
    let mut tokens = TokenSums::default();
    let mut estimated_cost_usd = None;
    let mut unpriced_tokens = 0.0;
    for (model, sums) in &stored.models {
        tokens = tokens.plus(*sums);
        if sums.all() > 0.0 {
            match prices::estimate(model, *sums) {
                Some(cost) => estimated_cost_usd = Some(estimated_cost_usd.unwrap_or(0.0) + cost),
                None => unpriced_tokens += sums.all(),
            }
        }
    }
    Session {
        stored,
        tokens,
        estimated_cost_usd,
        unpriced_tokens,
    }
}

/// Whether `last_use` is at most [`RECENT`] before `now` (AIS-026).
#[must_use]
pub fn is_recent(last_use: DateTime<Utc>, now: DateTime<Utc>) -> bool {
    now - last_use <= RECENT
}
