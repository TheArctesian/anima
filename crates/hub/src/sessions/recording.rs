//! Session recordings (RA-040, D-13, K4 §11): what a session's owner saw, kept encrypted on
//! the hub's disk with a key per recording.
//!
//! The file format is `format`, pure. No log line holds recording content, a key or a
//! recording's path (LOG-007).

#[cfg_attr(
    not(test),
    expect(
        dead_code,
        unused_imports,
        reason = "the recorder (WP-48 T4) is the first caller"
    )
)]
mod format;
