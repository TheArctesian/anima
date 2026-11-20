//! Where a sign-in inside the app comes back (mobile plan M3-5): a loopback address of the shell, as
//! RFC 8252 Section 7.3 describes for a native application. `WorkOS` takes `http://127.0.0.1` as a return
//! address of a native client in production. The shell listens for one sign-in only, reads the
//! request line of the browser, and answers with one small page. Nothing here logs the request: it
//! holds the code (LOG-007).

use std::time::Duration;

use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
};
use zeroize::Zeroizing;

use super::CloudError;

/// The port of the return address. It is fixed, because the return address is registered in `WorkOS`.
pub(super) const PORT: u16 = 47113;
const PATH: &str = "/sign-in";
/// The longest request line that is read.
const LINE_LIMIT: u64 = 4096;
/// How long one connection may take to send its request line.
const LINE_WAIT: Duration = Duration::from_secs(5);

const PAGE: &str = "<!doctype html><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width\">\
<title>Anima.ing</title><p style=\"font:16px system-ui;margin:2rem\">Go back to Anima.ing.</p>";

/// The return address for the port `port`.
pub(super) fn address(port: u16) -> String {
    format!("http://127.0.0.1:{port}{PATH}")
}

/// Listens at the return address. `Refused` when a different program holds the port.
pub(super) async fn listen(port: u16) -> Result<TcpListener, CloudError> {
    TcpListener::bind(("127.0.0.1", port))
        .await
        .map_err(|_| CloudError::Refused)
}

/// Waits for the browser to come back with the answer to the sign-in with `state`, and gives the
/// code. A request for a different path, or one that is not complete, gets "not found" and the wait
/// goes on. An answer with a different state, or with no code, ends the sign-in.
pub(super) async fn returned(
    listener: &TcpListener,
    state: &str,
) -> Result<Zeroizing<String>, CloudError> {
    loop {
        let (stream, _) = listener.accept().await.map_err(|_| CloudError::Refused)?;
        let mut stream = BufReader::new(stream);
        let mut line = String::new();
        let read = tokio::time::timeout(
            LINE_WAIT,
            (&mut stream).take(LINE_LIMIT).read_line(&mut line),
        )
        .await;
        let line = Zeroizing::new(line);
        let query = matches!(read, Ok(Ok(_))).then(|| query_of(&line)).flatten();
        let Some(query) = query else {
            answer(stream.into_inner(), "404 Not Found", "").await;
            continue;
        };
        answer(stream.into_inner(), "200 OK", PAGE).await;
        return code_of(query, state);
    }
}

/// The query of a request line for the return path, such as `GET /sign-in?code=1&state=2 HTTP/1.1`.
fn query_of(line: &str) -> Option<&str> {
    let target = line.strip_prefix("GET ")?.split(' ').next()?;
    let (path, query) = target.split_once('?').unwrap_or((target, ""));
    (path == PATH).then_some(query)
}

/// The code of the answer in `query`, when its state is `state`. `access_denied` is a sign-in that
/// the person or the provider stopped.
fn code_of(query: &str, state: &str) -> Result<Zeroizing<String>, CloudError> {
    let value = |name: &str| {
        query
            .split('&')
            .filter_map(|pair| pair.split_once('='))
            .find_map(|(key, value)| (key == name).then_some(value))
    };
    if value("state") != Some(state) {
        return Err(CloudError::Refused);
    }
    match (value("code"), value("error")) {
        (Some(code), None) if !code.is_empty() => Ok(Zeroizing::new(code.to_owned())),
        (_, Some("access_denied")) => Err(CloudError::Cancelled),
        _ => Err(CloudError::Refused),
    }
}

/// Sends one answer and closes the connection. A browser that went away is not an error.
async fn answer(mut stream: TcpStream, status: &str, body: &str) {
    let text = format!(
        "HTTP/1.1 {status}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\n\
Cache-Control: no-store\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let _ = stream.write_all(text.as_bytes()).await;
    let _ = stream.shutdown().await;
}

#[cfg(test)]
mod tests {
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpStream,
    };

    use super::{CloudError, PORT, address, code_of, listen, query_of, returned};

    #[test]
    fn m3_5_only_a_get_of_the_return_path_has_a_query() {
        assert_eq!(
            query_of("GET /sign-in?code=1&state=2 HTTP/1.1\r\n"),
            Some("code=1&state=2")
        );
        assert_eq!(query_of("GET /sign-in HTTP/1.1\r\n"), Some(""));
        assert_eq!(query_of("GET /favicon.ico HTTP/1.1\r\n"), None);
        assert_eq!(query_of("POST /sign-in?code=1 HTTP/1.1\r\n"), None);
        assert_eq!(query_of("GET /sign-in/more?code=1 HTTP/1.1\r\n"), None);
        assert_eq!(query_of(""), None);
    }

    #[test]
    fn m3_5_the_code_is_taken_only_with_the_state_of_this_sign_in() {
        assert_eq!(
            code_of("code=abc&state=mine", "mine")
                .ok()
                .as_deref()
                .map(String::as_str),
            Some("abc")
        );
        assert_eq!(
            code_of("state=mine&code=abc", "mine")
                .ok()
                .map(|code| code.len()),
            Some(3)
        );
        assert_eq!(
            code_of("code=abc&state=other", "mine").err(),
            Some(CloudError::Refused)
        );
        assert_eq!(code_of("code=abc", "mine").err(), Some(CloudError::Refused));
        assert_eq!(
            code_of("code=&state=mine", "mine").err(),
            Some(CloudError::Refused)
        );
        assert_eq!(
            code_of("state=mine", "mine").err(),
            Some(CloudError::Refused)
        );
    }

    #[test]
    fn m3_5_a_stopped_sign_in_is_cancelled_and_any_other_error_is_a_refusal() {
        assert_eq!(
            code_of("error=access_denied&error_description=x&state=mine", "mine").err(),
            Some(CloudError::Cancelled)
        );
        assert_eq!(
            code_of("error=server_error&state=mine", "mine").err(),
            Some(CloudError::Refused)
        );
        assert_eq!(
            code_of("code=abc&error=server_error&state=mine", "mine").err(),
            Some(CloudError::Refused)
        );
    }

    async fn get(port: u16, target: &str) -> String {
        let mut stream = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
        let request = format!("GET {target} HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n");
        stream.write_all(request.as_bytes()).await.unwrap();
        let mut answer = String::new();
        stream.read_to_string(&mut answer).await.unwrap();
        answer
    }

    #[tokio::test]
    async fn m3_5_the_listener_passes_other_requests_and_gives_the_code_of_the_return() {
        let listener = listen(0).await.unwrap();
        let port = listener.local_addr().unwrap().port();
        assert_eq!(address(port), format!("http://127.0.0.1:{port}/sign-in"));
        assert_eq!(address(PORT), "http://127.0.0.1:47113/sign-in");
        let browser = tokio::spawn(async move {
            let other = get(port, "/favicon.ico").await;
            let back = get(port, "/sign-in?code=the-code&state=mine").await;
            (other, back)
        });
        let code = returned(&listener, "mine").await.unwrap();
        assert_eq!(code.as_str(), "the-code");
        let (other, back) = browser.await.unwrap();
        assert!(other.starts_with("HTTP/1.1 404 "), "{other}");
        assert!(back.starts_with("HTTP/1.1 200 "), "{back}");
        assert!(back.contains("Go back to Anima.ing."));
        assert!(!back.contains("the-code"));
    }

    #[tokio::test]
    async fn m3_5_a_port_that_is_held_is_a_refusal() {
        let first = listen(0).await.unwrap();
        let port = first.local_addr().unwrap().port();
        assert_eq!(listen(port).await.err(), Some(CloudError::Refused));
    }
}
