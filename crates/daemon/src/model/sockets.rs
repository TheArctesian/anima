//! The socket table of the process tree of a server (Section 9). The tree is the process
//! group of the server, which the daemon makes at the start. The daemon reads the table from
//! the system and never connects to find a socket.
//!
//! On Linux: `/proc/<pid>/stat` for the group, the socket inodes of `/proc/<pid>/fd`, and the
//! tables `/proc/<leader>/net/{tcp,tcp6,udp,udp6}`. On macOS the contract names
//! `proc_pidinfo` with `PROC_PIDFDSOCKETINFO`; the workspace forbids `unsafe`, so this build
//! runs `/usr/sbin/lsof` by its absolute path with an empty environment (a deviation of task
//! L1-6).

use std::io;

use daemon_core::model_work::loopback::Socket;

/// The listening TCP sockets and the bound UDP sockets of the process group `group`.
///
/// # Errors
/// An I/O error when the table cannot be read; the caller then treats the server as exposed.
pub async fn table(group: i32) -> io::Result<Vec<Socket>> {
    #[cfg(target_os = "linux")]
    {
        linux(group).await
    }
    #[cfg(not(target_os = "linux"))]
    {
        lsof(group).await
    }
}

#[cfg(not(target_os = "linux"))]
async fn lsof(group: i32) -> io::Result<Vec<Socket>> {
    let output = tokio::process::Command::new("/usr/sbin/lsof")
        .args(["-nP", "-a", "-g", &group.to_string(), "-i", "-F", "PnT"])
        .env_clear()
        .stdin(std::process::Stdio::null())
        .kill_on_drop(true)
        .output()
        .await?;
    // `lsof` ends with 1 and no output when no socket matches.
    let none = output.status.code() == Some(1) && output.stdout.is_empty();
    if !output.status.success() && !none {
        return Err(io::Error::other("lsof did not read the socket table"));
    }
    Ok(daemon_core::model_work::loopback::read_lsof(
        &String::from_utf8_lossy(&output.stdout),
    ))
}

#[cfg(target_os = "linux")]
fn group_of(stat: &str) -> Option<i32> {
    let after = stat.rsplit_once(')')?.1;
    after.split_whitespace().nth(2)?.parse().ok()
}

#[cfg(target_os = "linux")]
async fn linux(group: i32) -> io::Result<Vec<Socket>> {
    use daemon_core::model_work::loopback::{Protocol, read_proc_net};
    use std::collections::HashSet;

    let mut inodes = HashSet::new();
    let mut processes = tokio::fs::read_dir("/proc").await?;
    while let Some(entry) = processes.next_entry().await? {
        let Some(pid) = entry
            .file_name()
            .to_str()
            .and_then(|name| name.parse::<i32>().ok())
        else {
            continue;
        };
        let stat = tokio::fs::read_to_string(format!("/proc/{pid}/stat"))
            .await
            .unwrap_or_default();
        if group_of(&stat) != Some(group) {
            continue;
        }
        let Ok(mut fds) = tokio::fs::read_dir(format!("/proc/{pid}/fd")).await else {
            continue;
        };
        while let Ok(Some(fd)) = fds.next_entry().await {
            let Ok(target) = tokio::fs::read_link(fd.path()).await else {
                continue;
            };
            let target = target.to_string_lossy();
            if let Some(inode) = target
                .strip_prefix("socket:[")
                .and_then(|rest| rest.strip_suffix(']'))
            {
                inodes.extend(inode.parse::<u64>().ok());
            }
        }
    }
    let mut sockets = Vec::new();
    for (name, protocol) in [
        ("tcp", Protocol::Tcp),
        ("tcp6", Protocol::Tcp),
        ("udp", Protocol::Udp),
        ("udp6", Protocol::Udp),
    ] {
        let text = tokio::fs::read_to_string(format!("/proc/{group}/net/{name}")).await?;
        sockets.extend(
            read_proc_net(&text, protocol)
                .into_iter()
                .filter(|(_, inode)| inodes.contains(inode))
                .map(|(socket, _)| socket),
        );
    }
    Ok(sockets)
}
