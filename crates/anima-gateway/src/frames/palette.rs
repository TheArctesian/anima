//! Pixels as numbers into a short list of colours. Text and window content have few
//! colours, and a PNG that stores one to eight bits for a pixel, not twenty-four, is
//! several times smaller.

use std::collections::HashMap;

use crate::screen::BYTES_PER_PIXEL;

/// The most colours a PNG palette holds.
const MOST_COLOURS: usize = 256;

type Colour = [u8; BYTES_PER_PIXEL];

/// An area's pixels by palette number.
pub(super) struct Indexed {
    /// The palette: each colour's red, green and blue, in number order.
    pub(super) colours: Vec<u8>,
    /// The bits for one pixel: 1, 2, 4 or 8.
    pub(super) depth: u8,
    /// The rows, each packed from the highest bit and filled up to a whole byte.
    pub(super) rows: Vec<u8>,
}

impl Indexed {
    /// The pixels of `rgb`, in rows of `width`, by palette number; `None` when they have
    /// more colours than a palette holds.
    pub(super) fn of(rgb: &[u8], width: usize) -> Option<Self> {
        let (pixels, _) = rgb.as_chunks::<BYTES_PER_PIXEL>();
        let mut numbers: HashMap<Colour, u8> = HashMap::new();
        let mut colours = Vec::new();
        let mut numbered = Vec::with_capacity(pixels.len());
        for pixel in pixels {
            let number = if let Some(number) = numbers.get(pixel) {
                *number
            } else {
                let number = u8::try_from(numbers.len()).ok()?;
                numbers.insert(*pixel, number);
                colours.extend_from_slice(pixel);
                number
            };
            numbered.push(number);
        }
        debug_assert!(numbers.len() <= MOST_COLOURS);
        let depth = match numbers.len() {
            0..=2 => 1,
            3..=4 => 2,
            5..=16 => 4,
            _ => 8,
        };
        let rows = numbered
            .chunks(width.max(1))
            .flat_map(|row| packed(row, depth))
            .collect();
        Some(Self {
            colours,
            depth,
            rows,
        })
    }
}

/// One row of palette numbers, `depth` bits each, from the highest bit of each byte.
fn packed(row: &[u8], depth: u8) -> Vec<u8> {
    let in_a_byte = usize::from(8 / depth);
    row.chunks(in_a_byte)
        .map(|numbers| {
            numbers.iter().zip(1u8..).fold(0, |byte, (number, place)| {
                byte | (number << (8 - depth * place))
            })
        })
        .collect()
}
