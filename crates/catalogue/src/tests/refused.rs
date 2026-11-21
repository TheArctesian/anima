//! A sound recipe passes; each fault gives its fixed code.

use compose_rules::{Refusal, Rule};

use super::{COMPOSE, ENTRY, ENVIRONMENT, IMAGE, codes, replaced, sound, with_compose, with_entry};
use crate::{Code, Recipe, check, entry::Mark};

#[test]
fn host_047_a_sound_recipe_has_no_finding() {
    assert_eq!(check(&sound()), []);
}

#[test]
fn host_047_an_entry_that_is_not_there_or_has_an_unknown_key_is_not_readable() {
    let missing = Recipe {
        entry: None,
        ..sound()
    };
    let unknown = with_entry(r#""pinned""#, r#""installs": 1, "pinned""#);
    let no_test = with_entry(r#""test""#, r#""probe""#);
    for recipe in [missing, unknown, no_test] {
        assert_eq!(codes(&recipe), [Code::EntryNotReadable]);
    }
}

#[test]
fn host_047_the_identifier_is_the_name_of_the_folder() {
    let other_folder = Recipe {
        folder: "other".to_owned(),
        ..sound()
    };
    assert_eq!(codes(&other_folder), [Code::IdNotPermitted]);
    assert_eq!(
        codes(&with_entry(r#""id": "notes""#, r#""id": "Notes""#)),
        [Code::IdNotPermitted]
    );
}

#[test]
fn host_047_empty_fields_addresses_and_dates_are_refused() {
    assert_eq!(
        codes(&with_entry(r#""tags": ["notes"]"#, r#""tags": []"#)),
        [Code::FieldEmpty]
    );
    for (old, new) in [
        (
            r#""compose_url": "compose.yaml""#,
            r#""compose_url": "../compose.yaml""#,
        ),
        ("https://example.org/notes", "http://example.org/notes"),
        (
            r#""icon_url": null"#,
            r#""icon_url": "ftp://example.org/i.png""#,
        ),
    ] {
        assert_eq!(
            codes(&with_entry(old, new)),
            [Code::AddressNotPermitted],
            "{new}"
        );
    }
    assert_eq!(
        codes(&with_entry(
            r#""pinned": "2026-11-20""#,
            r#""pinned": "2026-13-01""#
        )),
        [Code::DateNotPermitted]
    );
}

#[test]
fn host_047_a_missing_compose_or_example_file_is_refused() {
    let recipe = Recipe {
        environment: None,
        ..sound()
    };
    assert_eq!(codes(&recipe), [Code::FileNotFound]);
}

#[test]
fn r_233_an_image_with_a_tag_and_no_digest_is_refused() {
    let tagged = replaced(COMPOSE, IMAGE, "example/notes:1.2.3");
    let recipe = Recipe {
        entry: Some(replaced(ENTRY, IMAGE, "example/notes:1.2.3")),
        compose: Some(tagged),
        ..sound()
    };
    assert_eq!(codes(&recipe), [Code::ImageNotPinned]);
}

#[test]
fn r_233_an_image_of_the_file_that_the_entry_does_not_list_is_refused() {
    let other = IMAGE.replace("0123", "fedc");
    let recipe = with_compose(IMAGE, &other);
    assert_eq!(codes(&recipe), [Code::ImageNotListed, Code::ImageNotUsed]);
}

#[test]
fn host_047_a_file_that_the_default_rules_refuse_gives_their_rule() {
    let recipe = with_compose("    environment:", "    privileged: true\n    environment:");
    assert_eq!(
        codes(&recipe),
        [Code::ComposeRefused(Refusal::File {
            rule: Rule::KeyNotPermitted,
            service: Some(0)
        })]
    );
}

#[test]
fn r_229_a_port_on_the_network_needs_its_mark() {
    let open = with_compose("host_ip: 127.0.0.1", "host_ip: 0.0.0.0");
    assert_eq!(
        codes(&open),
        [Code::ComposeRefused(Refusal::NotAtMachine {
            service: Some(0)
        })]
    );
    let marked = Recipe {
        entry: Some(replaced(
            ENTRY,
            r#""marks": []"#,
            r#""marks": [{ "mark": "network-port", "port": 8080, "protocols": ["tcp"] }]"#,
        )),
        ..open
    };
    assert_eq!(check(&marked), []);
}

#[test]
fn r_229_a_mark_that_is_not_needed_or_names_a_different_port_is_refused() {
    let not_needed = with_entry(
        r#""marks": []"#,
        r#""marks": [{ "mark": "network-port", "port": 8080, "protocols": ["tcp"] }]"#,
    );
    assert_eq!(
        codes(&not_needed),
        [Code::MarkNotNeeded, Code::MarkNotMatched]
    );
    let other_protocol = Recipe {
        entry: Some(replaced(
            ENTRY,
            r#""marks": []"#,
            r#""marks": [{ "mark": "network-port", "port": 8080, "protocols": ["udp"] }]"#,
        )),
        compose: Some(replaced(COMPOSE, "host_ip: 127.0.0.1", "host_ip: 0.0.0.0")),
        ..sound()
    };
    assert_eq!(codes(&other_protocol), [Code::MarkNotMatched]);
}

#[test]
fn r_247_an_entry_that_holds_the_socket_of_the_engine_is_refused() {
    let recipe = with_entry(
        r#""marks": []"#,
        r#""marks": [{ "mark": "engine-socket" }]"#,
    );
    let found = check(&recipe);
    assert_eq!(
        found
            .iter()
            .map(|finding| &finding.code)
            .collect::<Vec<_>>(),
        [&Code::MarkNotPermitted(Mark::EngineSocket)]
    );
    assert_eq!(
        found[0].to_string(),
        "notes: mark_not_permitted (engine-socket)"
    );
}

#[test]
fn host_047_a_volume_with_no_kind_of_copy_is_refused() {
    let recipe = with_entry(r#"[{ "volume": "notes", "copy": "sqlite" }]"#, "[]");
    assert_eq!(codes(&recipe), [Code::VolumeNotDescribed]);
}

#[test]
fn host_047_a_test_of_a_machine_that_is_not_complete_is_refused() {
    for (old, new) in [
        (r#""port": 8080"#, r#""port": 9090"#),
        (
            r#""service": "notes",
      "port""#,
            r#""service": "other",
      "port""#,
        ),
        (r#""path": "/health""#, r#""path": null"#),
        (r#""statuses": [200]"#, r#""statuses": []"#),
        (r#""reach": "http""#, r#""reach": "tcp""#),
        (r#""limit_seconds": 60"#, r#""limit_seconds": 0"#),
    ] {
        assert_eq!(
            codes(&with_entry(old, new)),
            [Code::TestNotComplete],
            "{new}"
        );
    }
}

#[test]
fn host_047_the_example_file_holds_no_secret_value() {
    let valued = Recipe {
        environment: Some(replaced(
            ENVIRONMENT,
            "NOTES_SECRET=",
            "NOTES_SECRET=hunter2",
        )),
        ..sound()
    };
    assert_eq!(codes(&valued), [Code::SecretInExample]);
    let password = Recipe {
        environment: Some(format!("{ENVIRONMENT}ADMIN_PASSWORD=hunter2\n")),
        ..sound()
    };
    assert_eq!(codes(&password), [Code::SecretInExample]);
}

#[test]
fn host_047_a_generated_variable_is_marked_in_the_example_file() {
    let unmarked = Recipe {
        environment: Some(replaced(ENVIRONMENT, "# generated at install\n", "")),
        ..sound()
    };
    assert_eq!(codes(&unmarked), [Code::GeneratedNotMarked]);
    let unlisted = with_entry(r#""generated": ["NOTES_SECRET"]"#, r#""generated": []"#);
    assert_eq!(codes(&unlisted), [Code::GeneratedNotMarked]);
}

#[test]
fn host_047_an_entry_whose_last_test_on_a_machine_failed_leaves_the_list() {
    let recipe = with_entry(
        r#"{ "result": "not-tested" }"#,
        r#"{ "result": "failed", "date": "2026-11-21" }"#,
    );
    assert_eq!(codes(&recipe), [Code::MachineTestFailed]);
}

#[test]
fn sf_revcatvars_the_image_a_published_port_or_its_address_is_never_a_variable() {
    let image = Recipe {
        compose: Some(replaced(COMPOSE, IMAGE, "${NOTES_IMAGE}")),
        environment: Some(format!("{ENVIRONMENT}NOTES_IMAGE={IMAGE}\n")),
        ..sound()
    };
    let port = with_compose("published: 8080", "published: ${NOTES_PORT:-8080}");
    let address = with_compose("host_ip: 127.0.0.1", "host_ip: ${NOTES_ADDRESS:-127.0.0.1}");
    for recipe in [image, port, address] {
        assert_eq!(
            codes(&recipe),
            [Code::VariableNotPermitted],
            "{:?}",
            recipe.compose
        );
    }
}

#[test]
fn sf_revcatsecret_the_compose_file_gives_a_secret_only_as_one_variable() {
    for line in [
        "DB_PASSWORD: hunter2",
        "API_KEY: ${NOTES_KEY:-abc123}",
        "smtp_passwd: hunter2",
        "- NOTES_TOKEN=hunter2",
    ] {
        let compose = if let Some(item) = line.strip_prefix("- ") {
            replaced(
                COMPOSE,
                "      SECRET: ${NOTES_SECRET}",
                &format!("      - SECRET=${{NOTES_SECRET}}\n      - {item}"),
            )
        } else {
            replaced(
                COMPOSE,
                "      SECRET: ${NOTES_SECRET}",
                &format!("      SECRET: ${{NOTES_SECRET}}\n      {line}"),
            )
        };
        let recipe = Recipe {
            compose: Some(compose),
            ..sound()
        };
        assert_eq!(codes(&recipe), [Code::SecretInFile], "{line}");
    }
}

#[test]
fn sf_revcatsecret_each_name_that_marks_a_secret_has_no_value_in_the_example() {
    for name in [
        "SMTP_PASSWD",
        "MYAPIKEY",
        "STRIPE_API_KEY",
        "AWS_CREDENTIALS",
        "SSH_PRIVATE_KEY",
    ] {
        let recipe = Recipe {
            environment: Some(format!("{ENVIRONMENT}{name}=hunter2\n")),
            ..sound()
        };
        assert_eq!(codes(&recipe), [Code::SecretInExample], "{name}");
    }
}
