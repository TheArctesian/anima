//! The cheapest form of one changed area: a colour, a PNG with a palette, a JPEG or a
//! full-colour PNG.

use std::collections::HashSet;

use super::{Formats, FrameError, palette::Indexed};
use crate::screen::{Area, BYTES_PER_PIXEL};

/// JPEG quality: text edges stay readable and a photograph is about a third of its PNG.
const JPEG_QUALITY: u8 = 80;
/// Below this many pixels a JPEG's header outweighs what it saves.
const SMALLEST_JPEG_PIXELS: usize = 4096;
/// How many pixels are looked at to tell a photograph from flat content.
const SAMPLES: usize = 512;

/// How an area goes to the client.
pub(super) enum Picture {
    /// Every pixel has this colour.
    OneColour([u8; BYTES_PER_PIXEL]),
    Image {
        mimetype: &'static str,
        bytes: Vec<u8>,
    },
}

impl Picture {
    /// The form of `area`, whose pixels are `rgb`, for a client that draws `formats`.
    pub(super) fn of(rgb: &[u8], area: Area, formats: Formats) -> Result<Self, FrameError> {
        let (pixels, _) = rgb.as_chunks::<BYTES_PER_PIXEL>();
        let first = pixels.first().copied().unwrap_or_default();
        if pixels.iter().all(|pixel| *pixel == first) {
            return Ok(Self::OneColour(first));
        }
        if let Some(indexed) = Indexed::of(rgb, area.width) {
            return Ok(Self::Image {
                mimetype: "image/png",
                bytes: indexed_png(&indexed, area)?,
            });
        }
        if formats.jpeg && is_photograph(rgb) {
            return Ok(Self::Image {
                mimetype: "image/jpeg",
                bytes: jpeg(rgb, area)?,
            });
        }
        Ok(Self::Image {
            mimetype: "image/png",
            bytes: png(rgb, area)?,
        })
    }
}

/// Whether most sampled pixels have a colour of their own, as in a photograph or a video.
/// Text and window content repeat a few colours, and PNG keeps them exact and small.
fn is_photograph(rgb: &[u8]) -> bool {
    let pixels = rgb.len() / BYTES_PER_PIXEL;
    if pixels < SMALLEST_JPEG_PIXELS {
        return false;
    }
    let (all, _) = rgb.as_chunks::<BYTES_PER_PIXEL>();
    let sampled: Vec<&[u8; BYTES_PER_PIXEL]> = all.iter().step_by(pixels / SAMPLES).collect();
    let colours: HashSet<&[u8; BYTES_PER_PIXEL]> = sampled.iter().copied().collect();
    colours.len() * 2 > sampled.len()
}

/// A screen side as PNG wants it. A side is at most 8192, so it fits; one that did not
/// would fail to encode.
fn side(side: usize) -> u32 {
    u32::try_from(side).unwrap_or(u32::MAX)
}

fn png(rgb: &[u8], area: Area) -> Result<Vec<u8>, png::EncodingError> {
    let mut bytes = Vec::new();
    let mut encoder = png::Encoder::new(&mut bytes, side(area.width), side(area.height));
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.set_compression(png::Compression::Fast);
    let mut writer = encoder.write_header()?;
    writer.write_image_data(rgb)?;
    writer.finish()?;
    Ok(bytes)
}

/// A PNG with a palette. Its pixel data is small, so the stronger compression is cheap.
fn indexed_png(indexed: &Indexed, area: Area) -> Result<Vec<u8>, png::EncodingError> {
    let mut bytes = Vec::new();
    let mut encoder = png::Encoder::new(&mut bytes, side(area.width), side(area.height));
    encoder.set_color(png::ColorType::Indexed);
    encoder.set_depth(match indexed.depth {
        1 => png::BitDepth::One,
        2 => png::BitDepth::Two,
        4 => png::BitDepth::Four,
        _ => png::BitDepth::Eight,
    });
    encoder.set_palette(indexed.colours.as_slice());
    encoder.set_compression(png::Compression::Balanced);
    let mut writer = encoder.write_header()?;
    writer.write_image_data(&indexed.rows)?;
    writer.finish()?;
    Ok(bytes)
}

fn jpeg(rgb: &[u8], area: Area) -> Result<Vec<u8>, jpeg_encoder::EncodingError> {
    let mut bytes = Vec::new();
    let side = |side: usize| u16::try_from(side).unwrap_or(u16::MAX);
    let encoder = jpeg_encoder::Encoder::new(&mut bytes, JPEG_QUALITY);
    encoder.encode(
        rgb,
        side(area.width),
        side(area.height),
        jpeg_encoder::ColorType::Rgb,
    )?;
    Ok(bytes)
}
