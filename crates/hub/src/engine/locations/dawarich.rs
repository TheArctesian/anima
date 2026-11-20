//! The intake of the Dawarich pull (K-E4 Section 11; K-E1 Section 3; R-NEW-dawarichkey): each point of a
//! page is one location sample of the provider `dawarich`, stored through the one storing path.
//! The intake is of the engine, so its consumer is the entry that is always on; the operator
//! stops the pull with the pause of the account.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::DateTime;
use provider_dawarich::{DECLARATION, Point, begin, next_request, normalise};
use serde_json::Value;
use source_exchange::Request;
use sqlx::PgConnection;

use super::{Error, NewSample, Origin, Position, Provider, Seal, store, store_samples};
use crate::{
    engine::{
        extensions::{INTEGRATIONS, Key},
        sources::{Counts, Intake, IntakeError, Page, Taken},
    },
    ids::SourceAccountId,
};

struct DawarichIntake {
    seal: Seal,
}

/// The intake that the wiring gives to the source framework.
#[must_use]
pub fn dawarich_intake(seal: Seal) -> Arc<dyn Intake> {
    Arc::new(DawarichIntake { seal })
}

fn sample(point: Point) -> NewSample {
    NewSample {
        key: point.id,
        observed_at: point
            .timestamp
            .and_then(|seconds| DateTime::from_timestamp(seconds, 0)),
        position: Position {
            latitude: point.latitude,
            longitude: point.longitude,
            accuracy_m: point.accuracy_m,
            altitude_m: point.altitude_m,
            speed_mps: point.speed_mps,
        },
    }
}

fn count(samples: u64) -> u32 {
    u32::try_from(samples).unwrap_or(u32::MAX)
}

#[async_trait]
impl Intake for DawarichIntake {
    fn source(&self) -> &'static str {
        DECLARATION.key
    }

    fn extension(&self) -> Key {
        INTEGRATIONS
    }

    fn slice_key(&self) -> &'static str {
        "dawarich.locations"
    }

    fn begin(&self, slice: &Value) -> Value {
        begin(slice)
    }

    fn next_request(&self, slice: &Value) -> Option<Request> {
        next_request(slice)
    }

    async fn take(&self, conn: &mut PgConnection, page: Page<'_>) -> Result<Taken, IntakeError> {
        let read = normalise(page.request, &page.response, page.slice);
        let origin = Origin {
            provider: Provider::Dawarich,
            account: page.account.to_string(),
        };
        let samples: Vec<NewSample> = read.records.into_iter().map(sample).collect();
        let stored = store_samples(conn, &self.seal, (&origin, &samples), page.now)
            .await
            .map_err(|error| match error {
                Error::Database(error) => IntakeError::Database(error),
                Error::Invalid(_) => IntakeError::Refused("a point of Dawarich is not valid"),
                _ => IntakeError::Refused("the positions cannot be sealed"),
            })?;
        Ok(Taken {
            slice: read.state,
            counts: Counts {
                inserted: count(stored.added),
                unchanged: count(stored.known),
                ..Counts::default()
            },
            problems: read.problems,
        })
    }

    async fn forget(
        &self,
        conn: &mut PgConnection,
        account: SourceAccountId,
    ) -> Result<u64, IntakeError> {
        let origin = Origin {
            provider: Provider::Dawarich,
            account: account.to_string(),
        };
        Ok(store::delete_samples(conn, &origin).await?)
    }
}
