//! `anima ssh <device>` with standard input from a pipe (K4 Section 8): no raw mode, the
//! input goes to the device as it is, and the device's output comes back unchanged.

use super::{DEVICE, LOGIN_TOKEN, ended, started};
use crate::kit::Anima;
use crate::kit::fake_hub::refusal;
use crate::kit::tunnel_hub::{Files, TERMINAL_TICKET, Terminal, TunnelHub};

fn speaking(reason: &'static str, code: u16) -> Terminal {
    Terminal::Speak {
        output: b"hello from the device\r\n".to_vec(),
        reason,
        code,
    }
}

#[tokio::test]
async fn k4_8_ssh_relays_piped_input_and_the_devices_output() {
    let hub = TunnelHub::start(speaking("REMOTE_CLOSED", 1000), Files::default()).await;
    hub.answer("SessionStart", started(false))
        .answer("SessionEnd", ended("REMOTE_CLOSED"));
    let anima = Anima::new().with("ANIMA_TOKEN", LOGIN_TOKEN);
    let run = anima
        .run_with_input(&["ssh", DEVICE, "--hub", &hub.origin], "uptime\n")
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    assert_eq!(run.stdout, "hello from the device\r\n");
    let received = hub.received();
    let first: serde_json::Value = serde_json::from_str(&received[0]).expect("JSON first");
    assert_eq!(first["type"], "resize", "K4 8.4: resize comes first");
    assert!(
        received.contains(&"binary:uptime\n".to_owned()),
        "{received:?}"
    );
    assert_eq!(hub.queries(), vec![format!("token={TERMINAL_TICKET}")]);
    assert_eq!(hub.operations(), vec!["SessionStart", "SessionEnd"]);
    run.assert_never_shows(TERMINAL_TICKET);
    run.assert_never_shows(LOGIN_TOKEN);
}

#[tokio::test]
async fn k4_8_6_a_changed_host_key_warns_and_exits_non_zero() {
    let hub = TunnelHub::start(speaking("HOST_KEY_MISMATCH", 4001), Files::default()).await;
    hub.answer("SessionStart", started(true))
        .answer("SessionEnd", ended("HOST_KEY_MISMATCH"));
    let anima = Anima::new().with("ANIMA_TOKEN", LOGIN_TOKEN);
    let run = anima
        .run_with_input(&["ssh", DEVICE, "--hub", &hub.origin], "x")
        .await;
    assert_eq!(run.code, 6, "{run:?}");
    assert!(run.stderr.contains("WARNING"), "{run:?}");
    assert!(
        run.stderr
            .contains(&format!("anima device trust accept-host-key {DEVICE}")),
        "{run:?}"
    );
    assert!(run.stderr.contains("wake packet"), "{run:?}");
    run.assert_never_shows(TERMINAL_TICKET);
}

#[tokio::test]
async fn int_003_an_api_key_is_told_to_sign_in() {
    let hub = TunnelHub::start(Terminal::Hold, Files::default()).await;
    hub.answer("SessionStart", refusal("FORBIDDEN", "Not with an API key."));
    let anima = Anima::new().with("ANIMA_TOKEN", "anima_readwritekey0123");
    let run = anima.run(&["ssh", DEVICE, "--hub", &hub.origin]).await;
    assert_eq!(run.code, 5, "{run:?}");
    assert!(run.stderr.contains("anima auth login"), "{run:?}");
    assert!(hub.queries().is_empty(), "no tunnel without a ticket");
}

#[tokio::test]
async fn api_009_a_spent_ticket_is_refused_and_named_nowhere() {
    let hub = TunnelHub::start(speaking("REMOTE_CLOSED", 1000), Files::default()).await;
    hub.answer("SessionStart", started(false))
        .answer("SessionEnd", ended("REMOTE_CLOSED"));
    let anima = Anima::new().with("ANIMA_TOKEN", LOGIN_TOKEN);
    let first = anima
        .run_with_input(&["ssh", DEVICE, "--hub", &hub.origin], "a")
        .await;
    assert_eq!(first.code, 0, "{first:?}");
    let again = anima
        .run_with_input(&["ssh", DEVICE, "--json", "--hub", &hub.origin], "a")
        .await;
    assert_eq!(again.code, 5, "{again:?}");
    assert_eq!(again.error()["code"], "FORBIDDEN");
    again.assert_never_shows(TERMINAL_TICKET);
}
