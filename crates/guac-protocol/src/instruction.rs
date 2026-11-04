//! One Guacamole instruction and its wire encoding.

use std::fmt;

/// A Guacamole instruction: an opcode and its arguments, in wire order.
///
/// `Debug` prints the opcode (at most 32 characters, then `…`) and the
/// argument count only: `connect` carries passwords and keys, and
/// no argument value may reach a log (LOG-007).
#[derive(Clone, PartialEq, Eq)]
pub struct Instruction {
    pub opcode: String,
    pub args: Vec<String>,
}

/// Longest prefix of a peer-supplied name that `Debug` and errors print.
const DEBUG_OPCODE_CHARS: usize = 32;

impl Instruction {
    #[must_use]
    pub fn new<A>(opcode: impl Into<String>, args: A) -> Self
    where
        A: IntoIterator,
        A::Item: Into<String>,
    {
        Self {
            opcode: opcode.into(),
            args: args.into_iter().map(Into::into).collect(),
        }
    }
}

impl fmt::Debug for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Instruction")
            .field("opcode", &truncated(&self.opcode))
            .field("arg_count", &self.args.len())
            .finish()
    }
}

/// The first 32 characters of `name`, then `…` when it is longer.
///
/// Opcodes and argument names come from the peer, so they are bounded before
/// they reach `Debug` or an error message.
pub(crate) fn truncated(name: &str) -> String {
    let mut prefix: String = name.chars().take(DEBUG_OPCODE_CHARS).collect();
    if name.chars().nth(DEBUG_OPCODE_CHARS).is_some() {
        prefix.push('…');
    }
    prefix
}

/// Encodes `i` as `LENGTH.VALUE` elements separated by `,` and ended by `;`.
///
/// `LENGTH` counts Unicode code points, as guacd does. The encoder does not
/// apply the [`Decoder`](crate::Decoder) limits; the caller builds instructions
/// that fit them.
#[must_use]
pub fn encode(i: &Instruction) -> String {
    let mut out = String::new();
    push_element(&mut out, &i.opcode);
    for arg in &i.args {
        out.push(',');
        push_element(&mut out, arg);
    }
    out.push(';');
    out
}

fn push_element(out: &mut String, value: &str) {
    out.push_str(&value.chars().count().to_string());
    out.push('.');
    out.push_str(value);
}
