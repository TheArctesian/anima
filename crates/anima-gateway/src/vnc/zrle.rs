//! ZRLE tiles (RFC 6143 Section 7.7.6), after the zlib stream: 64 x 64 tiles, each raw,
//! one colour, a packed palette, run-length, or palette run-length. A pixel is three
//! bytes: blue, green, red.

use crate::screen::{Area, BYTES_PER_PIXEL};

use super::Malformed;

const TILE: usize = 64;

/// The unread tile bytes.
struct Unread<'a>(&'a [u8]);

impl<'a> Unread<'a> {
    fn take(&mut self, count: usize) -> Result<&'a [u8], Malformed> {
        let (taken, rest) = self.0.split_at_checked(count).ok_or(Malformed::BadTile)?;
        self.0 = rest;
        Ok(taken)
    }

    fn byte(&mut self) -> Result<u8, Malformed> {
        Ok(self.take(1)?[0])
    }

    /// A run length: 1 and the sum of its bytes, where 255 means another byte follows. A
    /// run longer than `most` is malformed, so the sum cannot grow without limit.
    fn run(&mut self, most: usize) -> Result<usize, Malformed> {
        let mut length = 1;
        loop {
            let byte = self.byte()?;
            length += usize::from(byte);
            if length > most {
                return Err(Malformed::BadTile);
            }
            if byte != 255 {
                return Ok(length);
            }
        }
    }
}

/// Gives `draw` each tile of `area` with its pixels as red, green, blue, in order. The
/// tile bytes must end with the last tile.
pub(super) fn tiles(
    bytes: &[u8],
    area: Area,
    mut draw: impl FnMut(Area, &[u8]) -> Result<(), Malformed>,
) -> Result<(), Malformed> {
    let mut unread = Unread(bytes);
    let mut rgb = Vec::with_capacity(TILE * TILE * BYTES_PER_PIXEL);
    for top in (0..area.height).step_by(TILE) {
        for left in (0..area.width).step_by(TILE) {
            let tile = Area {
                x: area.x + left,
                y: area.y + top,
                width: TILE.min(area.width - left),
                height: TILE.min(area.height - top),
            };
            rgb.clear();
            tile_pixels(&mut unread, tile, &mut rgb)?;
            draw(tile, &rgb)?;
        }
    }
    if unread.0.is_empty() {
        Ok(())
    } else {
        Err(Malformed::BadTile)
    }
}

/// Adds `count` pixels of the blue-green-red `colour` to `rgb`.
fn push(rgb: &mut Vec<u8>, colour: &[u8], count: usize) {
    for _ in 0..count {
        rgb.extend_from_slice(&[colour[2], colour[1], colour[0]]);
    }
}

/// Reads one tile into `rgb`.
fn tile_pixels(unread: &mut Unread<'_>, tile: Area, rgb: &mut Vec<u8>) -> Result<(), Malformed> {
    let count = tile.width * tile.height;
    match unread.byte()? {
        0 => {
            let (colours, _) = unread.take(count * 3)?.as_chunks::<3>();
            for colour in colours {
                push(rgb, colour, 1);
            }
        }
        1 => push(rgb, unread.take(3)?, count),
        kind @ 2..=16 => packed_palette(unread, tile, usize::from(kind), rgb)?,
        128 => {
            let mut left = count;
            while left > 0 {
                let colour = unread.take(3)?;
                let run = unread.run(left)?;
                push(rgb, colour, run);
                left -= run;
            }
        }
        kind @ 130..=255 => {
            let palette = unread.take((usize::from(kind) - 128) * 3)?;
            let mut left = count;
            while left > 0 {
                let index = unread.byte()?;
                let run = if index & 0x80 == 0 {
                    1
                } else {
                    unread.run(left)?
                };
                let at = usize::from(index & 0x7f) * 3;
                push(rgb, palette.get(at..at + 3).ok_or(Malformed::BadTile)?, run);
                left -= run;
            }
        }
        _ => return Err(Malformed::BadTile),
    }
    Ok(())
}

/// A tile of palette numbers packed into 1, 2 or 4 bits, each row filled to a whole byte.
fn packed_palette(
    unread: &mut Unread<'_>,
    tile: Area,
    colours: usize,
    rgb: &mut Vec<u8>,
) -> Result<(), Malformed> {
    let palette = unread.take(colours * 3)?;
    let bits = match colours {
        2 => 1,
        3..=4 => 2,
        _ => 4,
    };
    for _ in 0..tile.height {
        let row = unread.take((tile.width * bits).div_ceil(8))?;
        for x in 0..tile.width {
            let shift = 8 - bits - (x * bits) % 8;
            let index = usize::from(row[x * bits / 8] >> shift) & ((1 << bits) - 1);
            push(
                rgb,
                palette
                    .get(index * 3..index * 3 + 3)
                    .ok_or(Malformed::BadTile)?,
                1,
            );
        }
    }
    Ok(())
}
