//! A client for the tests: it draws Guacamole instructions as guacamole-common-js does,
//! after each one has gone over the wire and through the hub's decoder.

use base64::Engine as _;
use guac_protocol::{Decoder, Instruction, encode};

const BYTES_PER_PIXEL: usize = 3;
/// The default layer, the only one the gateway draws on.
const SCREEN: &str = "0";
/// The channel mask guacd sends with opaque drawing (`GUAC_COMP_OVER`).
const OVER: &str = "14";

/// What a client shows, and what it received.
#[derive(Default)]
pub(crate) struct Drawn {
    pub(crate) width: usize,
    pub(crate) rgb: Vec<u8>,
    /// The bytes of every instruction on the wire.
    pub(crate) wire_bytes: usize,
    /// The opcodes, in order.
    pub(crate) opcodes: Vec<String>,
    /// The mimetype of each image.
    pub(crate) images: Vec<String>,
    /// The timestamps of the `sync` instructions.
    pub(crate) synced: Vec<String>,
    rect: Option<[usize; 4]>,
    stream: Option<(String, [usize; 2], Vec<u8>)>,
}

fn numbers<const N: usize>(args: &[String]) -> [usize; N] {
    std::array::from_fn(|at| args[at].parse().expect("a number"))
}

impl Drawn {
    /// A client that shows a black screen of this size.
    pub(crate) fn black(width: usize, height: usize) -> Self {
        Self {
            width,
            rgb: vec![0; width * height * BYTES_PER_PIXEL],
            ..Self::default()
        }
    }

    /// The same picture, with nothing counted as received yet.
    pub(crate) fn counted_from_here(self) -> Self {
        Self {
            width: self.width,
            rgb: self.rgb,
            ..Self::default()
        }
    }

    /// Receives one frame.
    pub(crate) fn receive(&mut self, frame: &[Instruction]) {
        for instruction in frame {
            let wire = encode(instruction);
            self.wire_bytes += wire.len();
            let decoded = Decoder::new()
                .feed(wire.as_bytes())
                .expect("the hub decodes it");
            assert_eq!(decoded, std::slice::from_ref(instruction));
            self.opcodes.push(instruction.opcode.clone());
            self.draw(&instruction.opcode, &instruction.args);
        }
    }

    fn draw(&mut self, opcode: &str, args: &[String]) {
        match opcode {
            "size" => {
                let [layer, width, height] = numbers(args);
                assert_eq!(layer, 0);
                self.width = width;
                self.rgb = vec![0; width * height * BYTES_PER_PIXEL];
            }
            "copy" => {
                assert_eq!([&args[0], &args[5], &args[6]], [SCREEN, OVER, SCREEN]);
                let [x, y, width, height] = numbers(&args[1..5]);
                let [to_x, to_y] = numbers(&args[7..9]);
                let pixels = self.read([x, y, width, height]);
                self.paint([to_x, to_y, width, height], &pixels);
            }
            "rect" => {
                assert_eq!(args[0], SCREEN);
                self.rect = Some(numbers(&args[1..5]));
            }
            "cfill" => {
                assert_eq!([&args[0], &args[1]], [OVER, SCREEN]);
                let [red, green, blue, alpha] =
                    numbers(&args[2..6]).map(|part| u8::try_from(part).expect("a colour part"));
                assert_eq!(alpha, 255);
                let rect = self.rect.take().expect("a rect before cfill");
                self.paint(rect, &[red, green, blue].repeat(rect[2] * rect[3]));
            }
            "img" => {
                assert!(self.stream.is_none(), "one image at a time on the stream");
                assert_eq!([&args[1], &args[2]], [OVER, SCREEN]);
                self.images.push(args[3].clone());
                self.stream = Some((args[3].clone(), numbers(&args[4..6]), Vec::new()));
            }
            "blob" => {
                let bytes = base64::engine::general_purpose::STANDARD.decode(&args[1]);
                let stream = self.stream.as_mut().expect("a blob inside an image");
                stream.2.extend(bytes.expect("base64"));
            }
            "end" => {
                let (mimetype, [x, y], bytes) = self.stream.take().expect("an image to end");
                let (width, height, pixels) = decode(&mimetype, &bytes);
                self.paint([x, y, width, height], &pixels);
            }
            "sync" => self.synced.push(args[0].clone()),
            other => panic!("the client does not draw `{other}`"),
        }
    }

    fn read(&self, [x, y, width, height]: [usize; 4]) -> Vec<u8> {
        (y..y + height)
            .flat_map(|row| {
                let start = (row * self.width + x) * BYTES_PER_PIXEL;
                self.rgb[start..start + width * BYTES_PER_PIXEL]
                    .iter()
                    .copied()
            })
            .collect()
    }

    fn paint(&mut self, [x, y, width, height]: [usize; 4], pixels: &[u8]) {
        assert_eq!(pixels.len(), width * height * BYTES_PER_PIXEL);
        for row in 0..height {
            let from = row * width * BYTES_PER_PIXEL;
            let to = ((y + row) * self.width + x) * BYTES_PER_PIXEL;
            let bytes = width * BYTES_PER_PIXEL;
            self.rgb[to..to + bytes].copy_from_slice(&pixels[from..from + bytes]);
        }
    }
}

/// An image as its size and RGB pixels.
fn decode(mimetype: &str, bytes: &[u8]) -> (usize, usize, Vec<u8>) {
    match mimetype {
        "image/png" => {
            let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
            // A palette becomes the colours it stands for, as a browser draws it.
            decoder.set_transformations(png::Transformations::EXPAND);
            let mut reader = decoder.read_info().expect("a PNG");
            let mut pixels = vec![0; reader.output_buffer_size().expect("a size")];
            let frame = reader.next_frame(&mut pixels).expect("pixels");
            assert_eq!(frame.color_type, png::ColorType::Rgb);
            (frame.width as usize, frame.height as usize, pixels)
        }
        "image/jpeg" => {
            let mut decoder = jpeg_decoder::Decoder::new(bytes);
            let pixels = decoder.decode().expect("a JPEG");
            let size = decoder.info().expect("a JPEG header");
            (usize::from(size.width), usize::from(size.height), pixels)
        }
        other => panic!("the client does not draw {other}"),
    }
}
