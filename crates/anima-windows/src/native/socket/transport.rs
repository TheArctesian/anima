//! Windows `AF_UNIX` sockets use Winsock handles. Register them with Tokio's socket
//! readiness driver; no worker thread, polling loop, or loopback TCP port is needed.
use std::{
    io,
    mem::ManuallyDrop,
    os::windows::io::{AsRawSocket, FromRawSocket, IntoRawSocket, RawSocket},
    path::Path,
    pin::Pin,
    task::{Context, Poll},
};
use tokio::{
    io::{AsyncRead, AsyncWrite, Interest, ReadBuf},
    net::TcpStream,
};

#[derive(Debug)]
pub struct Stream(TcpStream);

impl Stream {
    /// Connect only to a private native endpoint of this user.
    /// # Errors
    /// Its path/ACL is untrusted or connection/registration fails.
    pub async fn connect(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref().to_owned();
        let stream = tokio::task::spawn_blocking(move || {
            super::trusted(&path)?;
            uds_windows::UnixStream::connect(path)
        })
        .await
        .map_err(io::Error::other)??;
        stream.set_nonblocking(true)?;
        register(stream.into_raw_socket()).map(Self)
    }
}

/// Only stream I/O and readiness methods are used on this registration; no TCP operation
/// or socket-address conversion is applied to the `AF_UNIX` handle.
fn register(socket: RawSocket) -> io::Result<TcpStream> {
    // SAFETY: socket is newly transferred from its former owner. Windows socket operations
    // are defined for every Winsock family. from_std owns/closes it even if registration fails.
    let stream = unsafe { std::net::TcpStream::from_raw_socket(socket) };
    TcpStream::from_std(stream)
}

pub struct Listener(TcpStream);

impl Listener {
    /// Bind within an existing private directory, replacing only an old private socket.
    /// # Errors
    /// The directory/socket is untrusted, or bind/registration fails.
    pub fn bind(path: &Path) -> io::Result<Self> {
        super::private_folder(path.parent().ok_or(io::ErrorKind::InvalidInput)?)?;
        match std::fs::symlink_metadata(path) {
            Ok(_) => {
                super::trusted(path)?;
                std::fs::remove_file(path)?;
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
        let listener = uds_windows::UnixListener::bind(path)?;
        super::trusted(path)?;
        listener.set_nonblocking(true)?;
        register(listener.into_raw_socket()).map(Self)
    }

    /// Await a connection without taking ownership of the listening socket a second time.
    /// # Errors
    /// Accept or registering the accepted socket fails.
    pub async fn accept(&self) -> io::Result<Stream> {
        let stream = self
            .0
            .async_io(Interest::READABLE, || {
                // SAFETY: self keeps this socket alive through the closure. ManuallyDrop prevents
                // closing the borrowed handle; only the newly accepted stream is transferred.
                let listener = ManuallyDrop::new(unsafe {
                    uds_windows::UnixListener::from_raw_socket(self.0.as_raw_socket())
                });
                listener.accept().map(|(stream, _)| stream)
            })
            .await?;
        stream.set_nonblocking(true)?;
        register(stream.into_raw_socket()).map(Stream)
    }
}

impl AsyncRead for Stream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buffer: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        Pin::new(&mut self.0).poll_read(cx, buffer)
    }
}

impl AsyncWrite for Stream {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bytes: &[u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.0).poll_write(cx, bytes)
    }
    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.0).poll_flush(cx)
    }
    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.0).poll_shutdown(cx)
    }
}

#[cfg(test)]
mod tests;
