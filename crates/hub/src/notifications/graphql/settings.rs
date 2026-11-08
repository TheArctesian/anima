//! `Query.notificationSettings` and `Mutation.setNotificationSettings`: the certificate
//! thresholds and the default template (NOT-006, NOT-007, D-21, D-22). The mutation writes the
//! row, one audit entry and one event in one transaction (API-003, LOG-005).
//!
//! Thresholds: whole days from 1 to 365, at most 5 once duplicates are removed. They may be
//! sent in any order; the hub keeps them largest first, as the database check and K1 say.

use async_graphql::{Context, InputObject, Object, Result};
use serde_json::json;

use super::{input::TemplateShape, node::TemplateView, to_graphql};
use crate::{
    api_errors::{FieldError, begin, commit, internal, invalid, pool},
    events::{AuditAction, AuditTarget, Severity, Source, Trail, TrailEntry},
    notifications::{InstallationSettings, store, template},
};

const DAYS_FIELD: &str = "certificateDays";
const DAYS_MAX: i32 = 365;
const DAYS_COUNT_MAX: usize = 5;

#[Object(name = "NotificationSettings")]
impl InstallationSettings {
    async fn certificate_days(&self) -> Vec<i32> {
        self.certificate_days
            .iter()
            .map(|d| i32::from(*d))
            .collect()
    }

    async fn template(&self) -> TemplateView {
        TemplateView {
            title: self.title_template.clone(),
            body: self.body_template.clone(),
        }
    }

    /// The placeholders with their braces, as an operator types them.
    async fn placeholders(&self) -> Vec<String> {
        template::placeholder_texts()
    }
}

#[derive(InputObject)]
#[graphql(name = "NotificationSettingsInput")]
pub(super) struct SettingsInput {
    certificate_days: Vec<i32>,
    template: TemplateShape,
}

/// The thresholds sorted largest first without duplicates.
fn thresholds(days: &[i32]) -> Result<Vec<u16>, FieldError> {
    let mut kept = Vec::new();
    for day in days {
        match u16::try_from(*day) {
            Ok(day) if (1..=DAYS_MAX).contains(&i32::from(day)) => kept.push(day),
            _ => {
                return Err(FieldError::new(
                    DAYS_FIELD,
                    format!("Use whole days from 1 to {DAYS_MAX}."),
                ));
            }
        }
    }
    kept.sort_unstable_by(|a, b| b.cmp(a));
    kept.dedup();
    if kept.is_empty() || kept.len() > DAYS_COUNT_MAX {
        return Err(FieldError::new(
            DAYS_FIELD,
            format!("Give 1 to {DAYS_COUNT_MAX} different days."),
        ));
    }
    Ok(kept)
}

impl SettingsInput {
    /// The settings to store, or every field to fix.
    fn parse(&self) -> Result<InstallationSettings, Vec<FieldError>> {
        let TemplateShape { title, body } = &self.template;
        let mut errors = template::field_errors(title, body);
        let days = thresholds(&self.certificate_days);
        match days {
            Ok(certificate_days) if errors.is_empty() => Ok(InstallationSettings {
                certificate_days,
                title_template: title.clone(),
                body_template: body.clone(),
            }),
            Ok(_) => Err(errors),
            Err(error) => {
                errors.insert(0, error);
                Err(errors)
            }
        }
    }
}

pub(super) async fn read(ctx: &Context<'_>) -> Result<InstallationSettings> {
    store::installation_settings(pool(ctx)?)
        .await
        .map_err(to_graphql)
}

pub(super) async fn set(ctx: &Context<'_>, input: &SettingsInput) -> Result<InstallationSettings> {
    let wanted = input.parse().map_err(|errors| invalid(&errors))?;
    let trail = Trail::from_context(ctx)?;
    let mut tx = begin(ctx).await?;
    store::write_installation_settings(&mut tx, &wanted)
        .await
        .map_err(to_graphql)?;
    let entry = TrailEntry {
        source: Source::Notifications,
        severity: Severity::Info,
        device: None,
        message: "The notification settings were changed.".to_owned(),
        action: AuditAction::Update,
        target: AuditTarget::Setting("notifications.settings"),
        change: json!({
            "certificateDays": wanted.certificate_days,
            "template": { "title": wanted.title_template, "body": wanted.body_template },
        }),
    };
    trail.write(&mut tx, entry).await.map_err(internal)?;
    commit(tx).await?;
    Ok(wanted)
}
