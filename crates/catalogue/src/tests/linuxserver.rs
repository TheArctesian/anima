//! The recipe form of LinuxServer.io (task H39): the entries that ship are what the form makes
//! from the saved fleet list, each passes the static test, an image that cannot fit gives its
//! reason, and the generator changes an entry only by its digest or its version.

use std::path::Path;

use crate::{
    Code, Recipe, check,
    entry::{Class, Entry, Optional, Protocol},
    linuxserver::{
        Assessment, Capability, Change, Custom, Device, Image, Pin, Port, Unfit, Variable, Volume,
        assess, list, recipe, update,
    },
    read,
};

/// One change of an image of the fleet list.
type Edit = fn(&mut Image);

const FLEET: &str = include_str!("../../linuxserver/fleet.json");
const PINS: &str = include_str!("../../linuxserver/pins.json");

fn images() -> Vec<Image> {
    list(FLEET)
        .expect("the saved fleet list is readable")
        .images
}

fn image(name: &str) -> Image {
    images()
        .into_iter()
        .find(|image| image.name == name)
        .expect("the image is in the saved fleet list")
}

fn pin(name: &str) -> Pin {
    let pins: serde_json::Value = serde_json::from_str(PINS).expect("the saved pins are JSON");
    let text = |key: &str| {
        pins[name][key]
            .as_str()
            .expect("a pin has each key")
            .to_owned()
    };
    Pin {
        tag: text("tag"),
        digest: text("digest"),
        resolved: text("resolved"),
        read: "2026-11-17".to_owned(),
    }
}

fn made(name: &str) -> Recipe {
    recipe(&image(name), &pin(name)).expect("the image fits")
}

fn entry(recipe: &Recipe) -> Entry {
    serde_json::from_str(recipe.entry.as_deref().expect("an entry")).expect("an entry")
}

#[test]
fn h39_each_generated_entry_is_what_the_form_makes_and_passes_the_static_test() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("entries");
    let shipped = read(&root).expect("the entries folder of the crate is readable");
    let names: Vec<String> = images().into_iter().map(|image| image.name).collect();
    assert_eq!(names.len(), 10);
    for name in &names {
        let made = made(name);
        assert_eq!(check(&made), [], "{name}");
        let folder = shipped.iter().find(|recipe| recipe.folder == made.folder);
        assert_eq!(folder, Some(&made), "{name}");
    }
}

#[test]
fn h39_the_form_writes_streams_an_optional_device_volumes_and_a_shell() {
    let streams = entry(&made("qbittorrent")).anima.streams;
    assert_eq!(streams.len(), 1);
    assert_eq!(
        (streams[0].port, streams[0].protocols.clone()),
        (6881, vec![Protocol::Tcp, Protocol::Udp])
    );
    let jellyfin = made("jellyfin");
    assert_eq!(
        entry(&jellyfin).anima.optional,
        [Optional::Device {
            path: "/dev/dri".to_owned()
        }]
    );
    assert!(!jellyfin.compose.unwrap_or_default().contains("/dev/"));
    let volumes: Vec<String> = entry(&made("radarr"))
        .anima
        .volumes
        .into_iter()
        .map(|kept| kept.volume)
        .collect();
    assert_eq!(volumes, ["config", "movies", "downloads"]);
    let shell = entry(&made("code-server")).anima;
    assert_eq!(
        (shell.class, shell.generated),
        (Class::Shell, vec!["PASSWORD".to_owned()])
    );
    let heimdall = made("heimdall").compose.unwrap_or_default();
    assert!(heimdall.contains("published: 8080") && heimdall.contains("published: 8444"));
    let duplicati = entry(&made("duplicati")).anima.generated;
    assert_eq!(duplicati, ["SETTINGS_ENCRYPTION_KEY"]);
}

/// `radarr` with one change of its configuration.
fn changed(change: Edit) -> Result<Recipe, Unfit> {
    let mut changed = image("radarr");
    change(&mut changed);
    recipe(&changed, &pin("radarr"))
}

fn variable(name: &str, value: &str, optional: bool) -> Variable {
    Variable {
        name: name.to_owned(),
        value: value.to_owned(),
        optional,
    }
}

#[test]
fn h39_an_image_that_needs_a_right_of_the_machine_gives_its_reason() {
    let cases: [(Edit, Unfit); 5] = [
        (|image| image.config.privileged = true, Unfit::Privileged),
        (
            |image| image.config.networking = Some("host".to_owned()),
            Unfit::HostNetwork,
        ),
        (
            |image| {
                image.config.caps.push(Capability {
                    cap_add: "NET_ADMIN".to_owned(),
                    optional: false,
                });
            },
            Unfit::Capability("NET_ADMIN".to_owned()),
        ),
        (
            |image| {
                image.config.devices.push(Device {
                    path: "/dev/ttyUSB0".to_owned(),
                    optional: false,
                });
            },
            Unfit::Device,
        ),
        (
            |image| {
                image.config.custom.push(Custom {
                    name_compose: "shm_size".to_owned(),
                    optional: false,
                });
            },
            Unfit::SharedMemory,
        ),
    ];
    for (change, reason) in cases {
        assert_eq!(changed(change), Err(reason.clone()), "{reason}");
    }
}

#[test]
fn h39_an_image_whose_settings_cannot_fit_gives_its_reason() {
    let cases: [(Edit, Unfit); 6] = [
        (
            |image| {
                image
                    .config
                    .env_vars
                    .push(variable("DOCKER_MODS", "x", false));
            },
            Unfit::ImageMods,
        ),
        (
            |image| {
                image
                    .config
                    .env_vars
                    .push(variable("VERSION", "latest", false));
            },
            Unfit::VersionNotFixed,
        ),
        (
            |image| {
                image.config.env_vars.push(variable("DB_HOST", "", false));
            },
            Unfit::ValueNeeded,
        ),
        (
            |image| {
                image.config.volumes.push(Volume {
                    path: "/var/run/docker.sock".to_owned(),
                    host_path: "/var/run/docker.sock".to_owned(),
                    optional: false,
                });
            },
            Unfit::EngineSocket,
        ),
        (
            |image| {
                image.config.ports = vec![Port {
                    external: "30000".to_owned(),
                    internal: "30000/udp".to_owned(),
                    optional: false,
                }];
            },
            Unfit::NoPort,
        ),
        (
            |image| image.name = "syncthing".to_owned(),
            Unfit::ShellWithoutCredential,
        ),
    ];
    for (change, reason) in cases {
        assert_eq!(changed(change), Err(reason.clone()), "{reason}");
    }
}

#[test]
fn h39_an_optional_part_is_left_off_and_a_secret_is_made_at_the_install() {
    let mut radarr = image("radarr");
    radarr.config.caps.push(Capability {
        cap_add: "SYS_ADMIN".to_owned(),
        optional: true,
    });
    radarr
        .config
        .env_vars
        .push(variable("DOCKER_MODS", "linuxserver/mods:x", true));
    radarr
        .config
        .env_vars
        .push(variable("API_KEY", "abc", false));
    let made = recipe(&radarr, &pin("radarr")).expect("fits with the part off");
    let compose = made.compose.clone().unwrap_or_default();
    assert!(!compose.contains("DOCKER_MODS") && !compose.contains("SYS_ADMIN"));
    assert!(compose.contains("API_KEY: ${API_KEY}"));
    assert!(!made.environment.clone().unwrap_or_default().contains("abc"));
    assert_eq!(assess(&radarr, &pin("radarr")), Assessment::FitsWithPartOff);
    assert_eq!(
        assess(&image("heimdall"), &pin("heimdall")),
        Assessment::FitsWithPortMoved
    );
    assert_eq!(assess(&image("sonarr"), &pin("sonarr")), Assessment::Fits);
}

#[test]
fn sf_lsgenerator_the_generator_changes_an_entry_only_by_its_digest_or_its_version() {
    let existing = made("radarr");
    let mut newer = pin("radarr");
    newer.tag = "7.0.4.11000-ls330".to_owned();
    newer.digest = format!("sha256:{}", "ab".repeat(32));
    newer.resolved = "2026-12-21".to_owned();
    newer.read = "2026-12-20".to_owned();
    let repinned = recipe(&image("radarr"), &newer).expect("fits");
    assert_eq!(update(&existing, &repinned), Ok(()));
    let mut wider = image("radarr");
    wider.config.ports.push(Port {
        external: "9898".to_owned(),
        internal: "9898".to_owned(),
        optional: false,
    });
    let widened = recipe(&wider, &newer).expect("fits");
    assert_eq!(update(&existing, &widened), Err(Change::NotOnlyPin));
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("entries");
    let gitea = read(&root)
        .expect("readable")
        .into_iter()
        .find(|recipe| recipe.folder == "gitea")
        .expect("shipped");
    assert_eq!(update(&gitea, &existing), Err(Change::Reviewed));
}

#[test]
fn h39_the_static_test_checks_streams_optional_parts_and_the_class_shell() {
    let with = |name: &str, old: &str, new: &str| {
        let made = made(name);
        Recipe {
            entry: made.entry.map(|text| super::replaced(&text, old, new)),
            ..made
        }
    };
    let stream = with("qbittorrent", "\"port\": 6881", "\"port\": 6882");
    assert_eq!(super::codes(&stream), [Code::StreamNotPublished]);
    let device = with("jellyfin", "\"/dev/dri\"", "\"/etc/dri\"");
    assert_eq!(super::codes(&device), [Code::OptionalNotPermitted]);
    let open = with(
        "code-server",
        "\"generated\": [\n      \"PASSWORD\"\n    ]",
        "\"generated\": []",
    );
    assert!(super::codes(&open).contains(&Code::ShellNotContained));
}

#[test]
fn sf_revlsform_an_image_name_out_of_the_rules_is_refused() {
    for name in ["radarr\nx: y", "Radarr", "radarr_2", "radarr:latest", ""] {
        let mut odd = image("radarr");
        odd.name = name.to_owned();
        assert_eq!(
            recipe(&odd, &pin("radarr")).map(|_| ()),
            Err(Unfit::NameNotPermitted),
            "{name:?}"
        );
    }
}
