//! The checked parts accept ordinary values and refuse hostile ones (OD-E14, PRJ-027 to PRJ-035).

use proptest::prelude::*;

use super::long;
use crate::{
    BranchName, CommitSha, Number, PartError, RepositoryName, RunId, StatusContext, TargetUrl,
    WorkflowId,
};

/// Hostile text that tries to leave a path segment or a value.
const HOSTILE: &[&str] = &[
    "",
    ".",
    "..",
    "../x",
    "x/..",
    "a/b",
    "a?b",
    "a#b",
    "a%2e",
    "a%2Fb",
    "a\0b",
    "a\nb",
    "a\rb",
    "a\u{1b}[31m",
    "a b",
    "a@b",
    "a\\b",
    "https://evil.example",
];

#[test]
fn od_e14_a_repository_name_is_two_plain_segments() {
    let name = RepositoryName::parse("octo-org/hello.world_2").unwrap();
    assert_eq!((name.owner(), name.name()), ("octo-org", "hello.world_2"));
    assert_eq!(name.to_string(), "octo-org/hello.world_2");
    for hostile in HOSTILE {
        for full in [format!("{hostile}/n"), format!("o/{hostile}")] {
            assert_eq!(
                RepositoryName::parse(&full),
                Err(PartError::Repository),
                "{full:?}"
            );
        }
    }
    for full in [
        "",
        "o",
        "o/n/x",
        "/n",
        "o/",
        &format!("{}/n", long('a', 40)),
        &format!("o/{}", long('a', 101)),
    ] {
        assert_eq!(
            RepositoryName::parse(full),
            Err(PartError::Repository),
            "{full:?}"
        );
    }
    assert!(RepositoryName::parse(&format!("{}/{}", long('a', 39), long('b', 100))).is_ok());
}

#[test]
fn od_e14_a_number_is_one_to_two_to_the_fifty_three() {
    assert_eq!(Number::new(12).unwrap().get(), 12);
    assert!(Number::new(1 << 53).is_ok());
    assert_eq!(Number::new(0), Err(PartError::Number));
    assert_eq!(Number::new((1 << 53) + 1), Err(PartError::Number));
}

#[test]
fn od_e14_a_run_id_is_canonical_digits() {
    assert_eq!(RunId::parse("30433642").unwrap().get(), 30_433_642);
    assert!(RunId::parse("9007199254740992").is_ok());
    for text in [
        "0",
        "01",
        "+1",
        "-1",
        " 1",
        "1 ",
        "1e3",
        "\u{661}",
        "9007199254740993",
        "99999999999999999999",
    ] {
        assert_eq!(RunId::parse(text), Err(PartError::Number), "{text:?}");
    }
    for hostile in &HOSTILE[1..] {
        assert_eq!(
            RunId::parse(&format!("1{hostile}")),
            Err(PartError::Number),
            "{hostile:?}"
        );
    }
}

#[test]
fn od_e14_a_commit_is_forty_hex_digits_kept_lower_case() {
    let sha = "6dcb09b5b57875f334f61aebed695e2e4193db5e";
    assert_eq!(CommitSha::parse(sha).unwrap().as_str(), sha);
    assert_eq!(
        CommitSha::parse(&sha.to_ascii_uppercase())
            .unwrap()
            .as_str(),
        sha
    );
    for text in [
        &sha[..39],
        &format!("{sha}0"),
        &format!("{}g", &sha[..39]),
        &format!("../{}", &sha[3..]),
    ] {
        assert_eq!(CommitSha::parse(text), Err(PartError::Sha), "{text:?}");
    }
}

#[test]
fn prj_035_a_workflow_is_a_number_or_a_yaml_file_name() {
    for text in ["161335", "build.yml", "ci-main_2.yaml"] {
        assert_eq!(WorkflowId::parse(text).unwrap().as_str(), text);
    }
    for text in [
        "0",
        ".yml",
        "build",
        "build.json",
        "../build.yml",
        "a/build.yml",
        "build.yml?x",
        "build.yml#x",
        "%2e.yml",
        "b\0.yml",
        "b uild.yml",
    ] {
        assert_eq!(
            WorkflowId::parse(text),
            Err(PartError::Workflow),
            "{text:?}"
        );
    }
    assert!(WorkflowId::parse(&format!("{}.yml", long('a', 252))).is_err());
}

#[test]
fn prj_035_a_branch_is_one_git_allows() {
    for text in [
        "main",
        "feature/x-1",
        "release/v1.2",
        "fix#12",
        "renovate/@types-node",
        "ünïcode",
    ] {
        assert_eq!(BranchName::parse(text).unwrap().as_str(), text);
    }
    let refused = [
        "",
        "..",
        "a/../b",
        "a/./b",
        ".hidden",
        "a/.b",
        "/a",
        "a/",
        "a//b",
        "a.lock",
        "a/b.lock/c",
        "a b",
        "a~1",
        "a^",
        "a:b",
        "a?b",
        "a*b",
        "a[b",
        "a\\b",
        "a\0",
        "a\nb",
        "a\u{7f}",
        "@",
        "a@{1}",
        "-x",
        "a.",
    ];
    for text in refused {
        assert_eq!(BranchName::parse(text), Err(PartError::Branch), "{text:?}");
    }
    assert!(BranchName::parse(&long('a', 255)).is_ok());
    assert!(BranchName::parse(&long('a', 256)).is_err());
}

#[test]
fn pro_010_a_status_context_stays_under_anima() {
    for text in ["anima/build", "anima/deploy-2"] {
        assert_eq!(StatusContext::parse(text).unwrap().as_str(), text);
    }
    let refused = [
        "anima/",
        "ci/build",
        "continuous-integration/travis",
        "Anima/x",
        "anima/Build",
        "anima/a/b",
        "anima/../x",
        "anima/a?b",
        "anima/a b",
        "x/anima/a",
    ];
    for text in refused {
        assert_eq!(
            StatusContext::parse(text),
            Err(PartError::StatusContext),
            "{text:?}"
        );
    }
    assert!(StatusContext::parse(&format!("anima/{}", long('a', 40))).is_ok());
    assert!(StatusContext::parse(&format!("anima/{}", long('a', 41))).is_err());
}

#[test]
fn pro_010_a_status_address_is_below_the_hub_origin() {
    let origin = "https://hub.example";
    let url = "https://hub.example/projects/anima/builds/12";
    assert_eq!(TargetUrl::parse(url, origin).unwrap().as_str(), url);
    let refused = [
        "https://hub.example",
        "https://hub.example.evil/x",
        "https://hub.example@evil/x",
        "https://hub.example//evil.example/x",
        "https://evil.example/x",
        "javascript:alert(1)",
        "https://hub.example/a b",
        "https://hub.example/a\nb",
        "https://hub.example/a\\b",
    ];
    for text in refused {
        assert_eq!(
            TargetUrl::parse(text, origin),
            Err(PartError::TargetUrl),
            "{text:?}"
        );
    }
    assert!(TargetUrl::parse(&format!("{origin}/{}", long('a', 2048)), origin).is_err());
    for bad_origin in ["", "ftp://hub.example", "https://", "https://hub.example/"] {
        assert!(
            TargetUrl::parse("https://hub.example/x", bad_origin).is_err(),
            "{bad_origin:?}"
        );
    }
}

#[test]
fn log_007_a_refusal_never_quotes_the_refused_value() {
    let secret = "ghs_a1b2c3d4e5f6";
    let errors = [
        RepositoryName::parse(secret).unwrap_err(),
        WorkflowId::parse(secret).unwrap_err(),
        CommitSha::parse(secret).unwrap_err(),
        StatusContext::parse(secret).unwrap_err(),
        TargetUrl::parse(secret, "https://hub.example").unwrap_err(),
    ];
    for error in errors {
        assert!(!error.to_string().contains(secret));
        assert!(!format!("{error:?}").contains(secret));
    }
}

/// Whether `segment` is one segment a URL parser leaves as it is.
fn is_plain(segment: &str) -> bool {
    segment != "."
        && segment != ".."
        && !segment.is_empty()
        && segment
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_'))
}

proptest! {
    #[test]
    fn od_e14_every_accepted_path_part_is_a_plain_segment(
        text in prop_oneof![any::<String>(), "[a-z0-9./?#%@ \\\\\\x00-\\x1f-]{0,12}"],
    ) {
        if let Ok(name) = RepositoryName::parse(&text) {
            prop_assert!(is_plain(name.owner()) && is_plain(name.name()));
        }
        if let Ok(workflow) = WorkflowId::parse(&text) {
            prop_assert!(is_plain(workflow.as_str()));
        }
        if let Ok(sha) = CommitSha::parse(&text) {
            prop_assert!(is_plain(sha.as_str()));
        }
        if let Ok(run) = RunId::parse(&text) {
            prop_assert_eq!(run.get().to_string(), text);
        }
    }

    #[test]
    fn prj_035_an_accepted_branch_has_no_dot_segment_or_control_character(
        text in prop_oneof![any::<String>(), "[a-z./@{~^:?*\\[\\\\ \\x00-\\x1f-]{0,12}"],
    ) {
        if let Ok(branch) = BranchName::parse(&text) {
            prop_assert!(branch.as_str().split('/').all(|s| s != "." && s != ".." && !s.is_empty()));
            prop_assert!(!branch.as_str().chars().any(char::is_control));
        }
    }
}
