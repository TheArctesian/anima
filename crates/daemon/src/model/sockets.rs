//! The socket table of the process tree of a server (Section 9). The tree is the cgroup of the
//! server on Linux and the walk of its descendants on macOS ([`super::tree`]). The daemon
//! reads the table from the system and never connects to find a socket.
//!
//! On Linux: the socket inodes of `/proc/<pid>/fd` of each process of the cgroup, and the
//! tables `/proc/<pid>/net/{tcp,tcp6,udp,udp6}` of the first one. On macOS the contract names
//! `proc_pidinfo` with `PROC_PIDFDSOCKETINFO`; the workspace forbids `unsafe`, so this build
//! runs `/usr/sbin/lsof` by its absolute path with an empty environment (a deviation of task
//! L1-6).

use std::io;

use daemon_core::model_work::loopback::Socket;

use super::tree::Identity;

/// The listening TCP sockets and the bound UDP sockets of the processes `members` (the tree of
/// a server, [`super::tree`]).
///
/// # Errors
/// An I/O error when the table cannot be read; the caller then treats the server as exposed.
pub async fn table(members: &[Identity]) -> io::Result<Vec<Socket>> {
    if members.is_empty() {
        return Ok(Vec::new());
    }
    #[cfg(target_os = "linux")]
    {
        linux(members).await
    }
    #[cfg(not(target_os = "linux"))]
    {
        lsof(members).await
    }
}

/// The most time of one read of `lsof`; a slower read counts as a table that does not read.
#[cfg(not(target_os = "linux"))]
const LSOF_SECONDS: u64 = 5;

#[cfg(not(target_os = "linux"))]
async fn lsof(members: &[Identity]) -> io::Result<Vec<Socket>> {
    let pids: Vec<String> = members
        .iter()
        .map(|member| member.pid.to_string())
        .collect();
    let run = tokio::process::Command::new("/usr/sbin/lsof")
        .args(["-nP", "-a", "-p", &pids.join(","), "-i", "-F", "PnT"])
        .env_clear()
        .stdin(std::process::Stdio::null())
        .kill_on_drop(true)
        .output();
    let output = tokio::time::timeout(std::time::Duration::from_secs(LSOF_SECONDS), run)
        .await
        .map_err(|_| io::Error::other("lsof did not end in 5 seconds"))??;
    // `lsof` ends with 1 when a selection matched nothing; its output is still the table.
    if !output.status.success() && output.status.code() != Some(1) {
        return Err(io::Error::other("lsof did not read the socket table"));
    }
    Ok(daemon_core::model_work::loopback::read_lsof(
        &String::from_utf8_lossy(&output.stdout),
    ))
}

#[cfg(target_os = "linux")]
async fn linux(members: &[Identity]) -> io::Result<Vec<Socket>> {
    use daemon_core::model_work::loopback::{Protocol, read_proc_net};
    use std::collections::HashSet;

    let mut inodes = HashSet::new();
    for member in members {
        let Ok(mut fds) = tokio::fs::read_dir(format!("/proc/{}/fd", member.pid)).await else {
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
    let leader = members[0].pid;
    let mut sockets = Vec::new();
    for (name, protocol) in [
        ("tcp", Protocol::Tcp),
        ("tcp6", Protocol::Tcp),
        ("udp", Protocol::Udp),
        ("udp6", Protocol::Udp),
    ] {
        let text = tokio::fs::read_to_string(format!("/proc/{leader}/net/{name}")).await?;
        sockets.extend(
            read_proc_net(&text, protocol)
                .into_iter()
                .filter(|(_, inode)| inodes.contains(inode))
                .map(|(socket, _)| socket),
        );
    }
    Ok(sockets)
}
