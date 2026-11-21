//! The lists a batch announces besides its points: the GPUs, the drives, the batteries, the tool
//! inventory and the agent usage. Each goes in the next batch and is kept until one is accepted, except after
//! `MALFORMED` (K8 Section 4).

mod agent_usage;
mod batteries;
mod drives;
mod gpus;
mod inventory;
