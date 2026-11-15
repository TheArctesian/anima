//! What the hub asked for in `connect`: the device, the sign-in, the clipboard switches and
//! the client's view (K4 Section 3).

use std::fmt;
use std::net::{IpAddr, SocketAddr};

use zeroize::Zeroizing;

use super::Refused;
use crate::frames::Formats;
use crate::screen::MAX_SIDE;

/// The longest user name or password the gateway takes, in bytes.
pub(super) const LONGEST_SECRET: usize = 4096;

/// A user name or password. Zeroed when dropped; `Debug` does not show it (LOG-007).
pub struct Secret(Zeroizing<String>);

impl Secret {
    pub(super) fn new(text: String) -> Self {
        Self(Zeroizing::new(text))
    }

    /// The text, to sign in with.
    #[must_use]
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("<redacted>")
    }
}

/// One session, as the hub opened it.
#[derive(Debug)]
pub struct Opening {
    /// The device's address. The hub resolved and checked it (R-032); the gateway takes an
    /// IP address only, so it never resolves a name itself.
    pub device: SocketAddr,
    pub username: Option<Secret>,
    pub password: Option<Secret>,
    /// The device's clipboard may go to the client (RA-033).
    pub copy: bool,
    /// The client's clipboard may go to the device (RA-033).
    pub paste: bool,
    /// The client's view, in pixels.
    pub view: (usize, usize),
    pub formats: Formats,
}

impl Opening {
    /// Reads the arguments of `connect`: the hub's version, then one value for each name
    /// in `args`, in that order.
    pub(super) fn from_connect(
        mut args: Vec<String>,
        view: (usize, usize),
        formats: Formats,
    ) -> Result<Self, Refused> {
        let bad = |argument| Refused::BadValue { argument };
        // The values may hold a password: each is zeroed when this function ends, however
        // it ends.
        let values = Zeroizing::new(args.split_off(args.len().min(1)));
        // One value for each name in `ARGUMENTS`, in its order.
        let [hostname, port, username, password, no_copy, no_paste] = values.as_slice() else {
            return Err(bad("connect"));
        };
        let address: IpAddr = hostname.parse().map_err(|_| bad("hostname"))?;
        if !may_be_a_device(address) {
            return Err(bad("hostname"));
        }
        let port = port
            .parse::<u16>()
            .ok()
            .filter(|port| *port != 0)
            .ok_or(bad("port"))?;
        Ok(Self {
            device: SocketAddr::new(address, port),
            username: secret(username).map_err(|()| bad("username"))?,
            password: secret(password).map_err(|()| bad("password"))?,
            copy: no_copy != "true",
            paste: no_paste != "true",
            view,
            formats,
        })
    }
}

/// A `connect` value as a secret: `None` when it is empty, refused when it is too long.
fn secret(text: &str) -> Result<Option<Secret>, ()> {
    match text.len() {
        0 => Ok(None),
        1..=LONGEST_SECRET => Ok(Some(Secret::new(text.to_owned()))),
        _ => Err(()),
    }
}

/// Whether `address` can be a device. The hub has already applied its own rule (R-032) and
/// sends an address in its plain form. The gateway refuses again what is never a device:
/// no address, every address, a group of hosts, and an IPv4 address written as IPv6.
fn may_be_a_device(address: IpAddr) -> bool {
    match address {
        IpAddr::V4(v4) => !(v4.is_unspecified() || v4.is_multicast() || v4.is_broadcast()),
        IpAddr::V6(v6) => {
            !(v6.is_unspecified() || v6.is_multicast() || v6.to_ipv4_mapped().is_some())
        }
    }
}

/// The client's view from the arguments of `size`: width, height and, not used, DPI.
pub(super) fn size(args: &[String]) -> Result<(usize, usize), Refused> {
    let side = |at: usize| {
        let side = args.get(at)?.parse::<usize>().ok()?;
        (1..=MAX_SIDE).contains(&side).then_some(side)
    };
    side(0)
        .zip(side(1))
        .ok_or(Refused::BadValue { argument: "size" })
}
