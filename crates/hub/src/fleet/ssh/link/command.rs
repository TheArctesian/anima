//! A command made only from static text and values the hub renders itself (CHK-012, MET-002,
//! D-3, HOST-001). Pure.

use crate::fleet::devices::MacAddress;

/// The lower-case hex digits, by value.
const HEX: &[u8; 16] = b"0123456789abcdef";

/// The most digits a [`HexDigits`] holds: a full Docker container identifier.
const MAX_HEX_DIGITS: usize = 64;

/// One word after the program.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Rendered {
    /// Text written in the hub's source.
    Static(&'static str),
    /// A hardware address, rendered by the hub from its six bytes.
    Mac(MacAddress),
    /// Hex digits, rendered by the hub from their values.
    Hex(HexDigits),
    /// A whole number, rendered by the hub in decimal.
    Number(u32),
}

/// 1 to 64 hex digits, kept as their values (0 to 15), so the word they render to is only
/// `0-9a-f` whatever text they were read from (HOST-001).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HexDigits(Vec<u8>);

impl HexDigits {
    /// The digits of `text` when it is 1 to 64 lower-case hex digits and nothing else.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        if text.is_empty() || text.len() > MAX_HEX_DIGITS {
            return None;
        }
        text.bytes()
            .map(|byte| {
                let value = HEX.iter().position(|digit| *digit == byte)?;
                u8::try_from(value).ok()
            })
            .collect::<Option<Vec<u8>>>()
            .map(Self)
    }
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

    /// The command with `digits` added at the end as lower-case hex.
    #[must_use]
    pub fn hex(mut self, digits: &HexDigits) -> Self {
        self.args.push(Rendered::Hex(digits.clone()));
        self
    }

    /// The command with `number` added at the end in decimal.
    #[must_use]
    pub fn number(mut self, number: u32) -> Self {
        self.args.push(Rendered::Number(number));
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
                Rendered::Hex(digits) => push_hex(&mut line, digits),
                Rendered::Number(number) => line.push_str(&number.to_string()),
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

/// Writes each digit of `digits` from its value.
fn push_hex(line: &mut String, digits: &HexDigits) {
    for value in &digits.0 {
        line.push(char::from(HEX[usize::from(value & 0x0f)]));
    }
}
