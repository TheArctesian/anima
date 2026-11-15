//! A screen larger than the client's view is shown as a smaller picture of it.

use anima_gateway::frames::{Formats, frame};
use anima_gateway::screen::{Area, BYTES_PER_PIXEL, Screen};
use anima_gateway::view::View;
use proptest::prelude::*;

use crate::drawn::Drawn;
use crate::screen::whole;

const RED: [u8; 3] = [200, 0, 0];
const BLUE: [u8; 3] = [0, 0, 200];

fn fill(screen: &mut Screen, area: Area, colour: [u8; 3]) {
    screen
        .write(area, &colour.repeat(area.width * area.height))
        .expect("inside");
}

/// A 400 x 200 screen: the left half red, the right half blue.
fn two_halves() -> Screen {
    let mut screen = Screen::new(400, 200).expect("a valid size");
    fill(
        &mut screen,
        Area {
            x: 0,
            y: 0,
            width: 200,
            height: 200,
        },
        RED,
    );
    fill(
        &mut screen,
        Area {
            x: 200,
            y: 0,
            width: 200,
            height: 200,
        },
        BLUE,
    );
    screen
}

/// Sends what the view shows to `client` as one frame; returns the frame's bytes.
fn send(view: &mut View, device: &mut Screen, client: &mut Drawn) -> usize {
    let before = client.wire_bytes;
    let (shown, changes) = view.take_changes(device);
    client.receive(&frame(shown, &changes, Formats::default(), 1).expect("a frame"));
    client.wire_bytes - before
}

#[test]
fn a_screen_that_fits_is_shown_as_it_is() {
    let mut device = two_halves();
    let mut view = View::new(&device, (800, 600));
    assert_eq!(view.size(&device), (400, 200));
    let mut client = Drawn::default();
    send(&mut view, &mut device, &mut client);
    assert_eq!(client.rgb, whole(&device));
    assert_eq!(view.to_device((399, 199), &device), (399, 199));
}

#[test]
fn a_screen_that_does_not_fit_is_shown_smaller_with_its_shape() {
    let mut device = two_halves();
    let mut view = View::new(&device, (200, 150));
    assert_eq!(view.size(&device), (200, 100));
    let mut client = Drawn::default();
    send(&mut view, &mut device, &mut client);
    assert_eq!(
        (client.width, client.rgb.len()),
        (200, 200 * 100 * BYTES_PER_PIXEL)
    );
    let pixel =
        |x: usize, y: usize| &client.rgb[(y * 200 + x) * BYTES_PER_PIXEL..][..BYTES_PER_PIXEL];
    assert_eq!(
        (pixel(0, 0), pixel(99, 99), pixel(100, 0), pixel(199, 99)),
        (&RED[..], &RED[..], &BLUE[..], &BLUE[..])
    );

    // A tall view is limited by its width, a wide one by its height.
    assert_eq!(View::new(&device, (100, 1000)).size(&device), (100, 50));
    assert_eq!(View::new(&device, (1000, 50)).size(&device), (100, 50));
}

#[test]
fn a_small_pixel_is_the_mean_of_the_pixels_it_covers() {
    let mut device = Screen::new(4, 2).expect("a valid size");
    device
        .write(
            Area {
                x: 0,
                y: 0,
                width: 4,
                height: 2,
            },
            &[
                10, 0, 0, 30, 0, 0, 0, 100, 0, 0, 100, 0, 50, 0, 0, 70, 0, 0, 0, 100, 0, 0, 100, 8,
            ],
        )
        .expect("inside");
    let mut view = View::new(&device, (2, 1));
    let mut client = Drawn::default();
    send(&mut view, &mut device, &mut client);
    assert_eq!(client.rgb, [40, 0, 0, 0, 100, 2]);
}

#[test]
fn only_the_part_that_changed_is_sent_again() {
    let mut device = two_halves();
    let mut view = View::new(&device, (200, 100));
    let mut client = Drawn::default();
    let first = send(&mut view, &mut device, &mut client);
    assert_eq!(
        send(&mut view, &mut device, &mut client),
        "4.sync,1.1;".len()
    );

    fill(
        &mut device,
        Area {
            x: 300,
            y: 100,
            width: 20,
            height: 20,
        },
        [0, 200, 0],
    );
    let changed = send(&mut view, &mut device, &mut client);
    assert!(
        changed < first,
        "{changed} bytes for a small change, {first} for the whole picture"
    );
    let pixel =
        |x: usize, y: usize| &client.rgb[(y * 200 + x) * BYTES_PER_PIXEL..][..BYTES_PER_PIXEL];
    assert_eq!(
        (pixel(155, 55), pixel(140, 55), pixel(155, 70)),
        (&[0, 200, 0][..], &BLUE[..], &BLUE[..])
    );
}

#[test]
fn moved_pixels_reach_a_smaller_picture_too() {
    let mut device = two_halves();
    let mut view = View::new(&device, (200, 100));
    let mut client = Drawn::default();
    send(&mut view, &mut device, &mut client);
    // The red half moves over the blue half.
    device
        .move_area(
            Area {
                x: 0,
                y: 0,
                width: 200,
                height: 200,
            },
            200,
            0,
        )
        .expect("inside");
    send(&mut view, &mut device, &mut client);
    assert_eq!(client.rgb, RED.repeat(200 * 100));
}

#[test]
fn a_new_client_view_or_device_size_starts_the_picture_again() {
    let mut device = two_halves();
    let mut view = View::new(&device, (200, 100));
    let mut client = Drawn::default();
    send(&mut view, &mut device, &mut client);

    view.set_client((100, 100), &mut device);
    assert!(view.has_changes(&device));
    send(&mut view, &mut device, &mut client);
    assert_eq!(
        (client.width, client.rgb.len()),
        (100, 100 * 50 * BYTES_PER_PIXEL)
    );

    view.set_client((4000, 4000), &mut device);
    send(&mut view, &mut device, &mut client);
    assert_eq!(client.rgb, whole(&device));

    view.set_client((100, 100), &mut device);
    device.resize(1000, 100).expect("a valid size");
    send(&mut view, &mut device, &mut client);
    assert_eq!(
        (client.width, client.rgb.len()),
        (100, 100 * 10 * BYTES_PER_PIXEL)
    );
}

#[test]
fn a_pointer_in_the_picture_is_over_the_device_pixel_it_shows() {
    let device = Screen::new(4104, 3404).expect("a valid size");
    let view = View::new(&device, (1440, 900));
    let (width, height) = view.size(&device);
    for (x, y) in [
        (0, 0),
        (1, 1),
        (width / 2, height / 2),
        (width - 1, height - 1),
        (9999, 9999),
    ] {
        let (real_x, real_y) = view.to_device((x, y), &device);
        assert!(real_x < 4104 && real_y < 3404);
        // The device pixel lies in the picture's pixel it was pointed at.
        let shown = (real_x * width / 4104, real_y * height / 3404);
        assert_eq!(shown, (x.min(width - 1), y.min(height - 1)));
    }
}

#[test]
fn a_mac_with_three_displays_costs_what_the_client_view_holds() {
    // The size the spike measured on a Mac with three displays, in a 1440 x 900 window.
    let mut device = Screen::new(4104, 3404).expect("a valid size");
    let all = Area {
        x: 0,
        y: 0,
        width: 4104,
        height: 3404,
    };
    let stripes: Vec<u8> = (0..all.width * all.height)
        .flat_map(|pixel| {
            if ((pixel % all.width) / 40).is_multiple_of(2) {
                RED
            } else {
                BLUE
            }
        })
        .collect();
    device.write(all, &stripes).expect("inside");
    let mut view = View::new(&device, (1440, 900));
    assert_eq!(view.size(&device), (1085, 900));
    let mut client = Drawn::default();
    let bytes = send(&mut view, &mut device, &mut client);
    // A whole-screen change is a picture of under 1 megapixel, not of 14.
    assert_eq!(client.rgb.len(), 1085 * 900 * BYTES_PER_PIXEL);
    assert!(bytes < 200_000, "{bytes} bytes for a whole-screen change");
}

/// The picture of `device` in `size`, made from the whole screen at once: each pixel is the
/// mean of the device pixels it covers.
fn whole_picture(device: &Screen, size: (usize, usize)) -> Vec<u8> {
    let real = whole(device);
    let (real_width, real_height) = (device.width(), device.height());
    let covered = |at: usize, shown: usize, real: usize| {
        let first = at * real / shown;
        first..((at + 1) * real / shown).max(first + 1).min(real)
    };
    let mut picture = Vec::new();
    for y in 0..size.1 {
        for x in 0..size.0 {
            let mut sums = [0usize; BYTES_PER_PIXEL];
            let (rows, columns) = (
                covered(y, size.1, real_height),
                covered(x, size.0, real_width),
            );
            for (row, column) in rows
                .clone()
                .flat_map(|row| columns.clone().map(move |column| (row, column)))
            {
                let at = (row * real_width + column) * BYTES_PER_PIXEL;
                sums.iter_mut()
                    .zip(&real[at..])
                    .for_each(|(sum, part)| *sum += usize::from(*part));
            }
            let count = rows.len() * columns.len();
            picture.extend(sums.map(|sum| u8::try_from(sum / count).expect("a mean of bytes")));
        }
    }
    picture
}

proptest! {
    /// After any writes, what a client has drawn from the view's changes is the picture of
    /// the whole device screen: no changed pixel at an edge is left out.
    #[test]
    fn the_picture_follows_every_change(
        device_size in (1..90usize, 1..90usize),
        client_size in (1..100usize, 1..100usize),
        writes in prop::collection::vec(((0..90usize, 0..90usize), (1..40usize, 1..40usize), any::<u8>()), 1..8),
    ) {
        let mut device = Screen::new(device_size.0, device_size.1).expect("a valid size");
        let mut view = View::new(&device, client_size);
        let mut client = Drawn::default();
        send(&mut view, &mut device, &mut client);
        for ((x, y), (width, height), seed) in writes {
            let (x, y) = (x % device_size.0, y % device_size.1);
            let area = Area { x, y, width: width.min(device_size.0 - x), height: height.min(device_size.1 - y) };
            let pixels: Vec<u8> = (0..area.width * area.height * BYTES_PER_PIXEL)
                .map(|at| seed.wrapping_mul(31).wrapping_add(at.to_le_bytes()[0]))
                .collect();
            device.write(area, &pixels).expect("inside");
            send(&mut view, &mut device, &mut client);
            let size = view.size(&device);
            prop_assert!(size.0 <= client_size.0.max(1) || size == device_size);
            prop_assert_eq!(&client.rgb, &whole_picture(&device, size));
        }
    }
}
