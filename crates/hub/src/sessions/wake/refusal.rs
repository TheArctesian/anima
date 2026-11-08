//! The fixed sentence the operator reads for each way a wake fails, in the event and in the
//! mutation's error (LOG-005, LOG-007). No sentence holds a name, a host, an address, the MAC
//! address or the relay's output. Pure.

use super::Error;
use crate::ssh::LinkError;

/// The sentence for `error`.
pub(super) fn sentence(error: &Error) -> &'static str {
    match error {
        Error::NotFound(_) => "No Wake-on-LAN packet was sent: the device does not exist.",
        Error::NoMacAddress => {
            "No Wake-on-LAN packet was sent: the device has no MAC address. Add it in the \
             device's remote access settings."
        }
        Error::NoRelay => {
            "No Wake-on-LAN packet was sent: the device's site has no wake relay. Choose one \
             in the site's settings."
        }
        Error::RelayIsDevice => {
            "No Wake-on-LAN packet was sent: the device is its own site's wake relay. Choose \
             another device of the site as the wake relay."
        }
        Error::Link(link) => link_sentence(link),
        Error::NotSent => {
            "No Wake-on-LAN packet was sent: wakeonlan failed on the wake relay. Check that \
             it is installed there."
        }
        Error::Devices(_) | Error::Events(_) | Error::Database(_) => HUB_FAULT,
    }
}

/// A store failure: the operator can only try again.
const HUB_FAULT: &str = "No Wake-on-LAN packet was sent: the hub could not read or write its \
                         records. Try again.";

/// The sentence for a link to the relay that failed.
fn link_sentence(error: &LinkError) -> &'static str {
    match error {
        LinkError::SshOff => {
            "No Wake-on-LAN packet was sent: SSH is turned off for the wake relay. Turn it on \
             in the relay's SSH settings."
        }
        LinkError::NoHostKey => {
            "No Wake-on-LAN packet was sent: no host key is accepted for the wake relay. Test \
             its connection and accept its key."
        }
        LinkError::HostKeyChanged => {
            "No Wake-on-LAN packet was sent: the wake relay presented another host key than \
             the accepted one. No credential was sent."
        }
        LinkError::Unresolved | LinkError::RefusedTarget => {
            "No Wake-on-LAN packet was sent: the wake relay's host does not resolve to an \
             address the hub may connect to."
        }
        LinkError::Unreachable
        | LinkError::Timeout
        | LinkError::NoKeyExchange
        | LinkError::Lost => {
            "No Wake-on-LAN packet was sent: the wake relay cannot be reached over SSH."
        }
        LinkError::NoCredential => {
            "No Wake-on-LAN packet was sent: no SSH credential is stored for the wake relay."
        }
        LinkError::CredentialRefused | LinkError::UnreadableKey => {
            "No Wake-on-LAN packet was sent: the wake relay did not accept its stored SSH \
             credential, or the stored key cannot be read."
        }
        LinkError::CommandRefused => {
            "No Wake-on-LAN packet was sent: the wake relay refused to run wakeonlan."
        }
        LinkError::TimeLimit => {
            "No Wake-on-LAN packet was sent: wakeonlan did not finish on the wake relay \
             within 10 seconds."
        }
        LinkError::Trust(_) | LinkError::Credentials(_) => HUB_FAULT,
    }
}
