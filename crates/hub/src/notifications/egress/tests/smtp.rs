//! SMTP egress: the connection goes to a checked address, TLS names the host, and nothing
//! that carries a credential or the message is sent before TLS (P3-003, D-17, SEC-005).

use std::{net::SocketAddr, time::Duration};

use base64::{Engine, engine::general_purpose::STANDARD};
use lettre::{Message, transport::smtp::authentication::Credentials};
use tokio::time::Instant;

use super::{
    local, loopback_denied,
    servers::{Behaviour, Server, certificate, serve},
};
use crate::{
    notifications::egress::{Egress, SendError, SmtpSecurity, SmtpServer, TIME_LIMIT, smtp::open},
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

fn target(server: &Server, security: SmtpSecurity) -> SmtpServer {
    SmtpServer {
        host: "localhost".to_owned(),
        port: server.port,
        security,
    }
}

async fn send(egress: &Egress, server: &Server, security: SmtpSecurity) -> Result<(), SendError> {
    let target = target(server, security);
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

/// The command verbs the server received, up to DATA: what lettre sends after the message
/// (QUIT) is its own business.
fn verbs(server: &Server) -> Vec<String> {
    let mut verbs: Vec<String> = server
        .lines()
        .iter()
        .map(|line| line.split(' ').next().unwrap_or_default().to_owned())
        .collect();
    if let Some(data) = verbs.iter().position(|verb| verb == "DATA") {
        verbs.truncate(data + 1);
    }
    verbs
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
    assert_eq!(verbs(&server), ["EHLO", "AUTH", "MAIL", "RCPT", "DATA"]);
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
async fn d_17_a_certificate_for_another_name_ends_implicit_tls_before_any_command() {
    let (der, acceptor) = certificate("other.test");
    let server = serve(Behaviour::SmtpImplicit(acceptor)).await;

    let sent = send(&local().trusting(der), &server, SmtpSecurity::Implicit).await;

    assert_eq!(sent, Err(SendError::Tls));
    server.settle().await;
    assert_eq!(server.lines(), Vec::<String>::new());
}

#[tokio::test]
async fn p3_003_open_reaches_the_given_address_and_names_the_given_host() {
    let (der, acceptor) = certificate("mail.invalid");
    let server = serve(Behaviour::SmtpImplicit(acceptor)).await;
    let tls = local()
        .trusting(der)
        .tls_parameters("mail.invalid")
        .unwrap();
    let address: SocketAddr = ([127, 0, 0, 1], server.port).into();

    // `.invalid` never resolves (RFC 6761): the connection reached the address it was given,
    // and the handshake named the host it was given.
    let mut connection = open(&[address], SmtpSecurity::Implicit, tls).await.unwrap();
    connection.abort().await;

    server.settle().await;
    assert_eq!(server.connections(), 1);
    assert_eq!(server.server_names(), ["mail.invalid"]);
}

#[tokio::test]
async fn d_17_without_a_login_the_message_goes_over_tls_with_no_auth() {
    let (der, acceptor) = certificate("localhost");
    let server = serve(Behaviour::SmtpImplicit(acceptor)).await;
    let target = target(&server, SmtpSecurity::Implicit);

    let sent = local()
        .trusting(der)
        .send_mail(&target, None, &message())
        .await;

    assert_eq!(sent, Ok(()));
    server.settle().await;
    assert_eq!(server.server_names(), ["localhost"]);
    assert_eq!(verbs(&server), ["EHLO", "MAIL", "RCPT", "DATA"]);
}

#[tokio::test]
async fn d_17_a_refused_login_is_its_reply_code_and_sends_no_message() {
    let (der, acceptor) = certificate("localhost");
    let server = serve(Behaviour::SmtpRefusingLogin(acceptor)).await;

    let sent = send(&local().trusting(der), &server, SmtpSecurity::Implicit).await;

    assert_eq!(sent, Err(SendError::SmtpReply(535)));
    server.settle().await;
    assert_eq!(verbs(&server)[..2], ["EHLO", "AUTH"]);
    assert!(!verbs(&server).contains(&"MAIL".to_owned()));
}

#[tokio::test]
async fn d_17_a_root_the_hub_cannot_read_is_a_setup_failure_with_no_connection() {
    let server = serve(Behaviour::SmtpPlain).await;
    let egress = local().trusting(b"not a certificate".to_vec());

    let sent = send(&egress, &server, SmtpSecurity::Implicit).await;

    assert_eq!(sent, Err(SendError::Setup));
    server.settle().await;
    assert_eq!(server.connections(), 0);
}

/// How late past the time limit a send may end on a loaded machine.
const SLACK: Duration = Duration::from_secs(3);

#[tokio::test]
async fn d_17_a_mail_server_that_never_answers_ends_at_the_time_limit() {
    let server = serve(Behaviour::Silent).await;
    let started = Instant::now();

    let sent = send(&local(), &server, SmtpSecurity::Implicit).await;

    assert_eq!(sent, Err(SendError::TimedOut));
    let took = started.elapsed();
    assert!(took >= TIME_LIMIT.saturating_sub(Duration::from_millis(100)), "{took:?}");
    assert!(took < TIME_LIMIT + SLACK, "{took:?}");
    server.settle().await;
    assert_eq!(server.connections(), 1);
}

#[tokio::test]
async fn d_16_a_queued_message_is_sent_though_quit_is_never_answered() {
    let (der, acceptor) = certificate("localhost");
    let server = serve(Behaviour::SmtpSilentAtQuit(acceptor)).await;
    let started = Instant::now();

    let sent = send(&local().trusting(der), &server, SmtpSecurity::Implicit).await;

    // The 250 after DATA decided (S-A): a retry would send the mail twice. QUIT got only what
    // was left of the one time limit.
    assert_eq!(sent, Ok(()));
    assert!(started.elapsed() < TIME_LIMIT + SLACK, "{:?}", started.elapsed());
    server.settle().await;
    assert_eq!(server.connections(), 1);
    assert_eq!(verbs(&server), ["EHLO", "AUTH", "MAIL", "RCPT", "DATA"]);
    assert_eq!(server.lines().last().map(String::as_str), Some("QUIT"));
}

#[tokio::test]
async fn log_007_the_greeting_names_no_host_of_the_hub() {
    let (der, acceptor) = certificate("localhost");
    let server = serve(Behaviour::SmtpStartTls(acceptor)).await;

    let sent = send(&local().trusting(der), &server, SmtpSecurity::StartTls).await;

    assert_eq!(sent, Ok(()));
    server.settle().await;
    let greetings: Vec<String> = server
        .lines()
        .into_iter()
        .filter(|line| line.starts_with("EHLO"))
        .collect();
    assert_eq!(greetings, ["EHLO [127.0.0.1]", "EHLO [127.0.0.1]"]);
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
        ["EHLO", "STARTTLS", "EHLO", "AUTH", "MAIL", "RCPT", "DATA"]
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
#[ignore = "lettre STARTTLS response injection; known limitation, WP-45 issue #1"]
async fn d_17_a_reply_injected_before_starttls_is_not_read_as_a_tls_reply() {
    let (der, acceptor) = certificate("localhost");
    let server = serve(Behaviour::SmtpStartTlsInjecting(acceptor)).await;

    let sent = send(&local().trusting(der), &server, SmtpSecurity::StartTls).await;

    // The 554 came in plaintext with the STARTTLS answer; a client that discards what it
    // buffered before the handshake never sees it.
    assert_ne!(sent, Err(SendError::SmtpReply(554)), "{:?}", verbs(&server));
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
