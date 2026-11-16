//! Which tool inventories a `capabilities` filter keeps (CAP-030): a tool only where it is
//! usable, a build target only where it is installed, both together.

use agent_core::inventory::{Architecture, Inventory, Os, Reason, Tool, ToolName, Verdict};

use crate::fleet::metrics::CapabilityFilter;

fn mac() -> Inventory {
    Inventory {
        architecture: Architecture::Aarch64,
        os: Os::Macos,
        os_version: "27.0".to_owned(),
        tools: vec![
            Tool {
                name: ToolName::Cargo,
                verdict: Verdict::Usable("1.91.0".to_owned()),
            },
            Tool {
                name: ToolName::Docker,
                verdict: Verdict::NotUsable(Reason::TimedOut),
            },
        ],
        build_targets: vec!["aarch64-apple-darwin".to_owned()],
    }
}

fn filter(tool: Option<&str>, build_target: Option<&str>) -> CapabilityFilter {
    CapabilityFilter {
        tool: tool.map(str::to_owned),
        build_target: build_target.map(str::to_owned),
    }
}

#[test]
fn cap_030_an_empty_filter_keeps_every_inventory() {
    assert!(CapabilityFilter::default().admits(&mac()));
}

#[test]
fn cap_030_a_tool_filter_keeps_a_device_only_where_the_tool_is_usable() {
    assert!(filter(Some("cargo"), None).admits(&mac()));
    assert!(
        !filter(Some("docker"), None).admits(&mac()),
        "found, failed"
    );
    assert!(!filter(Some("nix"), None).admits(&mac()), "not found");
    assert!(
        !filter(Some("zig"), None).admits(&mac()),
        "a name the hub does not know"
    );
}

#[test]
fn cap_030_a_target_filter_and_both_filters_together() {
    assert!(filter(None, Some("aarch64-apple-darwin")).admits(&mac()));
    assert!(!filter(None, Some("x86_64-apple-darwin")).admits(&mac()));
    assert!(filter(Some("cargo"), Some("aarch64-apple-darwin")).admits(&mac()));
    assert!(!filter(Some("cargo"), Some("x86_64-apple-darwin")).admits(&mac()));
    assert!(!filter(Some("docker"), Some("aarch64-apple-darwin")).admits(&mac()));
}
