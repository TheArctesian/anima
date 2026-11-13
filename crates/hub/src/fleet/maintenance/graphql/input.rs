//! The inputs of the window mutations (K1 `MaintenanceWindowInput`, `WindowTargetInput`).
//! Each converts to the form types the pure rules check.

use async_graphql::{ID, InputObject};
use chrono::{DateTime, Utc};

use crate::{
    fleet::maintenance::{
        FieldError, TargetType, Weekday, WindowKind,
        model::{TargetForm, WindowForm},
    },
    ids::WindowId,
};

#[derive(InputObject)]
#[graphql(name = "WindowTargetInput")]
pub(super) struct WindowTargetInput {
    #[graphql(name = "type")]
    kind: TargetType,
    target: String,
}

#[derive(InputObject)]
#[graphql(name = "MaintenanceWindowInput")]
pub(super) struct MaintenanceWindowInput {
    name: String,
    kind: WindowKind,
    timezone: String,
    starts_at: Option<DateTime<Utc>>,
    ends_at: Option<DateTime<Utc>>,
    first_start_at: Option<DateTime<Utc>>,
    every_days: Option<i32>,
    weekdays: Option<Vec<Weekday>>,
    days_of_month: Option<Vec<i32>>,
    start_time: Option<String>,
    duration_minutes: Option<i32>,
    targets: Vec<WindowTargetInput>,
}

impl From<MaintenanceWindowInput> for WindowForm {
    fn from(input: MaintenanceWindowInput) -> Self {
        Self {
            name: input.name,
            kind: input.kind,
            timezone: input.timezone,
            starts_at: input.starts_at,
            ends_at: input.ends_at,
            first_start_at: input.first_start_at,
            every_days: input.every_days,
            weekdays: input.weekdays,
            days_of_month: input.days_of_month,
            start_time: input.start_time,
            duration_minutes: input.duration_minutes,
            targets: input
                .targets
                .into_iter()
                .map(|target| TargetForm {
                    kind: target.kind,
                    target: target.target,
                })
                .collect(),
        }
    }
}

/// The window an `id` argument names.
pub(super) fn window_id(id: &ID) -> Result<WindowId, FieldError> {
    id.parse()
        .map_err(|_| FieldError::new("id", "That maintenance window id is not valid."))
}
