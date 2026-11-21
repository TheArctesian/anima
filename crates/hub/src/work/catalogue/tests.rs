//! The list of the catalogue in the hub and the refusal of the install (task H38).

use catalogue::{
    Recipe,
    entry::{MachineTest, Mark, Protocol},
};

use super::{Ask, EntryKind, Refusal, entries, entry, install, listing::listed, model::Tier};

/// A person in the Anima app in a browser, who names the digest of `listed` when it has one image.
fn person_with_digest(listed: &super::Listed) -> Ask<'_> {
    Ask {
        in_browser: true,
        digest: listed.image_digests().first().copied(),
    }
}

fn ids(found: &[super::Listed]) -> Vec<&str> {
    found
        .iter()
        .map(|listed| listed.entry.id.as_str())
        .collect()
}

#[test]
fn h38_the_hub_lists_each_shipped_recipe_that_passes_the_static_test() {
    // Pi-hole is refused by the static test (port 53, OD-HOST-45): it is not listed.
    assert_eq!(
        ids(entries()),
        [
            "calibre-web",
            "code-server",
            "dawarich",
            "duplicati",
            "freshrss",
            "gitea",
            "heimdall",
            "jellyfin",
            "nextcloud",
            "owntracks-recorder",
            "postgresql",
            "qbittorrent",
            "radarr",
            "sonarr",
            "uptime-kuma",
            "vaultwarden"
        ]
    );
    assert!(entry("pihole").is_none());
    let postgresql = entry("postgresql").expect("listed");
    assert_eq!(
        EntryKind::from(postgresql.entry.anima.kind),
        EntryKind::Database
    );
    assert_eq!(postgresql.machine_test_day(), None);
}

#[test]
fn h38_a_recipe_with_a_finding_is_not_listed() {
    let mut recipes = catalogue::shipped();
    let broken = recipes
        .iter_mut()
        .find(|recipe| recipe.folder == "gitea")
        .expect("shipped");
    broken.compose = None;
    recipes.push(Recipe {
        folder: "unreadable".to_owned(),
        entry: Some("{".to_owned()),
        ..Recipe::default()
    });
    let found = listed(recipes);
    assert!(!ids(&found).contains(&"gitea"), "{:?}", ids(&found));
    assert!(!ids(&found).contains(&"unreadable"));
    assert!(ids(&found).contains(&"vaultwarden"));
}

#[test]
fn h38_the_description_gives_the_names_of_the_variables_and_never_a_value() {
    let dawarich = entry("dawarich").expect("listed");
    assert_eq!(
        dawarich.variables,
        [
            "DATABASE_PASSWORD",
            "SECRET_KEY_BASE",
            "APPLICATION_HOSTS",
            "TIME_ZONE"
        ]
    );
    assert!(dawarich.compose.starts_with("services:"));
    for listed in entries() {
        assert!(
            listed.variables.iter().all(|name| !name.contains('=')),
            "{}",
            listed.entry.id
        );
    }
}

#[test]
fn h38_the_install_is_refused_because_task_h6_is_not_built() {
    for listed in entries() {
        let answer = install(Some(listed), person_with_digest(listed));
        assert_eq!(
            answer.refusal,
            Some(Refusal::InstallNotBuilt),
            "{}",
            listed.entry.id
        );
        assert!(!answer.needs_open_to_network);
    }
    assert_eq!(
        install(None, Ask::default()).refusal,
        Some(Refusal::NotListed)
    );
    assert!(Refusal::InstallNotBuilt.reason().contains("task H6"));
}

#[test]
fn r_247_an_entry_with_the_socket_of_the_engine_is_never_installable() {
    let mut listed = entry("uptime-kuma").expect("listed").clone();
    listed.entry.anima.marks = vec![Mark::EngineSocket];
    assert_eq!(
        install(Some(&listed), person_with_digest(&listed)).refusal,
        Some(Refusal::EngineSocket)
    );
}

#[test]
fn r_229_an_entry_with_a_port_on_the_network_needs_open_to_the_network() {
    let mut listed = entry("uptime-kuma").expect("listed").clone();
    listed.entry.anima.marks = vec![Mark::NetworkPort {
        port: 3001,
        protocols: vec![Protocol::Tcp],
    }];
    let answer = install(Some(&listed), person_with_digest(&listed));
    assert!(answer.needs_open_to_network);
    assert_eq!(answer.refusal, Some(Refusal::InstallNotBuilt));
}

#[test]
fn od_host_54_a_shell_is_installed_only_by_a_person_in_the_app_in_a_browser() {
    let shell = entry("code-server").expect("listed");
    let by_key = Ask {
        in_browser: false,
        digest: shell.image_digests().first().copied(),
    };
    assert_eq!(
        install(Some(shell), by_key).refusal,
        Some(Refusal::ShellNeedsPerson)
    );
    assert_eq!(
        install(Some(shell), person_with_digest(shell)).refusal,
        Some(Refusal::InstallNotBuilt)
    );
    let application = entry("vaultwarden").expect("listed");
    let by_key = Ask {
        in_browser: false,
        digest: None,
    };
    assert_eq!(
        install(Some(application), by_key).refusal,
        Some(Refusal::InstallNotBuilt)
    );
}

#[test]
fn od_host_50_a_generated_entry_needs_the_digest_of_its_image() {
    let generated = entry("radarr").expect("listed");
    assert_eq!(generated.tier_of_origin(), Tier::Generated);
    assert_eq!(
        entry("vaultwarden").expect("listed").tier_of_origin(),
        Tier::Tested
    );
    let digest = generated.image_digests()[0];
    assert!(digest.starts_with("sha256:"), "{digest}");
    for named in [
        None,
        Some("sha256:0"),
        Some(entry("sonarr").expect("listed").image_digests()[0]),
    ] {
        let ask = Ask {
            in_browser: true,
            digest: named,
        };
        assert_eq!(
            install(Some(generated), ask).refusal,
            Some(Refusal::GeneratedNeedsDigest),
            "{named:?}"
        );
    }
}

#[test]
fn sf_revlsform_a_generated_entry_from_outside_the_browser_needs_a_test_with_its_image() {
    let mut generated = entry("radarr").expect("listed").clone();
    let digest = generated.image_digests()[0].to_owned();
    let by_key = Ask {
        in_browser: false,
        digest: Some(&digest),
    };
    assert_eq!(
        install(Some(&generated), by_key).refusal,
        Some(Refusal::GeneratedNotTested)
    );
    generated.entry.anima.last_test = MachineTest::Passed {
        date: "2026-11-21".to_owned(),
        digests: vec![format!("sha256:{}", "ab".repeat(32))],
    };
    assert_eq!(
        install(Some(&generated), by_key).refusal,
        Some(Refusal::GeneratedNotTested),
        "a pass with an other image does not count"
    );
    generated.entry.anima.last_test = MachineTest::Passed {
        date: "2026-11-21".to_owned(),
        digests: vec![digest.clone()],
    };
    assert_eq!(
        install(Some(&generated), by_key).refusal,
        Some(Refusal::InstallNotBuilt)
    );
    let no_digest = Ask {
        in_browser: false,
        digest: None,
    };
    assert_eq!(
        install(Some(&generated), no_digest).refusal,
        Some(Refusal::GeneratedNeedsDigest)
    );
}
