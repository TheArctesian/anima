//! The collector logic with no I/O (COL-008): which folders a source may read (COL-006) and
//! where the read of a file continues (COL-004). `anima-agent` runs it (COL-001); the agent
//! does the file reads and the sends, and passes paths and bytes in.

mod folder_rule;
mod read_position;
#[cfg(test)]
mod tests;

pub use folder_rule::FolderRule;
pub use read_position::{ReadPosition, whole_lines};
