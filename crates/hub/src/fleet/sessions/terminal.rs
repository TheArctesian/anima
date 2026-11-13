//! The terminal tunnel (K4 Section 8, R-062): the hub runs the SSH connection itself, with the
//! accepted host key pinned (SEC-007) and the stored credential (OD-9), and relays terminal
//! bytes to the client, which draws them with xterm.js (RA-011). No SSH session goes to guacd.
//!
//! Pure rules: the text messages (`control`) and the end messages (`ending`). I/O: the
//! `/terminal` route and its checks before the upgrade (`route`), the device side up to a
//! signed-in session channel (`opening`), the terminal's part of the session task
//! (`session`; the task that owns the ticket from redeem to end is `sessions::upgrade`), the
//! upgraded terminal (`link`), the PTY and shell (`shell`), the relay after them (`relay`)
//! and the terminal's own writes (`writes`). The end's row, audit entry and event are written
//! by `end::record`.

mod control;
mod ending;
mod link;
mod opening;
mod relay;
mod route;
mod session;
mod shell;
#[cfg(test)]
mod tests;
mod writes;

pub use route::terminal_router;
