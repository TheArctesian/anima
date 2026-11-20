//! The one hub that the settings are made for (contract K10 Section 6.4.2): the pair of the
//! origin of the hub and the device identifier, which the daemon sends with each request.

use crate::{
    forms::{is_origin, is_uuid},
    strict::{self, Members},
};

/// The origin of a hub and the device identifier of the machine at that hub.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pair {
    origin: String,
    device: String,
}

impl Pair {
    /// The pair of a request; `None` when a part does not have its form. The origin is the
    /// scheme, the host in lower case and the port, always written; the device identifier is
    /// a UUID of 36 lower-case characters.
    #[must_use]
    pub fn of(origin: &str, device: &str) -> Option<Self> {
        (is_origin(origin) && is_uuid(device)).then(|| Self {
            origin: origin.to_owned(),
            device: device.to_owned(),
        })
    }
}

/// The record of the first pair, in a file of the applications program.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PairRecord {
    /// The time of "Manage applications" that the record was made for. The administrator
    /// who sets the setting on again gives it a new time, and the record is then not valid:
    /// this is how the record "is deleted".
    pub set_on_at: u64,
    pub first: Pair,
    /// True when a request had a different pair. Each container item is then refused until
    /// the administrator sets "Manage applications" on again.
    pub different_seen: bool,
}

/// The file of the record of the pair, as the caller found it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PairFile<'a> {
    /// The program has no such file.
    Missing,
    /// The file is there, but the caller cannot read it, or [`PairRecord::read`] does not
    /// accept its text. Each container item is then refused (`applications_off`): a file that
    /// cannot be read is never taken as no record, which would accept a different hub. The
    /// file must be deleted when the administrator sets "Manage applications" on again.
    Unreadable,
    /// The record in the file.
    Read(&'a PairRecord),
}

/// What the pair of one request means.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PairDecision {
    /// The pair is the recorded one.
    Recorded,
    /// No valid record exists: the pair is the first, and the program writes this record
    /// before it does the work.
    First(PairRecord),
    /// The pair is different, now or at an earlier request: the item is refused
    /// (`applications_off`). The program writes the record when it has one here.
    Different(Option<PairRecord>),
}

/// Decides what `sent` means with the file of the record that the program has, for the time
/// `set_on_at` of "Manage applications".
#[must_use]
pub fn decided(file: PairFile<'_>, set_on_at: u64, sent: &Pair) -> PairDecision {
    let record = match file {
        PairFile::Missing => None,
        PairFile::Unreadable => return PairDecision::Different(None),
        PairFile::Read(record) => Some(record),
    };
    match record.filter(|record| record.set_on_at == set_on_at) {
        None => PairDecision::First(PairRecord {
            set_on_at,
            first: sent.clone(),
            different_seen: false,
        }),
        Some(record) if record.different_seen => PairDecision::Different(None),
        Some(record) if record.first == *sent => PairDecision::Recorded,
        Some(record) => PairDecision::Different(Some(PairRecord {
            different_seen: true,
            ..record.clone()
        })),
    }
}

impl PairRecord {
    /// The text of the file of the record.
    #[must_use]
    pub fn written(&self) -> String {
        // The origin and the device have forms with no character that JSON escapes.
        format!(
            "{{\"setOnAt\":{},\"origin\":\"{}\",\"device\":\"{}\",\"differentSeen\":{}}}\n",
            self.set_on_at, self.first.origin, self.first.device, self.different_seen
        )
    }

    /// Reads the text of the file of the record; `None` for text that [`Self::written`] does
    /// not write. The caller then has no record that it can trust and refuses each item.
    #[must_use]
    pub fn read(text: &str) -> Option<Self> {
        let mut members = Members::of(strict::read(text)?)?;
        let record = Self {
            set_on_at: members.whole("setOnAt")?,
            first: Pair::of(&members.text("origin")?, &members.text("device")?)?,
            different_seen: members.truth("differentSeen")?,
        };
        members.is_empty().then_some(record)
    }
}
