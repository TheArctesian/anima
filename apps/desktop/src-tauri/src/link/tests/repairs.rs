//! The repair of a kept link while the app runs (HMD-058): when the count asks for it, and what
//! the shell asks the link program then.

use std::fs;

use super::super::{
    folder,
    repair::{DOWN_LOOKS, Repair, TRIES},
};
use super::kept::kept_in;
use super::runs::stand_in;

/// How many of `looks` looks that found no link asked for a repair.
fn repairs(repair: &mut Repair, looks: u32) -> u32 {
    (0..looks).map(|_| u32::from(repair.due(false))).sum()
}

#[test]
fn hmd_058_a_link_that_is_down_for_about_30_seconds_is_repaired_and_a_shorter_gap_is_not() {
    let mut repair = Repair::default();
    assert_eq!(repairs(&mut repair, DOWN_LOOKS - 1), 0);
    assert!(!repair.due(true), "a link that answers needs no repair");
    assert_eq!(
        repairs(&mut repair, DOWN_LOOKS - 1),
        0,
        "the count started again"
    );
    assert!(repair.due(false));
}

#[test]
fn hmd_058_one_run_of_the_app_repairs_a_few_times_and_then_leaves_the_unit() {
    let mut repair = Repair::default();
    assert_eq!(repairs(&mut repair, DOWN_LOOKS * (TRIES + 5)), TRIES);
    assert!(!repair.due(true));
    assert_eq!(
        repairs(&mut repair, DOWN_LOOKS * 2),
        0,
        "no try is left in this run"
    );
}

#[test]
fn hmd_058_a_unit_that_was_removed_is_installed_again_and_its_link_answers() {
    let state = tempfile::tempdir().unwrap();
    let link = kept_in(state.path(), 41001);
    folder::want_unit(state.path()).unwrap();
    link.install().unwrap();
    fs::remove_file(state.path().join("unit")).unwrap();
    assert_eq!(link.serving(), None);

    assert!(link.put_back(None, None));
    assert!(link.keeps_this_folder());
    assert_eq!(link.serving(), Some(41001));
}

#[test]
fn hmd_058_a_folder_that_is_not_marked_for_a_unit_gets_none() {
    let state = tempfile::tempdir().unwrap();
    let link = kept_in(state.path(), 41001);
    assert!(!link.put_back(None, None));
    assert!(!link.keeps_this_folder());
    let calls = fs::read_to_string(state.path().join("calls")).unwrap();
    assert!(!calls.contains("unit install"), "{calls}");
}

#[test]
fn hmd_058_a_link_program_that_is_gone_or_refuses_gives_no_repair() {
    let state = tempfile::tempdir().unwrap();
    folder::want_unit(state.path()).unwrap();
    let mut link = kept_in(state.path(), 41001);
    link.program = stand_in(state.path(), "exit 1");
    assert!(
        !link.put_back(None, None),
        "the link program refuses the unit"
    );
    fs::remove_file(&link.program).unwrap();
    assert!(!link.put_back(None, None), "no copy and no carried program");
}
