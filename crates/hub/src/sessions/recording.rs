//! Session recordings (RA-040, D-13, K4 §11): what a session's owner saw, kept encrypted on
//! the hub's disk with a key per recording.
//!
//! The file format is `format`, pure; SSH sessions are recorded as asciicast v2
//! (`asciicast`, D-12). No log line holds recording content, a key or a recording's path
//! (LOG-007).

mod asciicast;
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        unused_imports,
        reason = "the recorder (WP-48 T4) is the first caller"
    )
)]
mod format;
