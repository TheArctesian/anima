//! The clipboard channel of an RDP session, text only (RA-005). `IronRDP` calls this
//! backend when the device says something on the channel; the backend only notes it, and
//! the session answers under the device's copy and paste switches (RA-033).

use ironrdp_cliprdr::backend::CliprdrBackend;
use ironrdp_cliprdr::pdu::{
    ClipboardFormat, ClipboardFormatId, ClipboardGeneralCapabilityFlags, FileContentsRequest,
    FileContentsResponse, FormatDataRequest, FormatDataResponse, LockDataId,
};
use ironrdp_core::impl_as_any;

/// The longest clipboard text taken from the device, in bytes: as for VNC.
const LONGEST_COPY: usize = 1 << 20;
/// The most notes kept for the session. A device that says more is not answered for the
/// rest until the session has read these.
const MOST_NOTES: usize = 16;

/// The format the gateway offers and asks for.
pub(super) const TEXT: ClipboardFormatId = ClipboardFormatId::CF_UNICODETEXT;

/// What the device said on the clipboard channel.
#[derive(Debug, PartialEq, Eq)]
pub enum Said {
    /// The channel is ready, or the device wants to know what the client's clipboard
    /// holds.
    WhatIsThere,
    /// Something was copied on the device, and text is among its formats.
    CopiedText,
    /// It pastes, and wants the client's text.
    WantsText,
    /// The text that was copied on the device.
    Text(String),
}

/// The backend: notes of what the device said. The session takes them with
/// [`ClipboardNotes::take`].
#[derive(Debug, Default)]
pub struct ClipboardNotes {
    said: Vec<Said>,
}

impl ClipboardNotes {
    /// What the device said since the last call, in order.
    pub fn take(&mut self) -> Vec<Said> {
        std::mem::take(&mut self.said)
    }

    fn note(&mut self, said: Said) {
        if self.said.len() < MOST_NOTES {
            self.said.push(said);
        }
    }
}

impl_as_any!(ClipboardNotes);

impl CliprdrBackend for ClipboardNotes {
    #[expect(
        clippy::unnecessary_literal_bound,
        reason = "the signature is the trait's"
    )]
    fn temporary_directory(&self) -> &str {
        ""
    }

    fn client_capabilities(&self) -> ClipboardGeneralCapabilityFlags {
        ClipboardGeneralCapabilityFlags::empty()
    }

    fn on_ready(&mut self) {
        self.note(Said::WhatIsThere);
    }

    fn on_request_format_list(&mut self) {
        self.note(Said::WhatIsThere);
    }

    fn on_process_negotiated_capabilities(&mut self, _: ClipboardGeneralCapabilityFlags) {}

    fn on_remote_copy(&mut self, available_formats: &[ClipboardFormat]) {
        if available_formats.iter().any(|format| format.id() == TEXT) {
            self.note(Said::CopiedText);
        }
    }

    fn on_format_data_request(&mut self, _: FormatDataRequest) {
        self.note(Said::WantsText);
    }

    fn on_format_data_response(&mut self, response: FormatDataResponse<'_>) {
        // UTF-16 is at least as long in bytes as the same text in UTF-8 is in characters:
        // a response over twice the limit is not decoded at all.
        if response.is_error() || response.data().len() > 2 * LONGEST_COPY {
            return;
        }
        if let Ok(text) = response.to_unicode_string()
            && text.len() <= LONGEST_COPY
        {
            self.note(Said::Text(text));
        }
    }

    fn on_file_contents_request(&mut self, _: FileContentsRequest) {}

    fn on_file_contents_response(&mut self, _: FileContentsResponse<'_>) {}

    fn on_lock(&mut self, _: LockDataId) {}

    fn on_unlock(&mut self, _: LockDataId) {}
}

/// The switches of a device for the clipboard (RA-033).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sharing {
    /// The device's clipboard may go to the client.
    pub copy: bool,
    /// The client's clipboard may go to the device.
    pub paste: bool,
}

/// What the session does about something the device said.
#[derive(Debug, PartialEq, Eq)]
pub enum Reply<'a> {
    /// Tell the device whether the client's clipboard holds text.
    Offer(bool),
    /// Ask the device for the text that was copied on it.
    AskForText,
    /// Give the device the client's text; `None` when there is none for it.
    GiveText(Option<&'a str>),
    /// Send the device's text to the client.
    ToClient(String),
    Nothing,
}

/// The answer to `said` under the device's switches. `text` is the client's clipboard
/// text. With paste off the device is told there is no text and gets none; with copy off
/// its text is not asked for, and text it sends unasked is dropped. The hub's relay is
/// the first guard, this the second.
#[must_use]
pub fn reply(said: Said, sharing: Sharing, text: Option<&str>) -> Reply<'_> {
    let text = text.filter(|_| sharing.paste);
    match said {
        Said::WhatIsThere => Reply::Offer(text.is_some()),
        Said::WantsText => Reply::GiveText(text),
        Said::CopiedText if sharing.copy => Reply::AskForText,
        Said::Text(copied) if sharing.copy => Reply::ToClient(copied),
        Said::CopiedText | Said::Text(_) => Reply::Nothing,
    }
}
