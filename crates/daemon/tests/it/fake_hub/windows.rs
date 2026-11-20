//! The HTTP fixture listens on the same native private socket as the real link.
use anima_windows::socket::{Listener, Stream};
use std::{io, path::Path};

pub(super) struct UnixListener(Listener);
impl UnixListener {
    pub(super) fn bind(path: &Path) -> io::Result<Self> {
        Listener::bind(path).map(Self)
    }
}
impl axum::serve::Listener for UnixListener {
    type Io = Stream;
    type Addr = ();
    async fn accept(&mut self) -> (Stream, ()) {
        (self.0.accept().await.expect("fixture socket accepts"), ())
    }
    fn local_addr(&self) -> io::Result<()> {
        Ok(())
    }
}
