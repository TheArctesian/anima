//! Where a build step writes what cargo builds: inside the builder's clone, whatever the
//! builder's own environment or cargo configuration names (BLD-044).

use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use super::COMMIT;
use crate::builder::{
    child::Running,
    environment::Environment,
    job::Commit,
    plan::{self, Repository},
    steps::Workshop,
};

#[test]
fn bld_044_every_step_is_told_to_build_inside_the_clone_whatever_the_builder_configured() {
    let clone = PathBuf::from("/data/builds/anima");
    let running = Running::default();
    let parent = [
        ("HOME".into(), "/home/builder".into()),
        (
            "CARGO_TARGET_DIR".into(),
            "/home/builder/.cache/cargo-target".into(),
        ),
        ("CARGO_BUILD_TARGET_DIR".into(), "/elsewhere".into()),
    ];
    let workshop = Workshop {
        claim: running.claim(Arc::new(|_| {})).unwrap(),
        env: Environment::from_parent(parent),
        repository: Repository::local(Path::new("/data/anima.git")),
        clone: clone.clone(),
    };
    let variables = workshop.variables(&plan::gates(&clone, &Commit::parse(COMMIT).unwrap()));
    let named = |name: &str| -> Vec<_> {
        variables
            .iter()
            .filter(|(key, _)| key == name)
            .map(|(_, value)| PathBuf::from(value))
            .collect()
    };
    assert_eq!(named("CARGO_TARGET_DIR"), [clone.join("target")]);
    assert_eq!(named("CARGO_BUILD_TARGET_DIR"), [] as [PathBuf; 0]);
    assert_eq!(named("BASE_REF"), [PathBuf::from(COMMIT)]);
}
