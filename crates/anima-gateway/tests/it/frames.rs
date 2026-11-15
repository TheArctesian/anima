//! A frame draws the screen in the client, in the cheapest form for each part.

mod budget;
mod pacing;

use anima_gateway::frames::{Formats, frame};
use anima_gateway::screen::{Area, BYTES_PER_PIXEL, Screen, TILE};
use proptest::prelude::*;

use crate::drawn::Drawn;
use crate::screen::props::{Picture, apply, step};
use crate::screen::whole;

const PNG_ONLY: Formats = Formats { jpeg: false };
const WITH_JPEG: Formats = Formats { jpeg: true };

/// Sends the screen's changes to `client` as one frame.
pub(crate) fn send(screen: &mut Screen, client: &mut Drawn, formats: Formats, timestamp: u64) {
    let changes = screen.take_changes();
    client.receive(&frame(screen, &changes, formats, timestamp).expect("a frame"));
}

/// A screen and a client that already shows it.
fn shown(width: usize, height: usize) -> (Screen, Drawn) {
    let mut screen = Screen::new(width, height).expect("a valid size");
    let mut client = Drawn::default();
    send(&mut screen, &mut client, PNG_ONLY, 0);
    (screen, client.counted_from_here())
}

/// Pixels with noise in every part: the hardest content, with a colour for each pixel.
fn noise(area: Area) -> Vec<u8> {
    let mut state = 0x2545_f491_4f6c_dd1d_u64;
    (0..area.width * area.height * BYTES_PER_PIXEL)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state.to_le_bytes()[0]
        })
        .collect()
}

/// Smooth colour with a little grain, as a photograph has.
fn photograph(area: Area) -> Vec<u8> {
    let grain = noise(area);
    let part = |value: usize, grain: u8| u8::try_from(value % 200).unwrap_or(0) + grain % 8;
    (0..area.width * area.height)
        .flat_map(|pixel| {
            let (x, y) = (pixel % area.width, pixel / area.width);
            let at = pixel * BYTES_PER_PIXEL;
            [
                part(x, grain[at]),
                part(y, grain[at + 1]),
                part(usize::midpoint(x, y), grain[at + 2]),
            ]
        })
        .collect()
}

/// Two colours in a pattern, as text has.
fn text(area: Area) -> Vec<u8> {
    (0..area.width * area.height)
        .flat_map(|pixel| {
            if pixel % 5 < 2 || pixel % 13 == 0 {
                [20, 20, 20]
            } else {
                [240, 240, 240]
            }
        })
        .collect()
}

#[test]
fn a_new_screen_starts_with_its_size_and_ends_with_one_sync() {
    let mut screen = Screen::new(300, 200).expect("a valid size");
    let mut client = Drawn::default();
    send(&mut screen, &mut client, PNG_ONLY, 4711);
    assert_eq!(client.opcodes.first().map(String::as_str), Some("size"));
    assert_eq!(client.opcodes.last().map(String::as_str), Some("sync"));
    assert_eq!(client.synced, ["4711"]);
    assert_eq!(
        (client.width, client.rgb.len()),
        (300, 300 * 200 * BYTES_PER_PIXEL)
    );
}

#[test]
fn nothing_changed_is_a_sync_alone() {
    let (mut screen, mut client) = shown(300, 200);
    send(&mut screen, &mut client, PNG_ONLY, 9);
    assert_eq!(client.opcodes, ["sync"]);
}

#[test]
fn one_colour_goes_out_as_a_fill_and_not_as_an_image() {
    let (mut screen, mut client) = shown(320, 320);
    let area = Area {
        x: 0,
        y: 0,
        width: 320,
        height: 320,
    };
    screen
        .write(area, &[200, 30, 30].repeat(320 * 320))
        .expect("inside");
    send(&mut screen, &mut client, PNG_ONLY, 1);
    assert_eq!(client.opcodes, ["rect", "cfill", "sync"]);
    assert_eq!(client.rgb, whole(&screen));
    assert!(client.wire_bytes < 100, "{} bytes", client.wire_bytes);
}

#[test]
fn text_stays_exact_even_when_the_client_draws_jpeg() {
    let (mut screen, mut client) = shown(320, 320);
    let area = Area {
        x: 0,
        y: 0,
        width: 320,
        height: 320,
    };
    screen.write(area, &text(area)).expect("inside");
    send(&mut screen, &mut client, WITH_JPEG, 1);
    assert_eq!(client.images, ["image/png"]);
    assert_eq!(client.rgb, whole(&screen));
}

#[test]
fn flat_content_with_many_colours_stays_exact_when_the_client_draws_jpeg() {
    // 320 bands, each of one colour: more colours than a palette holds, but no photograph.
    let area = Area {
        x: 0,
        y: 0,
        width: 200,
        height: 320,
    };
    let bands: Vec<u8> = (0..area.height)
        .flat_map(|row| [row.to_le_bytes()[0], row.to_le_bytes()[1], 40].repeat(area.width))
        .collect();
    let (mut screen, mut client) = shown(200, 320);
    screen.write(area, &bands).expect("inside");
    send(&mut screen, &mut client, WITH_JPEG, 1);
    assert_eq!(client.images, ["image/png"]);
    assert_eq!(client.rgb, whole(&screen));
}

#[test]
fn a_photograph_goes_out_as_jpeg_only_when_the_client_draws_jpeg() {
    let area = Area {
        x: 0,
        y: 0,
        width: 256,
        height: 256,
    };
    let (mut screen, mut exact) = shown(256, 256);
    screen.write(area, &photograph(area)).expect("inside");
    send(&mut screen, &mut exact, PNG_ONLY, 1);
    assert_eq!(exact.images, ["image/png"]);
    assert_eq!(exact.rgb, whole(&screen));

    let (mut screen, mut lossy) = shown(256, 256);
    screen.write(area, &photograph(area)).expect("inside");
    send(&mut screen, &mut lossy, WITH_JPEG, 1);
    assert_eq!(lossy.images, ["image/jpeg"]);
    assert!(lossy.wire_bytes < exact.wire_bytes);
    // Lossy, but the same picture.
    let off: usize = lossy
        .rgb
        .iter()
        .zip(whole(&screen))
        .map(|(a, b)| usize::from(a.abs_diff(b)))
        .sum();
    assert!(
        off / lossy.rgb.len() < 8,
        "mean difference {}",
        off / lossy.rgb.len()
    );
}

#[test]
fn a_large_image_crosses_the_wire_in_blobs_the_hub_accepts() {
    let area = Area {
        x: 0,
        y: 0,
        width: 512,
        height: 512,
    };
    let (mut screen, mut client) = shown(512, 512);
    screen.write(area, &noise(area)).expect("inside");
    send(&mut screen, &mut client, PNG_ONLY, 1);
    let blobs = client
        .opcodes
        .iter()
        .filter(|opcode| *opcode == "blob")
        .count();
    assert!(blobs > 100, "{blobs} blobs");
    assert_eq!(client.rgb, whole(&screen));
}

#[test]
fn near_changes_share_an_image_and_far_ones_do_not() {
    let pixel = |x, y| Area {
        x,
        y,
        width: 1,
        height: 1,
    };
    let (mut screen, mut client) = shown(640, 640);
    screen.write(pixel(10, 10), &[255, 0, 0]).expect("inside");
    screen
        .write(pixel(10 + 2 * TILE, 10), &[0, 255, 0])
        .expect("inside");
    send(&mut screen, &mut client, PNG_ONLY, 1);
    assert_eq!(client.images.len(), 1);
    assert_eq!(client.rgb, whole(&screen));

    let (mut screen, mut client) = shown(640, 640);
    screen.write(pixel(10, 10), &[255, 0, 0]).expect("inside");
    screen.write(pixel(600, 600), &[0, 255, 0]).expect("inside");
    send(&mut screen, &mut client, PNG_ONLY, 1);
    assert_eq!(client.images.len(), 2);
    assert_eq!(client.rgb, whole(&screen));
}

#[test]
fn a_scroll_goes_out_as_a_copy_and_the_new_rows() {
    let area = Area {
        x: 40,
        y: 40,
        width: 400,
        height: 320,
    };
    let (mut screen, mut client) = shown(480, 400);
    let page = noise(Area {
        height: 400,
        ..area
    });
    let row_bytes = area.width * BYTES_PER_PIXEL;
    screen
        .write(area, &page[..area.height * row_bytes])
        .expect("inside");
    send(&mut screen, &mut client, PNG_ONLY, 1);
    let first = client.wire_bytes;

    // The page moves up by 16 rows; 16 new rows come in at the bottom.
    screen
        .write(area, &page[16 * row_bytes..(16 + area.height) * row_bytes])
        .expect("inside");
    send(&mut screen, &mut client, PNG_ONLY, 2);
    assert!(client.opcodes.iter().any(|opcode| opcode == "copy"));
    assert_eq!(client.rgb, whole(&screen));
    let scrolled = client.wire_bytes - first;
    assert!(
        scrolled * 5 < first,
        "the scroll cost {scrolled} bytes, the page {first}"
    );
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    /// Whatever a device does, a client that draws each frame shows the device's picture.
    #[test]
    fn a_client_that_draws_the_frames_shows_the_screen(
        (width, height) in (1..160usize, 1..160usize),
        rounds in prop::collection::vec(prop::collection::vec(step(), 0..6), 1..5),
    ) {
        let mut screen = Screen::new(width, height).expect("a valid size");
        let mut picture = Picture::black(width, height);
        let mut client = Drawn::default();
        for (timestamp, round) in rounds.iter().enumerate() {
            for step in round {
                apply(step, &mut screen, &mut picture);
            }
            send(&mut screen, &mut client, PNG_ONLY, timestamp as u64);
            prop_assert_eq!(&client.rgb, &picture.rgb);
        }
    }
}
