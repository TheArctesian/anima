//! The code and origin checks, the destination, the agent's environment and its arguments are
//! pure; `tests/trust.rs` holds the folder rule; `tests/flow.rs` installs a stand-in agent into a
//! temporary HOME inside the crate's own `target/` folder, so no real `~/.local`, systemctl or
//! launchctl is touched.

mod flow;
mod folders;
mod rig;
mod trust;

use std::{
    ffi::OsString,
    fs,
    path::Path,
    sync::atomic::Ordering,
    thread,
    time::{Duration, Instant},
};

use super::{
    AgentError, Installing, bundled,
    placement::{bundled_beside, destination},
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
fn met_019_the_agent_gets_home_and_the_session_bus_only() {
    let everything = |name: &str| Some(OsString::from(format!("{name}-value")));
    let env = agent_env(Path::new("/home/ada"), everything);
    let os = OsString::from;
    assert_eq!(
        env,
        vec![
            ("HOME", os("/home/ada")),
            ("XDG_RUNTIME_DIR", os("XDG_RUNTIME_DIR-value")),
            (
                "DBUS_SESSION_BUS_ADDRESS",
                os("DBUS_SESSION_BUS_ADDRESS-value")
            ),
        ]
    );
    let empty = agent_env(Path::new("/home/ada"), |_| Some(OsString::new()));
    assert_eq!(empty, vec![("HOME", os("/home/ada"))]);
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
    for error in AgentError::ALL {
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

#[test]
fn met_019_the_untrusted_message_names_the_group_the_acl_and_the_fix() {
    let text = AgentError::Untrusted.to_string();
    for part in ["its group", "an ACL", "`chmod g-w,o-w`", "`setfacl -b`"] {
        assert!(text.contains(part), "{text}");
    }
}

#[test]
fn d_10_only_a_release_build_offers_the_agent_beside_the_app() {
    let scratch = concat!(env!("CARGO_MANIFEST_DIR"), "/target/agent-rig");
    fs::create_dir_all(scratch).expect("the scratch folder");
    let dir = tempfile::tempdir_in(scratch).expect("a folder");
    let app = dir.path().join("anima-desktop");
    assert_eq!(
        bundled(&app, true),
        Err(AgentError::NotBundled),
        "none beside"
    );
    fs::write(dir.path().join("anima-agent"), b"agent").expect("the agent");
    assert_eq!(bundled(&app, true), Ok(dir.path().join("anima-agent")));
    // A debug build's target folder may hold an agent built from another commit.
    assert_eq!(bundled(&app, false), Err(AgentError::NotBundled));
}

#[test]
fn met_019_on_exit_the_stop_reaches_the_running_install_and_waits_for_its_claim() {
    let installing = Installing::default();
    let claim = installing.claim().expect("free");
    let work = thread::spawn(move || {
        let give_up = Instant::now() + Duration::from_secs(5);
        while !claim.stop.load(Ordering::Acquire) && Instant::now() < give_up {
            thread::sleep(Duration::from_millis(5));
        }
        thread::sleep(Duration::from_millis(100));
        drop(claim);
    });
    let started = Instant::now();
    installing.stop();
    assert!(
        started.elapsed() >= Duration::from_millis(100),
        "it did not wait"
    );
    assert!(installing.claim().is_ok(), "the claim was not freed");
    work.join().expect("the work");
}
