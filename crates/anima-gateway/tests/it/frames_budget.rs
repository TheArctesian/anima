//! The byte budget of a text session (S2 T2): the client must not get more than twice the
//! bytes that the device sent.
//!
//! The device is a terminal that prints one line at a time and scrolls, and sends its whole
//! window again for each line, as a VNC server does with the ZRLE encoding. Its bytes are
//! counted with a ZRLE-like encoding: 64 x 64 tiles, each packed to one bit for a pixel when
//! it has two colours, through one zlib stream for the session.

use anima_gateway::frames::Formats;
use anima_gateway::screen::{Area, BYTES_PER_PIXEL, Screen};
use flate2::{Compress, Compression, FlushCompress};

use crate::drawn::Drawn;
use crate::frames::send;
use crate::screen::whole;

const CELL: (usize, usize) = (8, 16);
const COLUMNS: usize = 80;
const LINES: usize = 25;
const WINDOW: Area = Area {
    x: 40,
    y: 40,
    width: COLUMNS * CELL.0,
    height: LINES * CELL.1,
};
const PAPER: [u8; 3] = [0x2e, 0x34, 0x40];
const INK: [u8; 3] = [0xd8, 0xde, 0xe9];

/// A small random number generator, the same in each run.
struct Random(u64);

impl Random {
    fn next(&mut self, below: usize) -> usize {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        let below = u64::try_from(below).expect("a small number");
        usize::try_from(self.0 % below).expect("below a small number")
    }
}

/// One line of "words": the character number for each cell, 0 for a space.
fn line(random: &mut Random) -> Vec<usize> {
    let length = 20 + random.next(50);
    (0..COLUMNS)
        .map(|column| {
            if column < length && random.next(6) != 0 {
                1 + random.next(40)
            } else {
                0
            }
        })
        .collect()
}

/// Whether the pixel (`x`, `y`) of character `character` has ink: a fixed shape for each.
fn has_ink(character: usize, x: usize, y: usize) -> bool {
    let inside = (1..7).contains(&x) && (3..13).contains(&y);
    character != 0 && inside && (character * 31 + x * 7 + y * 13 + x * y).is_multiple_of(3)
}

/// The window's pixels for `lines`, top to bottom.
fn window(lines: &[Vec<usize>]) -> Vec<u8> {
    let mut rgb = Vec::with_capacity(WINDOW.width * WINDOW.height * BYTES_PER_PIXEL);
    for y in 0..WINDOW.height {
        for x in 0..WINDOW.width {
            let character = lines[y / CELL.1][x / CELL.0];
            let ink = has_ink(character, x % CELL.0, y % CELL.1);
            rgb.extend_from_slice(if ink { &INK } else { &PAPER });
        }
    }
    rgb
}

/// Up to eight pixels as one byte, the first in the highest bit.
fn bits_as_byte(bits: &[bool]) -> u8 {
    bits.iter()
        .enumerate()
        .fold(0, |byte, (bit, ink)| byte | (u8::from(*ink) << (7 - bit)))
}

/// The bytes a VNC server sends for the window with a ZRLE-like encoding.
fn device_bytes(stream: &mut Compress, rgb: &[u8]) -> usize {
    let mut tiles = Vec::new();
    for top in (0..WINDOW.height).step_by(64) {
        for left in (0..WINDOW.width).step_by(64) {
            let rows = top..(top + 64).min(WINDOW.height);
            let columns = left..(left + 64).min(WINDOW.width);
            // Sub-encoding 2: a palette of two colours, then one bit for each pixel.
            tiles.push(2);
            tiles.extend_from_slice(&PAPER);
            tiles.extend_from_slice(&INK);
            for y in rows {
                let bits = columns.clone().map(|x| {
                    let at = (y * WINDOW.width + x) * BYTES_PER_PIXEL;
                    rgb[at..at + BYTES_PER_PIXEL] == INK
                });
                let row: Vec<bool> = bits.collect();
                tiles.extend(row.chunks(8).map(bits_as_byte));
            }
        }
    }
    let mut packed = Vec::with_capacity(tiles.len() + 64);
    let before = stream.total_out();
    stream
        .compress_vec(&tiles, &mut packed, FlushCompress::Sync)
        .expect("zlib");
    assert_eq!(
        stream.total_in() % tiles.len() as u64,
        0,
        "the output buffer was too small"
    );
    // The rectangle header and the length of the zlib data.
    16 + usize::try_from(stream.total_out() - before).expect("a small count")
}

#[test]
fn a_text_session_costs_the_client_at_most_twice_the_device_bytes() {
    let mut random = Random(0x9e37_79b9_7f4a_7c15);
    let mut lines: Vec<Vec<usize>> = (0..LINES).map(|_| line(&mut random)).collect();
    let mut screen = Screen::new(720, 480).expect("a valid size");
    let mut client = Drawn::default();
    let mut device = Compress::new(Compression::default(), true);
    let formats = Formats { jpeg: true };

    // The window as the session finds it. Both sides pay for it once; it is not counted.
    screen.write(WINDOW, &window(&lines)).expect("inside");
    send(&mut screen, &mut client, formats, 0);
    device_bytes(&mut device, &window(&lines));
    let opening = client.wire_bytes;

    let mut from_device = 0;
    for printed in 1..=60 {
        lines.remove(0);
        lines.push(line(&mut random));
        let rgb = window(&lines);
        from_device += device_bytes(&mut device, &rgb);
        screen.write(WINDOW, &rgb).expect("inside");
        send(&mut screen, &mut client, formats, printed);
    }
    assert_eq!(client.rgb, whole(&screen));
    let to_client = client.wire_bytes - opening;
    assert!(
        to_client <= 2 * from_device,
        "{to_client} bytes to the client for {from_device} bytes from the device"
    );
}
