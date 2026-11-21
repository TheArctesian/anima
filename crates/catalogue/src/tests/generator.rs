//! The generator over a recipe that exists (SF-NEW-lsgenerator, SF-NEW-revlsreview): it changes
//! an entry only by its digest or its version, keeps the read of the entry, and never writes
//! over a recipe that a person wrote.

use std::path::Path;

use super::linuxserver::{image, made, pin, read_by};
use crate::{
    linuxserver::{Change, Existing, Port, recipe, reviewed, update},
    read,
};

#[test]
fn sf_lsgenerator_the_generator_changes_an_entry_only_by_its_digest_or_its_version() {
    let existing = made("radarr");
    let mut newer = pin("radarr");
    newer.tag = "7.0.4.11000-ls330".to_owned();
    newer.digest = format!("sha256:{}", "ab".repeat(32));
    newer.resolved = "2026-12-21".to_owned();
    newer.read = "2026-12-20".to_owned();
    let unread = recipe(&image("radarr"), &newer, Existing::default()).expect("fits");
    assert_eq!(update(&existing, &unread), Err(Change::NotOnlyPin));
    let kept = reviewed(&existing);
    assert_eq!(kept, Some(read_by()));
    let around = Existing {
        reviewed: kept.as_ref(),
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
