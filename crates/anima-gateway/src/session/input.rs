//! What the client's instructions mean for the device (K4 Section 4): keys, the pointer,
//! the clipboard, the answer to a frame and the end. The hub's relay has already let
//! through only the instructions of its list; the gateway reads them again with its own
//! limits and ignores what it has no use for.
//!
//! Pure: no I/O.

use base64::Engine as _;
use guac_protocol::Instruction;

/// The longest clipboard text taken from the client, in bytes.
const LONGEST_PASTE: usize = 1 << 18;

/// What the client asked for.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Input {
    /// A key by its X11 keysym: VNC uses the same number.
    Key {
        keysym: u32,
        pressed: bool,
    },
    /// The pointer with its button mask: VNC uses the same mask for the first 8 buttons.
    Pointer {
        x: usize,
        y: usize,
        buttons: u8,
    },
    /// The client's clipboard text, complete.
    Paste(String),
    /// The client drew the frame with this timestamp.
    Drawn(String),
    /// The client's view has this size now.
    View(usize, usize),
    Disconnect,
    /// Nothing for the device.
    Nothing,
}

/// The client's clipboard stream while it arrives: `clipboard`, `blob`s, `end`.
#[derive(Default)]
pub(super) struct Clipboard {
    /// The stream index and the text so far; `None` in the text once it is too long.
    reading: Option<(String, Option<Vec<u8>>)>,
}

impl Clipboard {
    /// Reads one instruction from the client.
    pub(super) fn read(&mut self, instruction: &Instruction) -> Input {
        let args = &instruction.args;
        let number = |at: usize| args.get(at).and_then(|text| text.parse::<u32>().ok());
        match instruction.opcode.as_str() {
            "key" => match (number(0), number(1)) {
                (Some(keysym), Some(pressed)) => Input::Key {
                    keysym,
                    pressed: pressed != 0,
                },
                _ => Input::Nothing,
            },
            "mouse" => pointer(number(0), number(1), number(2)),
            "sync" => args
                .first()
                .map_or(Input::Nothing, |timestamp| Input::Drawn(timestamp.clone())),
            "size" => match (number(0), number(1)) {
                (Some(width), Some(height)) => Input::View(width as usize, height as usize),
                _ => Input::Nothing,
            },
            "disconnect" => Input::Disconnect,
            "clipboard" => {
                let text = args.get(1).is_some_and(|mimetype| mimetype == "text/plain");
                self.reading = args
                    .first()
                    .filter(|_| text)
                    .map(|stream| (stream.clone(), Some(Vec::new())));
                Input::Nothing
            }
            "blob" => self.on_blob(args),
            "end" => self.on_end(args),
            _ => Input::Nothing,
        }
    }

    fn on_blob(&mut self, args: &[String]) -> Input {
        let Some((stream, text)) = &mut self.reading else {
            return Input::Nothing;
        };
        if args.first() != Some(stream) {
            return Input::Nothing;
        }
        let more = args
            .get(1)
            .and_then(|blob| base64::engine::general_purpose::STANDARD.decode(blob).ok());
        *text = match (text.take(), more) {
            (Some(mut text), Some(more)) if text.len() + more.len() <= LONGEST_PASTE => {
                text.extend_from_slice(&more);
                Some(text)
            }
            _ => None,
        };
        Input::Nothing
    }

    fn on_end(&mut self, args: &[String]) -> Input {
        match self.reading.take() {
            Some((stream, text)) if args.first() == Some(&stream) => text
                .and_then(|text| String::from_utf8(text).ok())
                .map_or(Input::Nothing, Input::Paste),
            other => {
                self.reading = other;
                Input::Nothing
            }
        }
    }
}

/// `mouse,<x>,<y>,<mask>`.
fn pointer(x: Option<u32>, y: Option<u32>, mask: Option<u32>) -> Input {
    match (x, y, mask) {
        (Some(x), Some(y), Some(mask)) => Input::Pointer {
            x: x as usize,
            y: y as usize,
            buttons: mask.to_le_bytes()[0],
        },
        _ => Input::Nothing,
    }
}

/// The device's clipboard `text` as a stream to the client: `clipboard`, `blob`s, `end`.
pub(super) fn copy_to_client(text: &str) -> Vec<Instruction> {
    /// The stream for clipboard text; images use stream 0.
    const STREAM: &str = "1";
    /// As `frames`: 6048 bytes are 8064 characters of base64, inside one instruction.
    const BLOB_BYTES: usize = 6048;
    let mut stream = vec![Instruction::new("clipboard", [STREAM, "text/plain"])];
    for chunk in text.as_bytes().chunks(BLOB_BYTES) {
        let blob = base64::engine::general_purpose::STANDARD.encode(chunk);
        stream.push(Instruction::new("blob", [STREAM.to_owned(), blob]));
    }
    stream.push(Instruction::new("end", [STREAM]));
    stream
}
