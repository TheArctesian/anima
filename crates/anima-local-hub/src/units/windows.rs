//! Per-user login tasks and graceful stop requests through the hub's private socket.
use super::unit_file::RenderError;
use crate::Folders;
use sha2::{Digest, Sha256};
use std::{
    io,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
use tokio::io::AsyncWriteExt;

pub(super) fn task_name(root: &Path) -> String {
    use std::fmt::Write;
    let mut name = String::from("Anima.ing-Hub-");
    for byte in Sha256::digest(root.as_os_str().as_encoded_bytes()) {
        let _ = write!(name, "{byte:02x}");
    }
    name
}

pub(super) fn task_program() -> io::Result<PathBuf> {
    let root = std::env::var_os("SystemRoot").ok_or(io::ErrorKind::NotFound)?;
    let program = Path::new(&root).join("System32/schtasks.exe");
    anima_windows::files::read(&program, false)?;
    anima_windows::files::directory(program.parent().ok_or(io::ErrorKind::InvalidInput)?)?;
    Ok(program)
}

pub(super) fn render(words: &[&str]) -> Result<String, RenderError> {
    let sid = anima_windows::process::current_user().map_err(|_| RenderError::Identity)?;
    if let Some(word) = words.iter().find(|word| word.contains('%')) {
        // Task Scheduler expands environment references even inside quoted arguments.
        return Err(RenderError::Path((*word).to_owned()));
    }
    let program = xml(words[0]);
    let arguments = xml(&words[1..]
        .iter()
        .map(|word| quote(word))
        .collect::<Vec<_>>()
        .join(" "));
    Ok(format!(
        r#"<?xml version="1.0"?>
<Task version="1.2" xmlns="http://schemas.microsoft.com/windows/2004/02/mit/task">
<Triggers><LogonTrigger><Enabled>true</Enabled><UserId>{sid}</UserId></LogonTrigger></Triggers>
<Principals><Principal id="User"><UserId>{sid}</UserId><LogonType>InteractiveToken</LogonType><RunLevel>LeastPrivilege</RunLevel></Principal></Principals>
<Settings><MultipleInstancesPolicy>IgnoreNew</MultipleInstancesPolicy><DisallowStartIfOnBatteries>false</DisallowStartIfOnBatteries><StopIfGoingOnBatteries>false</StopIfGoingOnBatteries><StartWhenAvailable>true</StartWhenAvailable><ExecutionTimeLimit>PT0S</ExecutionTimeLimit><RestartOnFailure><Interval>PT1M</Interval><Count>3</Count></RestartOnFailure></Settings>
<Actions Context="User"><Exec><Command>{program}</Command><Arguments>{arguments}</Arguments></Exec></Actions>
</Task>"#
    ))
}

fn xml(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn quote(text: &str) -> String {
    let mut result = String::from("\"");
    let mut slashes = 0;
    for character in text.chars() {
        if character == '\\' {
            slashes += 1;
            continue;
        }
        result.extend(std::iter::repeat_n(
            '\\',
            if character == '"' {
                slashes * 2 + 1
            } else {
                slashes
            },
        ));
        result.push(character);
        slashes = 0;
    }
    result.extend(std::iter::repeat_n('\\', slashes * 2));
    result.push('"');
    result
}

pub(super) fn stop(folders: &Folders, limit: Duration) -> io::Result<()> {
    if !crate::run::is_running(folders) {
        return Ok(());
    }
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let deadline = Instant::now() + limit;
    let path = folders.root().join("stop.sock");
    let mut asked = false;
    while crate::run::is_running(folders) {
        if Instant::now() >= deadline {
            return Err(io::ErrorKind::TimedOut.into());
        }
        if !asked {
            let result = runtime.block_on(async {
                tokio::time::timeout(Duration::from_secs(2), async {
                    let mut socket = anima_windows::socket::Stream::connect(&path).await?;
                    socket.write_all(b"stop\n").await
                })
                .await
                .map_err(|_| io::Error::from(io::ErrorKind::TimedOut))?
            });
            match result {
                Ok(()) => asked = true,
                Err(error)
                    if matches!(
                        error.kind(),
                        io::ErrorKind::NotFound | io::ErrorKind::ConnectionRefused
                    ) => {}
                Err(error) => return Err(error),
            }
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_native_task_tool_is_trusted_and_the_task_uses_the_current_user_without_a_password() {
        assert!(super::task_program().unwrap().is_file());
        let sid = anima_windows::process::current_user().unwrap();
        let text = super::render(&[
            r"C:\Users\Ada\Anima & hub.exe",
            "run",
            "--root",
            r"C:\Users\Ada\Hub root",
        ])
        .unwrap();
        assert!(text.contains(&format!("<UserId>{sid}</UserId>")));
        assert!(text.contains("<LogonType>InteractiveToken</LogonType>"));
        assert!(text.contains("<RunLevel>LeastPrivilege</RunLevel>"));
        assert!(text.contains("Anima &amp; hub.exe"));
        assert!(super::render(&[r"C:\%PATH%\hub.exe"]).is_err());
    }
}
