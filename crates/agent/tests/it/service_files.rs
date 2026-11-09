//! The service files parse and name the binary and `run` (MET-019, MET-020).

use anima_agent::service_files::{LAUNCH_AGENT, SYSTEMD_SYSTEM, SYSTEMD_USER};

/// The `key=value` lines of a unit with the section they are in.
fn unit(text: &str) -> Vec<(String, String, String)> {
    let mut section = String::new();
    let mut found = Vec::new();
    for line in text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
    {
        if let Some(name) = line
            .strip_prefix('[')
            .and_then(|rest| rest.strip_suffix(']'))
        {
            name.clone_into(&mut section);
        } else {
            let (key, value) = line.split_once('=').expect("every unit line is key=value");
            found.push((section.clone(), key.to_owned(), value.to_owned()));
        }
    }
    found
}

fn value<'a>(lines: &'a [(String, String, String)], key: &str) -> Option<&'a str> {
    lines
        .iter()
        .find(|(_, name, _)| name == key)
        .map(|(_, _, value)| value.as_str())
}

#[test]
fn met_019_the_systemd_units_parse_and_run_the_binary_with_run() {
    for text in [SYSTEMD_USER, SYSTEMD_SYSTEM] {
        let lines = unit(text);
        let exec = value(&lines, "ExecStart").expect("an ExecStart");
        assert!(exec.ends_with("/anima-agent run"), "{exec}");
        assert_eq!(value(&lines, "Type"), Some("simple"));
        assert!(lines.iter().any(|(section, ..)| section == "Install"));
    }
}

#[test]
fn met_019_the_system_unit_is_sandboxed_and_listens_on_nothing() {
    let lines = unit(SYSTEMD_SYSTEM);
    assert_eq!(value(&lines, "NoNewPrivileges"), Some("yes"));
    assert_eq!(value(&lines, "ProtectSystem"), Some("strict"));
    assert_eq!(value(&lines, "PrivateTmp"), Some("yes"));
    assert_eq!(value(&lines, "ProtectHome"), Some("yes"));
    assert_eq!(value(&lines, "User"), Some("anima-agent"));
    assert_eq!(value(&lines, "StateDirectory"), Some("anima-agent"));
    assert_eq!(
        value(&lines, "Environment"),
        Some("ANIMA_AGENT_HOME=/var/lib/anima-agent")
    );
    let families = value(&lines, "RestrictAddressFamilies").expect("restricted");
    for family in families.split_whitespace() {
        assert!(
            ["AF_INET", "AF_INET6", "AF_UNIX"].contains(&family),
            "{family}"
        );
    }
    assert!(
        lines.iter().all(|(_, key, _)| !key.starts_with("Listen")),
        "no socket is listened on"
    );
}

#[test]
fn met_019_the_user_unit_keeps_the_basic_limits_and_the_network_families() {
    let lines = unit(SYSTEMD_USER);
    assert_eq!(value(&lines, "NoNewPrivileges"), Some("yes"));
    assert_eq!(value(&lines, "PrivateTmp"), Some("yes"));
    assert_eq!(
        value(&lines, "RestrictAddressFamilies"),
        Some("AF_INET AF_INET6 AF_UNIX")
    );
    assert_eq!(value(&lines, "WantedBy"), Some("default.target"));
}

#[test]
fn met_019_the_launch_agent_parses_and_runs_the_binary_with_run() {
    let list = plist::Value::from_reader_xml(LAUNCH_AGENT.as_bytes()).expect("a property list");
    let dict = list.as_dictionary().expect("a dictionary");
    assert_eq!(
        dict.get("Label").and_then(plist::Value::as_string),
        Some("ing.anima.agent")
    );
    let arguments: Vec<&str> = dict
        .get("ProgramArguments")
        .and_then(plist::Value::as_array)
        .expect("arguments")
        .iter()
        .filter_map(plist::Value::as_string)
        .collect();
    assert!(arguments[0].ends_with("/anima-agent"));
    assert_eq!(arguments[1..], ["run"]);
    assert_eq!(
        dict.get("KeepAlive").and_then(plist::Value::as_boolean),
        Some(true)
    );
    assert!(
        !dict.contains_key("Sockets"),
        "the agent listens on no port"
    );
}
