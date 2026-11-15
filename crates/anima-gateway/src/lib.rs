//! The Anima gateway: it connects to a device's remote display itself and speaks the
//! Guacamole protocol to the hub, as guacd does (spec Section 4.3, K4 Section 3). The hub
//! and the client do not know which gateway they talk to.
//!
//! So far: the gateway's copy of the remote screen (`screen`) and the instructions that
//! draw its changes in the client (`frames`).

pub mod frames;
pub mod screen;
