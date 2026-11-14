//! The gateway's copy of a device's screen, and what changed on it since the client was
//! last told.
//!
//! A device often sends pixels that are the same as before: a whole window for one new
//! line of text, or the whole screen on request. Each write is compared with the copy, so
//! only pixels that differ count as changed and are sent on to the client. Pure: no I/O,
//! no clock.

mod area;
mod changed;

pub use area::Area;
use changed::ChangedTiles;
pub use changed::TILE;

/// Each pixel is three bytes: red, green, blue.
pub const BYTES_PER_PIXEL: usize = 3;
/// The longest side of a screen: the largest size the Guacamole tunnel carries (K4
/// Section 4). A screen of this size on both sides takes 192 MiB.
pub const MAX_SIDE: usize = 8192;

/// Why a screen cannot have a size.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[error("a screen side must be from 1 to {MAX_SIDE} pixels")]
pub struct BadSize;

/// Why pixels were not read or written.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum AreaError {
    #[error("the area is not inside the screen")]
    OutsideScreen,
    #[error("the pixels do not fill the area")]
    WrongLength,
}

/// What changed on the screen since the last [`Screen::take_changes`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Changes {
    /// The screen got a new size (or is new): the client must set its own size first.
    pub resized: bool,
    /// The parts to send again. They do not overlap and lie inside the screen.
    pub areas: Vec<Area>,
}

/// One device's screen as the gateway knows it.
pub struct Screen {
    width: usize,
    height: usize,
    rgb: Vec<u8>,
    changed: ChangedTiles,
    resized: bool,
}

impl Screen {
    /// A black screen that the client has not seen.
    ///
    /// # Errors
    /// [`BadSize`] when a side is 0 or longer than [`MAX_SIDE`].
    pub fn new(width: usize, height: usize) -> Result<Self, BadSize> {
        let sides = 1..=MAX_SIDE;
        if !sides.contains(&width) || !sides.contains(&height) {
            return Err(BadSize);
        }
        Ok(Self {
            width,
            height,
            rgb: vec![0; width * height * BYTES_PER_PIXEL],
            changed: ChangedTiles::all(width, height),
            resized: true,
        })
    }

    #[must_use]
    pub const fn width(&self) -> usize {
        self.width
    }

    #[must_use]
    pub const fn height(&self) -> usize {
        self.height
    }

    /// The device changed its screen size: the copy starts again, black and unseen.
    ///
    /// # Errors
    /// [`BadSize`] as [`Screen::new`]; the screen is then unchanged.
    pub fn resize(&mut self, width: usize, height: usize) -> Result<(), BadSize> {
        *self = Self::new(width, height)?;
        Ok(())
    }

    /// Writes `rgb`, row by row, into `area`. Only pixels that differ from the copy count
    /// as changed.
    ///
    /// # Errors
    /// [`AreaError`] when `area` is not inside the screen or `rgb` does not fill it; the
    /// screen is then unchanged.
    pub fn write(&mut self, area: Area, rgb: &[u8]) -> Result<(), AreaError> {
        let row_bytes = self.row_bytes(area)?;
        if area.pixels().map(|pixels| pixels * BYTES_PER_PIXEL) != Some(rgb.len()) {
            return Err(AreaError::WrongLength);
        }
        if row_bytes == 0 {
            return Ok(());
        }
        for (row, new) in rgb.chunks_exact(row_bytes).enumerate() {
            let y = area.y + row;
            let start = self.offset(area.x, y);
            let old = &mut self.rgb[start..start + row_bytes];
            if old != new {
                self.changed.mark_differences((area.x, y), old, new);
                old.copy_from_slice(new);
            }
        }
        Ok(())
    }

    /// Copies the pixels of `from` to the same-sized area whose corner is (`x`, `y`), as a
    /// device asks when it scrolls or moves a window. The areas may overlap.
    ///
    /// # Errors
    /// [`AreaError::OutsideScreen`] when either area is not inside the screen.
    pub fn move_area(&mut self, from: Area, x: usize, y: usize) -> Result<(), AreaError> {
        let moved: Vec<u8> = self.rows(from)?.flatten().copied().collect();
        self.write(Area { x, y, ..from }, &moved)
    }

    /// The rows of `area`, top to bottom, each as its pixels' bytes.
    ///
    /// # Errors
    /// [`AreaError::OutsideScreen`] when `area` is not inside the screen.
    pub fn rows(&self, area: Area) -> Result<impl Iterator<Item = &[u8]>, AreaError> {
        let row_bytes = self.row_bytes(area)?;
        Ok((area.y..area.y + area.height).map(move |y| {
            let start = self.offset(area.x, y);
            &self.rgb[start..start + row_bytes]
        }))
    }

    /// What changed since the last call; after it nothing counts as changed.
    pub fn take_changes(&mut self) -> Changes {
        Changes {
            resized: std::mem::take(&mut self.resized),
            areas: self.changed.take(self.width, self.height),
        }
    }

    /// The bytes of one row of `area`, when `area` is inside the screen.
    fn row_bytes(&self, area: Area) -> Result<usize, AreaError> {
        if area.is_inside(self.width, self.height) {
            Ok(area.width * BYTES_PER_PIXEL)
        } else {
            Err(AreaError::OutsideScreen)
        }
    }

    /// Where pixel (`x`, `y`) starts in the copy.
    const fn offset(&self, x: usize, y: usize) -> usize {
        (y * self.width + x) * BYTES_PER_PIXEL
    }
}
