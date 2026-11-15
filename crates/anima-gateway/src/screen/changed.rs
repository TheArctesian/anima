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
    /// For each screen row, whether a pixel in it changed: an area is cut to the rows that
    /// did, so one new line of text is not sent with the unchanged line above it.
    rows: Vec<bool>,
}

impl ChangedTiles {
    /// The grid of a `width` by `height` screen, with every tile changed: the client has
    /// seen none of it.
    pub(super) fn all(width: usize, height: usize) -> Self {
        let columns = width.div_ceil(TILE);
        Self {
            columns,
            tiles: vec![true; columns * height.div_ceil(TILE)],
            rows: vec![true; height],
        }
    }

    /// Marks each tile of screen row `y` in which `old` and `new` differ. Both hold the
    /// row's pixels from column `x` on, and have the same length.
    pub(super) fn mark_differences(&mut self, (x, y): (usize, usize), old: &[u8], new: &[u8]) {
        self.rows[y] = true;
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

    /// Whether a tile that `area` touches is changed.
    pub(super) fn any_in(&self, area: Area) -> bool {
        if area.width == 0 || area.height == 0 {
            return false;
        }
        let columns = area.x / TILE..=(area.x + area.width - 1) / TILE;
        (area.y / TILE..=(area.y + area.height - 1) / TILE).any(|row| {
            columns
                .clone()
                .any(|column| self.tiles[row * self.columns + column])
        })
    }

    /// The changed tiles as areas that do not overlap, clipped to the `width` by `height`
    /// screen, ordered by row and then by column; then no tile is changed.
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
        finished.sort_unstable_by_key(|tiles| (tiles.y, tiles.x));
        let areas = finished
            .into_iter()
            .map(|tiles| self.changed_rows_of(in_pixels(tiles, width, height)))
            .collect();
        self.rows.fill(false);
        areas
    }
}

impl ChangedTiles {
    /// `area` without the unchanged rows at its top and bottom. An area of changed tiles
    /// has a changed row.
    fn changed_rows_of(&self, area: Area) -> Area {
        let rows = &self.rows[area.y..area.y + area.height];
        let first = rows.iter().position(|changed| *changed).unwrap_or(0);
        let last = rows
            .iter()
            .rposition(|changed| *changed)
            .unwrap_or(area.height - 1);
        Area {
            y: area.y + first,
            height: last + 1 - first,
            ..area
        }
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
