//! The tool inventory of a batch (K8 Section 4; CAP-020, CAP-024, CAP-029): optional, so an
//! older daemon's batch is taken as it was; held to fixed alphabets and counts; a tool name the
//! hub does not know is dropped, not refused.

use proptest::prelude::*;
use serde_json::json;

use super::*;
use crate::inventory::{Architecture, Inventory, Os, Reason, Tool, ToolName, Verdict};
use crate::wire::{BatchInventory, BatchTool};

fn found(name: &str, version: &str) -> BatchTool {
    BatchTool {
        name: name.to_owned(),
        version: Some(version.to_owned()),
        state: None,
    }
}

fn not_usable(name: &str, reason: Reason) -> BatchTool {
    BatchTool {
        name: name.to_owned(),
        version: None,
        state: Some(reason),
    }
}

fn mac() -> BatchInventory {
    BatchInventory {
        architecture: Architecture::Aarch64,
        os: Os::Macos,
        os_version: "27.0".to_owned(),
        tools: vec![
            found("cargo", "1.91.0"),
            not_usable("bun", Reason::TimedOut),
        ],
        build_targets: vec!["aarch64-apple-darwin".to_owned()],
    }
}

fn with(change: impl FnOnce(&mut BatchInventory)) -> impl FnOnce(&mut Batch) {
    move |batch: &mut Batch| {
        let mut inventory = mac();
        change(&mut inventory);
        batch.inventory = Some(inventory);
    }
}

#[test]
fn cap_029_a_batch_without_an_inventory_is_taken_and_leaves_it_unchanged() {
    assert!(
        !BATCH.contains("inventory"),
        "the example is an older daemon's"
    );
    assert_eq!(checked(|_| {}).unwrap().inventory, None);
}

#[test]
fn cap_020_an_inventory_reaches_the_hub_with_its_tools_in_name_order() {
    let accepted = checked(with(|_| {})).unwrap();
    let expected = Inventory {
        architecture: Architecture::Aarch64,
        os: Os::Macos,
        os_version: "27.0".to_owned(),
        tools: vec![
            Tool {
                name: ToolName::Cargo,
                verdict: Verdict::Usable("1.91.0".to_owned()),
            },
            Tool {
                name: ToolName::Bun,
                verdict: Verdict::NotUsable(Reason::TimedOut),
            },
        ],
        build_targets: vec!["aarch64-apple-darwin".to_owned()],
    };
    assert_eq!(accepted.inventory, Some(expected));
}

#[test]
fn cap_029_the_inventory_travels_as_the_k8_json() {
    let text = r#"{ "architecture": "aarch64", "os": "macos", "osVersion": "27.0",
        "tools": [ { "name": "cargo", "version": "1.91.0" }, { "name": "bun", "state": "timed_out" } ],
        "buildTargets": ["aarch64-apple-darwin"] }"#;
    let parsed: BatchInventory = serde_json::from_str(text).unwrap();
    assert_eq!(parsed, mac());
    assert_eq!(
        serde_json::to_value(&parsed).unwrap(),
        json!({ "architecture": "aarch64", "os": "macos", "osVersion": "27.0",
            "tools": [ { "name": "cargo", "version": "1.91.0" }, { "name": "bun", "state": "timed_out" } ],
            "buildTargets": ["aarch64-apple-darwin"] })
    );
}

#[test]
fn cap_029_a_newer_daemon_s_tool_and_state_are_ignored_and_read_as_failed() {
    let accepted = checked(with(|inventory| {
        inventory.tools.push(found("zig", "0.15.1"));
    }))
    .unwrap();
    let names: Vec<ToolName> = accepted
        .inventory
        .unwrap()
        .tools
        .iter()
        .map(|tool| tool.name)
        .collect();
    assert_eq!(names, [ToolName::Cargo, ToolName::Bun]);
    let newer: BatchTool = serde_json::from_str(r#"{ "name": "bun", "state": "denied" }"#).unwrap();
    assert_eq!(newer.state, Some(Reason::Failed));
}

#[test]
fn cap_024_each_reason_travels_as_its_fixed_code_and_comes_back_whole() {
    for (reason, code) in [
        (Reason::TimedOut, "timed_out"),
        (Reason::Exited, "exited"),
        (Reason::NoVersion, "no_version"),
        (Reason::Failed, "failed"),
    ] {
        let tool = not_usable("bun", reason);
        assert_eq!(
            serde_json::to_value(&tool).unwrap(),
            json!({ "name": "bun", "state": code })
        );
        assert_eq!(Reason::parse(code), reason);
        let accepted = checked(with(move |inventory| inventory.tools[1] = tool)).unwrap();
        assert_eq!(
            accepted.inventory.unwrap().tools[1].reason(),
            Some(reason),
            "{code}"
        );
    }
    assert_eq!(Reason::parse("denied"), Reason::Failed);
}

#[test]
fn cap_029_an_unknown_architecture_or_system_does_not_parse() {
    for text in [
        r#"{ "architecture": "riscv64", "os": "linux", "osVersion": "1", "tools": [], "buildTargets": [] }"#,
        r#"{ "architecture": "x86_64", "os": "plan9", "osVersion": "1", "tools": [], "buildTargets": [] }"#,
    ] {
        assert!(
            serde_json::from_str::<BatchInventory>(text).is_err(),
            "{text}"
        );
    }
}

#[test]
fn cap_029_an_empty_inventory_is_taken() {
    let accepted = checked(with(|inventory| {
        inventory.tools.clear();
        inventory.build_targets.clear();
    }))
    .unwrap();
    let inventory = accepted.inventory.unwrap();
    assert!(inventory.tools.is_empty() && inventory.build_targets.is_empty());
}

#[test]
fn cap_029_versions_and_the_system_version_that_break_the_alphabet_are_malformed() {
    let long = "1".repeat(65);
    for bad in [
        "",
        "1.0 beta",
        "1.0\n",
        "1.0\u{7}",
        "versión",
        "1;rm",
        long.as_str(),
    ] {
        assert_malformed(&format!("os version {bad:?}"), {
            let bad = bad.to_owned();
            with(move |inventory| inventory.os_version = bad)
        });
        assert_malformed(&format!("version {bad:?}"), {
            let bad = bad.to_owned();
            with(move |inventory| inventory.tools[0].version = Some(bad))
        });
    }
    assert_accepted(
        "64 bytes",
        with(|inventory| inventory.tools[0].version = Some("1".repeat(64))),
    );
}

#[test]
fn cap_024_a_tool_needs_a_version_or_a_state_but_not_both() {
    assert_malformed(
        "both",
        with(|inventory| inventory.tools[0].state = Some(Reason::Exited)),
    );
    assert_malformed("neither", with(|inventory| inventory.tools[1].state = None));
}

#[test]
fn cap_029_tool_names_must_be_short_lower_case_and_unique() {
    assert_malformed(
        "a repeated name",
        with(|inventory| inventory.tools.push(found("cargo", "1.0"))),
    );
    for bad in ["", "Cargo", "car go", "cargo\u{0}", &"x".repeat(65)] {
        let bad = bad.to_owned();
        assert_malformed(&format!("name {bad:?}"), {
            let bad = bad.clone();
            with(move |inventory| inventory.tools[0].name = bad)
        });
    }
    let many: Vec<BatchTool> = (0..65).map(|i| found(&format!("t{i}"), "1")).collect();
    assert_malformed("65 tools", with(move |inventory| inventory.tools = many));
    let most: Vec<BatchTool> = (0..64).map(|i| found(&format!("t{i}"), "1")).collect();
    assert_accepted("64 tools", with(move |inventory| inventory.tools = most));
}

#[test]
fn cap_029_build_targets_must_be_short_lower_case_unique_and_at_most_128() {
    for bad in ["", "AARCH64-apple-darwin", "x86_64 linux", &"x".repeat(65)] {
        let bad = bad.to_owned();
        assert_malformed(&format!("target {bad:?}"), {
            with(move |inventory| inventory.build_targets.push(bad))
        });
    }
    assert_malformed(
        "a repeated target",
        with(|inventory| {
            inventory
                .build_targets
                .push("aarch64-apple-darwin".to_owned());
        }),
    );
    let many: Vec<String> = (0..129).map(|i| format!("t{i}")).collect();
    assert_malformed(
        "129 targets",
        with(move |inventory| inventory.build_targets = many),
    );
    let most: Vec<String> = (0..128).map(|i| format!("t{i}")).collect();
    assert_accepted(
        "128 targets",
        with(move |inventory| inventory.build_targets = most),
    );
}

fn any_tool() -> impl Strategy<Value = BatchTool> {
    (
        "\\PC{0,8}",
        proptest::option::of("\\PC{0,8}"),
        proptest::option::of(proptest::sample::select(vec![
            Reason::TimedOut,
            Reason::Exited,
            Reason::NoVersion,
            Reason::Failed,
        ])),
    )
        .prop_map(|(name, version, state)| BatchTool {
            name,
            version,
            state,
        })
}

proptest! {
    /// Whatever a device sends, the check never panics, and what it takes holds only text of the
    /// fixed alphabets, known tool names and at most the counts of K8 Section 4.
    #[test]
    fn cap_031_any_inventory_is_refused_or_taken_within_the_rules(
        os_version in "\\PC{0,10}",
        tools in proptest::collection::vec(any_tool(), 0..8),
        targets in proptest::collection::vec("[a-z0-9_.\\- A]{0,10}", 0..8),
    ) {
        let sent = BatchInventory {
            architecture: Architecture::X86_64,
            os: Os::Linux,
            os_version,
            tools,
            build_targets: targets,
        };
        if let Some(taken) = sent.accepted() {
            prop_assert!(crate::inventory::is_version(&taken.os_version));
            prop_assert!(taken.tools.iter().all(|tool| tool
                .version()
                .is_none_or(crate::inventory::is_version)));
            prop_assert!(taken
                .build_targets
                .iter()
                .all(|target| crate::inventory::is_build_target(target)));
        }
    }
}
