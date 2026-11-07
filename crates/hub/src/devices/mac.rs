//! The hardware address of a device's network card, the target of Wake-on-LAN (RA-031).

use std::{fmt, str::FromStr};

/// A unicast hardware address. `Display` renders lower-case `aa:bb:cc:dd:ee:ff` for the wake
/// command; `Debug` hides it, so a `?mac` in a log line shows no address (LOG-007).
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct MacAddress([u8; 6]);

/// Why text is not a Wake-on-LAN target.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum MacAddressError {
    #[error("a hardware address is six pairs of hex digits, such as aa:bb:cc:dd:ee:ff")]
    Malformed,
    #[error("the all-zero address is not a network card")]
    AllZero,
    #[error("a group address (multicast or broadcast) is not one network card")]
    Group,
}

impl MacAddress {
    #[must_use]
    pub const fn octets(self) -> [u8; 6] {
        self.0
    }
}

impl FromStr for MacAddress {
    type Err = MacAddressError;

    /// Six hex pairs split by `:` or `-` (not both), in any case.
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let separator = if text.contains(':') { ':' } else { '-' };
        let mut octets = [0_u8; 6];
        let mut pairs = text.split(separator);
        for octet in &mut octets {
            let pair = pairs.next().ok_or(MacAddressError::Malformed)?;
            if pair.len() != 2 || !pair.bytes().all(|b| b.is_ascii_hexdigit()) {
                return Err(MacAddressError::Malformed);
            }
            *octet = u8::from_str_radix(pair, 16).map_err(|_| MacAddressError::Malformed)?;
        }
        if pairs.next().is_some() {
            return Err(MacAddressError::Malformed);
        }
        if octets == [0; 6] {
            return Err(MacAddressError::AllZero);
        }
        if octets[0] & 1 == 1 {
            return Err(MacAddressError::Group);
        }
        Ok(Self(octets))
    }
}

impl fmt::Debug for MacAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("MacAddress(..)")
    }
}

impl fmt::Display for MacAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, octet) in self.0.iter().enumerate() {
            if index > 0 {
                f.write_str(":")?;
            }
            write!(f, "{octet:02x}")?;
        }
        Ok(())
    }
}
