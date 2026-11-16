//! PRJ-030: a file path and a ref put into a request cannot change the host, leave the
//! repository's contents, or add a query or a fragment.

use proptest::prelude::*;

use super::path_and_query;
use crate::live::contents::{directory_request, file_request};
use crate::live::{commits, path};

const ROOT: &str = "/repos/o/r/contents/";

fn requested(path: &str) -> Result<String, String> {
    directory_request("o/r", path, None)
        .map(|request| request.path)
        .map_err(|problem| problem.code.to_owned())
}

#[test]
fn prj_030_ordinary_paths_are_encoded_segment_by_segment() {
    let table = [
        ("", ""),
        ("README.md", "README.md"),
        (".github/workflows/ci.yml", ".github/workflows/ci.yml"),
        ("a b/c#d", "a%20b/c%23d"),
        ("what?.txt", "what%3F.txt"),
        ("100%.txt", "100%25.txt"),
        ("caf\u{e9}/\u{2603}.txt", "caf%C3%A9/%E2%98%83.txt"),
        ("a+b", "a%2Bb"),
        ("a@b&c=d", "a%40b%26c%3Dd"),
        ("~tilde_ok-1.2", "~tilde_ok-1.2"),
        ("a%2eb", "a%252eb"),
        ("...", "..."),
    ];
    for (given, encoded) in table {
        assert_eq!(
            requested(given),
            Ok(format!("{ROOT}{encoded}")),
            "{given:?}"
        );
    }
}

#[test]
fn prj_030_the_file_and_the_directory_request_are_the_same_request() {
    let file = file_request("o/r", "src/a b.rs", Some("main")).unwrap();
    let directory = directory_request("o/r", "src/a b.rs", Some("main")).unwrap();

    assert_eq!(file, directory);
    assert_eq!(
        path_and_query(&file),
        "/repos/o/r/contents/src/a%20b.rs?ref=main"
    );
}

#[test]
fn prj_030_a_path_that_could_move_the_request_is_refused() {
    let long_segment = "a".repeat(256);
    let long_path = format!(
        "{}/{}/{}/{}/{}",
        "a".repeat(255),
        "a".repeat(255),
        "a".repeat(255),
        "a".repeat(252),
        "a".repeat(4)
    );
    let many = vec!["a"; 41].join("/");
    let refused = [
        "/etc/passwd",
        "a//b",
        "a/",
        "./a",
        "a/./b",
        "a/../b",
        "..",
        ".",
        "a\\b",
        "a\0b",
        "a\nb",
        "a\u{7f}b",
        "a\u{85}b",
        long_segment.as_str(),
        long_path.as_str(),
        many.as_str(),
    ];
    for path in refused {
        assert_eq!(requested(path), Err("path_unusable".to_owned()), "{path:?}");
    }
}

#[test]
fn prj_030_the_largest_allowed_paths_are_accepted() {
    let segment = "a".repeat(255);
    let widest = format!(
        "{}/{}/{}/{}/{}",
        "a".repeat(255),
        "a".repeat(255),
        "a".repeat(255),
        "a".repeat(252),
        "aaa"
    );
    let deepest = vec!["a"; 40].join("/");

    assert!(requested(&segment).is_ok());
    assert_eq!(widest.len(), 1_024);
    assert!(requested(&widest).is_ok());
    assert!(requested(&deepest).is_ok());
}

#[test]
fn prj_030_a_ref_goes_in_as_one_query_value_and_never_in_the_path() {
    let request = directory_request("o/r", "src", Some("feature/a&b#c?d")).unwrap();

    assert_eq!(request.path, "/repos/o/r/contents/src");
    assert_eq!(
        request.query,
        [("ref".to_owned(), "feature/a&b#c?d".to_owned())]
    );
}

#[test]
fn prj_030_a_hostile_ref_builds_no_request() {
    let long = "a".repeat(256);
    for git_ref in [
        "",
        "main branch",
        "a\tb",
        "a..b",
        "..",
        "-flag",
        "--upload-pack=x",
        "a\0b",
        "a\nb",
        "a\u{7f}b",
        "a\u{a0}b",
        long.as_str(),
    ] {
        assert_eq!(
            directory_request("o/r", "src", Some(git_ref))
                .unwrap_err()
                .code,
            "ref_unusable",
            "{git_ref:?}"
        );
        assert_eq!(
            file_request("o/r", "src", Some(git_ref)).unwrap_err().code,
            "ref_unusable",
            "{git_ref:?}"
        );
        assert_eq!(
            commits::list_request("o/r", Some(git_ref), 1)
                .unwrap_err()
                .code,
            "ref_unusable",
            "{git_ref:?}"
        );
    }
}

#[test]
fn prj_030_a_refused_path_or_ref_gives_fixed_words() {
    let hostile = "../secret";

    let problem = directory_request("o/r", hostile, None).unwrap_err();
    let ref_problem = directory_request("o/r", "src", Some(hostile)).unwrap_err();

    assert!(!problem.message.contains(hostile) && !problem.subject.contains(hostile));
    assert!(!ref_problem.message.contains(hostile) && !ref_problem.subject.contains(hostile));
    assert_eq!(path::encode("a/b"), Ok("a/b".to_owned()));
}

proptest! {
    #[test]
    fn prj_030_any_path_is_refused_or_stays_inside_the_contents_of_the_repository(
        path in prop_oneof![
            any::<String>(),
            "[a-z./ #?%+@\\\\\u{e9}~\\x00-\\x1f-]{0,80}",
        ],
    ) {
        match requested(&path) {
            Err(code) => prop_assert_eq!(code, "path_unusable"),
            Ok(built) => {
                prop_assert!(built.starts_with(ROOT));
                prop_assert!(!built.contains('?') && !built.contains('#'));
                prop_assert!(!built.contains("//"));
                prop_assert!(!built.contains("/../") && !built.contains("/./"));
                prop_assert!(!built.ends_with("/..") && !built.ends_with("/."));
                prop_assert!(built.bytes().all(|b| b.is_ascii_alphanumeric()
                    || matches!(b, b'-' | b'.' | b'_' | b'~' | b'/' | b'%')));
            }
        }
    }

    #[test]
    fn prj_030_any_ref_is_refused_or_is_the_one_unchanged_query_value(
        git_ref in prop_oneof![any::<String>(), "[a-z./ -]{0,20}"],
    ) {
        match directory_request("o/r", "src", Some(&git_ref)) {
            Err(problem) => prop_assert_eq!(problem.code, "ref_unusable"),
            Ok(request) => {
                prop_assert_eq!(request.path, "/repos/o/r/contents/src");
                prop_assert_eq!(request.query, vec![("ref".to_owned(), git_ref)]);
            }
        }
    }
}
