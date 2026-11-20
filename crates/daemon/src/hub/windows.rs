//! HTTP over native Windows `AF_UNIX`, preserving the same timeout and response bounds.
use super::{
    Call,
    answer::{Answer, Cause, answer},
    body::Body,
};
use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::{
    Request,
    header::{AUTHORIZATION, CONTENT_TYPE, HOST},
};
use hyper_util::client::legacy::Client;
use hyperlocal_with_windows::{UnixClientExt, UnixConnector, Uri};
use std::{path::PathBuf, time::Duration};

pub(super) struct Local {
    client: Client<UnixConnector, Full<Bytes>>,
    socket: PathBuf,
    limit: Duration,
}

impl Local {
    pub(super) fn new(socket: PathBuf, limit: Duration) -> Self {
        Self {
            client: Client::unix(),
            socket,
            limit,
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

    async fn exchange<T>(
        &self,
        call: Call<'_>,
        read_ok: impl FnOnce(u16, &[u8]) -> Option<T>,
    ) -> Answer<T> {
        let uri: hyper::Uri = Uri::new(&self.socket, &format!("/{}", call.route)).into();
        // The socket connector encodes its path in the URI authority. It is not the
        // HTTP host: the hub's link-mode guard requires a loopback Host on every request.
        let mut request = Request::post(uri)
            .header(HOST, "127.0.0.1")
            .header(CONTENT_TYPE, "application/json");
        if let Some(token) = call.bearer {
            request = request.header(AUTHORIZATION, format!("Bearer {}", token.expose()));
        }
        let Ok(request) = request.body(Full::new(Bytes::from_owner(call.body))) else {
            return Answer::Failed(Cause::Unreadable);
        };
        let Ok(response) = self.client.request(request).await else {
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
        if call.ok.contains(&status) {
            return read_ok(status, bytes.bytes())
                .map_or(Answer::Failed(Cause::Unreadable), Answer::Accepted);
        }
        answer(status, bytes.bytes(), call.ok[0], |_| None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use zeroize::Zeroizing;

    fn accept(listener: &uds_windows::UnixListener) -> uds_windows::UnixStream {
        let end = std::time::Instant::now() + Duration::from_secs(5);
        loop {
            if let Ok((stream, _)) = listener.accept() {
                return stream;
            }
            assert!(std::time::Instant::now() < end, "client never connected");
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    #[tokio::test]
    async fn native_socket_checks_its_acl_and_delivers_a_bounded_http_answer() {
        let folder = tempfile::tempdir().unwrap();
        anima_windows::files::protect(folder.path()).unwrap();
        let path = folder.path().join("hub.sock");
        let listener = uds_windows::UnixListener::bind(&path).unwrap();
        listener.set_nonblocking(true).unwrap();
        super::super::link_socket::trusted(&path).unwrap();
        let server = std::thread::spawn(move || {
            let mut stream = accept(&listener);
            // Winsock inherits the listener's nonblocking mode on accepted sockets.
            stream.set_nonblocking(false).unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            stream
                .set_write_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            let mut request = Vec::new();
            while !request.ends_with(b"\r\n\r\n") {
                assert!(request.len() < 4096, "oversized request headers");
                let mut byte = [0_u8];
                stream.read_exact(&mut byte).unwrap();
                request.push(byte[0]);
            }
            assert!(request.starts_with(b"POST /daemon/v1/push HTTP/1.1\r\n"));
            let headers = String::from_utf8(request).unwrap().to_ascii_lowercase();
            assert!(headers.contains("\r\nhost: 127.0.0.1\r\n"));
            stream
                .write_all(b"HTTP/1.1 204 No Content\r\nConnection: close\r\n\r\n")
                .unwrap();
        });
        let local = Local::new(path.clone(), Duration::from_secs(3));
        let call = Call {
            route: "daemon/v1/push",
            bearer: None,
            body: Zeroizing::new(Vec::new()),
            ok: &[204],
        };
        assert!(matches!(
            local.post(call, |_, _| Some(())).await,
            Answer::Accepted(())
        ));
        server.join().unwrap();
        let _ = std::fs::remove_file(path);
    }
}
