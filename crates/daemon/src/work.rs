//! The work route at the machine (`contracts/work-route.md`, K10; task W4): what the daemon
//! decides before it starts a job that the hub gave it.
//!
//! This part is pure and runs nothing: [`checks`] reads an item and refuses it by the fixed
//! causes of K10 Section 5, [`permitted`] is the list of the repositories that the owner of the
//! machine permits (R-185), [`refusal`] is the report of a refusal, and [`record`] is the entry
//! of the machine's own log (K10 Section 9). Nothing here asks the hub for work, and nothing
//! starts a program: the isolation of a job is the design of task W2.

pub mod checks;
pub mod permitted;
pub mod record;
pub mod refusal;
mod shape;
#[cfg(test)]
mod tests;
pub mod types;
