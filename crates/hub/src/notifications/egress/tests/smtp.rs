//! SMTP egress: the connection goes to a checked address, TLS names the host, and nothing
//! that carries a credential or the message is sent before TLS (P3-003, D-17, SEC-005).

use base64::{Engine, engine::general_purpose::STANDARD};
use lettre::{Message, transport::smtp::authentication::Credentials};

use super::{
    local, loopback_denied,
    servers::{Behaviour, Server, certificate, serve},
};
use crate::{
    notifications::egress::{
        Egress, SendError,
        smtp::{SmtpSecurity, SmtpServer},
    },
    targets::{AddressPolicy, TargetError},
};

const USER: &str = "hub@anima.test";
const PASSWORD: &str = "synthetic-password";

fn message() -> Message {
    Message::builder()
        .from("Anima <hub@anima.test>".parse().unwrap())
        .to("ops@anima.test".parse().unwrap())
        .subject("web is down")
        .body(String::from("The check web is down."))
        .unwrap()
}

fn login() -> Credentials {
    Credentials::new(USER.to_owned(), PASSWORD.to_owned())
}

async fn send(egress: &Egress, server: &Server, security: SmtpSecurity) -> Result<(), SendError> {
    let target = SmtpServer {
        host: "localhost".to_owned(),
        port: server.port,
        security,
    };
    egress.send_mail(&target, Some(&login()), &message()).await
}

/// Whether any line the server received names the login, or is a login or message command.
fn credential_or_message_reached(server: &Server) -> bool {
    let plain = STANDARD.encode(format!("\0{USER}\0{PASSWORD}"));
    server.lines().iter().any(|line| {
        let verb = line.get(..4).unwrap_or_default().to_ascii_uppercase();
        line.contains(PASSWORD)
            || line.contains(&plain)
            || line.contains(&STANDARD.encode(PASSWORD))
            || ["AUTH", "MAIL", "RCPT", "DATA"].contains(&verb.as_str())
    })
}

fn verbs(server: &Server) -> Vec<String> {
    server
        .lines()
        .iter()
        .map(|line| line.split(' ').next().unwrap_or_default().to_owned())
        .collect()
}

#[tokio::test]
async fn d_17_implicit_tls_names_the_host_and_then_logs_in() {
    let (der, acceptor) = certificate("localhost");
    let server = serve(Behaviour::SmtpImplicit(acceptor)).await;

    let sent = send(&local().trusting(der), &server, SmtpSecurity::Implicit).await;

    assert_eq!(sent, Ok(()));
    server.settle().await;
    assert_eq!(server.connections(), 1);
    assert_eq!(server.server_names(), ["localhost"]);
    assert_eq!(
        verbs(&server),
        ["EHLO", "AUTH", "MAIL", "RCPT", "DATA", "QUIT"]
    );
}

#[tokio::test]
async fn d_17_an_untrusted_certificate_ends_implicit_tls_before_any_command() {
    let (_, acceptor) = certificate("localhost");
    let server = serve(Behaviour::SmtpImplicit(acceptor)).await;

    let sent = send(&local(), &server, SmtpSecurity::Implicit).await;

    assert_eq!(sent, Err(SendError::Tls));
    server.settle().await;
    assert_eq!(server.connections(), 1);
    assert_eq!(server.server_names(), Vec::<String>::new());
    assert_eq!(server.lines(), Vec::<String>::new());
}

#[tokio::test]
async fn d_17_starttls_names_the_host_before_the_login() {
    let (der, acceptor) = certificate("localhost");
    let server = serve(Behaviour::SmtpStartTls(acceptor)).await;

    let sent = send(&local().trusting(der), &server, SmtpSecurity::StartTls).await;

    assert_eq!(sent, Ok(()));
    server.settle().await;
    assert_eq!(server.server_names(), ["localhost"]);
    assert_eq!(
        verbs(&server),
        [
            "EHLO", "STARTTLS", "EHLO", "AUTH", "MAIL", "RCPT", "DATA", "QUIT"
        ]
    );
}

#[tokio::test]
async fn d_17_a_server_without_starttls_gets_no_login_and_no_message() {
    let server = serve(Behaviour::SmtpPlain).await;

    let sent = send(&local(), &server, SmtpSecurity::StartTls).await;

    assert_eq!(sent, Err(SendError::NoStartTls));
    server.settle().await;
    assert_eq!(server.connections(), 1);
    assert!(
        !credential_or_message_reached(&server),
        "{:?}",
        verbs(&server)
    );
}

#[tokio::test]
async fn d_17_an_untrusted_certificate_ends_starttls_before_the_login() {
    let (_, acceptor) = certificate("localhost");
    let server = serve(Behaviour::SmtpStartTls(acceptor)).await;

    let sent = send(&local(), &server, SmtpSecurity::StartTls).await;

    assert_eq!(sent, Err(SendError::Tls));
    server.settle().await;
    assert_eq!(verbs(&server), ["EHLO", "STARTTLS"]);
    assert!(!credential_or_message_reached(&server));
}

#[tokio::test]
async fn p3_003_a_refused_mail_server_gets_no_connection() {
    let server = serve(Behaviour::SmtpPlain).await;
    let default = Egress::new(AddressPolicy::default());

    let refused = Err(SendError::Target(TargetError::Refused));
    assert_eq!(
        send(&default, &server, SmtpSecurity::StartTls).await,
        refused
    );
    assert_eq!(
        send(&loopback_denied(), &server, SmtpSecurity::Implicit).await,
        refused
    );
    server.settle().await;
    assert_eq!(server.connections(), 0);
}
