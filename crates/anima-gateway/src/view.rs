//! What the client is shown of the device's screen: the screen itself, or a smaller picture
//! of it when it does not fit the client's view.
//!
//! A Mac with several displays has one screen of 14 megapixels. A client window shows a
//! tenth of that, so the gateway makes the picture smaller before it encodes it: fewer
//! bytes for the link and less work for both sides. The smaller picture has its own copy
//! and its own change detection, so only the parts that changed are made again. Pure.

use crate::screen::{Area, BYTES_PER_PIXEL, Changes, MAX_SIDE, Screen};

/// The device's screen as the client sees it.
pub struct View {
    /// The client's view, in pixels.
    client: (usize, usize),
    /// The smaller picture; `None` while the device's screen fits the client's view.
    shown: Option<Screen>,
}

/// `a * b / c`, rounded down.
const fn scaled(a: usize, b: usize, c: usize) -> usize {
    a * b / c
}

/// The size of the picture of a `device` screen that fits `client` and keeps its shape;
/// `None` when the screen fits as it is.
fn fitted(device: (usize, usize), client: (usize, usize)) -> Option<(usize, usize)> {
    if device.0 <= client.0 && device.1 <= client.1 {
        return None;
    }
    // The side that needs the smaller factor decides.
    let by_width = device.1 * client.0 <= device.0 * client.1;
    let (width, height) = if by_width {
        (client.0, scaled(device.1, client.0, device.0))
    } else {
        (scaled(device.0, client.1, device.1), client.1)
    };
    Some((width.max(1), height.max(1)))
}

impl View {
    /// The view of `device` for a client whose view is `client` pixels.
    #[must_use]
    pub fn new(device: &Screen, client: (usize, usize)) -> Self {
        let mut view = Self {
            client: (client.0.clamp(1, MAX_SIDE), client.1.clamp(1, MAX_SIDE)),
            shown: None,
        };
        view.start_again(device);
        view
    }

    /// The client's view has a new size. When that changes the size of what it is shown,
    /// the picture starts again with the next [`View::take_changes`].
    pub fn set_client(&mut self, client: (usize, usize), device: &mut Screen) {
        let real = (device.width(), device.height());
        let before = fitted(real, self.client);
        self.client = (client.0.clamp(1, MAX_SIDE), client.1.clamp(1, MAX_SIDE));
        if fitted(real, self.client) != before {
            device.forget_what_was_told();
        }
    }

    /// The size of what the client is shown.
    #[must_use]
    pub fn size(&self, device: &Screen) -> (usize, usize) {
        self.shown
            .as_ref()
            .map_or((device.width(), device.height()), |shown| {
                (shown.width(), shown.height())
            })
    }

    /// The device pixel under the middle of the client's pixel (`x`, `y`), kept inside the
    /// screen.
    #[must_use]
    pub fn to_device(&self, (x, y): (usize, usize), device: &Screen) -> (usize, usize) {
        let (width, height) = self.size(device);
        // The device pixel under the middle of the client's pixel.
        let on = |at: usize, shown: usize, real: usize| {
            scaled(2 * at.min(shown - 1) + 1, real, 2 * shown).min(real - 1)
        };
        (on(x, width, device.width()), on(y, height, device.height()))
    }

    /// Whether the client would be told something now.
    #[must_use]
    pub fn has_changes(&self, device: &Screen) -> bool {
        device.has_changes() || self.shown.as_ref().is_some_and(Screen::has_changes)
    }

    /// Takes what changed on `device` and gives the screen to draw from with its changes:
    /// the device's own, or the smaller picture with the changed parts made again.
    pub fn take_changes<'a>(&'a mut self, device: &'a mut Screen) -> (&'a Screen, Changes) {
        let changes = device.take_changes();
        if changes.resized {
            self.start_again(device);
        }
        let Some(shown) = &mut self.shown else {
            return (device, changes);
        };
        if !changes.resized {
            let moved = changes.moved.iter().map(|moved| Area {
                x: moved.x,
                y: moved.y,
                ..moved.from
            });
            for area in changes.areas.iter().copied().chain(moved) {
                shrink(device, area, shown);
            }
        }
        let changes = shown.take_changes();
        (shown, changes)
    }

    /// Makes the smaller picture anew from the whole of `device`, or drops it when the
    /// screen fits.
    fn start_again(&mut self, device: &Screen) {
        let all = Area {
            x: 0,
            y: 0,
            width: device.width(),
            height: device.height(),
        };
        self.shown = fitted((all.width, all.height), self.client).and_then(|(width, height)| {
            let mut shown = Screen::new(width, height).ok()?;
            shrink(device, all, &mut shown);
            Some(shown)
        });
    }
}

/// Makes the part of `shown` that shows `area` of `device` again: each of its pixels is
/// the mean of the device pixels it covers.
fn shrink(device: &Screen, area: Area, shown: &mut Screen) {
    let (real_width, real_height) = (device.width(), device.height());
    let (width, height) = (shown.width(), shown.height());
    // The pixels of `shown` that hold any pixel of `area`.
    let left = scaled(area.x, width, real_width);
    let top = scaled(area.y, height, real_height);
    let right = (scaled(area.x + area.width, width, real_width) + 1).min(width);
    let bottom = (scaled(area.y + area.height, height, real_height) + 1).min(height);
    let part = Area {
        x: left,
        y: top,
        width: right - left,
        height: bottom - top,
    };
    let mut pixels = Vec::with_capacity(part.width * part.height * BYTES_PER_PIXEL);
    for y in top..bottom {
        // The device rows this row covers: at least one.
        let first_row = scaled(y, real_height, height);
        let rows = (scaled(y + 1, real_height, height).max(first_row + 1) - first_row)
            .min(real_height - first_row);
        let covered = Area {
            x: 0,
            y: first_row,
            width: real_width,
            height: rows,
        };
        let Ok(covered) = device.rows(covered) else {
            return;
        };
        let covered: Vec<&[u8]> = covered.collect();
        for x in left..right {
            let first = scaled(x, real_width, width);
            let columns =
                (scaled(x + 1, real_width, width).max(first + 1) - first).min(real_width - first);
            pixels.extend_from_slice(&mean(&covered, first, columns));
        }
    }
    // Inside `shown`, and filled: this cannot fail.
    let _ = shown.write(part, &pixels);
}

/// The mean colour of `columns` pixels from `first` in each of `rows`.
fn mean(rows: &[&[u8]], first: usize, columns: usize) -> [u8; BYTES_PER_PIXEL] {
    let mut sums = [0usize; BYTES_PER_PIXEL];
    for row in rows {
        let (pixels, _) = row[first * BYTES_PER_PIXEL..(first + columns) * BYTES_PER_PIXEL]
            .as_chunks::<BYTES_PER_PIXEL>();
        for pixel in pixels {
            for (sum, part) in sums.iter_mut().zip(pixel) {
                *sum += usize::from(*part);
            }
        }
    }
    let count = (rows.len() * columns).max(1);
    sums.map(|sum| u8::try_from(sum / count).unwrap_or(u8::MAX))
}
