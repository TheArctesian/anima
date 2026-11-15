//! What a device may not send, and what it may send slowly.

use anima_gateway::screen::Area;
use anima_gateway::vnc::{Device, Event, Malformed};

use super::rfb::{self, Deflater};
use super::{ALL, received, screen};

fn refusal(bytes: &[u8]) -> Option<Malformed> {
    received(bytes).err()
}

#[test]
fn messages_and_rectangles_a_device_may_not_send_end_the_session() {
    let outside = Area {
        x: 100,
        y: 0,
        width: 51,
        height: 1,
    };
    let pixel = Area {
        x: 0,
        y: 0,
        width: 1,
        height: 1,
    };
    assert_eq!(refusal(&[1]), Some(Malformed::UnknownMessage));
    assert_eq!(
        refusal(&rfb::update(&[rfb::header(pixel, 5)])),
        Some(Malformed::UnknownEncoding)
    );
    assert_eq!(
        refusal(&rfb::update(&[rfb::header(outside, 0)])),
        Some(Malformed::OutsideScreen)
    );
    assert_eq!(
        refusal(&rfb::update(&[rfb::copy_rect(pixel, (150, 0))])),
        Some(Malformed::OutsideScreen)
    );
    assert_eq!(
        refusal(&rfb::update(&[rfb::desktop_size(0, 10)])),
        Some(Malformed::BadSize)
    );
    assert_eq!(
        refusal(&rfb::update(&[rfb::desktop_size(8193, 10)])),
        Some(Malformed::BadSize)
    );
    let three_screens = vec![rfb::copy_rect(ALL, (0, 0)); 3];
    assert_eq!(
        refusal(&rfb::update(&three_screens)),
        Some(Malformed::TooManyPixels)
    );
    let two_sizes = [rfb::desktop_size(20, 10), rfb::desktop_size(30, 10)];
    assert_eq!(refusal(&rfb::update(&two_sizes)), Some(Malformed::BadSize));
}

#[test]
fn packed_data_a_device_may_not_send_ends_the_session() {
    let pixel = Area {
        x: 0,
        y: 0,
        width: 1,
        height: 1,
    };
    let block = Area {
        x: 0,
        y: 0,
        width: 8,
        height: 8,
    };
    let packed = |area, encoding, bytes: &[u8]| {
        rfb::update(&[Deflater::default().packed_as(area, encoding, bytes)])
    };
    // A zlib bomb: far more data than one pixel needs.
    assert_eq!(
        refusal(&packed(pixel, 6, &vec![0; 1 << 20])),
        Some(Malformed::TooManyBytes)
    );
    assert_eq!(
        refusal(&packed(pixel, 16, &vec![0; 1 << 20])),
        Some(Malformed::TooManyBytes)
    );
    // A tile that ends early, a run longer than its tile, a palette number outside the palette.
    assert_eq!(
        refusal(&packed(block, 16, &[0, 1])),
        Some(Malformed::BadTile)
    );
    assert_eq!(
        refusal(&packed(block, 16, &[128, 1, 2, 3, 255, 255, 0])),
        Some(Malformed::BadTile)
    );
    assert_eq!(
        refusal(&packed(block, 16, &[130, 1, 2, 3, 4, 5, 6, 9])),
        Some(Malformed::BadTile)
    );
    // Bytes after the last tile.
    assert_eq!(
        refusal(&packed(block, 16, &[1, 1, 2, 3, 7])),
        Some(Malformed::BadTile)
    );
    let not_zlib = [
        rfb::update(&[rfb::header(pixel, 6)]),
        vec![0, 0, 0, 3, 1, 2, 3],
    ]
    .concat();
    assert_eq!(refusal(&not_zlib), Some(Malformed::BadZlib));
    let huge_length = [rfb::update(&[rfb::header(pixel, 6)]), vec![0xff; 4]].concat();
    assert_eq!(refusal(&huge_length), Some(Malformed::TooManyBytes));
}

#[test]
fn an_update_that_trickles_in_costs_no_more_than_one_that_comes_at_once() {
    // 20 000 one-pixel rectangles in reads of 16 bytes: each read looks at the rectangle
    // it completes, not at the whole update again.
    let pixel = Area {
        x: 3,
        y: 3,
        width: 1,
        height: 1,
    };
    let bytes = rfb::update(&vec![rfb::copy_rect(pixel, (4, 4)); 20_000]);
    let mut device = Device::default();
    let mut screen = screen();
    let started = std::time::Instant::now();
    let mut events = Vec::new();
    for piece in bytes.chunks(16) {
        events.extend(device.receive(piece, &mut screen).expect("read"));
    }
    assert_eq!(events, [Event::Updated]);
    // Minutes when every read parses the update from its start; a moment when not.
    assert!(
        started.elapsed() < std::time::Duration::from_secs(20),
        "{:?}",
        started.elapsed()
    );
}

#[test]
fn a_rectangle_is_drawn_as_soon_as_it_is_whole() {
    let first = Area {
        x: 0,
        y: 0,
        width: 4,
        height: 4,
    };
    let second = Area {
        x: 10,
        y: 10,
        width: 4,
        height: 4,
    };
    let bytes = rfb::update(&[rfb::raw(first, &[9; 48]), rfb::raw(second, &[7; 48])]);
    let mut device = Device::default();
    let mut screen = screen();
    // All of the first rectangle and a part of the second.
    let events = device
        .receive(&bytes[..bytes.len() - 10], &mut screen)
        .expect("read");
    assert_eq!(events, []);
    let drawn: Vec<u8> = screen
        .rows(first)
        .expect("inside")
        .flatten()
        .copied()
        .collect();
    assert_eq!(drawn, [9; 48]);
    assert_eq!(
        device
            .receive(&bytes[bytes.len() - 10..], &mut screen)
            .expect("read"),
        [Event::Updated]
    );
}
