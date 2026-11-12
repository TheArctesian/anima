//! The retention GraphQL part: `Query.retention` and `Mutation.setRetention` (K1; HUB-005,
//! D-25).
//!
//! `setRetention` writes the period, one audit entry and one event in one transaction: if any
//! step fails, nothing is stored.

mod trail;

use async_graphql::{Context, Error as GraphQlError, Object, Result, SimpleObject};

use super::{
    Error, Periods, Subject,
    bounds::{bounds, checked},
    periods, store,
};
use crate::{
    access::Authenticated,
    api_errors::{begin, commit, internal, invalid, pool},
    events::Trail,
};

/// The GraphQL error with its `extensions.code`; anything the operator cannot act on is
/// `INTERNAL` with no detail (K7, API-010).
fn to_graphql(error: Error) -> GraphQlError {
    match error {
        Error::Validation(fields) => invalid(&fields),
        other => internal(other),
    }
}

/// One subject's period and the range it may be set in.
#[derive(SimpleObject)]
#[graphql(name = "RetentionPeriod")]
struct Period {
    subject: Subject,
    days: i32,
    min_days: i32,
    max_days: i32,
    default_days: i32,
}

impl Period {
    fn new(subject: Subject, days: u16) -> Self {
        let range = bounds(subject);
        Self {
            subject,
            days: days.into(),
            min_days: range.min.into(),
            max_days: range.max.into(),
            default_days: range.default.into(),
        }
    }

    fn all(periods: Periods) -> Vec<Self> {
        Subject::ALL
            .into_iter()
            .map(|subject| Self::new(subject, periods.days(subject)))
            .collect()
    }
}

#[derive(Default)]
pub struct Query;

#[Object(name = "RetentionQuery")]
impl Query {
    /// All five subjects, in the order of `RetentionSubject`.
    #[graphql(guard = "Authenticated")]
    async fn retention(&self, ctx: &Context<'_>) -> Result<Vec<Period>> {
        let stored = periods(pool(ctx)?).await.map_err(to_graphql)?;
        Ok(Period::all(stored))
    }
}

#[derive(Default)]
pub struct Mutation;

#[Object(name = "RetentionMutation")]
impl Mutation {
    /// A value outside the subject's range is refused with `VALIDATION` on `days`.
    #[graphql(guard = "Authenticated")]
    async fn set_retention(
        &self,
        ctx: &Context<'_>,
        subject: Subject,
        days: i32,
    ) -> Result<Period> {
        let days = checked(subject, days.into()).map_err(|e| invalid(&[e]))?;
        let trail = Trail::from_context(ctx)?;
        let mut tx = begin(ctx).await?;
        let previous = store::set(&mut tx, subject, days)
            .await
            .map_err(|e| to_graphql(Error::Database(e)))?
            .ok_or_else(|| to_graphql(Error::Unreadable))?;
        trail
            .write(&mut tx, trail::changed(subject, days, previous))
            .await
            .map_err(internal)?;
        commit(tx).await?;
        Ok(Period::new(subject, days))
    }
}
