//! WEB-004–008, D-2–D-6: the pure parts of device web views. The fingerprint form, the pinned
//! origin, every verdict and the leaf-certificate probe against a local TLS server on
//! 127.0.0.1; the address and scheme rules, the open-refusal rule, bounds, history steps, the
//! store per device, the tab IDs and bound, the command arguments and what crosses to the
//! client (events and error codes).

mod addresses;
mod bounds;
mod crossings;
mod fingerprints;
mod openings;
mod origins;
mod probe;
mod steps;
mod stores;
mod tabs;
mod verdicts;
