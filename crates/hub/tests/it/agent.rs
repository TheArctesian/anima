//! The Anima agent's enrolment, tokens, renewal and pushes through the hub's routes (K8,
//! MET-011 to MET-018, R-092, R-098).

pub(crate) mod batches;
mod enrolment;
mod gpus;
pub(crate) mod harness;
mod push;
mod refusals;
mod renewal;
mod revocation;
mod secrecy;
mod series;
mod stale;
mod surfaces;
