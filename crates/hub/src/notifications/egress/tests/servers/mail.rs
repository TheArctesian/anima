//! The SMTP side of the test servers: a scripted conversation, in plain text, over TLS from the
//! first byte, or upgraded by STARTTLS.

use tokio::{
    io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader},
    net::TcpStream,
};
use tokio_rustls::TlsAcceptor;

use super::{Seen, handshake};

/// How an SMTP server answers.
#[derive(Clone, Copy)]
pub(super) struct Script {
    /// Whether it writes the 220 greeting first (not after STARTTLS).
    pub(super) greet: bool,
    /// What it answers to STARTTLS; `None` when it does not offer it.
    pub(super) starttls: Option<&'static [u8]>,
    /// What it answers to AUTH.
    pub(super) auth: &'static [u8],
}

pub(super) const GO_AHEAD: &[u8] = b"220 go ahead\r\n";
/// A plain conversation with no STARTTLS and every login accepted.
pub(super) const PLAIN: Script = Script {
    greet: true,
    starttls: None,
    auth: b"235 accepted\r\n",
};
/// The conversation after STARTTLS: no new greeting, no second STARTTLS.
const AFTER_STARTTLS: Script = Script {
    greet: false,
    ..PLAIN
};

/// SMTP over TLS from the first byte, answering as `script` says.
pub(super) async fn implicit(
    acceptor: &TlsAcceptor,
    stream: TcpStream,
    script: Script,
    seen: &Seen,
) {
    if let Some(tls) = handshake(acceptor, stream, seen).await {
        smtp(tls, script, seen).await;
    }
}

/// SMTP that offers STARTTLS, answers it with `go_ahead` and upgrades.
pub(super) async fn starttls(
    acceptor: &TlsAcceptor,
    stream: TcpStream,
    go_ahead: &'static [u8],
    seen: &Seen,
) {
    let script = Script {
        starttls: Some(go_ahead),
        ..PLAIN
    };
    let Some(plain) = smtp(stream, script, seen).await else {
        return;
    };
    if let Some(tls) = handshake(acceptor, plain, seen).await {
        smtp(tls, AFTER_STARTTLS, seen).await;
    }
}

/// One SMTP conversation answered as `script` says. Records every command line but the message
/// content; returns the stream when the client asks for STARTTLS.
pub(super) async fn smtp<S: AsyncRead + AsyncWrite + Unpin>(
    stream: S,
    script: Script,
    seen: &Seen,
) -> Option<S> {
    let mut reader = BufReader::new(stream);
    if script.greet {
        reader
            .get_mut()
            .write_all(b"220 test ESMTP\r\n")
            .await
            .ok()?;
    }
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).await.ok()? == 0 {
            return None;
        }
        seen.lines.lock().unwrap().push(line.trim_end().to_owned());
        let verb = line.get(..4).unwrap_or_default().to_ascii_uppercase();
        let reply: &[u8] = match (verb.as_str(), script.starttls) {
            ("EHLO", Some(_)) => b"250-test\r\n250-STARTTLS\r\n250 AUTH PLAIN LOGIN\r\n",
            ("EHLO", None) => b"250-test\r\n250 AUTH PLAIN LOGIN\r\n",
            ("STAR", Some(go_ahead)) => {
                reader.get_mut().write_all(go_ahead).await.ok()?;
                return Some(reader.into_inner());
            }
            ("AUTH", _) => script.auth,
            ("MAIL" | "RCPT", _) => b"250 ok\r\n",
            ("DATA", _) => {
                reader.get_mut().write_all(b"354 go on\r\n").await.ok()?;
                skip_content(&mut reader).await?;
                b"250 queued\r\n"
            }
            ("QUIT", _) => {
                let _ = reader.get_mut().write_all(b"221 bye\r\n").await;
                return None;
            }
            _ => b"500 unknown\r\n",
        };
        reader.get_mut().write_all(reply).await.ok()?;
    }
}

/// Reads the message content up to the line holding a single dot.
async fn skip_content<S: AsyncRead + Unpin>(reader: &mut BufReader<S>) -> Option<()> {
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).await.ok()? == 0 {
            return None;
        }
        if line == ".\r\n" {
            return Some(());
        }
    }
}
