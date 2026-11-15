//! Stand-in programs take the place of `sudo`, `pmset`, `osascript` and `systemd-inhibit`, so
//! no test changes a power setting or asks for an administrator. One macOS test takes and
//! ends a real `caffeinate` assertion, which needs no rights.

mod controls;
mod holds;
mod levels;
mod rig;
mod set_up;
