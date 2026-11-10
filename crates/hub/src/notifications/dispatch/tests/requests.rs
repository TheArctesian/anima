//! What each kind puts on the wire, before any connection (NOT-002, D-18, N-4).

use reqwest::Method;
use serde_json::{Value, json};

use super::{DISCORD, apprise, apprise_key, email, ntfy, secret, webhook};
use crate::notifications::{
    DeliveryReason, NotificationKind,
    channels::{AppriseSecrets, EmailSettings, Secrets, Settings},
    dispatch::{
        failure::Failure,
        outgoing::{Notice, Outgoing, outgoing},
    },
    egress::{
        HttpRequest,
        tests::servers::{Behaviour, Server, serve},
    },
    template::{self, Rendered, Values},
};

fn values() -> Values {
    Values {
        device: Some("pve-01".to_owned()),
        check: Some("web".to_owned()),
        status: Some("down".to_owned()),
        previous: Some("up".to_owned()),
        message: Some("timed out".to_owned()),
        at: Some("2026-11-08T10:00:00Z".to_owned()),
        days: None,
    }
}

fn rendered(values: &Values) -> Rendered {
    template::render("{check} on {device} is {status}", "{message}", values).unwrap()
}

/// What `notice` becomes through a channel with `settings` and `secrets`.
fn request((settings, secrets): (Settings, Secrets)) -> Result<Outgoing, Failure> {
    let values = values();
    let rendered = rendered(&values);
    let notice = Notice {
        kind: NotificationKind::Down,
        values: &values,
        rendered: &rendered,
    };
    outgoing(&settings, Some(secrets), &notice)
}

fn http(outgoing: Result<Outgoing, Failure>) -> HttpRequest {
    match outgoing {
        Ok(Outgoing::Http(request)) => request,
        Ok(Outgoing::Mail { .. }) => panic!("a mail, not an HTTP request"),
        Err(failure) => panic!("refused: {failure:?}"),
    }
}

fn header<'a>(request: &'a HttpRequest, name: &str) -> &'a str {
    request.headers[name].to_str().unwrap()
}

fn json_body(request: &HttpRequest) -> Value {
    serde_json::from_slice(&request.body).unwrap()
}

/// A server that is never contacted: the fixture builders read only its port.
async fn nowhere() -> Server {
    serve(Behaviour::Silent).await
}

#[tokio::test]
async fn not_002_a_webhook_posts_its_json_with_its_headers_to_its_whole_url() {
    let server = nowhere().await;

    let request = http(request(webhook(&server)));

    assert_eq!(request.method, Method::POST);
    assert_eq!(
        request.url.as_str(),
        format!(
            "https://localhost:{}/hooks/synthetic-path-secret",
            server.port
        )
    );
    assert_eq!(header(&request, "content-type"), "application/json");
    assert_eq!(header(&request, "x-token"), "synthetic-header-secret");
    let body = json_body(&request);
    assert_eq!(body["kind"], "down");
    assert_eq!(body["title"], "web on pve-01 is down");
    assert_eq!(body["body"], "timed out");
}

#[tokio::test]
async fn not_002_ntfy_posts_the_text_to_the_topic_with_title_priority_and_token() {
    let server = nowhere().await;

    let request = http(request(ntfy(&server)));

    assert_eq!(request.method, Method::POST);
    assert_eq!(
        request.url.as_str(),
        format!("https://localhost:{}/synthetic-topic-secret", server.port)
    );
    assert_eq!(header(&request, "title"), "web on pve-01 is down");
    assert_eq!(header(&request, "priority"), "4");
    assert_eq!(
        header(&request, "authorization"),
        "Bearer synthetic-token-secret"
    );
    assert_eq!(request.body, b"timed out");
}

#[tokio::test]
async fn not_002_a_topic_is_one_path_segment() {
    let server = nowhere().await;
    let (settings, _) = ntfy(&server);
    let secrets = Secrets::Ntfy(crate::notifications::channels::NtfySecrets {
        topic: secret("a/../b?c"),
        token: None,
    });

    let request = http(request((settings, secrets)));

    assert_eq!(request.url.path(), "/a%2F..%2Fb%3Fc");
    assert!(request.headers.get("authorization").is_none());
}

#[tokio::test]
async fn not_002_an_email_goes_to_its_server_from_its_sender_to_its_recipients() {
    let server = nowhere().await;

    let Ok(Outgoing::Mail {
        server: mail_server,
        login,
        message,
    }) = request(email(&server))
    else {
        panic!("not a mail");
    };

    assert_eq!(mail_server.host, "localhost");
    assert_eq!(mail_server.port, server.port);
    assert!(login.is_some());
    let text = String::from_utf8(message.formatted()).unwrap();
    assert!(text.contains("Subject: web on pve-01 is down"), "{text}");
    assert!(text.contains("From: anima@example.test"), "{text}");
    assert!(text.contains("To: ops@example.test"), "{text}");
    assert!(text.contains("timed out"), "{text}");
    assert!(!text.contains("synthetic-password-secret"), "{text}");
}

#[tokio::test]
async fn not_002_an_email_without_a_stored_password_logs_in_to_nothing() {
    let server = nowhere().await;
    let (Settings::Email(settings), _) = email(&server) else {
        unreachable!()
    };
    let settings = Settings::Email(EmailSettings {
        password_stored: false,
        ..settings
    });
    let values = values();
    let rendered = rendered(&values);
    let notice = Notice {
        kind: NotificationKind::Down,
        values: &values,
        rendered: &rendered,
    };

    let Ok(Outgoing::Mail { login, .. }) = outgoing(&settings, None, &notice) else {
        panic!("not a mail");
    };

    assert!(login.is_none());
}

#[tokio::test]
async fn not_002_an_unreadable_mail_address_is_bad_address() {
    let server = nowhere().await;
    let (Settings::Email(settings), secrets) = email(&server) else {
        unreachable!()
    };
    let settings = Settings::Email(EmailSettings {
        to: vec!["not an address".to_owned()],
        ..settings
    });

    let refused = request((settings, secrets)).err();

    assert_eq!(refused, Some(Failure::of(DeliveryReason::BadAddress)));
}

#[tokio::test]
async fn d_18_stateless_apprise_posts_its_urls_to_notify_and_never_an_attachment() {
    let server = nowhere().await;

    let request = http(request(apprise(&server, &[DISCORD, DISCORD])));

    assert_eq!(request.method, Method::POST);
    assert_eq!(
        request.url.as_str(),
        format!("https://localhost:{}/notify", server.port)
    );
    let body = json_body(&request);
    assert_eq!(body["urls"], json!(format!("{DISCORD},{DISCORD}")));
    assert_eq!(body["title"], "web on pve-01 is down");
    assert_eq!(body["tag"], "ops");
    let mut keys: Vec<&String> = body.as_object().unwrap().keys().collect();
    keys.sort();
    assert_eq!(keys, ["body", "format", "tag", "title", "type", "urls"]);
}

#[tokio::test]
async fn d_18_config_key_apprise_posts_to_notify_under_its_key() {
    let server = nowhere().await;

    let request = http(request(apprise_key(&server)));

    assert_eq!(
        request.url.as_str(),
        format!(
            "https://localhost:{}/notify/synthetic-key-secret",
            server.port
        )
    );
    let body = json_body(&request);
    assert!(body.get("urls").is_none());
    assert!(body.get("attach").is_none() && body.get("attachment").is_none());
}

#[tokio::test]
async fn n_4_a_stored_apprise_url_no_longer_permitted_is_refused_before_any_send() {
    let server = nowhere().await;
    for refused in [
        "json://internal.example/hook",
        "discord://synthetic-apprise-secret/x?attach=file:///etc/passwd",
        "https://example.test/",
    ] {
        let failure = request(apprise(&server, &[DISCORD, refused])).err();

        assert_eq!(
            failure,
            Some(Failure::of(DeliveryReason::TargetNotPermitted)),
            "{refused}"
        );
    }
    assert_eq!(server.connections(), 0);
}

#[tokio::test]
async fn sec_005_secrets_of_another_kind_are_secret_unreadable() {
    let server = nowhere().await;
    let (settings, _) = webhook(&server);
    let (_, other) = ntfy(&server);

    assert_eq!(
        request((settings, other)).err(),
        Some(Failure::of(DeliveryReason::SecretUnreadable))
    );
}

#[tokio::test]
async fn sec_005_an_apprise_mode_without_its_secret_is_secret_unreadable() {
    let server = nowhere().await;
    let (settings, _) = apprise(&server, &[DISCORD]);

    let failure = request((settings, Secrets::Apprise(AppriseSecrets::Urls(Vec::new())))).err();

    assert_eq!(failure, Some(Failure::of(DeliveryReason::SecretUnreadable)));
}
