//! The rules of a new release and the proposed version (BLD-001, BLD-002).

use crate::builds::{
    model::{JobState, Target},
    release::{self, Built, ReleaseInput},
    version::Version,
};

const COMMIT: &str = "0123456789abcdef0123456789abcdef01234567";

fn input(version: &str, commit: &str, targets: &[Target]) -> ReleaseInput {
    ReleaseInput {
        version: version.to_owned(),
        commit: commit.to_owned(),
        targets: targets.to_vec(),
    }
}

fn built(target: Target, version: Version, state: JobState) -> Built {
    Built {
        target,
        version,
        state,
    }
}

fn fields(errors: &[crate::builds::FieldError]) -> Vec<&str> {
    errors.iter().map(|e| e.field).collect()
}

#[test]
fn bld_001_a_release_names_each_target_once_and_a_lower_case_commit() {
    let targets = [
        Target::DarwinUniversal,
        Target::LinuxX86_64,
        Target::DarwinUniversal,
    ];
    let new = release::validate(&input(" 0.1.12 ", &COMMIT.to_uppercase(), &targets)).unwrap();
    assert_eq!(new.version, Version::new(0, 1, 12));
    assert_eq!(new.commit.as_str(), COMMIT);
    assert_eq!(new.targets, [Target::LinuxX86_64, Target::DarwinUniversal]);
}

#[test]
fn bld_001_every_bad_field_is_listed() {
    let errors = release::validate(&input("v0.1", "main", &[])).unwrap_err();
    assert_eq!(fields(&errors), ["version", "commit", "targets"]);
    let short = release::validate(&input("0.1.12", &COMMIT[..39], &[Target::LinuxX86_64]));
    assert_eq!(fields(&short.unwrap_err()), ["commit"]);
    let not_hex = format!("{}g", &COMMIT[..39]);
    let not_hex = release::validate(&input("0.1.12", &not_hex, &[Target::LinuxX86_64]));
    assert_eq!(fields(&not_hex.unwrap_err()), ["commit"]);
}

#[test]
fn bld_002_a_version_not_above_a_published_one_of_a_selected_target_is_refused() {
    let published = [built(
        Target::LinuxX86_64,
        Version::new(0, 1, 11),
        JobState::Published,
    )];
    let targets = [Target::LinuxX86_64, Target::DarwinUniversal];
    for version in [Version::new(0, 1, 11), Version::new(0, 1, 2)] {
        let refused = release::check_new(version, &targets, &published).unwrap_err();
        assert_eq!(refused.field, "version");
        assert!(refused.message.contains("linux-x86_64"), "{}", refused.message);
    }
    assert!(release::check_new(Version::new(0, 1, 12), &targets, &published).is_ok());
    // The other target's published versions do not count.
    let darwin_only = [Target::DarwinUniversal];
    assert!(release::check_new(Version::new(0, 1, 2), &darwin_only, &published).is_ok());
}

#[test]
fn bld_002_a_version_is_built_at_most_once_for_a_target() {
    let version = Version::new(0, 1, 12);
    let target = [Target::LinuxX86_64];
    for state in [JobState::Queued, JobState::Building, JobState::Uploading] {
        let under_way = [built(Target::LinuxX86_64, version, state)];
        assert!(release::check_new(version, &target, &under_way).is_err(), "{state:?}");
    }
    for state in [JobState::Failed, JobState::Cancelled] {
        let freed = [built(Target::LinuxX86_64, version, state)];
        assert!(release::check_new(version, &target, &freed).is_ok(), "{state:?}");
    }
}

#[test]
fn bld_002_the_form_proposes_the_highest_version_with_the_patch_plus_one() {
    assert_eq!(release::next_version(&[]), None);
    let built = [
        built(Target::LinuxX86_64, Version::new(0, 1, 9), JobState::Published),
        built(Target::DarwinUniversal, Version::new(0, 1, 10), JobState::Building),
        built(Target::LinuxX86_64, Version::new(0, 2, 0), JobState::Failed),
    ];
    assert_eq!(release::next_version(&built), Some(Version::new(0, 1, 11)));
}
