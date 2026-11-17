//! Who signs a macOS release (BLD-083, review S-2): the builder's own identity when the clone's
//! committed `tauri.conf.json` names the same, and the reading of that file.

use std::{fs, os::unix::fs::symlink};

use crate::builder::{
    carried::signer::{self, Signer},
    error::BuilderError,
};

const IDENTITY: &str = "Developer ID Application: Stephen Okita (7GDLG48PV6)";

#[test]
fn s12_s_2_the_signer_is_the_builders_own_identity_when_the_clone_names_the_same() {
    let signer = Signer::pinned(Some(IDENTITY), IDENTITY).unwrap();
    assert_eq!(
        (signer.identity.as_str(), signer.team.as_str()),
        (IDENTITY, "7GDLG48PV6")
    );
    let other = "Developer ID Application: Other Ltd (24VZTF6M5V)";
    for (own, clone) in [
        (Some(IDENTITY), other),
        (Some(other), IDENTITY),
        (None, IDENTITY),
        (Some(""), ""),
        (
            Some("Developer ID Application: X (bad)"),
            "Developer ID Application: X (bad)",
        ),
    ] {
        assert_eq!(
            Signer::pinned(own, clone),
            Err(BuilderError::Bundle),
            "{own:?} {clone}"
        );
    }
    assert_eq!(
        crate::builder::tools::readings::signing_identity().as_deref(),
        Some(IDENTITY),
        "the builder's own identity comes from the config it was built with"
    );
}

#[test]
fn s12_d_5_the_signing_identity_is_read_from_the_clones_own_config() {
    let root = tempfile::tempdir().unwrap();
    let clone = fs::canonicalize(root.path()).unwrap().join("clone");
    let config = clone.join(signer::CONFIG);
    fs::create_dir_all(config.parent().unwrap()).unwrap();
    let with = |identity: &str| {
        serde_json::json!({ "bundle": { "macOS": { "signingIdentity": identity } } }).to_string()
    };
    fs::write(&config, with(IDENTITY)).unwrap();
    assert_eq!(signer::in_clone(&clone).unwrap(), IDENTITY);
    let signer = Signer::from_identity(IDENTITY).unwrap();
    assert_eq!(signer.team.as_str(), "7GDLG48PV6");
    for bad in [
        with(""),
        r#"{"bundle":{}}"#.to_owned(),
        "not json".to_owned(),
    ] {
        fs::write(&config, bad).unwrap();
        assert_eq!(signer::in_clone(&clone), Err(BuilderError::Bundle));
    }
    // A config the build swapped for a link to another file is not read.
    let elsewhere = root.path().join("tauri.conf.json");
    fs::write(&elsewhere, with(IDENTITY)).unwrap();
    fs::remove_file(&config).unwrap();
    symlink(&elsewhere, &config).unwrap();
    assert_eq!(signer::in_clone(&clone), Err(BuilderError::Bundle));
}
