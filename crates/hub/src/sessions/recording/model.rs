//! What the hub knows about a recording: why it stopped, its state as a session shows it, its
//! stored row, its place in the catalogue and its file's name.

use chrono::{DateTime, Utc};

use super::format::Format;
use crate::{
    credentials::Sealed,
    ids::{DeviceId, RecordingId, SessionId},
    sessions::Protocol,
};

/// Why a recording stopped before its session ended (K4 §11), stored as lower-case text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::sessions) enum StopReason {
    /// The plaintext would have passed `ANIMA_RECORDING_MAX_BYTES`.
    SizeLimit,
    /// The recording root had less than 1 GiB free before a part.
    DiskFull,
    /// A write or sync failed, the file could not be created, or the queue was full.
    WriteFailed,
}

impl StopReason {
    pub(super) const fn as_str(self) -> &'static str {
        match self {
            Self::SizeLimit => "size_limit",
            Self::DiskFull => "disk_full",
            Self::WriteFailed => "write_failed",
        }
    }

    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "playback (WP-48 T6) reads stored recordings")
    )]
    pub(super) fn from_stored(text: &str) -> Option<Self> {
        match text {
            "size_limit" => Some(Self::SizeLimit),
            "disk_full" => Some(Self::DiskFull),
            "write_failed" => Some(Self::WriteFailed),
            _ => None,
        }
    }

    /// The Warning event's fixed sentence: no path, size or content (LOG-007).
    pub(super) const fn sentence(self) -> &'static str {
        match self {
            Self::SizeLimit => {
                "The session's recording stopped at the hub's size limit. The session goes on \
                 without recording."
            }
            Self::DiskFull => {
                "The session's recording stopped because the hub's recording folder has less \
                 than 1 GiB free. The session goes on without recording."
            }
            Self::WriteFailed => {
                "The session's recording stopped because the hub could not write it. The \
                 session goes on without recording."
            }
        }
    }
}

/// A session's recording as the session shows it (K1 `RecordingState`, G-5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::sessions) enum RecordingState {
    /// The session has no recording row: nothing was recorded.
    Off,
    /// Recording, or recorded to the session's end.
    On,
    /// The recording stopped early, with a reason.
    Stopped,
}

/// A format as `sessions.recordings.format` stores it.
pub(super) const fn format_text(format: Format) -> &'static str {
    match format {
        Format::Guacamole => "guacamole",
        Format::Asciicast => "asciicast",
    }
}

pub(super) fn format_from_stored(text: &str) -> Option<Format> {
    match text {
        "guacamole" => Some(Format::Guacamole),
        "asciicast" => Some(Format::Asciicast),
        _ => None,
    }
}

/// What a session's recording holds: SSH as asciicast v2, RDP and VNC as Guacamole
/// instructions (K4 §11, D-12).
pub(super) const fn format_of(protocol: Protocol) -> Format {
    match protocol {
        Protocol::Ssh => Format::Asciicast,
        Protocol::Rdp | Protocol::Vnc => Format::Guacamole,
    }
}

/// A recording's row, with its sealed key. No `Debug`: the sealed key stays out of every log
/// (K7).
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "playback (WP-48 T6) reads the fields")
)]
pub(in crate::sessions) struct Stored {
    pub(in crate::sessions) id: RecordingId,
    pub(in crate::sessions) session: SessionId,
    pub(in crate::sessions) device: Option<DeviceId>,
    pub(in crate::sessions) device_name: String,
    pub(in crate::sessions) protocol: Protocol,
    pub(in crate::sessions) format: Format,
    pub(in crate::sessions) key_sealed: Sealed,
    pub(in crate::sessions) started_at: DateTime<Utc>,
    pub(in crate::sessions) ended_at: Option<DateTime<Utc>>,
    /// Plaintext bytes written in parts.
    pub(in crate::sessions) size_bytes: u64,
    /// The file has its last part.
    pub(in crate::sessions) complete: bool,
    pub(in crate::sessions) stopped: Option<StopReason>,
    pub(in crate::sessions) expires_at: DateTime<Utc>,
}

/// A recording as the catalogue lists it (K1 `Recording`): its row without the sealed key, so
/// a page never reads a key it does not need.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::sessions) struct Catalogued {
    pub(in crate::sessions) id: RecordingId,
    pub(in crate::sessions) session: SessionId,
    /// `None` only for a row whose device id was not copied; kept after the device is deleted
    /// (D-15).
    pub(in crate::sessions) device: Option<DeviceId>,
    pub(in crate::sessions) device_name: String,
    pub(in crate::sessions) protocol: Protocol,
    pub(in crate::sessions) format: Format,
    pub(in crate::sessions) started_at: DateTime<Utc>,
    pub(in crate::sessions) ended_at: Option<DateTime<Utc>>,
    /// Plaintext bytes written in parts.
    pub(in crate::sessions) size_bytes: u64,
    /// The file has its last part.
    pub(in crate::sessions) complete: bool,
    pub(in crate::sessions) expires_at: DateTime<Utc>,
}

/// The recording a catalogue page starts after: newest first by `(started_at, id)`, which
/// together are unique and fixed for a row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::sessions) struct Position {
    pub(in crate::sessions) started_at: DateTime<Utc>,
    pub(in crate::sessions) id: RecordingId,
}

impl Position {
    pub(in crate::sessions) const fn of(recording: &Catalogued) -> Self {
        Self {
            started_at: recording.started_at,
            id: recording.id,
        }
    }
}

/// The name of a recording's file in the recording root (K4 §11): made only from its id, so
/// no operator text reaches a path.
pub(super) fn file_name(id: RecordingId) -> String {
    format!("{id}.anrec")
}
