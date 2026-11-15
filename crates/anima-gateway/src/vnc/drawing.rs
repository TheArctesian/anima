//! Drawing an update's rectangles into the screen copy (RFC 6143 Section 7.7): Raw,
//! `CopyRect`, Zlib, ZRLE and the desktop-size message.
//!
//! Everything a device sends is limited here before it costs memory: inflated data may
//! not be longer than its rectangle's pixels need.

use flate2::{Decompress, FlushDecompress, Status};

use super::Malformed;
use super::messages::{Body, PIXEL_BYTES, Rect};
use crate::screen::{Area, BYTES_PER_PIXEL, Screen};

/// The two zlib streams of a session. Each lives as long as the session, so each packed
/// rectangle must be inflated, in order, exactly once.
pub(super) struct Inflaters {
    zlib: Decompress,
    zrle: Decompress,
    /// Space for inflated data and for pixels on their way to the screen, kept between
    /// rectangles.
    inflated: Vec<u8>,
    rgb: Vec<u8>,
}

impl Default for Inflaters {
    fn default() -> Self {
        Self {
            zlib: Decompress::new(true),
            zrle: Decompress::new(true),
            inflated: Vec::new(),
            rgb: Vec::new(),
        }
    }
}

impl Inflaters {
    /// Draws `rects`, in order, into `screen`. Returns whether the screen got a new size.
    pub(super) fn draw(
        &mut self,
        rects: &[Rect<'_>],
        screen: &mut Screen,
    ) -> Result<bool, Malformed> {
        let mut resized = false;
        for rect in rects {
            let area = rect.area;
            let pixels = area.width * area.height;
            match rect.body {
                Body::Raw(bgrx) => self.write_bgrx(screen, area, bgrx)?,
                Body::CopyFrom { x, y } => {
                    let from = Area { x, y, ..area };
                    screen
                        .move_area(from, area.x, area.y)
                        .map_err(|_| Malformed::OutsideScreen)?;
                }
                Body::Zlib(packed) => {
                    let mut inflated = std::mem::take(&mut self.inflated);
                    inflate(&mut self.zlib, packed, pixels * PIXEL_BYTES, &mut inflated)?;
                    let written = self.write_bgrx(screen, area, &inflated);
                    self.inflated = inflated;
                    written?;
                }
                Body::Zrle(packed) => {
                    // A tile is at most one byte and then three for each pixel.
                    let most = pixels * 3 + area.width.div_ceil(64) * area.height.div_ceil(64);
                    inflate(&mut self.zrle, packed, most, &mut self.inflated)?;
                    super::zrle::tiles(&self.inflated, area, |tile, rgb| {
                        screen
                            .write(tile, rgb)
                            .map_err(|_| Malformed::OutsideScreen)
                    })?;
                }
                Body::NewSize => {
                    screen
                        .resize(area.width, area.height)
                        .map_err(|_| Malformed::BadSize)?;
                    resized = true;
                }
            }
        }
        Ok(resized)
    }

    /// Writes pixels that arrive as blue, green, red and one unused byte.
    fn write_bgrx(
        &mut self,
        screen: &mut Screen,
        area: Area,
        bgrx: &[u8],
    ) -> Result<(), Malformed> {
        let (pixels, rest) = bgrx.as_chunks::<PIXEL_BYTES>();
        if !rest.is_empty() || pixels.len() != area.width * area.height {
            return Err(Malformed::BadPixels);
        }
        self.rgb.clear();
        self.rgb.reserve(pixels.len() * BYTES_PER_PIXEL);
        for pixel in pixels {
            self.rgb.extend_from_slice(&[pixel[2], pixel[1], pixel[0]]);
        }
        screen
            .write(area, &self.rgb)
            .map_err(|_| Malformed::OutsideScreen)
    }
}

/// Inflates all of `packed` into `out`, which may not become longer than `most`.
fn inflate(
    stream: &mut Decompress,
    packed: &[u8],
    most: usize,
    out: &mut Vec<u8>,
) -> Result<(), Malformed> {
    out.clear();
    // One byte more than allowed, so data that is too long is seen and not cut.
    out.reserve(most + 1);
    let start = stream.total_in();
    loop {
        let used = usize::try_from(stream.total_in() - start).unwrap_or(usize::MAX);
        let status = stream
            .decompress_vec(
                &packed[used.min(packed.len())..],
                out,
                FlushDecompress::Sync,
            )
            .map_err(|_| Malformed::BadZlib)?;
        let used = usize::try_from(stream.total_in() - start).unwrap_or(usize::MAX);
        if out.len() > most {
            return Err(Malformed::TooManyBytes);
        }
        if used >= packed.len() {
            return Ok(());
        }
        // Input is left but nothing more can be done with it.
        if status == Status::StreamEnd || status == Status::BufError {
            return Err(Malformed::BadZlib);
        }
    }
}
