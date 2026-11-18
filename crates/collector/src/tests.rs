use std::path::{Path, PathBuf};

use crate::{FolderRule, ReadPosition, whole_lines};

const ROOT: &str = "/home/operator/.claude/projects";

fn permits(rule: &FolderRule, real: &str) -> bool {
    rule.permits(Path::new(ROOT), Path::new(real))
}

fn folders(names: &[&str]) -> Vec<PathBuf> {
    names
        .iter()
        .map(|name| Path::new(ROOT).join(name))
        .collect()
}

#[test]
fn col_006_the_default_rule_permits_each_folder_of_the_source() {
    let rule = FolderRule::default();

    assert!(permits(
        &rule,
        "/home/operator/.claude/projects/anima/a.jsonl"
    ));
    assert!(permits(&rule, ROOT));
}

#[test]
fn col_006_an_exclude_list_refuses_its_folders_and_the_folders_in_them() {
    let rule = FolderRule::AllBut(folders(&["client-work"]));

    assert!(!permits(
        &rule,
        "/home/operator/.claude/projects/client-work/a.jsonl"
    ));
    assert!(!permits(
        &rule,
        "/home/operator/.claude/projects/client-work/deep/b.jsonl"
    ));
    assert!(permits(
        &rule,
        "/home/operator/.claude/projects/client-work-notes/a.jsonl"
    ));
    assert!(permits(
        &rule,
        "/home/operator/.claude/projects/anima/a.jsonl"
    ));
}

#[test]
fn col_006_an_include_list_permits_only_its_folders() {
    let rule = FolderRule::Only(folders(&["anima"]));

    assert!(permits(
        &rule,
        "/home/operator/.claude/projects/anima/a.jsonl"
    ));
    assert!(!permits(
        &rule,
        "/home/operator/.claude/projects/other/a.jsonl"
    ));
    assert!(!permits(
        &FolderRule::Only(Vec::new()),
        "/home/operator/.claude/projects/anima/a.jsonl"
    ));
}

#[test]
fn col_006_a_path_that_a_link_resolved_out_of_the_source_is_refused() {
    let outside = "/home/operator/.ssh/id_ed25519";

    assert!(!permits(&FolderRule::All, outside));
    assert!(!permits(&FolderRule::AllBut(Vec::new()), outside));
    assert!(!permits(
        &FolderRule::Only(vec![PathBuf::from("/home/operator/.ssh")]),
        outside
    ));
}

#[test]
fn col_006_a_path_that_is_not_resolved_is_refused() {
    for path in [
        "/home/operator/.claude/projects/anima/../../../.ssh/id_ed25519",
        "anima/a.jsonl",
    ] {
        assert!(!permits(&FolderRule::All, path), "{path}");
    }
}

#[test]
fn col_004_only_complete_lines_are_read() {
    assert_eq!(whole_lines(b"one\ntwo\nthr"), b"one\ntwo\n");
    assert_eq!(whole_lines(b"one\n"), b"one\n");
    assert_eq!(whole_lines(b"no line feed yet"), b"");
    assert_eq!(whole_lines(b""), b"");
}

#[test]
fn col_004_the_position_moves_by_the_bytes_that_the_hub_stored() {
    let read = b"one\ntwo\nthr";
    let position = ReadPosition::default().after(whole_lines(read));

    assert_eq!(position.offset(), 8);
    assert_eq!(position.after(b"").offset(), 8);
}

#[test]
fn col_004_a_file_that_was_made_again_is_read_from_its_start() {
    let position = ReadPosition::at(4096);

    assert_eq!(position.in_file(8192), position);
    assert_eq!(position.in_file(4096), position);
    assert_eq!(position.in_file(100).offset(), 0);
}
