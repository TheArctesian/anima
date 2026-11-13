//! A command made only from static text and values the hub renders itself (CHK-012, MET-002,
//! D-3). Pure.

use crate::fleet::devices::MacAddress;

/// The lower-case hex digits, by value.
const HEX: &[u8; 16] = b"0123456789abcdef";

/// One word after the program.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Rendered {
    /// Text written in the hub's source.
    Static(&'static str),
    /// A hardware address, rendered by the hub from its six bytes.
    Mac(MacAddress),
}

/// A command made only from static text and hub-rendered values; operator text cannot enter
/// it (CHK-012). No constructor or method takes text that is not `'static`, so a host name, a
/// tag or anything else an operator typed cannot become part of the line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FixedCommand {
    program: &'static str,
    args: Vec<Rendered>,
}

impl FixedCommand {
    /// A command that runs `program` with no arguments.
    #[must_use]
    pub const fn new(program: &'static str) -> Self {
        Self {
            program,
            args: Vec::new(),
        }
    }

    /// The command with the static word `arg` added at the end.
    #[must_use]
    pub fn arg(mut self, arg: &'static str) -> Self {
        self.args.push(Rendered::Static(arg));
        self
    }

    /// The command with `mac` added at the end as `aa:bb:cc:dd:ee:ff`.
    #[must_use]
    pub fn mac(mut self, mac: MacAddress) -> Self {
        self.args.push(Rendered::Mac(mac));
        self
    }

    /// The line the device runs: the program and each argument, joined by single spaces.
    pub(super) fn line(&self) -> String {
        let mut line = self.program.to_owned();
        for arg in &self.args {
            line.push(' ');
            match arg {
                Rendered::Static(text) => line.push_str(text),
                Rendered::Mac(mac) => push_mac(&mut line, *mac),
            }
        }
        line
    }
}

/// Writes the six bytes of `mac` as lower-case hex pairs split by `:`, from the bytes, so the
/// line does not depend on how `MacAddress` displays itself.
fn push_mac(line: &mut String, mac: MacAddress) {
    for (index, octet) in mac.octets().iter().enumerate() {
        if index > 0 {
            line.push(':');
        }
        line.push(char::from(HEX[usize::from(octet >> 4)]));
        line.push(char::from(HEX[usize::from(octet & 0x0f)]));
    }
}
