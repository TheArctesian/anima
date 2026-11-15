//! The gateway's copy of a device's screen, and what changed on it since the client was
//! last told.
//!
//! A device often sends pixels that are the same as before: a whole window for one new
//! line of text, or the whole screen on request. Each write is compared with the copy, so
//! only pixels that differ count as changed and are sent on to the client. Pure: no I/O,
//! no clock.

mod area;
mod changed;
mod scrolling;

pub use area::Area;
use changed::ChangedTiles;
pub use changed::TILE;

/// Each pixel is three bytes: red, green, blue.
pub const BYTES_PER_PIXEL: usize = 3;
/// The longest side of a screen: the largest size the Guacamole tunnel carries (K4
/// Section 4). A screen of this size on both sides takes 192 MiB.
pub const MAX_SIDE: usize = 8192;
/// The most moves told to the client between two [`Screen::take_changes`], so in one
/// frame. A move costs the client about 50 bytes and the device 12, so each frame can cost
/// about 3 kB for moves and no more; further moves are done as writes. How many frames go
/// out is the caller's limit (`frames::Pacing`).
pub const MOST_MOVES: usize = 64;

/// Why a screen cannot have a size.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[error("a screen side must be from 1 to {MAX_SIDE} pixels")]
pub struct BadSize;

/// An area that is not inside the screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[error("the area is not inside the screen")]
pub struct OutsideScreen;

/// Why pixels were not written.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum WriteError {
    #[error(transparent)]
    OutsideScreen(#[from] OutsideScreen),
    #[error("the pixels do not fill the area")]
    WrongLength,
}

/// Pixels that moved on the screen: the client copies `from`, as it shows it, to the
/// same-sized area whose corner is (`x`, `y`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Moved {
    pub from: Area,
    pub x: usize,
    pub y: usize,
}

/// What changed on the screen since the last [`Screen::take_changes`]. A client that has
/// the screen as it was then gets the screen as it is now by doing `moved` in order and
/// then drawing `areas`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Changes {
    /// The screen got a new size (or is new): the client must set its own size first.
    pub resized: bool,
    /// The moves to do first, in this order.
    pub moved: Vec<Moved>,
    /// The parts to send again. They do not overlap, lie inside the screen and are ordered
    /// by row, then by column.
    pub areas: Vec<Area>,
}

/// One device's screen as the gateway knows it.
pub struct Screen {
    width: usize,
    height: usize,
    rgb: Vec<u8>,
    changed: ChangedTiles,
    resized: bool,
    moved: Vec<Moved>,
    /// The pixels of an area while it moves; kept so a scroll does not allocate each time.
    moving: Vec<u8>,
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
            moved: Vec::new(),
            moving: Vec::new(),
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

    /// The device announced a screen size. A new size starts the copy again, black and
    /// unseen; the size the screen already has changes nothing.
    ///
    /// # Errors
    /// [`BadSize`] as [`Screen::new`]; the screen is then unchanged.
    pub fn resize(&mut self, width: usize, height: usize) -> Result<(), BadSize> {
        if (width, height) == (self.width, self.height) {
            return Ok(());
        }
        let sides = 1..=MAX_SIDE;
        if !sides.contains(&width) || !sides.contains(&height) {
            return Err(BadSize);
        }
        // Freed first, so the old and the new copy are never both in memory.
        self.rgb = Vec::new();
        *self = Self::new(width, height)?;
        Ok(())
    }

    /// Writes `rgb`, row by row, into `area`. Only pixels that differ from the copy count
    /// as changed.
    ///
    /// # Errors
    /// [`WriteError`] when `area` is not inside the screen or `rgb` does not fill it; the
    /// screen is then unchanged.
    pub fn write(&mut self, area: Area, rgb: &[u8]) -> Result<(), WriteError> {
        let row_bytes = self.row_bytes(area)?;
        if area.pixels().map(|pixels| pixels * BYTES_PER_PIXEL) != Some(rgb.len()) {
            return Err(WriteError::WrongLength);
        }
        if row_bytes == 0 {
            return Ok(());
        }
        self.follow_scroll(area, rgb.chunks_exact(row_bytes));
        for (row, new) in rgb.chunks_exact(row_bytes).enumerate() {
            let y = area.y + row;
            let start = self.offset(area.x, y);
            let old = &mut self.rgb[start..start + row_bytes];
            // Most rows a device repeats are equal as a whole: one comparison, no tile work.
            if old != new {
                self.changed.mark_differences((area.x, y), old, new);
                old.copy_from_slice(new);
            }
        }
        Ok(())
    }

    /// Copies the pixels of `from` to the same-sized area whose corner is (`x`, `y`), as a
    /// device asks when it scrolls or moves a window. The areas may overlap. The work is in
    /// proportion to the area, so the caller limits how many moves a device may ask for.
    ///
    /// # Errors
    /// [`OutsideScreen`] when either area is not inside the screen; the screen is then
    /// unchanged.
    pub fn move_area(&mut self, from: Area, x: usize, y: usize) -> Result<(), OutsideScreen> {
        let to = Area { x, y, ..from };
        self.row_bytes(to)?;
        self.row_bytes(from)?;
        if to == from || from.width == 0 || from.height == 0 {
            return Ok(());
        }
        let mut moving = std::mem::take(&mut self.moving);
        moving.clear();
        for row in self.rows(from)? {
            moving.extend_from_slice(row);
        }
        let told = self.moved.len() < MOST_MOVES && !self.changed.any_in(from);
        let written = if told {
            self.put(to, &moving);
            self.moved.push(Moved { from, x, y });
            Ok(())
        } else {
            // The client does not have these pixels yet, or has been told enough moves.
            self.write(to, &moving).map_err(|_| OutsideScreen)
        };
        self.moving = moving;
        written
    }

    /// When the `new` rows of `area` are its old rows scrolled, and the client has those
    /// old rows, moves them in the copy and notes the move for the client. The write that
    /// follows then finds only the rows the scroll brought in.
    fn follow_scroll<'a>(&mut self, area: Area, new: impl Iterator<Item = &'a [u8]>) {
        let Ok(old) = self.rows(area) else { return };
        let old = scrolling::row_marks(old);
        let Some(scroll) = scrolling::scroll(&old, &scrolling::row_marks(new)) else {
            return;
        };
        let from = Area {
            y: area.y + scroll.from,
            height: scroll.rows,
            ..area
        };
        // Inside the screen, as `area` is. A source the client lacks is written, not moved.
        let _ = self.move_area(from, area.x, area.y + scroll.to);
    }

    /// Puts `rgb` into `to` without counting it as changed. `to` is inside the screen and
    /// `rgb` fills it.
    fn put(&mut self, to: Area, rgb: &[u8]) {
        let row_bytes = to.width * BYTES_PER_PIXEL;
        if row_bytes == 0 {
            return;
        }
        for (row, pixels) in rgb.chunks_exact(row_bytes).enumerate() {
            let start = self.offset(to.x, to.y + row);
            self.rgb[start..start + row_bytes].copy_from_slice(pixels);
        }
    }

    /// The rows of `area`, top to bottom, each as its pixels' bytes.
    ///
    /// # Errors
    /// [`OutsideScreen`] when `area` is not inside the screen.
    pub fn rows(&self, area: Area) -> Result<impl Iterator<Item = &[u8]>, OutsideScreen> {
        let row_bytes = self.row_bytes(area)?;
        Ok((area.y..area.y + area.height).map(move |y| {
            let start = self.offset(area.x, y);
            &self.rgb[start..start + row_bytes]
        }))
    }

    /// Whether [`Screen::take_changes`] would give something to tell the client.
    #[must_use]
    pub fn has_changes(&self) -> bool {
        self.resized || !self.moved.is_empty() || self.changed.any()
    }

    /// What changed since the last call; after it nothing counts as changed.
    pub fn take_changes(&mut self) -> Changes {
        Changes {
            resized: std::mem::take(&mut self.resized),
            moved: std::mem::take(&mut self.moved),
            areas: self.changed.take(self.width, self.height),
        }
    }

    /// The bytes of one row of `area`, when `area` is inside the screen.
    fn row_bytes(&self, area: Area) -> Result<usize, OutsideScreen> {
        if area.is_inside(self.width, self.height) {
            Ok(area.width * BYTES_PER_PIXEL)
        } else {
            Err(OutsideScreen)
        }
    }

    /// Where pixel (`x`, `y`) starts in the copy.
    const fn offset(&self, x: usize, y: usize) -> usize {
        (y * self.width + x) * BYTES_PER_PIXEL
    }
}
