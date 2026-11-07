//! What one RDP or VNC session may move between the operator's computer and the device, read
//! from the device record when the tunnel opens and fixed for the session's life (K4 Section
//! 4, RA-021, RA-033, GW-015). The relay is the first guard; guacd's own switches in
//! `connect` (`arguments`) are the second.

use crate::devices::{Device, TransferPolicy};
use crate::sessions::Desktop;

/// The credential names guacd's RDP plugin asks for with `required` (GW-015).
const RDP_PROMPTS: &[&str] = &["username", "password", "domain"];
/// The credential names guacd's VNC plugin asks for with `required` (GW-015).
const VNC_PROMPTS: &[&str] = &["username", "password"];

/// One session's switches. VNC has no audio input and no shared drive, so for VNC both are
/// always off, whatever the device's RDP settings say.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Policy {
    pub(super) desktop: Desktop,
    pub(super) transfer: TransferPolicy,
    /// The client may send RDP audio input.
    pub(super) microphone: bool,
    /// The RDP shared drive is mounted, so the client may browse it.
    pub(super) shared_drive: bool,
}

impl Policy {
    /// The policy of a `desktop` session on `device`, as the record stands now.
    pub(super) const fn of(desktop: Desktop, device: &Device) -> Self {
        let rdp = matches!(desktop, Desktop::Rdp);
        Self {
            desktop,
            transfer: device.transfer,
            microphone: rdp && device.rdp.microphone,
            shared_drive: rdp && device.rdp.shared_drive,
        }
    }

    /// The client may write files to the shared drive.
    pub(super) const fn uploads(self) -> bool {
        self.shared_drive && self.transfer.upload
    }

    /// The client may receive a file's content from the shared drive.
    pub(super) const fn downloads(self) -> bool {
        self.shared_drive && self.transfer.download
    }

    /// The credential names the client may answer once each after guacd's `required`. Any
    /// other name, such as `hostname`, `port` or a path, stays hub-only even if guacd asks
    /// (GW-013).
    pub(super) const fn prompts(self) -> &'static [&'static str] {
        match self.desktop {
            Desktop::Rdp => RDP_PROMPTS,
            Desktop::Vnc => VNC_PROMPTS,
        }
    }
}
