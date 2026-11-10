//! The code and origin checks, the destination, folder trust, the agent's environment and its
//! arguments are pure; `tests/flow.rs` installs a stand-in agent into a temporary HOME inside
//! the crate's `target/` folder, so no real `~/.local`, systemctl or launchctl is touched.

mod flow;

use std::path::Path;

use super::{
    AgentError, Installing,
    placement::{bundled_beside, destination, folder_trusted},
    request::{EnrolmentCode, HubOrigin},
    run::{INSTALL_ARGS, agent_env, enrol_args},
};

const CODE: &str = "anima_enrol_AbCdEfGhIjKlMnOpQrSt-_";

#[test]
fn met_019_a_code_is_the_prefix_and_22_base64url_characters() {
    assert!(EnrolmentCode::parse(CODE).is_some());
    assert!(EnrolmentCode::parse("anima_enrol_0123456789abcdefghijkl").is_some());
    for refused in [
        "",
        "anima_enrol_",
        "anima_enrol_AbCdEfGhIjKlMnOpQrStU",
        "anima_enrol_AbCdEfGhIjKlMnOpQrStUvW",
        "anima_enrol_AbCdEfGhIjKlMnOpQrSt+/",
        "anima_enrol_AbCdEfGhIjKlMnOpQrStU=",
        "anima_enrol_AbCdEfGhIjKlMnOpQrStU ",
        "anima_enrol_AbCdEfGhIjKlMnOpQrStU\n",
        "anima_enrol_AbCdEfGhIjKlMnOpQrStUé",
        "Anima_enrol_AbCdEfGhIjKlMnOpQrStUv",
        " anima_enrol_AbCdEfGhIjKlMnOpQrStUv",
        "anima_token_AbCdEfGhIjKlMnOpQrStUv",
    ] {
        assert!(EnrolmentCode::parse(refused).is_none(), "{refused:?}");
    }
}

#[test]
fn log_007_the_code_reaches_only_its_input_line_and_never_its_debug_text() {
    let code = EnrolmentCode::parse(CODE).expect("a code");
    assert_eq!(&code.input_line()[..], format!("{CODE}\n").as_bytes());
    let shown = format!("{code:?}");
    assert!(!shown.contains("anima_enrol"), "{shown}");
}

#[test]
fn met_020_the_origin_is_https_and_a_host_on_port_443() {
    for (given, sent) in [
        ("https://hub.anima.ing", "https://hub.anima.ing"),
        ("https://hub.anima.ing:443", "https://hub.anima.ing"),
        ("https://HUB.Anima.ing", "https://hub.anima.ing"),
        ("https://hub.anima.ing:", "https://hub.anima.ing"),
        ("https://100.64.0.1", "https://100.64.0.1"),
    ] {
        let origin = HubOrigin::parse(given).expect(given);
        assert_eq!(origin.as_str(), sent, "{given}");
    }
}

#[test]
fn met_020_an_origin_with_more_than_a_host_or_another_scheme_or_port_is_refused() {
    for refused in [
        "",
        "hub.anima.ing",
        "http://hub.anima.ing",
        "HTTPS://hub.anima.ing",
        "https://",
        "https://hub.anima.ing/",
        "https://hub.anima.ing/graphql",
        "https://hub.anima.ing?x=1",
        "https://hub.anima.ing#x",
        "https://user@hub.anima.ing",
        "https://user:pw@hub.anima.ing",
        "https://hub.anima.ing:8443",
        "https://hub.anima.ing\\evil",
        "https://hub.an\nima.ing",
        "https://hub.an\tima.ing",
        "https://hub anima.ing",
        "https://hub.anima.ing:99999",
    ] {
        assert!(HubOrigin::parse(refused).is_none(), "{refused:?}");
    }
}

#[test]
fn met_019_the_agent_goes_to_local_bin_and_is_found_beside_the_app() {
    let home = Path::new("/home/ada");
    assert_eq!(
        destination(home),
        Path::new("/home/ada/.local/bin/anima-agent")
    );
    let app = Path::new("/Applications/Anima.app/Contents/MacOS/anima-desktop");
    assert_eq!(
        bundled_beside(app).as_deref(),
        Some(Path::new(
            "/Applications/Anima.app/Contents/MacOS/anima-agent"
        ))
    );
}

#[test]
fn met_019_a_folder_is_trusted_when_the_user_or_root_owns_it_and_others_cannot_write() {
    let (user, other) = (1000, 1001);
    assert!(folder_trusted(0o040_755, user, user));
    assert!(folder_trusted(0o040_700, user, user));
    assert!(folder_trusted(0o040_755, 0, user));
    // Group write is left to the agent's own install check, which knows private groups.
    assert!(folder_trusted(0o040_775, user, user));
    assert!(!folder_trusted(0o040_755, other, user));
    assert!(!folder_trusted(0o040_777, user, user));
    assert!(!folder_trusted(0o041_777, 0, user));
    assert!(!folder_trusted(0o040_757, user, user));
}

#[test]
fn met_019_the_agent_gets_home_and_the_session_bus_only() {
    let everything = |name: &str| Some(format!("{name}-value"));
    let env = agent_env(Path::new("/home/ada"), everything);
    assert_eq!(
        env,
        vec![
            ("HOME", "/home/ada".to_owned()),
            ("XDG_RUNTIME_DIR", "XDG_RUNTIME_DIR-value".to_owned()),
            (
                "DBUS_SESSION_BUS_ADDRESS",
                "DBUS_SESSION_BUS_ADDRESS-value".to_owned()
            ),
        ]
    );
    let empty = agent_env(Path::new("/home/ada"), |_| Some(String::new()));
    assert_eq!(empty, vec![("HOME", "/home/ada".to_owned())]);
}

#[test]
fn log_007_the_code_is_never_an_argument_and_install_is_user_level() {
    let origin = HubOrigin::parse("https://hub.anima.ing").expect("an origin");
    assert_eq!(
        enrol_args(origin.as_str()),
        ["enrol", "--hub", "https://hub.anima.ing", "-"]
    );
    assert_eq!(INSTALL_ARGS, ["install"]);
}

#[test]
fn log_007_every_error_is_a_code_and_fixed_text() {
    let all = [
        AgentError::Unavailable,
        AgentError::NotBundled,
        AgentError::Busy,
        AgentError::Code,
        AgentError::Origin,
        AgentError::NoHome,
        AgentError::Untrusted,
        AgentError::Copy,
        AgentError::Enrol,
        AgentError::Install,
        AgentError::TimedOut,
        AgentError::Failed,
    ];
    for error in all {
        let json = serde_json::to_value(error).expect("serialised");
        assert_eq!(json["code"], error.code());
        assert_eq!(json["message"], error.to_string());
        assert_eq!(json.as_object().map(serde_json::Map::len), Some(2));
    }
}

#[test]
fn met_019_one_install_runs_at_a_time() {
    let installing = Installing::default();
    let first = installing.claim().expect("free");
    assert_eq!(installing.claim().err(), Some(AgentError::Busy));
    drop(first);
    assert!(installing.claim().is_ok());
}
