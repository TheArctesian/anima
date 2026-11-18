//! The repository name (AIS-002, R-134): the remote `origin` of a git configuration, without
//! credentials, and never a local path.

use std::path::Path;

use proptest::prelude::*;

use crate::agent_usage::is_repository;
use crate::agent_usage::repository::{claude_worktree_main, gitdir_link, origin_url, remote_name};

const CONFIG: &str = r#"[core]
	repositoryformatversion = 0
	url = https://wrong.example/not/origin
[remote "upstream"]
	url = git@github.com:other/project.git
[remote "origin"]
	fetch = +refs/heads/*:refs/remotes/origin/*
	url = "https://x-access-token:ghp_MARKER@github.com/Me/App.git" ; a comment
[branch "main"]
	url = https://wrong.example/after/origin
"#;

#[test]
fn ais_002_the_origin_url_is_read_from_its_section_only() {
    assert_eq!(
        origin_url(CONFIG),
        Some("https://x-access-token:ghp_MARKER@github.com/Me/App.git")
    );
    assert_eq!(
        origin_url("[remote.origin]\nurl = git@h.io:a/b # c"),
        Some("git@h.io:a/b")
    );
    assert_eq!(origin_url("[remote \"origin\"]\nfetch = x"), None);
    assert_eq!(origin_url(""), None);
}

#[test]
fn r_134_a_remote_url_becomes_host_owner_name_without_credentials() {
    let cases = [
        (
            "https://x-access-token:ghp_MARKER@github.com/Me/App.git",
            "github.com/Me/App",
        ),
        ("https://GitHub.com/me/app/", "github.com/me/app"),
        (
            "ssh://git@gitlab.example:2222/group/sub/app.git",
            "gitlab.example/group/sub/app",
        ),
        ("git@github.com:me/app.git", "github.com/me/app"),
        ("github.com:me/app", "github.com/me/app"),
        ("git://host.example/me/app?ref=x#y", "host.example/me/app"),
    ];
    for (url, name) in cases {
        assert_eq!(remote_name(url).as_deref(), Some(name), "{url}");
    }
}

#[test]
fn ais_018_a_local_or_odd_remote_has_no_name() {
    for url in [
        "/Users/me/repos/app.git",
        "./app",
        "../app.git",
        "file:///Users/me/app",
        "C:\\repos\\app",
        "https://github.com/app",
        "https://[::1]/me/app",
        "https://u:p/ss@host.example/me/app",
        "ftp://host.example/me/app",
        "https://host.example/me/../app",
        "",
    ] {
        assert_eq!(remote_name(url), None, "{url}");
    }
}

#[test]
fn ais_002_a_worktree_s_link_and_main_folder_are_found() {
    assert_eq!(
        gitdir_link("gitdir: /r/app/.git/worktrees/wt\n"),
        Some("/r/app/.git/worktrees/wt")
    );
    assert_eq!(gitdir_link("ref: refs/heads/main"), None);
    let worktree = Path::new("/r/app/.claude/worktrees/feature-x/crates");
    assert_eq!(claude_worktree_main(worktree), Some(Path::new("/r/app")));
    assert_eq!(
        claude_worktree_main(Path::new("/r/app/.claude/worktrees")),
        None
    );
    assert_eq!(claude_worktree_main(Path::new("/r/app")), None);
}

proptest! {
    /// Whatever the URL, a name is one the hub takes, with no `@`, `:` or space in it.
    #[test]
    fn r_134_any_remote_gives_a_valid_name_or_none(url in "\\PC{0,80}") {
        if let Some(name) = remote_name(&url) {
            prop_assert!(is_repository(&name));
            prop_assert!(!name.contains(['@', ':', ' ']));
        }
    }

    /// Credentials in any URL never reach the name.
    #[test]
    fn r_134_credentials_never_reach_the_name(user in "[a-z]{1,8}", secret in "[A-Za-z0-9/_.-]{1,16}") {
        let url = format!("https://{user}:{secret}MARKER@github.com/me/app.git");
        if let Some(name) = remote_name(&url) {
            prop_assert!(!name.contains("MARKER"));
        }
    }
}
