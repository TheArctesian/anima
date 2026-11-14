//! Which parts of the screen changed since the client was last told: a grid of tiles, each
//! changed or not. A tile is small enough that a blinking cursor costs one small image, and
//! large enough that the grid of a 8192 x 8192 screen is 64 KiB.

use super::{BYTES_PER_PIXEL, area::Area};

/// The side of one tile, in pixels.
pub const TILE: usize = 32;

/// The changed tiles of one screen.
pub(super) struct ChangedTiles {
    columns: usize,
    tiles: Vec<bool>,
}

impl ChangedTiles {
    /// The grid of a `width` by `height` screen, with every tile changed: the client has
    /// seen none of it.
    pub(super) fn all(width: usize, height: usize) -> Self {
        let columns = width.div_ceil(TILE);
        Self {
            columns,
            tiles: vec![true; columns * height.div_ceil(TILE)],
        }
    }

    /// Marks each tile of screen row `y` in which `old` and `new` differ. Both hold the
    /// row's pixels from column `x` on, and have the same length.
    pub(super) fn mark_differences(&mut self, (x, y): (usize, usize), old: &[u8], new: &[u8]) {
        let row = y / TILE * self.columns;
        let (mut column, mut at) = (x, 0);
        while at < new.len() {
            let tile_end = (column / TILE + 1) * TILE;
            let end = new.len().min(at + (tile_end - column) * BYTES_PER_PIXEL);
            if old[at..end] != new[at..end] {
                self.tiles[row + column / TILE] = true;
            }
            (column, at) = (tile_end, end);
        }
    }

    /// The changed tiles as areas that do not overlap, clipped to the `width` by `height`
    /// screen; then no tile is changed.
    pub(super) fn take(&mut self, width: usize, height: usize) -> Vec<Area> {
        let mut finished = Vec::new();
        // Areas, in tiles, that reach the row above the one being read.
        let mut open: Vec<Area> = Vec::new();
        for (y, row) in self.tiles.chunks(self.columns).enumerate() {
            let mut reaching = Vec::new();
            for (x, width) in runs(row) {
                let above = open
                    .iter()
                    .position(|area| area.x == x && area.width == width);
                reaching.push(match above {
                    Some(index) => {
                        let area = open.swap_remove(index);
                        Area {
                            height: area.height + 1,
                            ..area
                        }
                    }
                    None => Area {
                        x,
                        y,
                        width,
                        height: 1,
                    },
                });
            }
            finished.append(&mut open);
            open = reaching;
        }
        finished.append(&mut open);
        self.tiles.fill(false);
        finished
            .into_iter()
            .map(|tiles| in_pixels(tiles, width, height))
            .collect()
    }
}

/// The runs of changed tiles in one grid row, each as its first column and its length.
fn runs(row: &[bool]) -> Vec<(usize, usize)> {
    let mut runs = Vec::new();
    let mut start = None;
    for (column, changed) in row.iter().copied().chain([false]).enumerate() {
        match (changed, start) {
            (true, None) => start = Some(column),
            (false, Some(first)) => {
                runs.push((first, column - first));
                start = None;
            }
            _ => {}
        }
    }
    runs
}

/// An area of tiles as an area of pixels, cut at the screen's right and bottom edges.
fn in_pixels(tiles: Area, width: usize, height: usize) -> Area {
    let (x, y) = (tiles.x * TILE, tiles.y * TILE);
    Area {
        x,
        y,
        width: (tiles.width * TILE).min(width - x),
        height: (tiles.height * TILE).min(height - y),
    }
}
