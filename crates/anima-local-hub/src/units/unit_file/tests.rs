use std::path::{Path, PathBuf};

use anima_database::Platform;

use super::{Launch, RenderError, file, render};

fn launch(root: &str) -> Launch {
    Launch {
        program: PathBuf::from(format!("{root}/programs/current/bin/anima-local-hub")),
        args: [
            "run",
            "--root",
            root,
            "--socket-folder",
            "/run/user/1000/anima",
        ]
        .map(str::to_owned)
        .to_vec(),
    }
}

fn agent(root: &str) -> plist::Dictionary {
    let text = render(Platform::MacOs, &launch(root)).unwrap();
    plist::from_bytes::<plist::Value>(text.as_bytes())
        .unwrap()
        .into_dictionary()
        .unwrap()
}

fn unit_value(text: &str, key: &str) -> String {
    text.lines()
        .find_map(|line| line.strip_prefix(&format!("{key}=")))
        .unwrap_or_else(|| panic!("no {key} in the unit"))
        .to_owned()
}

#[test]
fn each_user_has_one_unit_file() {
    assert_eq!(
        file(Platform::MacOs, Path::new("/Users/ada")),
        PathBuf::from("/Users/ada/Library/LaunchAgents/ing.anima.hub.plist")
    );
    assert_eq!(
        file(Platform::Linux, Path::new("/home/ada")),
        PathBuf::from("/home/ada/.config/systemd/user/anima-hub.service")
    );
}

#[test]
fn the_launch_agent_starts_at_login_and_again_only_after_a_failure() {
    let agent = agent("/Users/ada/Library/Application Support/ing.anima.desktop/hub");

    assert_eq!(agent["Label"].as_string(), Some("ing.anima.hub"));
    assert_eq!(agent["RunAtLoad"].as_boolean(), Some(true));
    let keep_alive = agent["KeepAlive"].as_dictionary().unwrap();
    assert_eq!(keep_alive["SuccessfulExit"].as_boolean(), Some(false));
    assert_eq!(agent["ThrottleInterval"].as_signed_integer(), Some(10));
    let core = agent["HardResourceLimits"].as_dictionary().unwrap();
    assert_eq!(core["Core"].as_signed_integer(), Some(0));
}

#[test]
fn dbl_012_the_unit_gives_the_group_more_time_to_stop_than_its_members_need() {
    let limits = crate::group::LIMITS;
    let members = limits.hub_stop + limits.gateway_stop + limits.database_stop;
    let agent = agent("/Users/ada/hub");
    let unit = render(Platform::Linux, &launch("/home/ada/hub")).unwrap();

    let launchd = agent["ExitTimeOut"].as_unsigned_integer().unwrap();
    let systemd: u64 = unit_value(&unit, "TimeoutStopSec").parse().unwrap();

    assert!(launchd > members.as_secs() && systemd > members.as_secs());
    assert!(limits.database_stop.as_secs() >= 30);
}

#[test]
fn a_path_with_a_space_or_a_markup_character_stays_one_argument_of_the_launch_agent() {
    let root = "/Users/a&b <c>/Library/Application Support/hub";

    let agent = agent(root);

    let words: Vec<&str> = agent["ProgramArguments"]
        .as_array()
        .unwrap()
        .iter()
        .map(|word| word.as_string().unwrap())
        .collect();
    assert_eq!(
        words,
        [
            &format!("{root}/programs/current/bin/anima-local-hub"),
            "run",
            "--root",
            root,
            "--socket-folder",
            "/run/user/1000/anima"
        ]
    );
}

#[test]
fn the_systemd_unit_runs_the_program_once_and_lets_it_stop_its_own_group() {
    let unit = render(Platform::Linux, &launch("/home/ada/.local/share/anima/hub")).unwrap();

    assert_eq!(
        unit_value(&unit, "ExecStart"),
        "\"/home/ada/.local/share/anima/hub/programs/current/bin/anima-local-hub\" \"run\" \
         \"--root\" \"/home/ada/.local/share/anima/hub\" \"--socket-folder\" \
         \"/run/user/1000/anima\""
    );
    assert_eq!(unit_value(&unit, "Restart"), "on-failure");
    assert_eq!(unit_value(&unit, "KillMode"), "mixed");
    assert_eq!(unit_value(&unit, "LimitCORE"), "0");
    assert_eq!(unit_value(&unit, "WantedBy"), "default.target");
}

#[test]
fn systemd_reads_no_specifier_and_no_variable_in_a_path() {
    let unit = render(Platform::Linux, &launch("/home/a%h $HOME/hub")).unwrap();

    assert!(unit_value(&unit, "ExecStart").contains("\"/home/a%%h $$HOME/hub\""));
}

#[test]
fn a_path_that_would_change_the_meaning_of_the_unit_is_refused() {
    for root in ["/home/a\"b", "/home/a\\b", "/home/a'b"] {
        assert!(
            matches!(
                render(Platform::Linux, &launch(root)),
                Err(RenderError::Path(_))
            ),
            "{root}"
        );
    }
    for platform in [Platform::MacOs, Platform::Linux] {
        let broken = render(platform, &launch("/home/a\nExecStartPre=/bin/x"));
        assert!(matches!(broken, Err(RenderError::Path(_))), "{platform:?}");
    }
}
