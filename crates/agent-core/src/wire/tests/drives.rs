//! The drives of a batch (K8 Section 4, MET-022): optional, so an older agent's batch is taken
//! as it was, and each drive held to the label and count rules.

use super::*;
use crate::{
    readings::{captures::LSBLK_POP_OS, drives::drives},
    wire::{BatchDrive, DriveKind},
};

fn drive(name: &str, mounts: &[&str]) -> BatchDrive {
    BatchDrive {
        name: name.to_owned(),
        kind: DriveKind::Nvme,
        size_bytes: 1_000_204_886_016,
        mounts: mounts.iter().map(|mount| (*mount).to_owned()).collect(),
    }
}

fn with_drives(drives: Vec<BatchDrive>) -> impl FnOnce(&mut Batch) {
    move |batch: &mut Batch| batch.drives = Some(drives)
}

#[test]
fn met_022_a_batch_without_drives_is_taken_and_leaves_them_unchanged() {
    assert!(
        !BATCH.contains("drives"),
        "the K8 example is an older agent's"
    );
    let accepted = checked(|_| {}).unwrap();
    assert_eq!(accepted.drives, None);
}

#[test]
fn met_022_the_drives_of_a_batch_reach_the_hub_as_sent() {
    let sent = vec![drive("nvme1n1", &["/", "/boot/efi"]), drive("sda", &[])];
    let accepted = checked(with_drives(sent.clone())).unwrap();
    assert_eq!(accepted.drives, Some(sent));
    assert_eq!(
        checked(with_drives(Vec::new())).unwrap().drives,
        Some(Vec::new())
    );
}

#[test]
fn met_022_drives_travel_as_camel_case_json_and_an_unknown_kind_reads_as_disk() {
    let pop_os: Vec<BatchDrive> = drives(LSBLK_POP_OS)
        .unwrap()
        .iter()
        .map(BatchDrive::from)
        .collect();
    let text = serde_json::to_string(&pop_os[1]).unwrap();
    assert_eq!(
        text,
        r#"{"name":"nvme1n1","kind":"nvme","sizeBytes":1000204886016,"mounts":["/","/boot/efi"]}"#
    );
    let newer: BatchDrive =
        serde_json::from_str(r#"{"name":"sr0","kind":"optical","sizeBytes":1,"mounts":[]}"#)
            .unwrap();
    assert_eq!(newer.kind, DriveKind::Disk);
}

#[test]
fn met_022_drives_that_break_the_label_or_count_rules_are_malformed() {
    let many = (0..33).map(|i| drive(&format!("d{i}"), &[])).collect();
    assert_malformed("33 drives", with_drives(many));
    assert_malformed(
        "a repeated name",
        with_drives(vec![drive("sda", &[]), drive("sda", &[])]),
    );
    assert_malformed("an empty name", with_drives(vec![drive("", &[])]));
    assert_malformed(
        "a control character",
        with_drives(vec![drive("sd\u{7}", &[])]),
    );
    assert_malformed(
        "a relative mount",
        with_drives(vec![drive("sda", &["mnt"])]),
    );
    let long = format!("/{}", "x".repeat(128));
    assert_malformed("a long mount", with_drives(vec![drive("sda", &[&long])]));
    let mounts: Vec<String> = (0..33).map(|i| format!("/m{i}")).collect();
    let refs: Vec<&str> = mounts.iter().map(String::as_str).collect();
    assert_malformed("33 mounts", with_drives(vec![drive("sda", &refs)]));
    let mut huge = drive("sda", &[]);
    huge.size_bytes = u64::MAX;
    assert_malformed("a size past i64", with_drives(vec![huge]));
}
