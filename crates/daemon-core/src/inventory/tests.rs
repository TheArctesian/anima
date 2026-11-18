//! Tests of the probes on output captured from real tools and on garbage, of the folders a
//! program is looked for in, and of the operating system version readers.

use super::{PROBES, Probe, ToolName};

mod folders;
mod probes;
mod system;

/// The probe of `tool`.
fn probe(tool: ToolName) -> &'static Probe {
    PROBES
        .iter()
        .find(|probe| probe.tool == tool)
        .expect("every tool has a probe")
}
