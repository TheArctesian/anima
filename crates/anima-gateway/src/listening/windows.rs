//! Windows passes a process-specific Winsock record over private inherited pipes.
use super::{Workers, worker_command};
use anima_windows::socket::transfer;
use std::{
    io::{self, Read, Write},
    process::Stdio,
    time::Duration,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    process::Child,
};

pub(super) async fn start(connection: TcpStream, workers: &Workers) -> io::Result<Child> {
    let connection = connection.into_std()?;
    let mut command = worker_command(workers);
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .creation_flags(0x0800_0000);
    if let Some(root) = std::env::var_os("SystemRoot") {
        command.env("SystemRoot", root);
    }
    let mut child = command.spawn()?;
    let process = child.id().ok_or(io::ErrorKind::BrokenPipe)?;
    let record = transfer::export(&connection, process)?;
    let mut input = child.stdin.take().ok_or(io::ErrorKind::BrokenPipe)?;
    let mut output = child.stdout.take().ok_or(io::ErrorKind::BrokenPipe)?;
    tokio::time::timeout(Duration::from_secs(5), async {
        input.write_all(&record).await?;
        input.shutdown().await?;
        let mut acknowledged = [0];
        output.read_exact(&mut acknowledged).await?;
        if acknowledged != [1] {
            return Err(io::ErrorKind::InvalidData.into());
        }
        Ok::<_, io::Error>(())
    })
    .await??;
    // Drop the parent's descriptor only after the worker has imported its own.
    drop(connection);
    Ok(child)
}

/// Receive the connected socket from the listener without reading network credentials.
/// # Errors
/// The parent pipe closes early or the transferred socket cannot be imported.
pub fn hub_connection() -> io::Result<TcpStream> {
    let mut record = vec![0; transfer::RECORD_BYTES];
    io::stdin().lock().read_exact(&mut record)?;
    let connection = transfer::import(&record)?;
    connection.set_nonblocking(true)?;
    let connection = TcpStream::from_std(connection)?;
    connection.set_nodelay(true)?;
    let mut output = io::stdout().lock();
    output.write_all(&[1])?;
    output.flush()?;
    Ok(connection)
}
