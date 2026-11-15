//! Finding a scroll: new pixels that are the old pixels of the same area, moved up or down
//! by some rows. A terminal that prints a line and a page that scrolls both send the whole
//! area again. Told as one move, it costs the client a few bytes and not a new image.
//!
//! The answer is only a hint. Whatever it says, the screen compares the new pixels with
//! its copy afterwards, so a wrong hint costs bytes and never a wrong picture.

use std::hash::{DefaultHasher, Hasher as _};

/// Areas with fewer rows are cheap to send as they are.
pub(super) const FEWEST_ROWS: usize = 48;
/// A scroll is taken when this share of the rows it leaves in view match: 3 of 4.
const MATCH_SHARE: (usize, usize) = (3, 4);
/// How many rows of the new pixels are looked up in the old ones.
const PROBES: usize = 3;

/// Rows that moved inside an area: the row at `from` and the `rows - 1` below it are now at
/// `to`. Both are counted from the area's first row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Scroll {
    pub(super) from: usize,
    pub(super) to: usize,
    pub(super) rows: usize,
}

/// One number for each row, equal for equal rows.
pub(super) fn row_marks<'a>(rows: impl Iterator<Item = &'a [u8]>) -> Vec<u64> {
    rows.map(|row| {
        let mut hasher = DefaultHasher::new();
        hasher.write(row);
        hasher.finish()
    })
    .collect()
}

/// The scroll that turns `old` into `new`, when most rows moved together. `None` when most
/// rows did not change, or no single move explains the new rows.
pub(super) fn scroll(old: &[u64], new: &[u64]) -> Option<Scroll> {
    let rows = new.len();
    if rows < FEWEST_ROWS || old.len() != rows {
        return None;
    }
    let unchanged = old.iter().zip(new).filter(|(old, new)| old == new).count();
    if unchanged * 2 > rows {
        return None;
    }
    (1..=PROBES)
        .map(|probe| probe * rows / (PROBES + 1))
        .flat_map(|at| {
            let wanted = new[at];
            (0..rows)
                .filter(move |&row| row != at && old[row] == wanted)
                .map(move |row| (row, at))
        })
        .take(PROBES * 4)
        .map(|(was, is)| moved_together(was, is, rows))
        .find(|scroll| matches(old, new, *scroll))
}

/// The scroll in which the row that was at `was` is now at `is`.
const fn moved_together(was: usize, is: usize, rows: usize) -> Scroll {
    if was > is {
        let by = was - is;
        Scroll {
            from: by,
            to: 0,
            rows: rows - by,
        }
    } else {
        let by = is - was;
        Scroll {
            from: 0,
            to: by,
            rows: rows - by,
        }
    }
}

/// Whether enough of the moved rows are equal in `old` and `new`.
fn matches(old: &[u64], new: &[u64], scroll: Scroll) -> bool {
    let was = &old[scroll.from..scroll.from + scroll.rows];
    let is = &new[scroll.to..scroll.to + scroll.rows];
    let equal = was.iter().zip(is).filter(|(was, is)| was == is).count();
    equal * MATCH_SHARE.1 >= scroll.rows * MATCH_SHARE.0
}
