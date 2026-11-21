//! Model work at the machine (`contracts/compute/model-work.md` version 0.3; task L1-6): the
//! I/O edge. The checks are pure, in `daemon_core::model_work`.
//!
//! [`cycle`] asks the hub for one model item through the work route and reports it;
//! [`install`] installs a runtime from its release source ([`fetch`], [`unpack`]);
//! [`supervise`] starts, stops and supervises the servers ([`server`], [`ring`], [`sockets`]);
//! [`folders`] makes the folders of Section 10; [`machine_files`] reads `model-work.json` and
//! `model-sources.json`; [`state`] is the object of `model_report`.
//!
//! Built here: `model_report`, `runtime_install`, `server_start`, `server_stop`. Each other
//! model type is left out of the claim and refused with `type_unknown` until its task builds
//! it: `model_pull` and `copy_delete` (L1-7), `model_measure` (L1-8), `model_load`,
//! `model_unload` and `model_drain` (L1-8, L1-16).

pub mod cycle;
pub mod fetch;
pub mod folders;
pub mod install;
pub mod machine_files;
pub mod ring;
pub mod server;
pub mod sockets;
pub mod state;
pub mod supervise;
pub mod unpack;
