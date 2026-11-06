//! The pure repair rule: candidates, ports, names and texts.

use chrono::{TimeZone as _, Utc};
use serde_json::json;

use super::super::rule::{
    Port, Protocol, audit_change, is_candidate, ports, repaired_message, repaired_name,
    silent_message,
};
use crate::{
    checks::{
        CheckSettings,
        config::{CheckConfig, IpVersion},
        store::repairs::Facts,
    },
    devices::{Device, OperatingSystem, SshAuth, SshSettings},
    status::Status,
};

fn ping() -> CheckConfig {
    CheckConfig::Icmp {
        timeout_ms: 1000,
        ip_version: IpVersion::V4,
    }
}

/// A check that meets every condition: ICMP, Down after its 2 retries, never answered.
fn candidate() -> Facts {
    Facts {
        config: Some(ping()),
        settings: CheckSettings {
            retries: 2,
            ..CheckSettings::default()
        },
        status: Status::Down,
        failures: 3,
        paused: false,
        operator_edited: false,
        tried_at: None,
        answered: Some(false),
    }
}

fn device(ssh_enabled: bool, port: u16) -> Device {
    let at = Utc.with_ymd_and_hms(2026, 11, 1, 0, 0, 0).unwrap();
    Device {
        id: uuid::Uuid::nil().into(),
        site: uuid::Uuid::nil().into(),
        name: "mac-office-1".to_owned(),
        host: "10.20.0.10".to_owned(),
        ip: None,
        kind: "Mac mini".to_owned(),
        os: OperatingSystem::Macos,
        tags: Vec::new(),
        maintenance: false,
        ssh: SshSettings {
            enabled: ssh_enabled,
            port,
            username: "operator".to_owned(),
            auth: SshAuth::PrivateKey,
        },
        created_at: at,
        updated_at: at,
    }
}

const SSH: Port = Port {
    protocol: Protocol::Ssh,
    number: 22,
};

#[test]
fn chk_013_a_check_that_meets_every_condition_is_a_candidate() {
    assert!(is_candidate(&candidate(), false));
}

/// Makes one condition of a candidate false.
type Spoil = fn(&mut Facts);

#[test]
fn chk_013_each_condition_alone_blocks_the_repair() {
    let blocked: [(&str, Spoil); 11] = [
        ("ever answered", |f| f.answered = Some(true)),
        ("history not read", |f| f.answered = None),
        ("edited", |f| f.operator_edited = true),
        ("tried", |f| {
            f.tried_at = Some(Utc.with_ymd_and_hms(2026, 11, 2, 0, 0, 0).unwrap());
        }),
        ("paused", |f| f.paused = true),
        ("inverted", |f| f.settings.invert = true),
        ("not icmp", |f| {
            f.config = Some(CheckConfig::Tcp {
                port: 22,
                timeout_ms: 1000,
            });
        }),
        ("unreadable config", |f| f.config = None),
        ("up", |f| f.status = Status::Up),
        ("unknown", |f| f.status = Status::Unknown),
        ("retries not reached", |f| f.failures = 2),
    ];
    for (why, change) in blocked {
        let mut facts = candidate();
        change(&mut facts);
        assert!(!is_candidate(&facts, false), "{why}");
    }
    assert!(!is_candidate(&candidate(), true), "maintenance");
}

#[test]
fn chk_013_a_check_with_no_retries_is_a_candidate_after_one_failure() {
    let mut facts = candidate();
    facts.failures = 1;
    facts.settings.retries = 0;
    assert!(is_candidate(&facts, false));
}

#[test]
fn chk_013_ssh_disabled_gives_no_port() {
    assert_eq!(ports(&device(false, 22)), []);
}

#[test]
fn chk_013_ssh_is_the_first_port_tried_on_its_configured_number() {
    let found = ports(&device(true, 2222));
    assert_eq!(
        found,
        [Port {
            protocol: Protocol::Ssh,
            number: 2222
        }]
    );
    assert_eq!(
        found[0].config(),
        CheckConfig::Tcp {
            port: 2222,
            timeout_ms: 5000
        }
    );
}

#[test]
fn chk_013_the_repaired_check_is_named_for_the_port() {
    assert_eq!(repaired_name(SSH, "Ping", &[]), "SSH port");
    let other = Port {
        protocol: Protocol::Ssh,
        number: 2222,
    };
    assert_eq!(repaired_name(other, "Ping", &[]), "SSH port 2222");
}

#[test]
fn chk_013_a_taken_name_says_what_the_check_was() {
    let taken = vec!["SSH port".to_owned()];
    assert_eq!(repaired_name(SSH, "Ping", &taken), "SSH port (was Ping)");
    let both = vec!["SSH port".to_owned(), "SSH port (was Ping)".to_owned()];
    assert_eq!(repaired_name(SSH, "Ping", &both), "SSH port (was Ping) 2");
}

#[test]
fn chk_013_a_long_name_is_cut_to_200_characters() {
    let old = "é".repeat(200);
    let taken = vec!["SSH port".to_owned()];
    let name = repaired_name(SSH, &old, &taken);
    assert_eq!(name.chars().count(), 200);
    assert!(name.starts_with("SSH port (was é"));
    let cut = name.clone();
    let numbered = repaired_name(SSH, &old, &[taken[0].clone(), cut]);
    assert_eq!(numbered.chars().count(), 200);
    assert!(numbered.ends_with("é 2"), "{numbered}");
}

#[test]
fn chk_013_the_messages_name_the_device_protocol_and_port() {
    assert_eq!(
        repaired_message("mac-office-1", SSH),
        "Ping never answered on mac-office-1; SSH port 22 does, so its check now uses TCP 22."
    );
    assert_eq!(
        silent_message("mac-office-1"),
        "Ping never answered on mac-office-1, and no connection port answers either."
    );
}

#[test]
fn chk_013_the_audit_change_holds_the_method_before_and_after() {
    let change = audit_change(("Ping", &ping()), ("SSH port", &SSH.config()));
    assert_eq!(
        change,
        json!({
            "repairedBy": "hub",
            "before": {
                "type": "icmp",
                "config": {"type": "icmp", "timeoutMs": 1000, "ipVersion": "V4"},
                "name": "Ping",
            },
            "after": {
                "type": "tcp",
                "config": {"type": "tcp", "port": 22, "timeoutMs": 5000},
                "name": "SSH port",
            },
        })
    );
}
