//! The Anima gateway: it connects to a device's remote display itself and speaks the
//! Guacamole protocol to the hub, as guacd does (spec Section 4.3, K4 Section 3). The hub
//! and the client do not know which gateway they talk to.
//!
//! So far: the gateway's copy of the remote screen (`screen`), the instructions that draw
//! its changes in the client (`frames`), the opening of a session with the hub
//! (`handshake`), the VNC client (`vnc`), one session from end to end (`session`), and the
//! listener that starts a worker process for each session (`listening`).

pub mod frames;
pub mod handshake;
pub mod listening;
pub mod screen;
pub mod session;
pub mod vnc;
