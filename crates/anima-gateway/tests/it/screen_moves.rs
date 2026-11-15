//! When the screen copy tells the client to move pixels, and when it sends them instead.

use anima_gateway::frames::Formats;
use anima_gateway::screen::{Area, BYTES_PER_PIXEL, MOST_MOVES, Screen};

use crate::drawn::Drawn;
use crate::frames::send;
use crate::screen::whole;

const WIDTH: usize = 200;

/// `rows` rows in which every row differs from every other.
fn page(rows: usize) -> Vec<u8> {
    (0..rows)
        .flat_map(|row| {
            let colour = [row.to_le_bytes()[0], row.to_le_bytes()[1], 90];
            (0..WIDTH).flat_map(move |x| {
                if x % 3 == 0 {
                    colour
                } else {
                    [x.to_le_bytes()[0], 0, 0]
                }
            })
        })
        .collect()
}

/// A screen that shows the first `height` rows of the page, and a client that has it.
fn showing(height: usize) -> (Screen, Drawn, Area) {
    let area = Area {
        x: 0,
        y: 0,
        width: WIDTH,
        height,
    };
    let mut screen = Screen::new(WIDTH, height).expect("a valid size");
    screen.write(area, &page(height)).expect("inside");
    let mut client = Drawn::default();
    send(&mut screen, &mut client, Formats::default(), 0);
    (screen, client.counted_from_here(), area)
}

/// The page from row `first` on, as the pixels of `area`.
fn scrolled_to(first: usize, area: Area) -> Vec<u8> {
    let row_bytes = WIDTH * BYTES_PER_PIXEL;
    page(first + area.height)[first * row_bytes..].to_vec()
}

#[test]
fn a_scrolled_page_is_one_move() {
    let (mut screen, _, area) = showing(96);
    screen.write(area, &scrolled_to(8, area)).expect("inside");
    let moved = screen.take_changes().moved;
    assert_eq!(moved.len(), 1);
    assert_eq!(
        (moved[0].from.y, moved[0].y, moved[0].from.height),
        (8, 0, 88)
    );
}

#[test]
fn a_few_scrolled_rows_are_sent_and_not_moved() {
    let (mut screen, mut client, area) = showing(32);
    screen.write(area, &scrolled_to(8, area)).expect("inside");
    let changes = screen.take_changes();
    assert_eq!(changes.moved, []);
    client.receive(
        &anima_gateway::frames::frame(&screen, &changes, Formats::default(), 1).expect("a frame"),
    );
    assert_eq!(client.rgb, whole(&screen));
}

#[test]
fn rows_that_only_look_scrolled_in_places_are_not_moved() {
    let (mut screen, _, area) = showing(96);
    // New content, except that three rows repeat rows the screen had elsewhere.
    let row_bytes = WIDTH * BYTES_PER_PIXEL;
    let old = page(96);
    let mut new = scrolled_to(1000, area);
    for row in [24, 48, 72] {
        let from = (row + 5) * row_bytes;
        new[row * row_bytes..(row + 1) * row_bytes].copy_from_slice(&old[from..from + row_bytes]);
    }
    screen.write(area, &new).expect("inside");
    assert_eq!(screen.take_changes().moved, []);
    assert_eq!(whole(&screen), new);
}

#[test]
fn a_move_to_the_same_place_or_of_nothing_tells_the_client_nothing() {
    let (mut screen, _, area) = showing(96);
    screen.move_area(area, 0, 0).expect("inside");
    screen
        .move_area(Area { height: 0, ..area }, 0, 10)
        .expect("inside");
    let changes = screen.take_changes();
    assert_eq!((changes.moved, changes.areas), (vec![], vec![]));
}

#[test]
fn a_flood_of_small_moves_does_not_cost_more_than_sending_the_pixels() {
    let (mut screen, mut client, _) = showing(96);
    // 2000 one-pixel moves: 12 bytes each from a VNC device.
    for step in 0..2000 {
        let from = Area {
            x: step % 190,
            y: step % 90,
            width: 1,
            height: 1,
        };
        screen
            .move_area(from, from.x + 3, from.y + 2)
            .expect("inside");
    }
    send(&mut screen, &mut client, Formats::default(), 1);
    let copies = client
        .opcodes
        .iter()
        .filter(|opcode| *opcode == "copy")
        .count();
    assert_eq!(copies, MOST_MOVES);
    assert_eq!(client.rgb, whole(&screen));
    // The moves told are about 3 kB; the rest is the pixels of a small part of the screen.
    assert!(
        client.wire_bytes <= 10_000,
        "{} bytes for 24000 from the device",
        client.wire_bytes
    );
}
