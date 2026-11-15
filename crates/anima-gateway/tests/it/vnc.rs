//! A device's updates arrive on the screen copy, and a device cannot break the gateway.

mod limits;
pub(crate) mod rfb;
mod sign_in;

use anima_gateway::screen::{Area, BYTES_PER_PIXEL, Screen};
use anima_gateway::vnc::{Device, Event, Malformed, requests};
use proptest::prelude::*;

use crate::screen::whole;
use rfb::{Deflater, Tile};

const SIZE: (usize, usize) = (150, 90);
pub(crate) const ALL: Area = Area {
    x: 0,
    y: 0,
    width: SIZE.0,
    height: SIZE.1,
};

pub(crate) fn screen() -> Screen {
    Screen::new(SIZE.0, SIZE.1).expect("a valid size")
}

/// A picture of `colours` colours in bands and blocks, so tiles have runs and several colours.
fn picture(area: Area, colours: usize) -> Vec<u8> {
    (0..area.width * area.height)
        .flat_map(|pixel| {
            let (x, y) = (pixel % area.width, pixel / area.width);
            let colour = u8::try_from((x / 7 + y / 3) % colours).expect("few colours");
            [colour * 15, 255 - colour * 9, colour]
        })
        .collect()
}

pub(crate) fn received(bytes: &[u8]) -> Result<(Screen, Vec<Event>), Malformed> {
    let mut screen = screen();
    let events = Device::default().receive(bytes, &mut screen)?;
    Ok((screen, events))
}

#[test]
fn raw_pixels_arrive_with_their_colours_in_order() {
    let area = Area {
        x: 10,
        y: 5,
        width: 2,
        height: 1,
    };
    let (screen, events) =
        received(&rfb::update(&[rfb::raw(area, &[255, 0, 0, 0, 0, 255])])).expect("drawn");
    let drawn: Vec<u8> = screen
        .rows(area)
        .expect("inside")
        .flatten()
        .copied()
        .collect();
    assert_eq!(drawn, [255, 0, 0, 0, 0, 255]);
    assert_eq!(events, [Event::Updated]);
}

#[test]
fn every_form_of_zrle_tile_draws_the_picture() {
    for (kind, colours) in [
        (Tile::Raw, 16),
        (Tile::OneColour, 1),
        (Tile::Packed, 2),
        (Tile::Packed, 4),
        (Tile::Packed, 16),
        (Tile::Runs, 16),
        (Tile::PaletteRuns, 16),
    ] {
        let rgb = picture(ALL, colours);
        let update = rfb::update(&[Deflater::default().zrle(ALL, &rgb, kind)]);
        let (screen, _) = received(&update).unwrap_or_else(|error| panic!("{kind:?}: {error}"));
        assert_eq!(whole(&screen), rgb, "{kind:?} with {colours} colours");
    }
}

#[test]
fn the_zlib_streams_live_across_updates() {
    let (first, second) = (picture(ALL, 16), picture(ALL, 5));
    let (mut zlib, mut zrle) = (Deflater::default(), Deflater::default());
    let mut device = Device::default();
    let mut screen = screen();
    for (rgb, next) in [(&first, &second), (&second, &first)] {
        device
            .receive(&rfb::update(&[zlib.zlib(ALL, rgb)]), &mut screen)
            .expect("zlib");
        assert_eq!(&whole(&screen), rgb);
        device
            .receive(
                &rfb::update(&[zrle.zrle(ALL, next, Tile::Runs)]),
                &mut screen,
            )
            .expect("zrle");
        assert_eq!(&whole(&screen), next);
    }
}

#[test]
fn a_copied_rectangle_and_a_new_size_are_followed() {
    let rgb = picture(ALL, 16);
    let part = Area {
        x: 0,
        y: 0,
        width: 40,
        height: 30,
    };
    let moved = Area {
        x: 60,
        y: 50,
        ..part
    };
    let small = Area {
        x: 0,
        y: 0,
        width: 20,
        height: 10,
    };
    let mut device = Device::default();
    let mut screen = screen();
    device
        .receive(
            &rfb::update(&[rfb::raw(ALL, &rgb), rfb::copy_rect(moved, (0, 0))]),
            &mut screen,
        )
        .expect("drawn");
    let copied: Vec<u8> = screen
        .rows(moved)
        .expect("inside")
        .flatten()
        .copied()
        .collect();
    let source: Vec<u8> = screen
        .rows(part)
        .expect("inside")
        .flatten()
        .copied()
        .collect();
    assert_eq!(copied, source);

    // A new size and pixels for it in one update.
    let update = rfb::update(&[
        rfb::desktop_size(20, 10),
        rfb::raw(small, &picture(small, 3)),
    ]);
    device.receive(&update, &mut screen).expect("resized");
    assert_eq!((screen.width(), screen.height()), (20, 10));
    assert_eq!(whole(&screen), picture(small, 3));
}

#[test]
fn the_clipboard_arrives_as_text_and_an_over_long_one_is_skipped() {
    let mut bytes = rfb::cut_text(b"caf\xe9");
    bytes.extend(rfb::cut_text(&vec![b'x'; (1 << 20) + 1]));
    bytes.push(2);
    let (_, events) = received(&bytes).expect("read");
    assert_eq!(
        events,
        [Event::Clipboard("caf\u{e9}".to_owned()), Event::Bell]
    );
}

#[test]
fn input_goes_to_the_device_with_guacamoles_numbers() {
    assert_eq!(requests::key(0xff0d, true), [4, 1, 0, 0, 0, 0, 0xff, 0x0d]);
    assert_eq!(requests::pointer(0b101, 300, 2), [5, 5, 1, 44, 0, 2]);
    assert_eq!(
        requests::update(true, 150, 90),
        [3, 1, 0, 0, 0, 0, 0, 150, 0, 90]
    );
    assert_eq!(
        requests::cut_text("né€"),
        [6, 0, 0, 0, 0, 0, 0, 3, b'n', 0xe9, b'?']
    );
}

proptest! {
    /// The same bytes in pieces of any size give the same screen and events.
    #[test]
    fn bytes_may_arrive_in_any_pieces(cuts in prop::collection::vec(1..400usize, 1..40)) {
        let rgb = picture(ALL, 16);
        let part = Area { x: 5, y: 5, width: 70, height: 66 };
        let mut zrle = Deflater::default();
        let mut bytes = rfb::update(&[rfb::raw(ALL, &rgb)]);
        bytes.extend(rfb::cut_text(b"copied"));
        bytes.extend(rfb::update(&[zrle.zrle(part, &picture(part, 4), Tile::Packed), rfb::copy_rect(part, (60, 20))]));
        let (expected, expected_events) = received(&bytes).expect("in one piece");

        let mut device = Device::default();
        let mut screen = screen();
        let (mut events, mut rest) = (Vec::new(), bytes.as_slice());
        for cut in cuts.into_iter().cycle() {
            let (piece, later) = rest.split_at(cut.min(rest.len()));
            events.extend(device.receive(piece, &mut screen).expect("in pieces"));
            rest = later;
            if rest.is_empty() {
                break;
            }
        }
        prop_assert_eq!(whole(&screen), whole(&expected));
        prop_assert_eq!(events, expected_events);
    }

    /// No bytes from a device make the gateway panic, and the screen stays a valid size.
    #[test]
    fn no_bytes_break_the_gateway(
        pieces in prop::collection::vec(prop::collection::vec(any::<u8>(), 0..200), 0..20),
        starts_as_an_update in any::<bool>(),
    ) {
        let mut device = Device::default();
        let mut screen = screen();
        if starts_as_an_update {
            let _ = device.receive(&[0, 0, 0, 3], &mut screen);
        }
        for piece in pieces {
            if device.receive(&piece, &mut screen).is_err() {
                break;
            }
        }
        prop_assert!(screen.width() * screen.height() * BYTES_PER_PIXEL == whole(&screen).len());
    }
}
