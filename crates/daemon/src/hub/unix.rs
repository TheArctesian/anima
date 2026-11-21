//! HTTP on the Unix socket of `anima-link`, one connection for each request, written only after
//! the peer of the connection passed its check ([`super::peer`]).
use super::{
    Call,
    answer::{Answer, Cause, answer},
    body::Body,
    peer::{Expected, Peer},
};
use bytes::Bytes;
use http_body_util::{BodyExt as _, Full};
use hyper::{
    Request,
    header::{AUTHORIZATION, CONTENT_TYPE, HOST},
};
use hyper_util::rt::TokioIo;
use std::{
    path::{Path, PathBuf},
    time::Duration,
};
use tokio::net::UnixStream;

pub(super) struct Local {
    socket: PathBuf,
    limit: Duration,
    expected: Expected,
}

impl Local {
    pub(super) const fn new(socket: PathBuf, limit: Duration, expected: Expected) -> Self {
        Self {
            socket,
            limit,
            expected,
        }
    }

    pub(super) async fn post<T>(
        &self,
        call: Call<'_>,
        read_ok: impl FnOnce(u16, &[u8]) -> Option<T>,
    ) -> Answer<T> {
        match tokio::time::timeout(self.limit, self.exchange(call, read_ok)).await {
            Ok(answer) => answer,
            Err(_) => Answer::Failed(Cause::Timeout),
        }
    }

    async fn connect(&self) -> Result<UnixStream, Cause> {
        checked(&self.socket, &self.expected).await
    }

    async fn exchange<T>(
        &self,
        call: Call<'_>,
        read_ok: impl FnOnce(u16, &[u8]) -> Option<T>,
    ) -> Answer<T> {
        let stream = match self.connect().await {
            Ok(stream) => stream,
            Err(cause) => return Answer::Failed(cause),
        };
        let Ok((mut sender, connection)) =
            hyper::client::conn::http1::handshake(TokioIo::new(stream)).await
        else {
            return Answer::Failed(Cause::Network);
        };
        let driving = tokio::spawn(connection);
        // The Host is not the socket: the hub's link-mode guard requires a loopback Host.
        let mut request = Request::post(format!("/{}", call.route))
            .header(HOST, "127.0.0.1")
            .header(CONTENT_TYPE, "application/json");
        if let Some(token) = call.bearer {
            request = request.header(AUTHORIZATION, format!("Bearer {}", token.expose()));
        }
        let Ok(request) = request.body(Full::new(Bytes::from_owner(call.body))) else {
            driving.abort();
            return Answer::Failed(Cause::Unreadable);
        };
        let answer = read(sender.send_request(request).await, call.ok, read_ok).await;
        driving.abort();
        answer
    }
}

/// A connection to `socket` whose peer `expected` admits; nothing is written to it yet.
pub(super) async fn checked(socket: &Path, expected: &Expected) -> Result<UnixStream, Cause> {
    let stream = UnixStream::connect(socket)
        .await
        .map_err(|_| Cause::Network)?;
    let credentials = stream.peer_cred().map_err(|_| Cause::NotTheLink)?;
    let peer = Peer {
        user: credentials.uid(),
        process: credentials.pid(),
    };
    expected.admit(peer).await.map_err(|_| Cause::NotTheLink)?;
    Ok(stream)
}

/// The meaning of the answer to one request, with at most [`super::body::LIMIT`] bytes read.
async fn read<T>(
    sent: hyper::Result<hyper::Response<hyper::body::Incoming>>,
    ok: &[u16],
    read_ok: impl FnOnce(u16, &[u8]) -> Option<T>,
) -> Answer<T> {
    let Ok(response) = sent else {
        return Answer::Failed(Cause::Network);
    };
    let status = response.status().as_u16();
    let mut incoming = response.into_body();
    let mut bytes = Body::new();
    while !bytes.is_full() {
        match incoming.frame().await {
            Some(Ok(frame)) => {
                if let Ok(chunk) = frame.into_data() {
                    bytes.add(&chunk);
                }
            }
            Some(Err(_)) => return Answer::Failed(Cause::Network),
            None => break,
        }
    }
    if ok.contains(&status) {
        return read_ok(status, bytes.bytes())
            .map_or(Answer::Failed(Cause::Unreadable), Answer::Accepted);
    }
    answer(status, bytes.bytes(), ok[0], |_| None)
}
