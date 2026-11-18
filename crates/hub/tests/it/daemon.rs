//! The Anima daemon's enrolment, tokens, renewal and pushes through the hub's routes (K8,
//! MET-011 to MET-018, R-092, R-098, R-122).

pub(crate) mod batches;
mod enrolment;
mod gpus;
pub(crate) mod harness;
mod last_batch;
mod push;
mod refusals;
mod renewal;
mod revocation;
mod secrecy;
mod series;
mod surfaces;
