//! Daemon logic with no I/O: the pure readers of device command output, the points made from
//! them, the tool inventory, the counts of the AI agent programs, and the K8 push bodies, shared
//! by the hub and the device daemon (MET-014, MET-016, CAP-028, AIS-018, spec Section 13).

pub mod agent_threads;
pub mod agent_usage;
pub mod inventory;
pub mod points;
pub mod readings;
pub mod wire;
