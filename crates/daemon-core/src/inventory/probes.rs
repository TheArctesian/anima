//! The fixed probes (CAP-021 to CAP-023, CAP-025): for each tool, the program, the fixed
//! arguments that make it print its version and nothing else, and how to read that version.
//! No probe starts a build, a model or a network request. `claude` and `codex` read and write
//! files in their home folder even to print a version, so their probes get an empty home of
//! their own, never the operator's; only the version is kept (CAP-025).

use std::collections::BTreeSet;

use super::{MAX_BUILD_TARGETS, Os, Outcome, Reason, Tool, ToolName, Verdict, version};

/// A program and its fixed arguments.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Program {
    /// The file name, looked for in the folders of [`super::candidates`].
    pub name: &'static str,
    pub args: &'static [&'static str],
}

/// One tool's probe.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Probe {
    pub tool: ToolName,
    pub program: Program,
    /// `Some` for a probe that runs on that system only.
    pub only_on: Option<Os>,
    /// Whether the probe runs with an empty home folder of its own instead of none, because
    /// the program writes there (CAP-025).
    pub own_home: bool,
    shape: Shape,
}

/// Where the version is in a probe's output: on the first line that starts with the text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Shape {
    /// The word right after the text: `git version 2.54.0 (Apple Git-157)`.
    After(&'static str),
    /// The last word of the line: `nix (Determinate Nix 3.6.2) 2.29.0`.
    LastWord(&'static str),
}

const fn probe(tool: ToolName, args: &'static [&'static str], shape: Shape) -> Probe {
    Probe {
        tool,
        program: Program {
            name: tool.as_str(),
            args,
        },
        only_on: None,
        own_home: false,
        shape,
    }
}

const VERSION: &[&str] = &["--version"];

/// Every probe, in [`ToolName`] order (CAP-023).
pub const PROBES: [Probe; 13] = [
    probe(ToolName::Git, VERSION, Shape::After("git version ")),
    probe(ToolName::Rustc, VERSION, Shape::After("rustc ")),
    probe(ToolName::Cargo, VERSION, Shape::After("cargo ")),
    probe(
        ToolName::CargoNextest,
        &["nextest", "--version"],
        Shape::After("cargo-nextest "),
    ),
    probe(ToolName::Sccache, VERSION, Shape::After("sccache ")),
    probe(ToolName::Bun, VERSION, Shape::After("")),
    probe(ToolName::Node, VERSION, Shape::After("v")),
    Probe {
        only_on: Some(Os::Macos),
        ..probe(ToolName::Xcodebuild, &["-version"], Shape::After("Xcode "))
    },
    probe(ToolName::Docker, VERSION, Shape::After("Docker version ")),
    probe(ToolName::Podman, VERSION, Shape::After("podman version ")),
    probe(ToolName::Nix, VERSION, Shape::LastWord("nix ")),
    Probe {
        own_home: true,
        ..probe(ToolName::Claude, VERSION, Shape::After(""))
    },
    Probe {
        own_home: true,
        ..probe(ToolName::Codex, VERSION, Shape::After("codex-cli "))
    },
];

/// The installed Rust targets, one to a line (CAP-023).
pub const TARGETS: Program = Program {
    name: "rustup",
    args: &["target", "list", "--installed"],
};

impl Probe {
    /// Whether the probe runs on `os`.
    #[must_use]
    pub fn runs_on(&self, os: Os) -> bool {
        self.only_on.is_none_or(|only| only == os)
    }

    /// The tool as the inventory lists it after `outcome`; `None` when it was not found.
    #[must_use]
    pub fn tool(&self, outcome: &Outcome) -> Option<Tool> {
        let verdict = match outcome {
            Outcome::Missing => return None,
            Outcome::TimedOut => Verdict::NotUsable(Reason::TimedOut),
            Outcome::Exited => Verdict::NotUsable(Reason::Exited),
            Outcome::Printed(output) => self
                .version(output)
                .map_or(Verdict::NotUsable(Reason::NoVersion), Verdict::Usable),
        };
        Some(Tool {
            name: self.tool,
            verdict,
        })
    }

    /// The version in what the probe printed; `None` for output of another shape.
    #[must_use]
    pub fn version(&self, output: &[u8]) -> Option<String> {
        let prefix = match self.shape {
            Shape::After(prefix) | Shape::LastWord(prefix) => prefix,
        };
        let line = version::text_lines(output).find(|line| line.starts_with(prefix))?;
        let rest = &line[prefix.len()..];
        let word = match self.shape {
            Shape::After(_) => rest.split_whitespace().next(),
            Shape::LastWord(_) => rest.split_whitespace().next_back(),
        }?;
        version::leading_version(word)
    }
}

/// The build targets `rustup target list --installed` printed: each line that is a target,
/// unique, in byte order, at most 128.
#[must_use]
pub fn build_targets(output: &[u8]) -> Vec<String> {
    let found: BTreeSet<&str> = version::text_lines(output)
        .filter(|line| version::is_build_target(line))
        .collect();
    found
        .into_iter()
        .take(MAX_BUILD_TARGETS)
        .map(str::to_owned)
        .collect()
}
