//! The pure parts of the live path: the cache, the budget and the pace (K-E1 Section 13.1).

mod budget;
mod pace;
mod replies;

use chrono::{DateTime, TimeZone as _, Utc};
use uuid::Uuid;

use crate::ids::SourceAccountId;

fn now() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 14, 9, 0, 0).unwrap()
}

fn account() -> SourceAccountId {
    SourceAccountId::from(Uuid::now_v7())
}
