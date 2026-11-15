//! The server's messages after sign-in (RFC 6143 Section 7.6), read from a buffer of
//! received bytes. Nothing is decoded here: a message is cut into its parts and checked
//! against the limits, so the gateway never waits for more bytes than a device may send.

use crate::screen::{Area, MAX_SIDE};

use super::Malformed;

/// The bytes of one pixel as the gateway asks for it (`requests::pixel_format`).
pub(super) const PIXEL_BYTES: usize = 4;
/// The longest clipboard text taken from a device. A longer one is skipped, not an error:
/// the operator may copy a large text on the device for another purpose.
pub(super) const LONGEST_CUT_TEXT: usize = 1 << 20;
/// One update may send this many times the screen's pixels. A device has no reason to
/// send a pixel more than a few times in one update; without a limit it could keep the
/// gateway busy for a long time with one message.
const MOST_SCREENS_IN_AN_UPDATE: usize = 8;

const RAW: i32 = 0;
const COPY_RECT: i32 = 1;
const ZLIB: i32 = 6;
const ZRLE: i32 = 16;
const DESKTOP_SIZE: i32 = -223;
/// The encodings the gateway asks for, most wanted first.
pub(super) const ENCODINGS: [i32; 5] = [ZRLE, ZLIB, COPY_RECT, RAW, DESKTOP_SIZE];

/// How one rectangle's pixels arrive.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Body<'a> {
    /// Four bytes for each pixel.
    Raw(&'a [u8]),
    /// The pixels are on the screen already, at this corner.
    CopyFrom { x: usize, y: usize },
    /// Raw pixels through the session's zlib stream.
    Zlib(&'a [u8]),
    /// ZRLE tiles through the session's ZRLE zlib stream.
    Zrle(&'a [u8]),
    /// No pixels: the screen now has the rectangle's width and height.
    NewSize,
}

#[derive(Debug, PartialEq, Eq)]
pub(super) struct Rect<'a> {
    pub(super) area: Area,
    pub(super) body: Body<'a>,
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum ServerMessage<'a> {
    Update(Vec<Rect<'a>>),
    Bell,
    /// The device's clipboard, in Latin-1.
    CutText(&'a [u8]),
    /// A clipboard text over the limit: the caller drops this many bytes that follow.
    LongCutText {
        skip: usize,
    },
}

/// Why reading stopped.
enum Stop {
    /// The buffer ends inside the message.
    Incomplete,
    Malformed(Malformed),
}

impl From<Malformed> for Stop {
    fn from(malformed: Malformed) -> Self {
        Self::Malformed(malformed)
    }
}

/// The unread end of the buffer.
struct Unread<'a>(&'a [u8]);

impl<'a> Unread<'a> {
    fn take(&mut self, count: usize) -> Result<&'a [u8], Stop> {
        let (taken, rest) = self.0.split_at_checked(count).ok_or(Stop::Incomplete)?;
        self.0 = rest;
        Ok(taken)
    }

    fn u16(&mut self) -> Result<usize, Stop> {
        let bytes = self.take(2)?;
        Ok(usize::from(u16::from_be_bytes([bytes[0], bytes[1]])))
    }

    fn u32(&mut self) -> Result<u32, Stop> {
        let bytes = self.take(4)?;
        Ok(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }
}

/// The first message in `buffer` and its length in bytes, for a screen of `size`; `None`
/// when the buffer does not hold a whole message yet.
///
/// # Errors
/// [`Malformed`] when the bytes cannot be a message the gateway asked for, or break a
/// limit. The session then ends: the stream cannot be read further.
pub(super) fn first(
    buffer: &[u8],
    size: (usize, usize),
) -> Result<Option<(ServerMessage<'_>, usize)>, Malformed> {
    let mut unread = Unread(buffer);
    match message(&mut unread, size) {
        Ok(message) => Ok(Some((message, buffer.len() - unread.0.len()))),
        Err(Stop::Incomplete) => Ok(None),
        Err(Stop::Malformed(malformed)) => Err(malformed),
    }
}

fn message<'a>(unread: &mut Unread<'a>, size: (usize, usize)) -> Result<ServerMessage<'a>, Stop> {
    match unread.take(1)?[0] {
        0 => {
            unread.take(1)?;
            update(unread, size).map(ServerMessage::Update)
        }
        2 => Ok(ServerMessage::Bell),
        3 => {
            unread.take(3)?;
            let length = usize::try_from(unread.u32()?).unwrap_or(usize::MAX);
            if length > LONGEST_CUT_TEXT {
                return Ok(ServerMessage::LongCutText { skip: length });
            }
            Ok(ServerMessage::CutText(unread.take(length)?))
        }
        _ => Err(Malformed::UnknownMessage.into()),
    }
}

fn update<'a>(unread: &mut Unread<'a>, mut size: (usize, usize)) -> Result<Vec<Rect<'a>>, Stop> {
    let count = unread.u16()?;
    let mut rects = Vec::with_capacity(count.min(1024));
    let mut pixels_left = size.0 * size.1 * MOST_SCREENS_IN_AN_UPDATE;
    for _ in 0..count {
        let area = Area {
            x: unread.u16()?,
            y: unread.u16()?,
            width: unread.u16()?,
            height: unread.u16()?,
        };
        let encoding = unread.u32()?.cast_signed();
        let body = if encoding == DESKTOP_SIZE {
            if !(1..=MAX_SIDE).contains(&area.width) || !(1..=MAX_SIDE).contains(&area.height) {
                return Err(Malformed::BadSize.into());
            }
            size = (area.width, area.height);
            pixels_left = pixels_left.max(size.0 * size.1 * MOST_SCREENS_IN_AN_UPDATE);
            Body::NewSize
        } else {
            let inside = area.x + area.width <= size.0 && area.y + area.height <= size.1;
            let pixels = area.width * area.height;
            pixels_left = pixels_left
                .checked_sub(pixels)
                .ok_or(Malformed::TooManyPixels)?;
            if !inside {
                return Err(Malformed::OutsideScreen.into());
            }
            pixel_body(unread, encoding, pixels)?
        };
        rects.push(Rect { area, body });
    }
    Ok(rects)
}

/// The body of a rectangle of `pixels` pixels in `encoding`.
fn pixel_body<'a>(unread: &mut Unread<'a>, encoding: i32, pixels: usize) -> Result<Body<'a>, Stop> {
    match encoding {
        RAW => Ok(Body::Raw(unread.take(pixels * PIXEL_BYTES)?)),
        COPY_RECT => Ok(Body::CopyFrom {
            x: unread.u16()?,
            y: unread.u16()?,
        }),
        ZLIB | ZRLE => {
            let length = usize::try_from(unread.u32()?).unwrap_or(usize::MAX);
            // zlib adds a few bytes to data it cannot shrink, never a multiple.
            if length > pixels * PIXEL_BYTES + 1024 {
                return Err(Malformed::TooManyBytes.into());
            }
            let packed = unread.take(length)?;
            Ok(if encoding == ZLIB {
                Body::Zlib(packed)
            } else {
                Body::Zrle(packed)
            })
        }
        _ => Err(Malformed::UnknownEncoding.into()),
    }
}
