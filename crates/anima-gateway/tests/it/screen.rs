//! The screen copy: what counts as a change, and what it refuses.

mod moves;
pub(crate) mod props;

use anima_gateway::screen::{
    Area, BYTES_PER_PIXEL, BadSize, MAX_SIDE, OutsideScreen, Screen, TILE, WriteError,
};

pub(crate) fn whole(screen: &Screen) -> Vec<u8> {
    let all = Area {
        x: 0,
        y: 0,
        width: screen.width(),
        height: screen.height(),
    };
    screen
        .rows(all)
        .expect("the whole screen")
        .flatten()
        .copied()
        .collect()
}

fn filled(area: Area, colour: u8) -> Vec<u8> {
    vec![colour; area.width * area.height * BYTES_PER_PIXEL]
}

/// A seen, black screen: the first changes are taken.
fn seen(width: usize, height: usize) -> Screen {
    let mut screen = Screen::new(width, height).expect("a valid size");
    screen.take_changes();
    screen
}

#[test]
fn a_new_screen_is_wholly_unseen() {
    let mut screen = Screen::new(100, 70).expect("a valid size");
    let changes = screen.take_changes();
    assert!(changes.resized);
    assert_eq!(
        changes.areas,
        [Area {
            x: 0,
            y: 0,
            width: 100,
            height: 70
        }]
    );
}

#[test]
fn changes_are_reported_once() {
    let mut screen = Screen::new(100, 70).expect("a valid size");
    screen.take_changes();
    let again = screen.take_changes();
    assert!(!again.resized);
    assert_eq!(again.areas, []);
}

#[test]
fn pixels_the_same_as_before_are_not_a_change() {
    let mut screen = seen(200, 200);
    let area = Area {
        x: 10,
        y: 10,
        width: 150,
        height: 150,
    };
    screen.write(area, &filled(area, 7)).expect("inside");
    screen.take_changes();
    screen.write(area, &filled(area, 7)).expect("inside");
    assert_eq!(screen.take_changes().areas, []);
}

#[test]
fn one_changed_pixel_costs_one_row_of_one_tile() {
    let mut screen = seen(200, 200);
    let pixel = Area {
        x: 70,
        y: 40,
        width: 1,
        height: 1,
    };
    // The device sends a large area in which one pixel differs.
    let sent = Area {
        x: 0,
        y: 0,
        width: 200,
        height: 100,
    };
    let mut pixels = filled(sent, 0);
    pixels[(pixel.y * sent.width + pixel.x) * BYTES_PER_PIXEL] = 255;
    screen.write(sent, &pixels).expect("inside");
    assert_eq!(
        screen.take_changes().areas,
        [Area {
            x: 64,
            y: 40,
            width: TILE,
            height: 1
        }]
    );
}

#[test]
fn a_moved_area_keeps_its_pixels_when_it_overlaps_its_source() {
    let (width, height) = (120, 120);
    let mut screen = seen(width, height);
    let all = Area {
        x: 0,
        y: 0,
        width,
        height,
    };
    // Every pixel differs from its neighbours, so a pixel copied from the wrong place shows.
    let pattern: Vec<u8> = (0..=250u8)
        .cycle()
        .zip((0..=240u8).cycle())
        .take(width * height)
        .flat_map(|(fast, slow)| [fast, slow, 7])
        .collect();
    screen.write(all, &pattern).expect("inside");
    let from = Area {
        x: 0,
        y: 0,
        width: 50,
        height: 50,
    };
    screen.move_area(from, 20, 30).expect("inside");

    let mut expected = pattern.clone();
    for row in 0..from.height {
        let source = row * width * BYTES_PER_PIXEL;
        let target = ((30 + row) * width + 20) * BYTES_PER_PIXEL;
        let bytes = from.width * BYTES_PER_PIXEL;
        expected[target..target + bytes].copy_from_slice(&pattern[source..source + bytes]);
    }
    assert_eq!(whole(&screen), expected);
}

#[test]
fn the_size_the_screen_already_has_keeps_its_pixels() {
    let mut screen = seen(64, 64);
    let all = Area {
        x: 0,
        y: 0,
        width: 64,
        height: 64,
    };
    screen.write(all, &filled(all, 5)).expect("inside");
    screen.take_changes();
    screen.resize(64, 64).expect("a valid size");
    assert_eq!(whole(&screen), filled(all, 5));
    let changes = screen.take_changes();
    assert!(!changes.resized);
    assert_eq!(changes.areas, []);
}

#[test]
fn a_resize_starts_an_unseen_black_screen() {
    let mut screen = seen(64, 64);
    let all = Area {
        x: 0,
        y: 0,
        width: 64,
        height: 64,
    };
    screen.write(all, &filled(all, 5)).expect("inside");
    screen.resize(40, 90).expect("a valid size");
    assert_eq!(whole(&screen), vec![0; 40 * 90 * BYTES_PER_PIXEL]);
    let changes = screen.take_changes();
    assert!(changes.resized);
    assert_eq!(
        changes.areas,
        [Area {
            x: 0,
            y: 0,
            width: 40,
            height: 90
        }]
    );
}

#[test]
fn sizes_outside_the_tunnel_limits_are_refused() {
    assert_eq!(Screen::new(0, 10).err(), Some(BadSize));
    assert_eq!(Screen::new(10, MAX_SIDE + 1).err(), Some(BadSize));
    let mut screen = seen(10, 10);
    assert_eq!(screen.resize(MAX_SIDE + 1, 10), Err(BadSize));
    assert_eq!((screen.width(), screen.height()), (10, 10));
}

#[test]
fn writes_outside_the_screen_or_of_the_wrong_length_change_nothing() {
    let mut screen = seen(50, 50);
    let outside = Area {
        x: 40,
        y: 0,
        width: 11,
        height: 1,
    };
    assert_eq!(
        screen.write(outside, &filled(outside, 1)),
        Err(WriteError::OutsideScreen(OutsideScreen))
    );
    let huge = Area {
        x: usize::MAX,
        y: 0,
        width: 2,
        height: 1,
    };
    assert_eq!(
        screen.write(huge, &[]),
        Err(WriteError::OutsideScreen(OutsideScreen))
    );
    let inside = Area {
        x: 0,
        y: 0,
        width: 4,
        height: 4,
    };
    assert_eq!(
        screen.write(inside, &[1, 2, 3]),
        Err(WriteError::WrongLength)
    );
    assert_eq!(screen.move_area(inside, 47, 0), Err(OutsideScreen));
    assert_eq!(screen.take_changes().areas, []);
    assert_eq!(whole(&screen), vec![0; 50 * 50 * BYTES_PER_PIXEL]);
}
