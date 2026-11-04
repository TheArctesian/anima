//! The value rules: secrets inside strings under any key and in event text (LOG-007, D-3).

use serde_json::json;

use super::PEM;
use crate::events::redaction::{redact, redact_text};

/// Asserts that each `(text, expected)` pair is masked as expected.
fn masks(cases: &[(&str, &str)]) {
    for (text, expected) in cases {
        assert_eq!(redact_text(text), *expected, "{text}");
    }
}

/// Asserts that each text is kept as it is.
fn keeps(texts: &[&str]) {
    for text in texts {
        assert_eq!(redact_text(text), *text);
    }
}

#[test]
fn log_007_redaction_hides_private_keys_and_otpauth_uris_under_any_key() {
    let change = json!({
        "notes": PEM,
        "lines": ["fine", PEM],
        "enrolment": "otpauth://totp/anima:operator?secret=JBSWY3DPEHPK3PXP",
    });

    assert_eq!(
        redact(change),
        json!({ "notes": "[redacted]", "lines": ["fine", "[redacted]"], "enrolment": "[redacted]" })
    );
    masks(&[
        (PEM, "[redacted]"),
        (
            "curl -H 'Authorization: Bearer x' https://hub",
            "[redacted]",
        ),
        ("see OTPAUTH://totp/x?secret=Y", "[redacted]"),
        ("PuTTY-User-Key-File-3: ssh-ed25519", "[redacted]"),
    ]);
}

#[test]
fn log_007_redaction_masks_only_the_credential_after_bearer_basic_or_token() {
    masks(&[
        ("Bearer eyJhbGciOi", "Bearer [redacted]"),
        ("Bearer\tabc123def456", "Bearer\t[redacted]"),
        ("  basic YWRtaW46aHVudGVyMg==", "  basic [redacted]"),
        (
            "upstream refused header Bearer eyJhbGciOiJIUzI1NiJ9 twice",
            "upstream refused header Bearer [redacted] twice",
        ),
        ("token: dXNlcjpodW50ZXIy", "token: [redacted]"),
        ("TOKEN:abc12345xyz", "TOKEN:[redacted]"),
        ("(bearer ghp_AAAAsecret)", "(bearer [redacted])"),
        ("token: ab12cd34", "token: [redacted]"),
        ("basic a+b/c=de", "basic [redacted]"),
    ]);
    keeps(&[
        "Basic connectivity check passed",
        "Basic tier",
        "not a Bearer token",
        "basically fine",
        "authorization pending",
        "Bearer Required",
        "token expired after a restart",
        "token 1234567",
        "tokenization2024 run",
    ]);
    assert_eq!(
        redact(json!({ "plan": "Basic tier" })),
        json!({ "plan": "Basic tier" })
    );
}

#[test]
fn log_007_redaction_masks_a_jwt_anywhere() {
    masks(&[
        (
            "session eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxIn0.sig-_x ok",
            "session [redacted] ok",
        ),
        ("(eyJhbGciOiJub25lIn0.eyJzdWIiOiIxIn0.)", "([redacted])"),
        ("eyJa.eyJb.c.", "[redacted]."),
        ("eyJa.b..", "[redacted]."),
    ]);
    keeps(&["eyJa.b", "keyJa.b.c", "eyJ..c", "eyJa.b.c.d"]);
}

#[test]
fn log_007_redaction_masks_secret_key_value_pairs() {
    masks(&[
        (
            "host=db password=hunter2 user=anima",
            "host=db password=[redacted] user=anima",
        ),
        (
            "jdbc:postgresql://db/anima?user=a&password=x1&ssl=true",
            "jdbc:postgresql://db/anima?user=a&password=[redacted]&ssl=true",
        ),
        ("Pwd=s3cr3t;Server=db", "Pwd=[redacted];Server=db"),
        ("passwd=a,b", "passwd=[redacted],b"),
        (
            "password='hunter 2' host=db",
            "password='[redacted]' host=db",
        ),
        ("id_token=abc", "id_token=[redacted]"),
    ]);
    keeps(&[
        "cwd=/tmp",
        "OLDPWD=/home/op",
        "passwordless=true",
        "hasPassword=False",
        "password=",
        "password='' x",
        "password: {\"a\": 1}",
        "status: down, port=22",
        "password: [1, 2]",
        "public.key=AAAAC3Nza1",
        "public_key=AAAAC3Nza1",
        "public-key=AAAAC3Nza1",
    ]);
}

/// Any key that names a secret by the JSON-key rules, with `=` or `:` and spaces or quotes
/// around the separator, including embedded JSON (S2).
#[test]
fn log_007_redaction_masks_values_of_text_keys_named_like_secrets() {
    masks(&[
        ("password = hunter2", "password = [redacted]"),
        ("password= hunter2", "password= [redacted]"),
        ("password: hunter2", "password: [redacted]"),
        ("PGPASSWORD=hunter2 psql", "PGPASSWORD=[redacted] psql"),
        ("dbPassword=hunter2", "dbPassword=[redacted]"),
        (
            "wss://hub/tunnel?accessToken=abc123def",
            "wss://hub/tunnel?accessToken=[redacted]",
        ),
        ("api_key=abc123def456", "api_key=[redacted]"),
        ("secret=abc123def456", "secret=[redacted]"),
        ("client_secret=x&id=1", "client_secret=[redacted]&id=1"),
        (
            "body {\"password\":\"hunter2\",\"user\":\"a\"}",
            "body {\"password\":\"[redacted]\",\"user\":\"a\"}",
        ),
        (
            "body {\"token\":\"ghp_AAAAsecret1\"}",
            "body {\"token\":\"[redacted]\"}",
        ),
        ("{\"pin\": 1234}", "{\"pin\": [redacted]}"),
        ("token: \"abc123def456\"", "token: \"[redacted]\""),
        ("'password' : 'hunter2'", "'password' : '[redacted]'"),
        (
            "spring.datasource.password=x",
            "spring.datasource.password=[redacted]",
        ),
        ("Bearer \"ghp_AAAAsecret1\"", "Bearer \"[redacted]\""),
        ("password=\"abc def", "password=\"[redacted] def"),
        ("sshCreds=hunter2 ok", "sshCreds=[redacted] ok"),
        ("password\t= hunter2", "password\t= [redacted]"),
        (
            "run \"--password=hunter2\" now",
            "run \"--password=[redacted]\" now",
        ),
        (
            "run '--password=hunter2' now",
            "run '--password=[redacted]' now",
        ),
        ("password=token=x next", "password=[redacted] next"),
    ]);
    let long = format!("password=\"{} end\" next", "a".repeat(300));
    assert_eq!(redact_text(&long), "password=\"[redacted]\" next");
}

/// Inputs built to make a scan quadratic finish in linear time (S1). 200 KB each; a
/// quadratic scan took seconds in a release build, the bound leaves wide room for a debug one.
#[test]
fn log_007_redaction_of_pathological_text_takes_linear_time() {
    let units = [
        "eyJ.",
        "://",
        "://a:b",
        "eyJa.",
        "token.",
        "token ",
        "a=",
        "password=\"",
        "x:",
        "Bearer \"",
        "@",
        "password=a",
    ];
    for unit in units {
        let text = unit.repeat(200_000 / unit.len());
        let started = std::time::Instant::now();
        let _ = redact_text(&text);
        let elapsed = started.elapsed();
        assert!(
            elapsed < std::time::Duration::from_secs(2),
            "{unit:?}: {elapsed:?}"
        );
    }
}

#[test]
fn log_007_redaction_masks_the_password_in_a_url() {
    masks(&[
        (
            "postgres://anima:hunter2@db:5432/anima",
            "postgres://anima:[redacted]@db:5432/anima",
        ),
        (
            "https://a:b@one.example/x and ftp://c:d:e@two.example",
            "https://a:[redacted]@one.example/x and ftp://c:[redacted]@two.example",
        ),
        (
            "https://u:pa/ss@host.example/x",
            "https://u:[redacted]@host.example/x",
        ),
        (
            "postgres://u:p?w#d@db/anima ok",
            "postgres://u:[redacted]@db/anima ok",
        ),
        (
            "postgres://u:p@ss@db/anima",
            "postgres://u:[redacted]@db/anima",
        ),
        (
            "https://host.example/?next=https://v:q@k.example",
            "https://host.example/?next=https://v:[redacted]@k.example",
        ),
        (
            "ftp://john@example.com:hunter2@ftp.example/x",
            "ftp://john@example.com:[redacted]@ftp.example/x",
        ),
        (
            "https://ghp_AAAAsecret@github.com/o/r",
            "https://[redacted]@github.com/o/r",
        ),
        ("http://u:p@[::1]:8080/", "http://u:[redacted]@[::1]:8080/"),
        ("https://u:/pw@h", "https://u:[redacted]@h"),
    ]);
    keeps(&[
        "https://user@host.example/path",
        "https://host.example/a:b@c",
        "https://a.example:443/x,https://b.example/@me",
        "http://[::1]:8080/@me",
        "rtsp://cam.local:554/stream@main",
        "ssh://root@host",
        "imap://first.last%40example.com@mail.example",
        "ssh://deploy-user@host:22",
        "see https://example.com:abc",
        "see https://example.com:abc then me@x.example",
        "https://h.example:8443?q=a@b",
        "https://h.example:8443#a@b",
    ]);
}

#[test]
fn log_007_redaction_masks_a_scheme_credential_inside_a_secret_key_value() {
    masks(&[
        (
            "authorization=Basic dXNlcjpwYXNz",
            "authorization=[redacted] [redacted]",
        ),
        (
            "Authorization : Basic dXNlcjpwYXNz",
            "Authorization : [redacted] [redacted]",
        ),
        (
            "password: Basic dXNlcjpwYXNz",
            "password: [redacted] [redacted]",
        ),
        (
            "token=Bearer ghp_AAAAsecret1",
            "token=[redacted] [redacted]",
        ),
        (
            "{\"password\":\n  \"hunter2\"}",
            "{\"password\":\n  \"[redacted]\"}",
        ),
    ]);
}
