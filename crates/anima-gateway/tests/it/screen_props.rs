//! A client that redraws only what the screen copy reports always shows the screen.

use anima_gateway::screen::{Area, BYTES_PER_PIXEL, Screen};
use proptest::prelude::*;

use crate::screen::whole;

/// What a client holds: it starts black and draws only the areas it is told about.
struct Client {
    width: usize,
    rgb: Vec<u8>,
}

impl Client {
    fn of(screen: &Screen) -> Self {
        Self {
            width: screen.width(),
            rgb: vec![0; screen.width() * screen.height() * BYTES_PER_PIXEL],
        }
    }

    /// Redraws every changed area from the screen. Returns how often the most-drawn pixel
    /// was drawn in this round.
    fn follow(&mut self, screen: &mut Screen) -> u8 {
        let changes = screen.take_changes();
        if changes.resized {
            *self = Self::of(screen);
        }
        let mut drawn = vec![0u8; self.rgb.len() / BYTES_PER_PIXEL];
        for area in changes.areas {
            let rows = screen.rows(area).expect("a reported area is inside");
            for (row, pixels) in rows.enumerate() {
                let first = (area.y + row) * self.width + area.x;
                let start = first * BYTES_PER_PIXEL;
                self.rgb[start..start + pixels.len()].copy_from_slice(pixels);
                drawn[first..first + area.width]
                    .iter_mut()
                    .for_each(|count| *count += 1);
            }
        }
        drawn.into_iter().max().unwrap_or(0)
    }
}

/// One write a device may send: where, what colour pattern, and whether it only repeats
/// what the screen already shows.
#[derive(Clone, Debug)]
struct Sent {
    corner: (usize, usize),
    size: (usize, usize),
    seed: u8,
    repeat: bool,
}

fn sent() -> impl Strategy<Value = Sent> {
    (
        (0..100usize, 0..100usize),
        (0..100usize, 0..100usize),
        any::<u8>(),
        any::<bool>(),
    )
        .prop_map(|(corner, size, seed, repeat)| Sent {
            corner,
            size,
            seed,
            repeat,
        })
}

impl Sent {
    /// The write, fitted into `screen`.
    fn onto(&self, screen: &Screen) -> (Area, Vec<u8>) {
        let (x, y) = (
            self.corner.0 % screen.width(),
            self.corner.1 % screen.height(),
        );
        let area = Area {
            x,
            y,
            width: self.size.0 % (screen.width() - x + 1),
            height: self.size.1 % (screen.height() - y + 1),
        };
        let pixels = if self.repeat {
            screen
                .rows(area)
                .expect("inside")
                .flatten()
                .copied()
                .collect()
        } else {
            // Two colours in stripes, so neighbouring pixels differ and some stay the same.
            (0..area.width * area.height * BYTES_PER_PIXEL)
                .map(|at| {
                    if at / 7 % 2 == 0 {
                        self.seed
                    } else {
                        self.seed / 2
                    }
                })
                .collect()
        };
        (area, pixels)
    }
}

proptest! {
    /// A client that redraws only the reported areas always shows the screen, and is never
    /// told to draw a pixel twice in one round.
    #[test]
    fn a_client_that_follows_the_changes_shows_the_screen(
        (width, height) in (1..100usize, 1..100usize),
        rounds in prop::collection::vec(prop::collection::vec(sent(), 0..6), 1..6),
    ) {
        let mut screen = Screen::new(width, height).expect("a valid size");
        let mut client = Client::of(&screen);
        for round in rounds {
            for sent in round {
                let (area, pixels) = sent.onto(&screen);
                screen.write(area, &pixels).expect("fitted into the screen");
            }
            let most_drawn = client.follow(&mut screen);
            prop_assert!(most_drawn <= 1, "areas overlap: a pixel was drawn {most_drawn} times");
            prop_assert_eq!(&client.rgb, &whole(&screen));
        }
    }

    /// Sending what the screen already shows is never a change.
    #[test]
    fn repeating_the_screen_changes_nothing(
        (width, height) in (1..100usize, 1..100usize),
        first in prop::collection::vec(sent(), 0..6),
        again in prop::collection::vec(sent(), 0..6),
    ) {
        let mut screen = Screen::new(width, height).expect("a valid size");
        for sent in first {
            let (area, pixels) = sent.onto(&screen);
            screen.write(area, &pixels).expect("fitted into the screen");
        }
        screen.take_changes();
        for sent in again {
            let (area, pixels) = Sent { repeat: true, ..sent }.onto(&screen);
            screen.write(area, &pixels).expect("fitted into the screen");
        }
        prop_assert_eq!(screen.take_changes().areas, []);
    }
}
