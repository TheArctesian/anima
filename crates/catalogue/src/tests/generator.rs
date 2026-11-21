//! The generator over a recipe that exists (SF-244, SF-255): it changes
//! an entry only by its digest or its version, keeps the review state, resets a passed test on a
//! machine when the digest changes, and never writes over a recipe that a person wrote.

use std::path::Path;

use super::linuxserver::{image, made, pin, read_by};
use crate::{
    Recipe,
    entry::{Entry, MachineTest},
    linuxserver::{Change, Existing, Pin, Port, recipe, reviewed, tested, update},
    read,
};

/// The pin of radarr with a new digest and a new version.
fn newer() -> Pin {
    let mut newer = pin("radarr");
    newer.tag = "7.0.4.11000-ls330".to_owned();
    newer.digest = format!("sha256:{}", "ab".repeat(32));
    newer.resolved = "2026-12-21".to_owned();
    newer.read = "2026-12-20".to_owned();
    newer
}

fn last_test(recipe: &Recipe) -> MachineTest {
    let text = recipe.entry.as_deref().expect("an entry");
    serde_json::from_str::<Entry>(text)
        .expect("an entry")
        .anima
        .last_test
}

#[test]
fn sf_lsgenerator_the_generator_changes_an_entry_only_by_its_digest_or_its_version() {
    let existing = made("radarr");
    let newer = newer();
    let unread = recipe(&image("radarr"), &newer, Existing::default()).expect("fits");
    assert_eq!(update(&existing, &unread), Err(Change::NotOnlyPin));
    let kept = reviewed(&existing);
    assert_eq!(kept, Some(read_by()));
    let around = Existing {
        reviewed: kept.as_ref(),
        tested: None,
        taken: &[],
    };
    let repinned = recipe(&image("radarr"), &newer, around).expect("fits");
    assert_eq!(update(&existing, &repinned), Ok(()));
    let mut wider = image("radarr");
    wider.config.ports.push(Port {
        external: "9898".to_owned(),
        internal: "9898".to_owned(),
        optional: false,
    });
    let widened = recipe(&wider, &newer, around).expect("fits");
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
fn sf_revlsresidual_a_new_digest_never_keeps_a_passed_test_on_a_machine() {
    let read = read_by();
    let old = pin("radarr");
    let passed = MachineTest::Passed {
        date: "2026-11-22".to_owned(),
        digests: vec![old.digest.clone()],
    };
    let around = Existing {
        reviewed: Some(&read),
        tested: Some(&passed),
        taken: &[],
    };
    let existing = recipe(&image("radarr"), &old, around).expect("fits");
    assert_eq!(
        last_test(&existing),
        passed,
        "the same digest keeps its pass"
    );
    let kept = tested(&existing);
    let around = Existing {
        tested: kept.as_ref(),
        ..around
    };
    let repinned = recipe(&image("radarr"), &newer(), around).expect("fits");
    assert_eq!(last_test(&repinned), MachineTest::NotTested);
    assert_eq!(
        update(&existing, &repinned),
        Ok(()),
        "a reset is a pin update"
    );
    let failed = Recipe {
        entry: (repinned.entry.as_deref()).map(|text| {
            text.replace(
                "\"result\": \"not-tested\"",
                "\"result\": \"failed\",\n      \"date\": \"2026-12-22\"",
            )
        }),
        ..repinned.clone()
    };
    assert_eq!(
        update(&existing, &failed),
        Err(Change::NotOnlyPin),
        "a pin update sets no other result"
    );
}
