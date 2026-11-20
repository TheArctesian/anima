//! Windows loopback credentials go only to the live child or the protected persistent
//! link executable running as this user (WIN2, F-10). Other local servers are refused.
#[path = "origin.rs"]
mod origin;
use crate::{
    link::DeviceLink,
    os_program::{self, AfterLimit, Limit, Stdout},
};
use std::{ffi::OsStr, sync::Arc, time::Duration};
use tauri::{Runtime, State, Webview};

fn listener_pid(text: &str, port: u16) -> Option<u32> {
    let address = format!("127.0.0.1:{port}");
    let mut found = None;
    for line in text.lines() {
        let fields: Vec<_> = line.split_whitespace().collect();
        if fields.len() != 5 || fields[0] != "TCP" || fields[3] != "LISTENING" {
            continue;
        }
        let relevant = fields[1] == address
            || fields[1] == format!("0.0.0.0:{port}")
            || fields[1] == format!("[::]:{port}");
        if relevant {
            let pid = fields[4].parse::<u32>().ok().filter(|pid| *pid != 0)?;
            if found.is_some_and(|before| before != pid) {
                return None;
            }
            found = Some(pid);
        }
    }
    found
}

#[cfg(test)]
fn listens(text: &str, port: u16, pid: u32) -> bool {
    listener_pid(text, port) == Some(pid)
}

/// Fail closed unless the socket belongs to this user's verified device link.
#[tauri::command]
pub async fn hub_listener_check<R: Runtime>(
    webview: Webview<R>,
    device: State<'_, Arc<DeviceLink>>,
    origin: String,
) -> Result<&'static str, &'static str> {
    crate::web_view::caller(webview.label()).map_err(|_| "caller")?;
    let port = origin::port(&origin).ok_or("bad-origin")?;
    let device = Arc::clone(&device);
    if !device.state().serves(port) {
        return Ok("other");
    }
    let pid = device.child_pid();
    let own = tauri::async_runtime::spawn_blocking(move || {
        let Some(root) = std::env::var_os("SystemRoot") else {
            return false;
        };
        let program = std::path::PathBuf::from(root).join("System32/netstat.exe");
        let Some(program) = program.to_str() else {
            return false;
        };
        let printed = os_program::run(
            program,
            &[OsStr::new("-ano")],
            Limit {
                patience: Duration::from_secs(3),
                after: AfterLimit::Kill,
            },
            Stdout::Capture {
                max_bytes: 1024 * 1024,
            },
        );
        printed.is_ok_and(|output| {
            listener_pid(&String::from_utf8_lossy(&output), port).is_some_and(|listener| {
                if pid == 0 {
                    crate::link::owns_background(listener)
                } else {
                    listener == pid
                }
            })
        }) && device.child_pid() == pid
            && device.state().serves(port)
    })
    .await
    .unwrap_or(false);
    Ok(if own { "own" } else { "other" })
}

#[cfg(test)]
mod tests {
    use super::listens;
    #[test]
    fn only_the_live_child_may_receive_credentials() {
        assert!(listens(
            "TCP 127.0.0.1:1234 0.0.0.0:0 LISTENING 42",
            1234,
            42
        ));
        assert!(!listens(
            "TCP 127.0.0.1:1234 0.0.0.0:0 LISTENING 43",
            1234,
            42
        ));
        assert!(!listens(
            "TCP 127.0.0.1:1234 0.0.0.0:0 LISTENING 0",
            1234,
            0
        ));
        assert!(!listens(
            "TCP 127.0.0.1:1234 0.0.0.0:0 LISTENING 42\nTCP [::]:1234 [::]:0 LISTENING 43",
            1234,
            42
        ));
    }
}
