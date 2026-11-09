//! SMTP egress: one message to a mail server at its checked addresses, encrypted before any
//! command that carries a credential or the message (D-17).
//!
//! The connection goes to the checked IP addresses; the TLS parameters name the host, so SNI
//! and the certificate check use the host name. Implicit TLS or STARTTLS is required; there is
//! no plaintext or opportunistic mode, so a server that does not offer STARTTLS gets no login
//! and no message.
//!
//! The reply to the message (after DATA) decides the outcome, within the time limit. QUIT is
//! sent afterwards in whatever time is left and its answer is ignored, so a server that queued
//! the message but never answers QUIT still counts as sent and is not sent to again (S-A).
//!
//! The client greets with the address literal `[127.0.0.1]` (RFC 5321 4.1.4), never the
//! machine's host name, which would name the hub's container or host to every mail server.

use std::{
    io,
    net::{Ipv4Addr, SocketAddr},
};

use lettre::{
    Message,
    transport::smtp::{
        self,
        authentication::{Credentials, Mechanism},
        client::{AsyncSmtpConnection, Certificate, CertificateStore, TlsParameters},
        extension::ClientId,
    },
};

use serde::{Deserialize, Serialize};
use tokio::time::{Instant, timeout_at};

use super::{Egress, SendError, TIME_LIMIT, causes, tls_broke};
use crate::targets::resolve_checked;

/// How the connection to a mail server is encrypted. There is no plaintext choice. An e-mail
/// channel stores it as `tls` or `starttls` (K1 `SmtpSecurity`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum SmtpSecurity {
    /// TLS from the first byte (port 465).
    #[serde(rename = "tls")]
    Implicit,
    /// Plain connection upgraded with STARTTLS before anything else (port 587); a server that
    /// does not offer it is refused. Implicit TLS is the recommended mode: lettre reads a reply
    /// an attacker injects in plaintext right after the STARTTLS answer as if it came over TLS.
    #[serde(rename = "starttls")]
    StartTls,
}

/// The mail server a message goes to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SmtpServer {
    /// A host name or IP literal; resolved and checked on each send.
    pub(crate) host: String,
    pub(crate) port: u16,
    pub(crate) security: SmtpSecurity,
}

/// The login mechanisms offered, only ever over TLS.
const MECHANISMS: [Mechanism; 2] = [Mechanism::Plain, Mechanism::Login];

/// The name the client greets with: a fixed address literal, so no mail server learns the
/// hub's host name.
pub(super) const HELLO: ClientId = ClientId::Ipv4(Ipv4Addr::LOCALHOST);

impl Egress {
    /// Sends `message` to its envelope's recipients through `server`, logging in with `login`
    /// when given. The outcome is decided within [`TIME_LIMIT`] from the lookup; QUIT then
    /// gets only what is left of it, and its answer changes nothing (S-A).
    ///
    /// # Errors
    /// [`SendError::Target`] when the host does not resolve or names a refused address, with
    /// no connection made; [`SendError::NoStartTls`] when STARTTLS is required and not
    /// offered; [`SendError::Tls`] when the certificate is not trusted for the host name;
    /// [`SendError::SmtpReply`] when the server refuses a command; otherwise the setup,
    /// connection or time limit failure.
    pub(crate) async fn send_mail(
        &self,
        server: &SmtpServer,
        login: Option<&Credentials>,
        message: &Message,
    ) -> Result<(), SendError> {
        let deadline = Instant::now() + TIME_LIMIT;
        let (mut connection, outcome) =
            timeout_at(deadline, self.deliver(server, login, message))
                .await
                .map_err(|_| SendError::TimedOut)??;
        // The outcome is decided; a QUIT that is not answered in time is dropped with the
        // connection.
        let _ = timeout_at(deadline, connection.abort()).await;
        outcome
    }

    /// Connects, logs in and sends; returns the open connection and the server's verdict on
    /// the message, which the reply after DATA decides.
    async fn deliver(
        &self,
        server: &SmtpServer,
        login: Option<&Credentials>,
        message: &Message,
    ) -> Result<(AsyncSmtpConnection, Result<(), SendError>), SendError> {
        let addresses = resolve_checked(&server.host, server.port, &self.policy).await?;
        let tls = self.tls_parameters(&server.host)?;
        let mut connection = open(&addresses, server.security, tls).await?;
        let outcome = submit(&mut connection, login, message).await;
        Ok((connection, outcome))
    }

    /// TLS parameters that verify the server's certificate for `host` against the built-in
    /// roots (and a test's extra root).
    pub(super) fn tls_parameters(&self, host: &str) -> Result<TlsParameters, SendError> {
        let name = host
            .strip_prefix('[')
            .and_then(|inside| inside.strip_suffix(']'))
            .unwrap_or(host);
        let mut builder = TlsParameters::builder(name.to_owned())
            .certificate_store(CertificateStore::WebpkiRoots);
        for der in &self.extra_roots {
            let root = Certificate::from_der(der.clone()).map_err(|_| SendError::Setup)?;
            builder = builder.add_root_certificate(root);
        }
        builder.build_rustls().map_err(|_| SendError::Setup)
    }
}

/// Connects to the first of `addresses` that answers and encrypts the connection as
/// `security` says. Never returns a connection that is not encrypted.
pub(super) async fn open(
    addresses: &[SocketAddr],
    security: SmtpSecurity,
    tls: TlsParameters,
) -> Result<AsyncSmtpConnection, SendError> {
    let wrapper = (security == SmtpSecurity::Implicit).then(|| tls.clone());
    let mut connection =
        AsyncSmtpConnection::connect_tokio1(addresses, Some(TIME_LIMIT), &HELLO, wrapper, None)
            .await
            .map_err(|error| classify(&error))?;
    if security == SmtpSecurity::StartTls {
        if !connection.can_starttls() {
            connection.abort().await;
            return Err(SendError::NoStartTls);
        }
        connection
            .starttls(tls, &HELLO)
            .await
            .map_err(|error| classify(&error))?;
    }
    if connection.is_encrypted() {
        Ok(connection)
    } else {
        connection.abort().await;
        Err(SendError::NoStartTls)
    }
}

/// Logs in when `login` is given, then sends `message`, on an encrypted connection.
async fn submit(
    connection: &mut AsyncSmtpConnection,
    login: Option<&Credentials>,
    message: &Message,
) -> Result<(), SendError> {
    if let Some(login) = login {
        connection
            .auth(&MECHANISMS, login)
            .await
            .map_err(|error| classify(&error))?;
    }
    connection
        .send(message.envelope(), &message.formatted())
        .await
        .map_err(|error| classify(&error))?;
    Ok(())
}

/// The [`SendError`] a lettre error stands for, judged from its kind and causes, never its
/// text.
fn classify(error: &smtp::Error) -> SendError {
    if let Some(code) = error.status() {
        SendError::SmtpReply(u16::from(code))
    } else if error.is_timeout() {
        SendError::TimedOut
    } else if error.is_tls() || tls_broke(error) {
        SendError::Tls
    } else if causes(error).any(<dyn std::error::Error>::is::<io::Error>) {
        SendError::Connect
    } else {
        SendError::Exchange
    }
}
