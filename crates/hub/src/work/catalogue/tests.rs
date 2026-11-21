//! The list of the catalogue in the hub and the refusal of the install (task H38).

use catalogue::{
    Recipe,
    entry::{Mark, Protocol},
};

use super::{EntryKind, Refusal, entries, entry, install, listing::listed};

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
            "dawarich",
            "gitea",
            "owntracks-recorder",
            "postgresql",
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
            "TIME_ZONE",
            "DAWARICH_PORT"
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
        let answer = install(Some(listed));
        assert_eq!(
            answer.refusal,
            Some(Refusal::InstallNotBuilt),
            "{}",
            listed.entry.id
        );
        assert!(!answer.needs_open_to_network);
    }
    assert_eq!(install(None).refusal, Some(Refusal::NotListed));
    assert!(Refusal::InstallNotBuilt.reason().contains("task H6"));
}

#[test]
fn r_247_an_entry_with_the_socket_of_the_engine_is_never_installable() {
    let mut listed = entry("uptime-kuma").expect("listed").clone();
    listed.entry.anima.marks = vec![Mark::EngineSocket];
    assert_eq!(install(Some(&listed)).refusal, Some(Refusal::EngineSocket));
}

#[test]
fn r_229_an_entry_with_a_port_on_the_network_needs_open_to_the_network() {
    let mut listed = entry("uptime-kuma").expect("listed").clone();
    listed.entry.anima.marks = vec![Mark::NetworkPort {
        port: 3001,
        protocols: vec![Protocol::Tcp],
    }];
    let answer = install(Some(&listed));
    assert!(answer.needs_open_to_network);
    assert_eq!(answer.refusal, Some(Refusal::InstallNotBuilt));
}
