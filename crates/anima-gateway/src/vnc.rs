//! The VNC client (RFB 3.8, RFC 6143): the sign-in to a device, the device's updates drawn
//! into the screen copy, and the client's messages.
//!
//! The device is not trusted. Every length it sends is checked against a limit before the
//! gateway waits for the bytes or makes room for them, and no bytes from a device can make
//! this code panic.

mod auth;
mod drawing;
mod messages;
pub mod requests;
mod sign_in;
mod zrle;

use crate::screen::Screen;
use messages::{Body, Start};
pub use sign_in::{Account, Method, Offered, SignInError, offered, sign_in};

/// One update may send this many times the screen's pixels: a device has no reason to
/// send a pixel more often, and without a limit one message could keep the gateway busy
/// for a long time.
const MOST_SCREENS_IN_AN_UPDATE: usize = 2;
/// Above this many bytes of room, the unread buffer is given back once it is nearly empty.
const LARGE_BUFFER: usize = 1 << 20;

/// Why a device's bytes cannot be read further. The session ends.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum Malformed {
    #[error("a message type the gateway did not ask for")]
    UnknownMessage,
    #[error("an encoding the gateway did not ask for")]
    UnknownEncoding,
    #[error("a screen size outside the limits")]
    BadSize,
    #[error("more pixels in one update than the limit")]
    TooManyPixels,
    #[error("a rectangle outside the screen")]
    OutsideScreen,
    #[error("more bytes for a rectangle than its pixels need")]
    TooManyBytes,
    #[error("a ZRLE tile that does not decode")]
    BadTile,
    #[error("zlib data that does not inflate")]
    BadZlib,
    #[error("pixel data of the wrong length")]
    BadPixels,
}

/// What a device did, after the screen copy has taken it in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    /// An update arrived; the screen copy may have changed.
    Updated,
    /// The device's clipboard has this text.
    Clipboard(String),
    Bell,
}

/// An update whose rectangles are still arriving.
struct Update {
    rects_left: usize,
    /// The pixels its rectangles may still send.
    pixels_left: usize,
    /// It has changed the screen size; it may do so once.
    resized: bool,
}

/// The device's side of an open session: the bytes not yet read as messages, and the
/// state the encodings keep between updates.
#[derive(Default)]
pub struct Device {
    /// At most one rectangle or clipboard text, and the read that completed it.
    unread: Vec<u8>,
    /// Bytes of an over-long clipboard text still to drop.
    skip: usize,
    update: Option<Update>,
    inflaters: drawing::Inflaters,
}

impl Device {
    /// Takes `bytes` as they arrive from the device and draws each rectangle into `screen`
    /// as soon as it is whole.
    ///
    /// # Errors
    /// [`Malformed`]: the stream cannot be read further and the session ends.
    pub fn receive(&mut self, bytes: &[u8], screen: &mut Screen) -> Result<Vec<Event>, Malformed> {
        let dropped = self.skip.min(bytes.len());
        self.skip -= dropped;
        self.unread.extend_from_slice(&bytes[dropped..]);
        let mut events = Vec::new();
        let mut read = 0;
        while let Some(length) = self.read_one(read, screen, &mut events)? {
            read += length;
        }
        self.unread.drain(..read);
        // The room one large rectangle needed is not kept for the rest of the session.
        if self.unread.capacity() > LARGE_BUFFER && self.unread.len() < LARGE_BUFFER / 16 {
            self.unread.shrink_to(LARGE_BUFFER / 16);
        }
        Ok(events)
    }

    /// Reads what is at `from` in the unread bytes: a rectangle of the update in progress,
    /// or the start of a message. Gives its length; `None` when it is not all there yet.
    fn read_one(
        &mut self,
        from: usize,
        screen: &mut Screen,
        events: &mut Vec<Event>,
    ) -> Result<Option<usize>, Malformed> {
        let unread = &self.unread[from..];
        let Some(update) = &mut self.update else {
            let Some((start, mut length)) = messages::start(unread)? else {
                return Ok(None);
            };
            match start {
                Start::Update { rects: 0 } => events.push(Event::Updated),
                Start::Update { rects } => {
                    let pixels_left = screen.width() * screen.height() * MOST_SCREENS_IN_AN_UPDATE;
                    self.update = Some(Update {
                        rects_left: rects,
                        pixels_left,
                        resized: false,
                    });
                }
                Start::Bell => events.push(Event::Bell),
                // Latin-1: each byte is the character with that number.
                Start::CutText(latin1) => {
                    events.push(Event::Clipboard(
                        latin1.iter().map(|byte| char::from(*byte)).collect(),
                    ));
                }
                Start::LongCutText { skip } => {
                    let here = skip.min(unread.len() - length);
                    length += here;
                    self.skip = skip - here;
                }
            }
            return Ok(Some(length));
        };
        let size = (screen.width(), screen.height());
        let Some((rect, length)) = messages::rect(unread, size)? else {
            return Ok(None);
        };
        let pixels = rect.area.width * rect.area.height;
        if rect.body == Body::NewSize {
            // One new size in an update. What is left of the limit must at least hold the new
            // screen once, and is never more than the limit of the new screen.
            if std::mem::replace(&mut update.resized, true) {
                return Err(Malformed::BadSize);
            }
            update.pixels_left = update
                .pixels_left
                .min(pixels * MOST_SCREENS_IN_AN_UPDATE)
                .max(pixels);
        } else {
            update.pixels_left = update
                .pixels_left
                .checked_sub(pixels)
                .ok_or(Malformed::TooManyPixels)?;
        }
        self.inflaters.draw(&[rect], screen)?;
        update.rects_left -= 1;
        if update.rects_left == 0 {
            self.update = None;
            events.push(Event::Updated);
        }
        Ok(Some(length))
    }
}
