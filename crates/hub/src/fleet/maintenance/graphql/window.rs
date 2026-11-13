//! `MaintenanceWindow` and `WindowTarget` as the GraphQL types K1 describes.
//!
//! A field of a kind the window is not returns null (MW-004).

use std::sync::Arc;

use async_graphql::{Context, ID, Object, Result};
use chrono::{DateTime, Utc};

use crate::{
    api_errors::schema_dependency,
    clock::Clock,
    fleet::maintenance::{Target, TargetType, Weekday, Window, WindowKind, WindowRule},
};

#[Object(name = "WindowTarget")]
impl Target {
    #[graphql(name = "type")]
    async fn kind(&self) -> TargetType {
        self.target_type()
    }

    /// Device ID, check ID or tag name.
    async fn target(&self) -> String {
        self.value()
    }
}

#[Object(name = "MaintenanceWindow")]
impl Window {
    async fn id(&self) -> ID {
        ID(self.id.to_string())
    }

    async fn name(&self) -> &str {
        &self.name
    }

    async fn kind(&self) -> WindowKind {
        self.rule.kind()
    }

    async fn timezone(&self) -> &str {
        self.timezone.name()
    }

    /// Manual windows only.
    async fn active_manual(&self) -> Option<bool> {
        (self.rule == WindowRule::Manual).then_some(self.active_manual)
    }

    /// Single windows only.
    async fn starts_at(&self) -> Option<DateTime<Utc>> {
        match self.rule {
            WindowRule::Single { starts_at, .. } => Some(starts_at),
            _ => None,
        }
    }

    /// Single windows only.
    async fn ends_at(&self) -> Option<DateTime<Utc>> {
        match self.rule {
            WindowRule::Single { ends_at, .. } => Some(ends_at),
            _ => None,
        }
    }

    /// Recurring-interval windows only.
    async fn first_start_at(&self) -> Option<DateTime<Utc>> {
        match self.rule {
            WindowRule::RecurringInterval { first_start_at, .. } => Some(first_start_at),
            _ => None,
        }
    }

    /// Recurring-interval windows only.
    async fn every_days(&self) -> Option<i32> {
        match self.rule {
            WindowRule::RecurringInterval { every_days, .. } => i32::try_from(every_days).ok(),
            _ => None,
        }
    }

    /// Recurring-weekday windows only.
    async fn weekdays(&self) -> Option<Vec<Weekday>> {
        match &self.rule {
            WindowRule::RecurringWeekday { weekdays, .. } => Some(weekdays.clone()),
            _ => None,
        }
    }

    /// Recurring-day-of-month windows only: 1..31, or -1 for the last day of the month.
    async fn days_of_month(&self) -> Option<Vec<i32>> {
        match &self.rule {
            WindowRule::RecurringDayOfMonth { days_of_month, .. } => {
                Some(days_of_month.iter().map(|day| i32::from(*day)).collect())
            }
            _ => None,
        }
    }

    /// Recurring-weekday and recurring-day-of-month windows: `HH:MM` local; may end after midnight
    /// (MW-002).
    async fn start_time(&self) -> Option<String> {
        match self.rule {
            WindowRule::RecurringWeekday { start_time, .. }
            | WindowRule::RecurringDayOfMonth { start_time, .. } => {
                Some(start_time.format("%H:%M").to_string())
            }
            _ => None,
        }
    }

    /// Every recurring kind.
    async fn duration_minutes(&self) -> Option<i32> {
        self.rule
            .duration_minutes()
            .and_then(|minutes| i32::try_from(minutes).ok())
    }

    async fn targets(&self) -> &[Target] {
        &self.targets
    }

    /// Whether the window is in force now, by the schema's clock.
    #[expect(
        clippy::unused_async,
        clippy::unused_async_trait_impl,
        reason = "the Object macro requires async resolvers"
    )]
    async fn active_now(&self, ctx: &Context<'_>) -> Result<bool> {
        let clock = schema_dependency::<Arc<dyn Clock>>(ctx)?;
        Ok(self.is_active(clock.now()))
    }
}
