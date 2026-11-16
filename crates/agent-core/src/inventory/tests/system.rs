//! The operating system version (CAP-026), and the inventory the outcomes make (CAP-020).

use crate::inventory::{
    Architecture, Inventory, Os, Outcome, Probe, Tool, ToolName, build_targets, kernel_release,
    macos_version, os_release_version,
};

use super::probe;

#[test]
fn cap_026_the_macos_version_is_the_whole_first_line() {
    assert_eq!(macos_version(b"27.0.1\n").as_deref(), Some("27.0.1"));
    assert_eq!(macos_version(b"27.0").as_deref(), Some("27.0"));
    assert_eq!(macos_version(b""), None);
    assert_eq!(macos_version(b"27.0 beta\n"), None);
}

#[test]
fn cap_026_the_linux_version_is_version_id_quoted_or_not() {
    let ubuntu = b"NAME=\"Ubuntu\"\nVERSION=\"24.04.1 LTS (Noble Numbat)\"\nID=ubuntu\nVERSION_ID=\"24.04\"\n";
    assert_eq!(os_release_version(ubuntu).as_deref(), Some("24.04"));
    assert_eq!(
        os_release_version(b"ID=debian\nVERSION_ID='12'\n").as_deref(),
        Some("12")
    );
    assert_eq!(
        os_release_version(b"ID=fedora\nVERSION_ID=41\n").as_deref(),
        Some("41")
    );
    let arch = b"NAME=\"Arch Linux\"\nID=arch\nBUILD_ID=rolling\n";
    assert_eq!(
        os_release_version(arch),
        None,
        "a rolling distribution has none"
    );
    assert_eq!(os_release_version(b"VERSION_ID=\"1 2\"\n"), None);
    assert_eq!(
        kernel_release(b"6.8.0-48-generic\n").as_deref(),
        Some("6.8.0-48-generic")
    );
    assert_eq!(kernel_release(b"\xff\n"), None);
}

#[test]
fn cap_023_build_targets_are_the_target_lines_unique_and_sorted() {
    let output = b"x86_64-unknown-linux-gnu\naarch64-apple-darwin\n\
        info: this is not a target\naarch64-apple-darwin\nWASM32\n\n";
    assert_eq!(
        build_targets(output),
        ["aarch64-apple-darwin", "x86_64-unknown-linux-gnu"]
    );
    let many: Vec<String> = (0..200).map(|i| format!("t{i:03}")).collect();
    assert_eq!(build_targets(many.join("\n").as_bytes()).len(), 128);
    assert_eq!(build_targets(b""), Vec::<String>::new());
}

#[test]
fn cap_020_the_inventory_lists_the_tools_found_in_name_order() {
    let printed = |text: &str| Outcome::Printed(text.as_bytes().to_vec());
    let outcomes: Vec<(&Probe, Outcome)> = vec![
        (probe(ToolName::Codex), printed("codex-cli 0.169.0\n")),
        (probe(ToolName::Git), printed("git version 2.43.0\n")),
        (probe(ToolName::Docker), Outcome::Missing),
        (probe(ToolName::Bun), Outcome::Failed),
    ];
    let inventory = Inventory::new(
        (Architecture::X86_64, Os::Linux, "24.04".to_owned()),
        &outcomes,
        Some(b"x86_64-unknown-linux-gnu\n"),
    );
    assert_eq!(
        inventory.tools,
        [
            Tool {
                name: ToolName::Git,
                version: Some("2.43.0".to_owned())
            },
            Tool {
                name: ToolName::Bun,
                version: None
            },
            Tool {
                name: ToolName::Codex,
                version: Some("0.169.0".to_owned())
            },
        ]
    );
    assert_eq!(inventory.build_targets, ["x86_64-unknown-linux-gnu"]);
    let no_rustup = Inventory::new(
        (Architecture::Aarch64, Os::Macos, "27.0".to_owned()),
        &[],
        None,
    );
    assert!(no_rustup.tools.is_empty() && no_rustup.build_targets.is_empty());
}

#[test]
fn cap_026_architecture_and_system_names_are_rust_s() {
    assert_eq!(Architecture::parse("aarch64"), Some(Architecture::Aarch64));
    assert_eq!(
        Architecture::parse("x86_64").map(Architecture::as_str),
        Some("x86_64")
    );
    assert_eq!(Architecture::parse("riscv64"), None);
    assert_eq!(Os::parse("macos"), Some(Os::Macos));
    assert_eq!(Os::parse("linux").map(Os::as_str), Some("linux"));
    assert_eq!(Os::parse("freebsd"), None);
}
