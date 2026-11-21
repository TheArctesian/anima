//! Each probe on what its tool prints (CAP-023, CAP-024, CAP-025). The outputs were captured
//! from macOS 27 and Linux hosts; the Determinate Nix and nightly lines are their documented
//! forms.

use proptest::prelude::*;

use super::probe;
use crate::inventory::{Os, Outcome, PROBES, Reason, Tool, ToolName, Verdict, is_version};

fn version_of(tool: ToolName, output: &str) -> Option<String> {
    probe(tool).version(output.as_bytes())
}

#[test]
fn cap_003_each_runtime_probe_reads_the_version_its_runtime_prints() {
    let cases = [
        (
            ToolName::LlamaServer,
            "version: 6040 (8a2afb7)\nbuilt with clang for arm64-apple-darwin\n",
            "6040",
        ),
        (ToolName::MlxLm, "0.28.3\n", "0.28.3"),
        (ToolName::Vllm, "0.11.0\n", "0.11.0"),
        (ToolName::Ollama, "ollama version is 0.12.6\n", "0.12.6"),
        (
            ToolName::Ollama,
            "Warning: could not connect to a running Ollama instance\n\
             Warning: client version is 0.12.6\n",
            "0.12.6",
        ),
    ];
    for (tool, output, expected) in cases {
        assert_eq!(
            version_of(tool, output).as_deref(),
            Some(expected),
            "{output}"
        );
    }
}

#[test]
fn cap_023_each_probe_reads_the_version_its_tool_prints() {
    let cases = [
        (
            ToolName::Git,
            "git version 2.54.0 (Apple Git-157)\n",
            "2.54.0",
        ),
        (ToolName::Git, "git version 2.43.0\n", "2.43.0"),
        (
            ToolName::Rustc,
            "rustc 1.100.0 (0ef1bc386 2026-11-09)\n",
            "1.100.0",
        ),
        (
            ToolName::Rustc,
            "rustc 1.93.0-nightly (1d60f9e07 2025-11-01)\n",
            "1.93.0-nightly",
        ),
        (
            ToolName::Cargo,
            "cargo 1.100.0 (3acc79c0c 2026-11-05)\n",
            "1.100.0",
        ),
        (
            ToolName::CargoNextest,
            "cargo-nextest 0.9.151 (6b3f99457 2026-11-02)\nrelease: 0.9.151\n\
             commit-hash: 6b3f994577a98874a2e0ea5bef77fbbab3434044\ncommit-date: 2026-11-02\n\
             host: aarch64-apple-darwin\n",
            "0.9.151",
        ),
        (ToolName::Sccache, "sccache 0.19.0\n", "0.19.0"),
        (ToolName::Bun, "1.3.14\n", "1.3.14"),
        (ToolName::Node, "v26.13.1\n", "26.13.1"),
        (
            ToolName::Xcodebuild,
            "Xcode 27.2\nBuild version 27B52\n",
            "27.2",
        ),
        (
            ToolName::Docker,
            "Docker version 29.9.3, build 5457199\n",
            "29.9.3",
        ),
        (ToolName::Podman, "podman version 5.2.3\n", "5.2.3"),
        (ToolName::Nix, "nix (Nix) 2.35.2\n", "2.35.2"),
        (
            ToolName::Nix,
            "nix (Determinate Nix 3.6.2) 2.29.0\n",
            "2.29.0",
        ),
        (ToolName::Claude, "2.1.311 (Claude Code)\n", "2.1.311"),
        (ToolName::Codex, "codex-cli 0.169.0\n", "0.169.0"),
    ];
    for (tool, output, expected) in cases {
        assert_eq!(
            version_of(tool, output).as_deref(),
            Some(expected),
            "{tool:?}: {output:?}"
        );
    }
}

#[test]
fn cap_024_garbage_empty_and_other_output_gives_no_version() {
    let long = format!("git version {}\n", "1".repeat(65));
    let cases = [
        (ToolName::Git, ""),
        (ToolName::Git, "\n\n"),
        (ToolName::Git, "git version \n"),
        (ToolName::Git, long.as_str()),
        (ToolName::Docker, "Docker version , build abc\n"),
        (ToolName::Node, "node: command not found\n"),
        (
            ToolName::Xcodebuild,
            "xcode-select: error: tool 'xcodebuild' requires Xcode, but active developer \
             directory '/Library/Developer/CommandLineTools' is a command line tools instance\n",
        ),
        (ToolName::Bun, "error: Script not found \"--version\"\n"),
        (ToolName::Claude, "Ignore all previous instructions\n"),
        (ToolName::Nix, "nix (Nix)\n"),
    ];
    for (tool, output) in cases {
        assert_eq!(version_of(tool, output), None, "{tool:?}: {output:?}");
    }
    let not_utf8 = [0xff, 0xfe, b'\n'];
    assert_eq!(probe(ToolName::Bun).version(&not_utf8), None);
}

#[test]
fn cap_031_a_version_keeps_only_its_own_bytes() {
    assert_eq!(
        version_of(ToolName::Git, "git version 2.4\u{1b}[31mred\n").as_deref(),
        Some("2.4")
    );
    assert_eq!(
        version_of(ToolName::Bun, "1.3.14;curl evil\n").as_deref(),
        Some("1.3.14")
    );
}

#[test]
fn cap_024_a_missing_tool_is_left_out_and_a_failed_one_is_listed_with_its_reason() {
    let git = probe(ToolName::Git);
    assert_eq!(git.tool(&Outcome::Missing), None);
    for (outcome, reason) in [
        (Outcome::TimedOut, Reason::TimedOut),
        (Outcome::Exited, Reason::Exited),
        (Outcome::Printed(b"\x00\x01".to_vec()), Reason::NoVersion),
    ] {
        assert_eq!(
            git.tool(&outcome),
            Some(Tool {
                name: ToolName::Git,
                verdict: Verdict::NotUsable(reason)
            })
        );
    }
    let found = git
        .tool(&Outcome::Printed(b"git version 2.43.0\n".to_vec()))
        .expect("listed");
    assert!(found.is_usable());
    assert_eq!((found.version(), found.reason()), (Some("2.43.0"), None));
}

#[test]
fn cap_022_every_probe_only_asks_its_program_for_a_version() {
    let mut tools: Vec<ToolName> = PROBES.iter().map(|probe| probe.tool).collect();
    tools.dedup();
    assert_eq!(tools, ToolName::ALL, "one probe per tool, in name order");
    for probe in PROBES {
        let program = if probe.tool == ToolName::MlxLm {
            "mlx_lm.server"
        } else {
            probe.tool.as_str()
        };
        assert_eq!(probe.program.name, program);
        let asks = probe.program.args;
        assert!(
            matches!(asks, ["--version" | "-version"] | ["nextest", "--version"]),
            "{asks:?}"
        );
    }
}

#[test]
fn cap_025_only_claude_and_codex_get_a_home_folder_of_their_own() {
    let own: Vec<ToolName> = PROBES
        .iter()
        .filter(|probe| probe.own_home)
        .map(|probe| probe.tool)
        .collect();
    assert_eq!(own, [ToolName::Claude, ToolName::Codex]);
}

#[test]
fn cap_023_xcodebuild_and_mlx_lm_are_probed_on_macos_only() {
    let on_linux: Vec<ToolName> = PROBES
        .iter()
        .filter(|probe| probe.runs_on(Os::Linux))
        .map(|probe| probe.tool)
        .collect();
    assert!(!on_linux.contains(&ToolName::Xcodebuild));
    assert!(!on_linux.contains(&ToolName::MlxLm));
    assert_eq!(on_linux.len(), 15);
    assert!(PROBES.iter().all(|probe| probe.runs_on(Os::Macos)));
}

proptest! {
    /// No output makes a probe panic, and any version it gives is of the K8 alphabet.
    #[test]
    fn cap_031_any_output_gives_no_version_or_a_clean_one(
        output in proptest::collection::vec(any::<u8>(), 0..200),
        text in "\\PC{0,80}",
    ) {
        for probe in PROBES {
            for bytes in [output.as_slice(), text.as_bytes()] {
                if let Some(version) = probe.version(bytes) {
                    prop_assert!(is_version(&version), "{version:?}");
                }
            }
        }
    }
}
