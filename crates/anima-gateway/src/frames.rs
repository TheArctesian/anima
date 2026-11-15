//! One frame for the client: the Guacamole instructions that draw what changed on the
//! screen, ending with `sync` (guacamole-server `src/libguac/protocol.c`).
//!
//! The bytes a frame costs decide how a session feels over a slow link, so each changed
//! part goes out in its cheapest form: moved pixels as `copy`, one colour as `rect` and `cfill`, flat content as a
//! PNG (with a palette when it has few colours), photographic content as a JPEG when the
//! client draws JPEG. Parts that lie close
//! together go out as one image, because each image has a fixed cost.
//!
//! Pure: no I/O and no clock; the caller gives the timestamp.

mod joining;
mod palette;
mod picture;

use base64::Engine as _;
use guac_protocol::Instruction;

use crate::screen::{Area, Changes, Moved, OutsideScreen, Screen};
use picture::Picture;

/// `GUAC_COMP_OVER`: draw over what the layer holds.
const OVER: &str = "14";
/// The default layer: the remote screen.
const SCREEN_LAYER: &str = "0";
/// The stream every image uses. One image ends before the next begins, so one index is
/// enough, as in guacd.
const IMAGE_STREAM: &str = "0";
/// The image bytes in one `blob`, as guacd sends (`GUAC_PROTOCOL_BLOB_MAX_LENGTH`). In
/// base64 they are 8064 characters, inside the 8192 bytes of one instruction.
const BLOB_BYTES: usize = 6048;

/// Which image formats the client draws, from its `image` instruction. Every client
/// draws PNG.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Formats {
    pub jpeg: bool,
}

/// Why a frame was not made.
#[derive(Debug, thiserror::Error)]
pub enum FrameError {
    /// The changes are not from this screen.
    #[error(transparent)]
    OutsideScreen(#[from] OutsideScreen),
    #[error("the PNG encoder failed")]
    Png(#[from] png::EncodingError),
    #[error("the JPEG encoder failed")]
    Jpeg(#[from] jpeg_encoder::EncodingError),
}

/// The instructions that bring a client from the screen it has to `screen`, given the
/// `changes` taken from that screen. The last instruction is `sync` with `timestamp`,
/// which the client sends back when it has drawn the frame.
///
/// # Errors
/// [`FrameError`] when `changes` are not from `screen` or an image encoder fails.
pub fn frame(
    screen: &Screen,
    changes: &Changes,
    formats: Formats,
    timestamp: u64,
) -> Result<Vec<Instruction>, FrameError> {
    let mut frame = Vec::new();
    if changes.resized {
        let size = [screen.width(), screen.height()].map(|side| side.to_string());
        frame.push(instruction("size", [SCREEN_LAYER], &size));
    }
    frame.extend(changes.moved.iter().map(copy));
    for area in joining::joined(&changes.areas) {
        let mut rgb = Vec::with_capacity(joining::pixels(area) * crate::screen::BYTES_PER_PIXEL);
        for row in screen.rows(area)? {
            rgb.extend_from_slice(row);
        }
        draw(area, &Picture::of(&rgb, area, formats)?, &mut frame);
    }
    frame.push(Instruction::new("sync", [timestamp.to_string()]));
    Ok(frame)
}

/// `copy`: the client moves pixels it already shows.
fn copy(moved: &Moved) -> Instruction {
    let from = moved.from;
    let source = [from.x, from.y, from.width, from.height].map(|value| value.to_string());
    let target = [
        OVER.to_owned(),
        SCREEN_LAYER.to_owned(),
        moved.x.to_string(),
        moved.y.to_string(),
    ];
    let [x, y, width, height] = source;
    let [mask, layer, to_x, to_y] = target;
    instruction(
        "copy",
        [SCREEN_LAYER],
        &[x, y, width, height, mask, layer, to_x, to_y],
    )
}

/// Adds the instructions that draw `picture` at `area`.
fn draw(area: Area, picture: &Picture, frame: &mut Vec<Instruction>) {
    let corner = [area.x, area.y].map(|at| at.to_string());
    match picture {
        Picture::OneColour(colour) => {
            let size = [area.width, area.height].map(|side| side.to_string());
            let [x, y] = corner;
            let [width, height] = size;
            frame.push(instruction("rect", [SCREEN_LAYER], &[x, y, width, height]));
            let [red, green, blue] = colour.map(|part| part.to_string());
            let opaque = "255".to_owned();
            frame.push(instruction(
                "cfill",
                [OVER, SCREEN_LAYER],
                &[red, green, blue, opaque],
            ));
        }
        Picture::Image { mimetype, bytes } => {
            let [x, y] = corner;
            let placed = [(*mimetype).to_owned(), x, y];
            frame.push(instruction(
                "img",
                [IMAGE_STREAM, OVER, SCREEN_LAYER],
                &placed,
            ));
            for chunk in bytes.chunks(BLOB_BYTES) {
                let text = base64::engine::general_purpose::STANDARD.encode(chunk);
                frame.push(instruction("blob", [IMAGE_STREAM], &[text]));
            }
            frame.push(instruction("end", [IMAGE_STREAM], &[]));
        }
    }
}

/// An instruction whose arguments are `fixed` and then `values`.
fn instruction<const N: usize>(opcode: &str, fixed: [&str; N], values: &[String]) -> Instruction {
    let fixed = fixed.into_iter().map(str::to_owned);
    Instruction::new(opcode, fixed.chain(values.iter().cloned()))
}
