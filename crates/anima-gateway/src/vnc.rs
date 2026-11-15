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
use messages::ServerMessage;
pub use sign_in::{Account, Method, Offered, SignInError, offered, sign_in};

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

/// The device's side of an open session: the bytes not yet read as messages, and the
/// state the encodings keep between updates.
#[derive(Default)]
pub struct Device {
    unread: Vec<u8>,
    /// Bytes of an over-long clipboard text still to drop.
    skip: usize,
    inflaters: drawing::Inflaters,
}

impl Device {
    /// Takes `bytes` as they arrive from the device and draws every whole message into
    /// `screen`.
    ///
    /// # Errors
    /// [`Malformed`]: the stream cannot be read further and the session ends.
    pub fn receive(&mut self, bytes: &[u8], screen: &mut Screen) -> Result<Vec<Event>, Malformed> {
        let dropped = self.skip.min(bytes.len());
        self.skip -= dropped;
        self.unread.extend_from_slice(&bytes[dropped..]);
        let mut events = Vec::new();
        let mut read = 0;
        loop {
            let size = (screen.width(), screen.height());
            let Some((message, length)) = messages::first(&self.unread[read..], size)? else {
                break;
            };
            read += length;
            match message {
                ServerMessage::Update(rects) => {
                    self.inflaters.draw(&rects, screen)?;
                    events.push(Event::Updated);
                }
                ServerMessage::Bell => events.push(Event::Bell),
                // Latin-1: each byte is the character with that number.
                ServerMessage::CutText(latin1) => {
                    events.push(Event::Clipboard(
                        latin1.iter().map(|byte| char::from(*byte)).collect(),
                    ));
                }
                ServerMessage::LongCutText { skip } => {
                    let here = skip.min(self.unread.len() - read);
                    read += here;
                    self.skip = skip - here;
                }
            }
        }
        self.unread.drain(..read);
        Ok(events)
    }
}
