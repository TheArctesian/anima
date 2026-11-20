//! Local transport used by fake links and brokers on each host.
#[cfg(windows)]
pub use anima_windows::socket::{Listener, Stream};
#[cfg(unix)]
pub use tokio::net::{UnixListener as Listener, UnixStream as Stream};

pub fn protect(path: &std::path::Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        let mode = if path.is_dir() { 0o700 } else { 0o600 };
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).unwrap();
    }
    #[cfg(windows)]
    anima_windows::files::protect(path).unwrap();
}

pub async fn accept(listener: &Listener) -> std::io::Result<Stream> {
    #[cfg(unix)]
    {
        listener.accept().await.map(|(stream, _)| stream)
    }
    #[cfg(windows)]
    {
        listener.accept().await
    }
}
