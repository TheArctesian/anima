//! The location metadata of a JPEG, removed while the file streams (FIL-003). Pure: it walks
//! the segments before the image data, holds at most one segment (64 KiB) at a time, and passes
//! the rest of the file as it is.
//!
//! The EXIF GPS IFD becomes zero bytes (`exif::remove_gps`), and each XMP segment is left out,
//! because XMP can hold the same position.

use super::exif::{self, Position};

const EXIF: &[u8] = b"Exif\0\0";
const XMP: [&[u8]; 2] = [
    b"http://ns.adobe.com/xap/1.0/\0",
    b"http://ns.adobe.com/xmp/extension/\0",
];
const APP1: u8 = 0xE1;
/// Start of scan and end of image: the segments end here.
const LAST_HEADER_MARKERS: [u8; 2] = [0xDA, 0xD9];

/// The media types whose location the hub removes; it also cleans any file that starts as a
/// JPEG, whatever its stated type.
pub const CLEANED_TYPES: [&str; 2] = ["image/jpeg", "image/pjpeg"];

/// Whether `start`, the first bytes of a file, is the start of a JPEG.
#[must_use]
pub fn is_jpeg(start: &[u8]) -> bool {
    start.starts_with(&[0xFF, 0xD8, 0xFF])
}

/// The bytes do not have the form of a JPEG before its image data.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[error("the file is not a JPEG that the hub can clean")]
pub struct Malformed;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum State {
    Start,
    Marker,
    Length,
    Collect(usize),
    Skip(usize),
    Pass,
}

/// Walks one JPEG and gives a copy without its location metadata.
#[derive(Debug)]
pub struct Cleaner {
    state: State,
    pending: Vec<u8>,
    position: Option<Position>,
}

impl Default for Cleaner {
    fn default() -> Self {
        Self::new()
    }
}

impl Cleaner {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            state: State::Start,
            pending: Vec::new(),
            position: None,
        }
    }

    /// The first GPS position that the walk read, before it removed it.
    #[must_use]
    pub const fn position(&self) -> Option<Position> {
        self.position
    }

    /// Whether the walk has reached the image data: no more metadata comes.
    #[must_use]
    pub fn passed(&self) -> bool {
        self.state == State::Pass
    }

    /// The cleaned bytes of `input`. A segment that is not complete yet waits for the next
    /// call.
    ///
    /// # Errors
    /// [`Malformed`] when the bytes before the image data are not JPEG segments; give nothing
    /// more of the file then.
    pub fn feed(&mut self, mut input: &[u8]) -> Result<Vec<u8>, Malformed> {
        let mut out = Vec::with_capacity(input.len());
        while !input.is_empty() {
            match self.state {
                State::Pass => {
                    out.extend_from_slice(input);
                    break;
                }
                State::Skip(left) => {
                    let take = left.min(input.len());
                    out.extend_from_slice(&input[..take]);
                    input = &input[take..];
                    self.state = if take == left {
                        State::Marker
                    } else {
                        State::Skip(left - take)
                    };
                }
                State::Start | State::Marker | State::Length | State::Collect(_) => {
                    input = self.take(input);
                    if self.pending.len() == self.needed() {
                        self.step(&mut out)?;
                    }
                }
            }
        }
        Ok(out)
    }

    /// The bytes the current state needs in `pending`.
    fn needed(&self) -> usize {
        match self.state {
            State::Start | State::Marker => 2,
            State::Length => 4,
            State::Collect(total) => total,
            State::Skip(_) | State::Pass => 0,
        }
    }

    /// Moves what the state still needs from `input` to `pending`, and gives the rest.
    fn take<'a>(&mut self, input: &'a [u8]) -> &'a [u8] {
        let take = (self.needed() - self.pending.len()).min(input.len());
        self.pending.extend_from_slice(&input[..take]);
        &input[take..]
    }

    /// Acts on a complete `pending`.
    fn step(&mut self, out: &mut Vec<u8>) -> Result<(), Malformed> {
        let pending = std::mem::take(&mut self.pending);
        match self.state {
            State::Start if pending == [0xFF, 0xD8] => {
                out.extend_from_slice(&pending);
                self.state = State::Marker;
            }
            State::Marker => self.marker(pending, out)?,
            State::Length => {
                let length = usize::from(u16::from_be_bytes([pending[2], pending[3]]));
                if length < 2 {
                    return Err(Malformed);
                }
                if pending[1] == APP1 {
                    self.state = State::Collect(length + 2);
                    self.pending = pending;
                } else {
                    out.extend_from_slice(&pending);
                    self.state = Self::skip(length - 2);
                }
            }
            State::Collect(_) => {
                self.segment(pending, out);
                self.state = State::Marker;
            }
            _ => return Err(Malformed),
        }
        Ok(())
    }

    const fn skip(left: usize) -> State {
        if left == 0 {
            State::Marker
        } else {
            State::Skip(left)
        }
    }

    fn marker(&mut self, pending: Vec<u8>, out: &mut Vec<u8>) -> Result<(), Malformed> {
        let [0xFF, marker] = pending[..] else {
            return Err(Malformed);
        };
        match marker {
            // A fill byte: the marker is the byte after it.
            0xFF => {
                out.push(0xFF);
                self.pending = vec![0xFF];
            }
            0x01 | 0xD0..=0xD8 => out.extend_from_slice(&pending),
            _ if LAST_HEADER_MARKERS.contains(&marker) => {
                out.extend_from_slice(&pending);
                self.state = State::Pass;
            }
            _ => {
                self.pending = pending;
                self.state = State::Length;
            }
        }
        Ok(())
    }

    /// One whole APP1 segment: the EXIF GPS part becomes zeros and XMP is left out.
    fn segment(&mut self, mut segment: Vec<u8>, out: &mut Vec<u8>) {
        let payload = &mut segment[4..];
        if XMP.iter().any(|header| payload.starts_with(header)) {
            return;
        }
        if payload.starts_with(EXIF) {
            let found = exif::remove_gps(&mut payload[EXIF.len()..]);
            self.position = self.position.or(found);
        }
        out.extend_from_slice(&segment);
    }
}
