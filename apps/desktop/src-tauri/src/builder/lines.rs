//! Splits a step's output into lines for the web view (BLD-048). The output is the build's, so a
//! line's length is never trusted: a line longer than `MAX_LINE_BYTES` is cut, and the rest of it
//! is dropped up to the next line break. Pure.

/// The longest line passed on, in bytes; the hub keeps lines of up to 8 KiB.
pub const MAX_LINE_BYTES: usize = 4096;
const CUT: &str = " [line cut]";

/// The lines of one output stream, fed in chunks of any size.
#[derive(Debug, Default)]
pub struct Lines {
    pending: Vec<u8>,
    /// The current line was cut; drop bytes until its line break.
    dropping: bool,
}

fn text_of(bytes: &[u8]) -> String {
    let bytes = bytes.strip_suffix(b"\r").unwrap_or(bytes);
    String::from_utf8_lossy(bytes).into_owned()
}

impl Lines {
    /// The lines `chunk` completes, in order.
    pub fn push(&mut self, chunk: &[u8]) -> Vec<String> {
        let mut done = Vec::new();
        for &byte in chunk {
            if byte == b'\n' {
                if !self.dropping {
                    done.push(text_of(&self.pending));
                }
                self.pending.clear();
                self.dropping = false;
            } else if !self.dropping {
                self.pending.push(byte);
                if self.pending.len() == MAX_LINE_BYTES {
                    done.push(format!("{}{CUT}", text_of(&self.pending)));
                    self.pending.clear();
                    self.dropping = true;
                }
            }
        }
        done
    }

    /// The last line, when the stream ended without a line break.
    pub fn finish(&mut self) -> Option<String> {
        let rest = (!self.pending.is_empty() && !self.dropping).then(|| text_of(&self.pending));
        self.pending.clear();
        self.dropping = false;
        rest
    }
}
