//! The recipes that ship with Anima pass the static test, except the ones whose need no rule
//! permits yet.

use std::path::Path;

use compose_rules::{Refusal, Rule};

use crate::{Code, check, read, shipped};

#[test]
fn host_047_each_shipped_recipe_passes_or_has_its_known_refusal() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("entries");
    let recipes = read(&root).expect("the entries folder of the crate is readable");
    let folders: Vec<&str> = recipes
        .iter()
        .map(|recipe| recipe.folder.as_str())
        .collect();
    assert_eq!(
        folders,
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
            "pihole",
            "postgresql",
            "qbittorrent",
            "radarr",
            "sonarr",
            "uptime-kuma",
            "vaultwarden"
        ]
    );
    for recipe in &recipes {
        let codes: Vec<Code> = check(recipe)
            .into_iter()
            .map(|finding| finding.code)
            .collect();
        // Pi-hole serves DNS on port 53. The rules permit a published port from 1024 only,
        // also with "Open to the network" on, so its file is refused until a rule permits it.
        let expected = match recipe.folder.as_str() {
            "pihole" => vec![Code::ComposeRefused(Refusal::File {
                rule: Rule::PortNotPermitted,
                service: Some(0),
            })],
            _ => Vec::new(),
        };
        assert_eq!(codes, expected, "{}", recipe.folder);
    }
}

#[test]
fn h38_the_recipes_built_into_the_crate_are_the_recipes_of_the_folder() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("entries");
    let recipes = read(&root).expect("the entries folder of the crate is readable");
    assert_eq!(shipped(), recipes);
}
