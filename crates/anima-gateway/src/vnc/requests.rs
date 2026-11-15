//! The client's messages (RFC 6143 Section 7.5), as bytes to send to the device.

use super::messages::ENCODINGS;

/// X11 keysym and button mask are what Guacamole's `key` and `mouse` carry, so input
/// passes through with the same numbers.
#[must_use]
pub fn key(keysym: u32, pressed: bool) -> Vec<u8> {
    let mut message = vec![4, u8::from(pressed), 0, 0];
    message.extend_from_slice(&keysym.to_be_bytes());
    message
}

#[must_use]
pub fn pointer(buttons: u8, x: u16, y: u16) -> Vec<u8> {
    let mut message = vec![5, buttons];
    message.extend_from_slice(&x.to_be_bytes());
    message.extend_from_slice(&y.to_be_bytes());
    message
}

/// 32 bits for a pixel, depth 24, little-endian, true colour, red at bit 16, green at 8,
/// blue at 0: a pixel arrives as blue, green, red and one unused byte.
pub(super) fn pixel_format() -> Vec<u8> {
    let mut message = vec![0, 0, 0, 0];
    message.extend_from_slice(&[32, 24, 0, 1, 0, 255, 0, 255, 0, 255, 16, 8, 0, 0, 0, 0]);
    message
}

pub(super) fn encodings() -> Vec<u8> {
    let mut message = vec![2, 0, 0, 5];
    for encoding in ENCODINGS {
        message.extend_from_slice(&encoding.to_be_bytes());
    }
    message
}

/// Asks for the screen: all of it, or what changed since the last update.
#[must_use]
pub fn update(incremental: bool, width: u16, height: u16) -> Vec<u8> {
    let mut message = vec![3, u8::from(incremental), 0, 0, 0, 0];
    message.extend_from_slice(&width.to_be_bytes());
    message.extend_from_slice(&height.to_be_bytes());
    message
}

/// Sets the device's clipboard. The protocol carries Latin-1: a character outside it goes
/// as a question mark.
#[must_use]
pub fn cut_text(text: &str) -> Vec<u8> {
    let latin1: Vec<u8> = text
        .chars()
        .map(|c| u8::try_from(u32::from(c)).unwrap_or(b'?'))
        .collect();
    let mut message = vec![6, 0, 0, 0];
    message.extend_from_slice(
        &u32::try_from(latin1.len())
            .unwrap_or(u32::MAX)
            .to_be_bytes(),
    );
    message.extend_from_slice(&latin1);
    message
}
