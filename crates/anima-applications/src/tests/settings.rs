//! The settings at the machine (K10 Section 6.4.2).

use crate::settings::{Guarded, Limits, Machine, NotGuarded, Settings, SettingsFault, guard, read};

const AFTER_INSTALL: &str = r#"{
  "version": 1,
  "manageApplications": null,
  "allContainers": false,
  "openToNetwork": false,
  "hubMayDeleteVolumes": false,
  "adoptedProjects": [],
  "protected": [],
  "registriesWithLogin": [],
  "limits": { "memoryBytes": null, "processes": 512, "processorBillionths": null, "stopSeconds": 30, "freeDiskBytes": 10737418240 },
  "labelPrefixes": []
}"#;

const SET: &str = r#"{
  "version": 1,
  "manageApplications": { "engine": "user", "socket": "/Users/ada/.docker/run/docker.sock", "docker": "/usr/local/bin/docker", "compose": "/usr/local/bin/docker-compose", "acceptedUser": 501, "setOnAt": 1795000000 },
  "allContainers": true,
  "openToNetwork": true,
  "hubMayDeleteVolumes": true,
  "adoptedProjects": [{ "name": "tracks", "folder": "/srv/tracks", "files": ["/srv/tracks/compose.yaml"] }],
  "protected": ["backup", "Traefik.proxy-1"],
  "registriesWithLogin": ["ghcr.io", "registry.example:5000"],
  "limits": { "memoryBytes": 1073741824, "processes": 100, "processorBillionths": 500000000, "stopSeconds": 10, "freeDiskBytes": 1 },
  "labelPrefixes": ["traefik."]
}"#;

#[test]
fn k10_6_4_2_after_the_install_each_setting_is_off_and_each_list_is_empty() {
    let settings = Settings::after_install();
    assert!(settings.managed.is_none());
    assert!(!settings.all_containers && !settings.open_to_network);
    assert!(!settings.hub_may_delete_volumes);
    assert!(settings.adopted.is_empty() && settings.protected.is_empty());
    assert!(settings.registries.is_empty() && settings.label_prefixes.is_empty());
    assert_eq!(read(AFTER_INSTALL), Ok(settings));
}

#[test]
fn k10_6_4_2_the_limits_after_the_install_are_those_of_the_table() {
    let machine = Machine {
        memory_bytes: 16 << 30,
        processors: 8,
        free_disk_bytes: 0,
    };
    let limits = Limits::after_install();
    let own = limits.own(&machine);
    assert_eq!(own.memory_bytes, 4 << 30);
    assert_eq!(own.processes, 512);
    assert_eq!(own.processor_billionths, 8_000_000_000);
    assert_eq!(own.stop_seconds, 30);
    assert_eq!(limits.free_disk_bytes, 10 << 30);
}

#[test]
fn k10_6_4_2_a_file_with_each_setting_set_is_read() {
    let settings = read(SET).expect("the file is read");
    let managed = settings.managed.clone().expect("on");
    assert_eq!(managed.accepted_user, 501);
    assert_eq!(managed.socket, "/Users/ada/.docker/run/docker.sock");
    assert!(settings.all_containers && settings.open_to_network);
    assert!(settings.hub_may_delete_volumes);
    assert_eq!(settings.adopted[0].files, ["/srv/tracks/compose.yaml"]);
    assert!(settings.protects("backup") && !settings.protects("other"));
    assert_eq!(settings.limits.memory_bytes, Some(1 << 30));
    assert_eq!(settings.of_a_file().label_prefixes, ["traefik."]);
}

#[test]
fn k10_6_4_2_a_member_that_the_program_does_not_know_is_refused_at_each_level() {
    for (old, new) in [
        ("\"version\": 1,", "\"version\": 1, \"profile\": \"full\","),
        (
            "\"engine\": \"user\",",
            "\"engine\": \"user\", \"sudo\": true,",
        ),
        (
            "\"name\": \"tracks\",",
            "\"name\": \"tracks\", \"shell\": \"sh\",",
        ),
        ("\"processes\": 100,", "\"processes\": 100, \"none\": true,"),
    ] {
        assert!(SET.contains(old), "{old}");
        let text = SET.replacen(old, new, 1);
        assert_eq!(read(&text), Err(SettingsFault::MemberUnknown), "{new}");
    }
}

#[test]
fn k10_6_4_2_a_member_that_is_not_there_or_there_two_times_is_refused() {
    let lacking = AFTER_INSTALL.replacen("\"allContainers\": false,", "", 1);
    assert_eq!(read(&lacking), Err(SettingsFault::ValueNotPermitted));
    let twice = AFTER_INSTALL.replacen(
        "\"allContainers\": false,",
        "\"allContainers\": false, \"allContainers\": true,",
        1,
    );
    assert_eq!(read(&twice), Err(SettingsFault::NotJson));
    assert_eq!(read("not json"), Err(SettingsFault::NotJson));
    assert_eq!(
        read(&format!("{AFTER_INSTALL} {{}}")),
        Err(SettingsFault::NotJson)
    );
    // The strict reader has only whole numbers of 0 or more.
    for number in ["-1", "1.5", "1e3"] {
        let text = SET.replacen(
            "\"acceptedUser\": 501",
            &format!("\"acceptedUser\": {number}"),
            1,
        );
        assert_eq!(read(&text), Err(SettingsFault::NotJson), "{number}");
    }
    let large = " ".repeat(crate::settings::LARGEST_SETTINGS + 1);
    assert_eq!(read(&large), Err(SettingsFault::TooLarge));
}

#[test]
fn k10_6_4_2_a_value_that_is_not_permitted_is_refused() {
    for (old, new) in [
        ("\"version\": 1", "\"version\": 2"),
        ("\"allContainers\": true", "\"allContainers\": \"true\""),
        ("\"engine\": \"user\"", "\"engine\": \"remote\""),
        // The socket is a Unix socket of the machine: an address of a different machine is refused.
        (
            "\"/Users/ada/.docker/run/docker.sock\"",
            "\"tcp://10.0.0.5:2375\"",
        ),
        (
            "\"/Users/ada/.docker/run/docker.sock\"",
            "\"ssh://ada@host\"",
        ),
        ("\"/usr/local/bin/docker\"", "\"docker\""),
        ("\"/usr/local/bin/docker\"", "\"/usr/local/../bin/docker\""),
        ("\"acceptedUser\": 501", "\"acceptedUser\": 4294967296"),
        // A project whose name starts with `anima-` cannot be adopted (Section 6.4.3).
        (
            "\"name\": \"tracks\"",
            "\"name\": \"anima-0192f6b07c1e7a3b9d2f4e5a6b7c8d9e\"",
        ),
        ("\"name\": \"tracks\"", "\"name\": \"--project-directory\""),
        ("[\"/srv/tracks/compose.yaml\"]", "[]"),
        ("[\"/srv/tracks/compose.yaml\"]", "[\"compose.yaml\"]"),
        ("\"backup\"", "\"--all\""),
        ("\"backup\"", "\"Traefik.proxy-1\""),
        ("\"ghcr.io\"", "\"https://ghcr.io\""),
        // A limit of zero is "no limit" to the engine.
        ("\"processes\": 100", "\"processes\": 0"),
        ("\"memoryBytes\": 1073741824", "\"memoryBytes\": 0"),
        ("\"stopSeconds\": 10", "\"stopSeconds\": null"),
        // A prefix that starts with `com.docker.` cannot be named (SF-043).
        ("\"traefik.\"", "\"com.docker.compose.\""),
        ("\"traefik.\"", "\"com.\""),
        ("\"traefik.\"", "\"\""),
    ] {
        assert!(SET.contains(old), "{old}");
        let text = SET.replacen(old, new, 1);
        assert_eq!(read(&text), Err(SettingsFault::ValueNotPermitted), "{new}");
    }
}

#[test]
fn k10_6_4_2_the_file_is_of_the_identifier_0_with_no_write_right_for_a_group_or_others() {
    let sound = Guarded {
        owner: 0,
        mode: 0o644,
        regular: true,
    };
    assert_eq!(guard(&sound), Ok(()));
    assert_eq!(
        guard(&Guarded {
            mode: 0o600,
            ..sound
        }),
        Ok(())
    );
    assert_eq!(
        guard(&Guarded {
            owner: 501,
            ..sound
        }),
        Err(NotGuarded::Owner)
    );
    assert_eq!(
        guard(&Guarded {
            mode: 0o664,
            ..sound
        }),
        Err(NotGuarded::Writable)
    );
    assert_eq!(
        guard(&Guarded {
            mode: 0o646,
            ..sound
        }),
        Err(NotGuarded::Writable)
    );
    assert_eq!(
        guard(&Guarded {
            regular: false,
            ..sound
        }),
        Err(NotGuarded::NotRegular)
    );
}
