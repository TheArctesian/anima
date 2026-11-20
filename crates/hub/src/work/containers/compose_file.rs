//! The text of a Compose file before an install (HOST-022, R-150): the check of its syntax,
//! and the change of a `docker run` command into a Compose file.
//!
//! Both are pure functions of the text that the caller gives. They run nothing, reach no
//! machine and no network, and keep nothing. An environment value in a Compose file or in a
//! command is a secret of the operator (LOG-007): a problem of the check holds a fixed code,
//! a position and at most the name of a service and of a key, never a value of the file. The
//! hub writes neither text to a log.
//!
//! The check is of the syntax only. It does not say that a machine accepts the file: the
//! rules of HOST-028 (`compose-rules`) are a different check.

mod docker_run;
mod draft;
mod keys;
mod mounts;
mod options;
mod syntax;
#[cfg(test)]
mod tests;
mod words;
mod writer;
mod yaml;

pub use self::docker_run::{Converted, Note, Refusal, RefusalCode, convert};
pub use self::syntax::{Check, Problem, ProblemCode, Severity, check};
pub use self::yaml::Place;

/// The most bytes of a Compose file that the check reads.
pub const LARGEST_FILE: usize = 256 * 1024;
/// The most bytes of a `docker run` command that the converter reads.
pub const LARGEST_COMMAND: usize = 64 * 1024;
