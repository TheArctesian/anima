//! The rules of the values of a service and of the top level.

use super::{Refusal, Rule, SOUND, app, checked, refusal, top, with};
use crate::{Settings, check};

#[test]
fn host_028_an_image_is_a_reference_with_a_tag_or_a_digest_and_never_an_option() {
    for image in [
        "--privileged",
        "-v",
        "Example/Tracks:1",
        "example/tracks",
        "a b:1",
        "tracks:",
        "a;id:1",
        "",
    ] {
        let file = with("example/tracks:${TAG:-0.36}", &format!("\"{image}\""));
        assert_eq!(refusal(&file), app(Rule::ImageNotPermitted), "{image}");
    }
    let no_image = with("    image: example/tracks:${TAG:-0.36}\n", "");
    assert_eq!(refusal(&no_image), app(Rule::ImageNotPermitted));
    let registry = with("example/tracks:${TAG:-0.36}", "ghcr.io/example/tracks:v1_x");
    assert!(checked(&registry).is_ok());
}

#[test]
fn host_028_a_limit_of_zero_or_below_is_refused_because_it_means_no_limit() {
    for (old, new) in [
        ("mem_limit: 512m", "mem_limit: 0"),
        ("mem_limit: 512m", "mem_limit: -1"),
        ("mem_limit: 512m", "mem_limit: lots"),
        ("pids_limit: 200", "pids_limit: 0"),
        ("pids_limit: 200", "pids_limit: -1"),
        ("mem_limit: 512m", "cpus: 0"),
        ("mem_limit: 512m", "cpus: \"0.0\""),
        ("mem_limit: 512m", "stop_grace_period: 0s"),
    ] {
        assert_eq!(
            refusal(&with(old, new)),
            app(Rule::LimitNotPermitted),
            "{new}"
        );
    }
    for sound in [
        "mem_limit: 2g",
        "mem_limit: 1048576",
        "cpus: 0.5",
        "cpus: 2",
        "stop_grace_period: 1m30s",
    ] {
        assert!(checked(&with("mem_limit: 512m", sound)).is_ok(), "{sound}");
    }
}

#[test]
fn host_028_an_entry_of_the_environment_has_a_value() {
    // An entry with a name only would be filled by Compose from its own process.
    let name_only = with("      RAILS_ENV: production", "      RAILS_ENV:");
    assert_eq!(refusal(&name_only), app(Rule::ValueNotPermitted));
    let list = with(
        "      - POSTGRES_PASSWORD=${DB_PASSWORD}",
        "      - POSTGRES_PASSWORD",
    );
    assert_eq!(
        refusal(&list),
        Refusal::File {
            rule: Rule::ValueNotPermitted,
            service: Some(1)
        }
    );
}

#[test]
fn host_028_a_label_needs_a_prefix_that_the_owner_named_at_the_machine() {
    let labelled = with(
        "restart: unless-stopped",
        "labels: {traefik.enable: \"true\"}",
    );
    assert_eq!(refusal(&labelled), app(Rule::LabelNotPermitted));
    let named = Settings {
        label_prefixes: vec!["traefik.".to_owned()],
        ..Settings::default()
    };
    assert!(check(&labelled, super::ENVIRONMENT, &named).is_ok());
    // The prefix of Compose itself can never be set, also when the owner named it.
    let compose = with(
        "restart: unless-stopped",
        "labels: [com.docker.compose.project=other]",
    );
    let wide = Settings {
        label_prefixes: vec!["com.".to_owned(), String::new()],
        ..Settings::default()
    };
    assert_eq!(
        check(&compose, super::ENVIRONMENT, &wide),
        Err(app(Rule::LabelNotPermitted))
    );
}

#[test]
fn host_028_the_single_values_of_a_service_have_their_forms() {
    for (old, new) in [
        ("restart: unless-stopped", "restart: sometimes"),
        ("restart: unless-stopped", "restart: on-failure:x"),
        ("restart: unless-stopped", "read_only: maybe"),
        (
            "restart: unless-stopped",
            "security_opt: [seccomp=unconfined]",
        ),
        (
            "restart: unless-stopped",
            "security_opt: [\"no-new-privileges:false\"]",
        ),
        ("restart: unless-stopped", "logging: {driver: syslog}"),
        ("restart: unless-stopped", "hostname: Bad_Host.Name"),
        ("restart: unless-stopped", "expose: [\"0\"]"),
        ("restart: unless-stopped", "cap_drop: [all lower]"),
        ("      interval: 30s", "      interval: 1s"),
        ("      interval: 30s", "      interval: 500ms"),
        ("      retries: 3", "      retries: many"),
        (
            "        condition: service_healthy",
            "        condition: whenever",
        ),
        (
            "      db:\n        condition: service_healthy",
            "      other:\n        condition: service_healthy",
        ),
    ] {
        assert_eq!(
            refusal(&with(old, new)),
            app(Rule::ValueNotPermitted),
            "{new}"
        );
    }
    for sound in [
        "restart: on-failure:3",
        "security_opt: [\"no-new-privileges:true\"]",
        "logging: {driver: local}",
        "init: true",
        "cap_drop: [ALL]",
        "user: \"1000:1000\"",
    ] {
        assert!(
            checked(&with("restart: unless-stopped", sound)).is_ok(),
            "{sound}"
        );
    }
}

#[test]
fn host_028_names_and_counts_of_the_top_level_have_their_limits() {
    let bad_name = SOUND
        .replacen("  db:\n    image", "  Db.Server:\n    image", 1)
        .replacen("      db:\n", "      Db.Server:\n", 1);
    assert_eq!(refusal(&bad_name), top(Rule::NameNotPermitted));
    let many = (0..33)
        .map(|n| {
            format!(
                "  s{n}:
    image: a:1
"
            )
        })
        .collect::<Vec<_>>()
        .concat();
    assert_eq!(refusal(&format!("services:\n{many}")), top(Rule::TooLarge));
    let networks = (0..17)
        .map(|n| format!("  n{n}:\n"))
        .collect::<Vec<_>>()
        .concat();
    assert_eq!(
        refusal(&format!(
            "services:\n  a:\n    image: a:1\nnetworks:\n{networks}"
        )),
        top(Rule::TooLarge)
    );
    assert_eq!(refusal("services: {}\n"), top(Rule::ValueNotPermitted));
    assert_eq!(refusal("volumes:\n  data:\n"), top(Rule::ValueNotPermitted));
}

#[test]
fn host_028_the_readers_of_the_final_form_read_a_value_as_the_check_reads_it() {
    use crate::forms::{bind_name, bytes, processor_billionths, seconds};
    assert_eq!(processor_billionths("2"), Some(2_000_000_000));
    assert_eq!(processor_billionths("0.5"), Some(500_000_000));
    assert_eq!(processor_billionths("1.0000000019"), Some(1_000_000_001));
    for wrong in ["", ".5", "1.", "-1", "1e3", "0x1", "1.5.2"] {
        assert_eq!(processor_billionths(wrong), None, "{wrong}");
    }
    assert_eq!(bind_name("binds/config"), Some("config"));
    assert_eq!(bind_name("./binds/config"), Some("config"));
    for wrong in [
        "binds/",
        "binds/a/b",
        "binds/..",
        "/binds/a",
        "files",
        "binds/A",
    ] {
        assert_eq!(bind_name(wrong), None, "{wrong}");
    }
    assert_eq!(seconds("1m30s"), Some(90));
    assert_eq!(bytes("512m"), Some(512 << 20));
}
