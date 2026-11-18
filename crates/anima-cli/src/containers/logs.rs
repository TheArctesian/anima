//! `anima containers logs <device> <container>`: the last lines of the log of one container on
//! standard output (HOST-001). A log can hold a secret, so the hub refuses an API key (D-23)
//! and no MCP tool returns it (MCP-004).

use super::documents::{ContainerLogRead, container_log_read};
use super::list::fetch;
use crate::failure::{Failure, Hint};
use crate::hub::Hub;
use crate::invocation::Invocation;
use crate::output::Table;

#[derive(Debug, clap::Args)]
pub struct LogsArgs {
    /// The device's ID or its exact name.
    device: String,
    /// The container: its exact name, or 12 to 64 digits of its identifier.
    container: String,
    /// How many of the last lines, 1 to 1000; 200 when not given.
    #[arg(long, short = 'n', value_parser = clap::value_parser!(i64).range(1..=1000))]
    lines: Option<i64>,
    /// Print the time Docker took each line before it.
    #[arg(long, short = 't')]
    timestamps: bool,
}

impl LogsArgs {
    /// Prints the lines, oldest first (to a terminal with their control characters made
    /// harmless) or, with `--json`, the K1 `ContainerLog`.
    ///
    /// # Errors
    /// [`Failure::NotFound`] for an unknown device or container; the hub's refusal for an API
    /// key, or when the device cannot be read.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        let container_id = identifier(&hub, &self.device, &self.container).await?;
        let variables = container_log_read::Variables {
            device_id: crate::devices::get::resolve(&hub, &self.device).await?,
            container_id,
            lines: self.lines,
        };
        let log = hub.run::<ContainerLogRead>(variables).await?.container_log;
        if invocation.output.json() {
            return invocation.output.data(&log, |_| Table::record());
        }
        let mut text = String::new();
        for line in &log.lines {
            if let (true, Some(at)) = (self.timestamps, &line.at) {
                text.push_str(at);
                text.push(' ');
            }
            text.push_str(&line.text);
            text.push('\n');
        }
        invocation.output.file(&text)?;
        if log.cut {
            invocation.output.note(
                "The log passed 4 MiB: the newest lines are missing. Ask for fewer lines with \
                 --lines.",
            );
        }
        Ok(())
    }
}

/// True for what the hub accepts as a container identifier: 12 to 64 lower-case hex digits.
fn is_identifier(text: &str) -> bool {
    (12..=64).contains(&text.len())
        && text
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

/// The identifier of `container`: itself when it is one, or that of the device's one container
/// with this exact name. The name never goes to the hub's command.
async fn identifier(hub: &Hub, device: &str, container: &str) -> Result<String, Failure> {
    if is_identifier(container) {
        return Ok(container.to_owned());
    }
    let mut matches: Vec<String> = fetch(hub, device)
        .await?
        .into_iter()
        .filter(|candidate| candidate.name == container)
        .map(|found| found.id)
        .collect();
    if matches.len() == 1 {
        return Ok(matches.remove(0));
    }
    Err(Failure::NotFound {
        message: format!("The device has no container named {container}."),
        hint: Some(Hint::run(format!("anima containers list {device}"))),
    })
}

#[cfg(test)]
mod tests {
    use super::is_identifier;
    use crate::containers::list::short_id;

    #[test]
    fn host_001_only_12_to_64_lower_case_hex_digits_are_an_identifier() {
        let full = "4f5c1d2e3a4b5c6d7e8f90123456789abcdef0123456789abcdef0123456789a";
        assert!(is_identifier(full) && is_identifier(short_id(full)));
        assert_eq!(short_id(full), "4f5c1d2e3a4b");
        for name in [
            "dawarich_app",
            "4f5c1d2e3a4",
            "4F5C1D2E3A4B",
            "deadbeefcafe; id",
        ] {
            assert!(!is_identifier(name), "{name}");
        }
    }
}
