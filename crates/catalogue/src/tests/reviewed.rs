//! The read of a generated entry (SF-NEW-revlsreview): the static test refuses a generated entry
//! that does not say who read it and when. A recipe that a person wrote needs no read.

use std::path::Path;

use crate::{Code, Recipe, check, linuxserver::reviewed, read};

const READ: &str = r#""reviewed": {
      "by": "security reviewer agent, overnight session of the orchestrator",
      "on": "2026-11-21"
    },"#;

fn shipped(name: &str) -> Recipe {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("entries");
    read(&root)
        .expect("the entries folder of the crate is readable")
        .into_iter()
        .find(|recipe| recipe.folder == name)
        .expect("the recipe ships")
}

fn with_read(recipe: &Recipe, new: &str) -> Recipe {
    let entry = recipe.entry.as_deref().expect("an entry");
    assert!(entry.contains(READ), "the shipped entry has its read");
    Recipe {
        entry: Some(entry.replace(READ, new)),
        ..recipe.clone()
    }
}

fn codes(recipe: &Recipe) -> Vec<Code> {
    check(recipe)
        .into_iter()
        .map(|finding| finding.code)
        .collect()
}

#[test]
fn sf_revlsreview_a_generated_entry_says_who_read_it_and_when() {
    let radarr = shipped("radarr");
    assert_eq!(codes(&radarr), []);
    assert_eq!(codes(&with_read(&radarr, "")), [Code::NotReviewed]);
    let nobody = r#""reviewed": { "by": " ", "on": "2026-11-21" },"#;
    assert_eq!(codes(&with_read(&radarr, nobody)), [Code::NotReviewed]);
    let no_day = r#""reviewed": { "by": "a person", "on": "21 November" },"#;
    assert_eq!(codes(&with_read(&radarr, no_day)), [Code::DateNotPermitted]);
    let gitea = shipped("gitea");
    assert_eq!(
        reviewed(&gitea),
        None,
        "a recipe that a person wrote needs no read"
    );
    assert_eq!(codes(&gitea), []);
}
