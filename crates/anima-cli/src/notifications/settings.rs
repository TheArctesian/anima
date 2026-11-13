//! `anima notify settings get|set`: the days before a certificate expires at which the hub
//! warns, and the message template (NOT-006, NOT-007).

use super::documents::{SettingsGet, SettingsSet, settings_get, settings_set};
use crate::failure::{Failure, Hint};
use crate::hub::Hub;
use crate::invocation::Invocation;
use crate::output::Table;

#[derive(Debug, clap::Subcommand)]
pub enum SettingsCommand {
    /// Show the warning days, the template and the placeholders it may use.
    Get,
    /// Change the warning days or the template; what you leave out stays as it is.
    Set(SetArgs),
}

#[derive(Debug, clap::Args)]
pub struct SetArgs {
    /// Days before expiry, 1 to 365, at most 5, such as 21,14,7.
    #[arg(long, value_delimiter = ',')]
    certificate_days: Vec<i64>,
    /// The template's title, with placeholders such as {device}.
    #[arg(long)]
    template_title: Option<String>,
    /// The template's body, with placeholders.
    #[arg(long)]
    template_body: Option<String>,
}

impl SettingsCommand {
    /// Runs the command.
    ///
    /// # Errors
    /// [`Failure`] for a refusal, or for `set` with nothing to change.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        match self {
            Self::Get => {
                let answer = current(&hub).await?;
                invocation
                    .output
                    .data(&answer.notification_settings, get_table)
            }
            Self::Set(args) => args.run(invocation, &hub).await,
        }
    }
}

async fn current(hub: &Hub) -> Result<settings_get::ResponseData, Failure> {
    Ok(hub.run::<SettingsGet>(settings_get::Variables).await?)
}

impl SetArgs {
    async fn run(self, invocation: &Invocation, hub: &Hub) -> Result<(), Failure> {
        if self.certificate_days.is_empty()
            && self.template_title.is_none()
            && self.template_body.is_none()
        {
            return Err(Failure::Usage {
                message:
                    "Nothing to change: pass --certificate-days, --template-title or --template-body."
                        .to_owned(),
                hint: Some(Hint::run("anima notify settings set --help")),
            });
        }
        let now = current(hub).await?.notification_settings;
        let input = settings_set::NotificationSettingsInput {
            certificate_days: if self.certificate_days.is_empty() {
                now.certificate_days
            } else {
                self.certificate_days
            },
            template: settings_set::NotificationTemplateInput {
                title: self.template_title.unwrap_or(now.template.title),
                body: self.template_body.unwrap_or(now.template.body),
            },
        };
        let answer = hub
            .run::<SettingsSet>(settings_set::Variables { input })
            .await?;
        invocation
            .output
            .data(&answer.set_notification_settings, set_table)
    }
}

fn get_table(settings: &settings_get::SettingsGetNotificationSettings) -> Table {
    record(
        &settings.certificate_days,
        (&settings.template.title, &settings.template.body),
        &settings.placeholders,
    )
}

fn set_table(settings: &settings_set::SettingsSetSetNotificationSettings) -> Table {
    record(
        &settings.certificate_days,
        (&settings.template.title, &settings.template.body),
        &settings.placeholders,
    )
}

fn record(days: &[i64], template: (&str, &str), placeholders: &[String]) -> Table {
    let mut table = Table::record();
    let days: Vec<String> = days.iter().map(ToString::to_string).collect();
    table.pair("Certificate days", days.join(", "));
    table.pair("Template title", template.0);
    table.pair("Template body", template.1);
    table.pair("Placeholders", placeholders.join(", "));
    table
}
