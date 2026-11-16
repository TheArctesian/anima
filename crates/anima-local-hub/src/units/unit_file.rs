//! The text of the unit file, and where it goes. Pure.

use std::path::{Path, PathBuf};

use anima_database::Platform;

#[cfg(test)]
mod tests;

/// The launchd label, which is also the file name without `.plist`.
pub const LABEL: &str = "ing.anima.hub";
/// The systemd unit.
pub const SYSTEMD_UNIT: &str = "anima-hub.service";

/// Seconds the unit gives the program to stop the group in order before it kills it. More
/// than the sum of the members' stop limits (`group::LIMITS`), the link's included.
pub const STOP_SECONDS: u32 = 90;
/// Seconds between a failure and the next start.
const RESTART_SECONDS: u32 = 10;

/// What the unit runs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Launch {
    pub program: PathBuf,
    pub args: Vec<String>,
}

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum RenderError {
    #[error("a path of the hub cannot be written in a unit file: {0}")]
    Path(String),
}

/// Where the unit file of this user is.
#[must_use]
pub fn file(platform: Platform, home: &Path) -> PathBuf {
    match platform {
        Platform::MacOs => home
            .join("Library/LaunchAgents")
            .join(format!("{LABEL}.plist")),
        // As the agent: `XDG_CONFIG_HOME` is not read, so the file is where the next
        // `install` or `remove` looks for it whatever the environment of that day.
        Platform::Linux => home.join(".config/systemd/user").join(SYSTEMD_UNIT),
    }
}

/// The unit file's text for `launch`.
///
/// # Errors
/// [`RenderError::Path`] when the program or an argument is not text, or has a character
/// that would change the meaning of the file.
pub fn render(platform: Platform, launch: &Launch) -> Result<String, RenderError> {
    let program = launch
        .program
        .to_str()
        .ok_or_else(|| RenderError::Path(launch.program.display().to_string()))?;
    let words: Vec<&str> = std::iter::once(program)
        .chain(launch.args.iter().map(String::as_str))
        .collect();
    if let Some(word) = words.iter().find(|word| word.chars().any(char::is_control)) {
        return Err(RenderError::Path((*word).escape_debug().to_string()));
    }
    match platform {
        Platform::MacOs => Ok(launch_agent(&words)),
        Platform::Linux => systemd_unit(&words),
    }
}

/// Start at login; start again only after a failure; no core file.
fn launch_agent(words: &[&str]) -> String {
    let arguments = words.iter().fold(String::new(), |all, word| {
        format!("{all}        <string>{}</string>\n", xml_text(word))
    });
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<!-- The Anima hub on this computer (one-click plan S4). `anima-local-hub install` writes this file. -->
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>{LABEL}</string>
    <key>ProgramArguments</key>
    <array>
{arguments}    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <dict>
        <key>SuccessfulExit</key>
        <false/>
    </dict>
    <key>ThrottleInterval</key>
    <integer>{RESTART_SECONDS}</integer>
    <key>ExitTimeOut</key>
    <integer>{STOP_SECONDS}</integer>
    <key>SoftResourceLimits</key>
    <dict>
        <key>Core</key>
        <integer>0</integer>
    </dict>
    <key>HardResourceLimits</key>
    <dict>
        <key>Core</key>
        <integer>0</integer>
    </dict>
</dict>
</plist>
"#
    )
}

fn xml_text(word: &str) -> String {
    word.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// `KillMode=mixed`: SIGTERM goes to the program only, which stops its group in order.
fn systemd_unit(words: &[&str]) -> Result<String, RenderError> {
    let command = words
        .iter()
        .map(|word| systemd_word(word))
        .collect::<Result<Vec<_>, _>>()?
        .join(" ");
    Ok(format!(
        "# The Anima hub on this computer (one-click plan S4). `anima-local-hub install` writes this file.
[Unit]
Description=Anima hub on this computer

[Service]
Type=simple
ExecStart={command}
Restart=on-failure
RestartSec={RESTART_SECONDS}
TimeoutStopSec={STOP_SECONDS}
KillMode=mixed
NoNewPrivileges=yes
LimitCORE=0

[Install]
WantedBy=default.target
"
    ))
}

/// One word of `ExecStart` in double quotes. systemd reads `%` and `$` as the start of a
/// specifier and of a variable; doubled, each is itself.
fn systemd_word(word: &str) -> Result<String, RenderError> {
    if word.contains(['"', '\\', '\'']) {
        return Err(RenderError::Path(word.to_owned()));
    }
    Ok(format!(
        "\"{}\"",
        word.replace('%', "%%").replace('$', "$$")
    ))
}
