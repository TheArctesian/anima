//! The model work of the machine (`contracts/compute/model-work.md`, version 0.3; task L1-6):
//! the pure part. It reads files and runs nothing.
//!
//! [`names`] holds the fixed lists (runtimes, backends, formats, probes, actors), [`cause`] the
//! causes of a refusal or a failure, [`settings`] the flag policy (Section 2.1), [`files`] the
//! files that a copy may have (Section 2.2), [`order`] the check of a model item (Section 5),
//! [`machine`] the settings at the machine and the actor check (Section 10.1), [`releases`] the
//! release sources and the versions that the program accepts (Section 6), [`command`] the
//! command line and the environment of a runtime (Section 2.1), [`supervision`] the waits and
//! the counts of a restart (Section 9), [`loopback`] the verdict over a socket table
//! (Section 9), and [`folders`] the layout of the folders (Section 10).

pub mod cause;
pub mod command;
pub mod files;
pub mod folders;
pub mod loopback;
pub mod machine;
pub mod names;
pub mod order;
pub mod releases;
pub mod settings;
pub mod supervision;
#[cfg(test)]
mod tests;
