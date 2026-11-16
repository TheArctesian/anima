//! Which changed areas go out as one image.
//!
//! Each image costs a fixed number of bytes: its `img` and `end` instructions and the
//! format's own header. Two areas that lie close together are cheaper as one image over
//! both, although that image also holds pixels that did not change.

use crate::screen::Area;

/// How many unchanged pixels one image may take in to save another image. About what the
/// fixed cost of an image buys: unchanged screen content compresses well, and two tiles of
/// it cost less than a second image's instructions and header.
const SPARE_PIXELS: usize = 2048;
/// How many of the latest joined areas a new area is compared with. The areas arrive in
/// row order, so near ones are recent ones; the limit keeps the work linear.
const LOOK_BACK: usize = 32;

/// The pixels in `area`.
pub(super) const fn pixels(area: Area) -> usize {
    area.width * area.height
}

/// `areas` with near neighbours joined into one area over both. The result covers every
/// pixel of `areas`; joined areas may overlap each other.
pub(super) fn joined(areas: &[Area]) -> Vec<Area> {
    let mut joined: Vec<Area> = Vec::new();
    for &area in areas {
        let near = joined
            .iter_mut()
            .rev()
            .take(LOOK_BACK)
            .find(|other| spare(**other, area) <= SPARE_PIXELS);
        match near {
            Some(other) => *other = over_both(*other, area),
            None => joined.push(area),
        }
    }
    joined
}

/// The pixels in the area over both that are in neither.
const fn spare(a: Area, b: Area) -> usize {
    pixels(over_both(a, b)).saturating_sub(pixels(a) + pixels(b))
}

/// The smallest area that holds both.
const fn over_both(a: Area, b: Area) -> Area {
    let x = min(a.x, b.x);
    let y = min(a.y, b.y);
    Area {
        x,
        y,
        width: max(a.x + a.width, b.x + b.width) - x,
        height: max(a.y + a.height, b.y + b.height) - y,
    }
}

const fn min(a: usize, b: usize) -> usize {
    if a < b { a } else { b }
}

const fn max(a: usize, b: usize) -> usize {
    if a > b { a } else { b }
}
