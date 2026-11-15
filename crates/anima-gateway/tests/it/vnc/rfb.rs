//! A VNC device for the tests: the bytes a server sends after sign-in (RFC 6143).

use anima_gateway::screen::{Area, BYTES_PER_PIXEL};
use flate2::{Compress, Compression, FlushCompress};

fn two(value: usize) -> [u8; 2] {
    u16::try_from(value)
        .expect("fits the protocol")
        .to_be_bytes()
}

fn four(value: usize) -> [u8; 4] {
    u32::try_from(value)
        .expect("fits the protocol")
        .to_be_bytes()
}

/// A rectangle header for `area` in `encoding`.
pub(crate) fn header(area: Area, encoding: i32) -> Vec<u8> {
    let mut bytes = [area.x, area.y, area.width, area.height].map(two).concat();
    bytes.extend_from_slice(&encoding.to_be_bytes());
    bytes
}

/// A `FramebufferUpdate` of `rects`.
pub(crate) fn update(rects: &[Vec<u8>]) -> Vec<u8> {
    let mut bytes = vec![0, 0];
    bytes.extend_from_slice(&two(rects.len()));
    bytes.extend(rects.iter().flatten());
    bytes
}

/// RGB pixels as the gateway asks for them: blue, green, red and an unused byte.
fn bgrx(rgb: &[u8]) -> Vec<u8> {
    let (pixels, _) = rgb.as_chunks::<BYTES_PER_PIXEL>();
    pixels
        .iter()
        .flat_map(|pixel| [pixel[2], pixel[1], pixel[0], 0])
        .collect()
}

pub(crate) fn raw(area: Area, rgb: &[u8]) -> Vec<u8> {
    [header(area, 0), bgrx(rgb)].concat()
}

pub(crate) fn copy_rect(area: Area, from: (usize, usize)) -> Vec<u8> {
    [header(area, 1), two(from.0).to_vec(), two(from.1).to_vec()].concat()
}

pub(crate) fn desktop_size(width: usize, height: usize) -> Vec<u8> {
    header(
        Area {
            x: 0,
            y: 0,
            width,
            height,
        },
        -223,
    )
}

/// `ServerCutText` with `text` in Latin-1.
pub(crate) fn cut_text(text: &[u8]) -> Vec<u8> {
    [vec![3, 0, 0, 0], four(text.len()).to_vec(), text.to_vec()].concat()
}

/// One of a session's zlib streams, on the device's side.
pub(crate) struct Deflater(Compress);

impl Default for Deflater {
    fn default() -> Self {
        Self(Compress::new(Compression::default(), true))
    }
}

impl Deflater {
    /// `bytes` through the stream, with its length in front.
    fn packed(&mut self, bytes: &[u8]) -> Vec<u8> {
        let mut out = Vec::with_capacity(bytes.len() + 1024);
        self.0
            .compress_vec(bytes, &mut out, FlushCompress::Sync)
            .expect("zlib");
        [four(out.len()).to_vec(), out].concat()
    }

    pub(crate) fn zlib(&mut self, area: Area, rgb: &[u8]) -> Vec<u8> {
        [header(area, 6), self.packed(&bgrx(rgb))].concat()
    }

    /// `area` as ZRLE, every tile in the form `kind`.
    pub(crate) fn zrle(&mut self, area: Area, rgb: &[u8], kind: Tile) -> Vec<u8> {
        let mut tiles = Vec::new();
        for top in (0..area.height).step_by(64) {
            for left in (0..area.width).step_by(64) {
                let width = 64.min(area.width - left);
                let pixels = tile_pixels(
                    rgb,
                    area.width,
                    (left, top),
                    (width, 64.min(area.height - top)),
                );
                tiles.extend(tile(&pixels, width, kind));
            }
        }
        [header(area, 16), self.packed(&tiles)].concat()
    }

    /// A rectangle in `encoding` whose zlib data is `bytes` packed.
    pub(crate) fn packed_as(&mut self, area: Area, encoding: i32, bytes: &[u8]) -> Vec<u8> {
        [header(area, encoding), self.packed(bytes)].concat()
    }
}

/// The blue-green-red pixels of the tile at `corner` with `size`, from `rgb` in rows of `row`.
fn tile_pixels(
    rgb: &[u8],
    row: usize,
    corner: (usize, usize),
    size: (usize, usize),
) -> Vec<[u8; 3]> {
    (0..size.0 * size.1)
        .map(|pixel| {
            let (x, y) = (corner.0 + pixel % size.0, corner.1 + pixel / size.0);
            let at = (y * row + x) * BYTES_PER_PIXEL;
            [rgb[at + 2], rgb[at + 1], rgb[at]]
        })
        .collect()
}

/// The forms of a ZRLE tile.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Tile {
    Raw,
    OneColour,
    /// A palette, and each pixel packed into as few bits as its size needs.
    Packed,
    Runs,
    PaletteRuns,
}

/// A run length as ZRLE writes it: bytes that sum to the length less 1.
fn run_length(length: usize) -> Vec<u8> {
    let mut left = length - 1;
    let mut bytes = Vec::new();
    while left >= 255 {
        bytes.push(255);
        left -= 255;
    }
    bytes.push(u8::try_from(left).expect("under 255"));
    bytes
}

/// The colours of `pixels`, in the order they first appear.
fn palette(pixels: &[[u8; 3]]) -> Vec<[u8; 3]> {
    let mut colours = Vec::new();
    for pixel in pixels {
        if !colours.contains(pixel) {
            colours.push(*pixel);
        }
    }
    colours
}

/// Runs of equal pixels: each colour and how often it repeats.
fn runs(pixels: &[[u8; 3]]) -> Vec<([u8; 3], usize)> {
    let mut runs: Vec<([u8; 3], usize)> = Vec::new();
    for pixel in pixels {
        match runs.last_mut() {
            Some((colour, length)) if colour == pixel => *length += 1,
            _ => runs.push((*pixel, 1)),
        }
    }
    runs
}

/// One tile of blue-green-red `pixels`, `width` wide, in the form `kind`.
fn tile(pixels: &[[u8; 3]], width: usize, kind: Tile) -> Vec<u8> {
    let colours = palette(pixels);
    let number = |pixel: &[u8; 3]| {
        colours
            .iter()
            .position(|colour| colour == pixel)
            .expect("in the palette")
    };
    // A palette of one colour is not a form of its own: such a tile is one colour.
    let kind = match kind {
        Tile::Packed | Tile::PaletteRuns if colours.len() == 1 => Tile::OneColour,
        other => other,
    };
    match kind {
        Tile::Raw => [vec![0], pixels.concat()].concat(),
        Tile::OneColour => [vec![1], pixels[0].to_vec()].concat(),
        Tile::Packed => {
            let bits = match colours.len() {
                2 => 1,
                3..=4 => 2,
                _ => 4,
            };
            let mut bytes = vec![u8::try_from(colours.len()).expect("a small palette")];
            bytes.extend(colours.concat());
            for row in pixels.chunks(width) {
                let mut packed = vec![0u8; (row.len() * bits).div_ceil(8)];
                for (x, pixel) in row.iter().enumerate() {
                    let shift = 8 - bits - (x * bits) % 8;
                    packed[x * bits / 8] |= u8::try_from(number(pixel)).expect("small") << shift;
                }
                bytes.extend(packed);
            }
            bytes
        }
        Tile::Runs => {
            let body = runs(pixels)
                .into_iter()
                .flat_map(|(colour, length)| [colour.to_vec(), run_length(length)].concat());
            [vec![128], body.collect()].concat()
        }
        Tile::PaletteRuns => {
            let mut bytes = vec![128 + u8::try_from(colours.len()).expect("a small palette")];
            bytes.extend(colours.concat());
            for (colour, length) in runs(pixels) {
                let index = u8::try_from(number(&colour)).expect("small");
                if length == 1 {
                    bytes.push(index);
                } else {
                    bytes.push(index | 0x80);
                    bytes.extend(run_length(length));
                }
            }
            bytes
        }
    }
}
