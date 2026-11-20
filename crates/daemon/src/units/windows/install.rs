//! Validate the enrolled daemon before registering its native login task (WIN2, MET-019).
use super::{plan::Target, step::Step};
use crate::{
    config::{home::Home, private_file},
    readers::Platform,
    token_store::TokenStore,
};
use chrono::{DateTime, Utc};
use std::{
    io,
    path::{Path, PathBuf},
};

pub struct Host<'a> {
    pub platform: Platform,
    pub program: &'a Path,
    pub home: &'a Home,
    pub now: DateTime<Utc>,
}

#[derive(Debug, thiserror::Error)]
pub enum InstallError {
    #[error("Windows supports per-user installation only")]
    System,
    #[error("the daemon must be enrolled with a current token in its default Windows folder")]
    NotReady,
    #[error("the daemon executable or its folders are not trusted")]
    Exposed,
    #[error("task registration failed: {0}")]
    Io(#[from] io::Error),
    #[error("the daemon task could not be registered or started")]
    Task,
    #[error("the daemon's private task file: {0}")]
    File(#[from] private_file::PrivateFileError),
}

/// Install and start a per-user login task only after its binary, state and token pass checks.
/// # Errors
/// Refuses an exposed binary, another home, an expired enrolment, or a failed task command.
pub fn install(
    target: Target,
    host: &Host<'_>,
    env: impl Fn(&str) -> Option<String>,
    mut run: impl FnMut(&Step) -> io::Result<bool>,
) -> Result<PathBuf, InstallError> {
    if target != Target::User || host.platform != Platform::Windows {
        return Err(InstallError::System);
    }
    let default = Home::locate(
        |key| {
            if key == "ANIMA_DAEMON_HOME" {
                None
            } else {
                env(key)
            }
        },
        false,
    )
    .map_err(|_| InstallError::NotReady)?;
    if default.dir() != host.home.dir() {
        return Err(InstallError::NotReady);
    }
    let credential = TokenStore::open(host.home)
        .load()
        .map_err(|_| InstallError::NotReady)?;
    if credential.is_none_or(|credential| credential.expires_at <= host.now)
        || host
            .home
            .read_config()
            .map_err(|_| InstallError::NotReady)?
            .is_none()
    {
        return Err(InstallError::NotReady);
    }
    trusted(host.program)?;
    let sid = anima_windows::process::current_user()?;
    let name = format!("Anima.ing-Daemon-{sid}");
    let text = render(host.program, &sid)?;
    let encoded: Vec<u8> = std::iter::once(0xfeff_u16)
        .chain(text.encode_utf16())
        .flat_map(u16::to_le_bytes)
        .collect();
    private_file::write(host.home.dir(), "task.xml", &encoded)?;
    let file = host.home.dir().join("task.xml");
    let file_text = file.to_str().ok_or(InstallError::Exposed)?;
    let mut stop = Step::new("schtasks.exe", &["/End", "/TN", &name]);
    stop.may_fail = true;
    for step in [
        stop,
        Step::new(
            "schtasks.exe",
            &["/Create", "/TN", &name, "/XML", file_text, "/F"],
        ),
        Step::new("schtasks.exe", &["/Run", "/TN", &name]),
    ] {
        let result = run(&step)?;
        if !result && !step.may_fail {
            return Err(InstallError::Task);
        }
    }
    Ok(file)
}

fn trusted(program: &Path) -> Result<(), InstallError> {
    let _file = anima_windows::files::read(program, false)?;
    for path in program.ancestors().skip(1) {
        if !anima_windows::files::secure(path, false)? {
            return Err(InstallError::Exposed);
        }
    }
    Ok(())
}

fn render(program: &Path, sid: &str) -> Result<String, InstallError> {
    let program = program
        .to_str()
        .filter(|value| {
            !value
                .chars()
                .any(|c| c.is_control() || c == '"' || c == '%')
        })
        .ok_or(InstallError::Exposed)?;
    let program = program
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;");
    Ok(format!(
        r#"<?xml version="1.0"?>
<Task version="1.2" xmlns="http://schemas.microsoft.com/windows/2004/02/mit/task">
<Triggers><LogonTrigger><Enabled>true</Enabled><UserId>{sid}</UserId></LogonTrigger></Triggers>
<Principals><Principal id="User"><UserId>{sid}</UserId><LogonType>InteractiveToken</LogonType><RunLevel>LeastPrivilege</RunLevel></Principal></Principals>
<Settings><MultipleInstancesPolicy>IgnoreNew</MultipleInstancesPolicy><DisallowStartIfOnBatteries>false</DisallowStartIfOnBatteries><StopIfGoingOnBatteries>false</StopIfGoingOnBatteries><StartWhenAvailable>true</StartWhenAvailable><ExecutionTimeLimit>PT0S</ExecutionTimeLimit><RestartOnFailure><Interval>PT1M</Interval><Count>3</Count></RestartOnFailure></Settings>
<Actions Context="User"><Exec><Command>{program}</Command><Arguments>run</Arguments></Exec></Actions>
</Task>"#
    ))
}
