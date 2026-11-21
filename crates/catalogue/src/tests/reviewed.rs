//! The review state of a generated entry (SF-255): the static test refuses a
//! generated entry with no state, and a review that names no person: an empty name, a bad day,
//! an agent, a session or a program. A recipe that a person wrote needs no state.

use std::path::Path;

use crate::{Code, Recipe, check, entry::Reviewed, linuxserver::reviewed, read};

const STATE: &str = r#""reviewed": {
      "state": "not-reviewed"
    },"#;

fn shipped(name: &str) -> Recipe {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("entries");
    read(&root)
        .expect("the entries folder of the crate is readable")
        .into_iter()
        .find(|recipe| recipe.folder == name)
        .expect("the recipe ships")
}

fn with_state(recipe: &Recipe, new: &str) -> Recipe {
    let entry = recipe.entry.as_deref().expect("an entry");
    assert!(entry.contains(STATE), "the shipped entry is not reviewed");
    Recipe {
        entry: Some(entry.replace(STATE, new)),
        ..recipe.clone()
    }
}

fn by(name: &str) -> String {
    format!(r#""reviewed": {{ "state": "reviewed", "by": "{name}", "on": "2026-11-21" }},"#)
}

fn codes(recipe: &Recipe) -> Vec<Code> {
    check(recipe)
        .into_iter()
        .map(|finding| finding.code)
        .collect()
}

#[test]
fn sf_revlsreview_a_generated_entry_gives_its_review_state() {
    let radarr = shipped("radarr");
    assert_eq!(reviewed(&radarr), Some(Reviewed::NotReviewed));
    assert_eq!(
        codes(&radarr),
        [],
        "not reviewed is a state that the list shows"
    );
    assert_eq!(codes(&with_state(&radarr, "")), [Code::NotReviewed]);
    assert_eq!(codes(&with_state(&radarr, &by("Ada Lovelace"))), []);
    let no_day =
        r#""reviewed": { "state": "reviewed", "by": "Ada Lovelace", "on": "21 November" },"#;
    assert_eq!(
        codes(&with_state(&radarr, no_day)),
        [Code::DateNotPermitted]
    );
    let gitea = shipped("gitea");
    assert_eq!(
        reviewed(&gitea),
        None,
        "a recipe that a person wrote has no state"
    );
    assert_eq!(codes(&gitea), []);
}

#[test]
fn sf_revlsreview_only_a_person_is_named_as_the_reviewer() {
    let radarr = shipped("radarr");
    for name in [
        " ",
        "security reviewer agent",
        "Claude",
        "release-bot",
        "Bot",
        "AUTOMATED check",
        "overnight session",
        "the orchestrator",
        "a subagent",
        "a0ef3ffeba3311ab3",
        "local_a764338c-3744",
    ] {
        assert_eq!(
            codes(&with_state(&radarr, &by(name))),
            [Code::ReviewerNotPermitted],
            "{name}"
        );
    }
    for name in ["Ada Lovelace", "Abbott", "J. Talbot", "Bea Cafe"] {
        assert_eq!(codes(&with_state(&radarr, &by(name))), [], "{name}");
    }
}
