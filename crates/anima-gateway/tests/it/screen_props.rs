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

/// One thing a device may do to its screen.
#[derive(Clone, Debug)]
enum Step {
    Write(Sent),
    /// Move the area of `from` so that its corner is at `to` (fitted into the screen).
    Move {
        from: Sent,
        to: (usize, usize),
    },
    Resize(usize, usize),
}

fn step() -> impl Strategy<Value = Step> {
    prop_oneof![
        6 => sent().prop_map(Step::Write),
        2 => (sent(), (0..100usize, 0..100usize)).prop_map(|(from, to)| Step::Move { from, to }),
        1 => (1..100usize, 1..100usize).prop_map(|(width, height)| Step::Resize(width, height)),
    ]
}

/// The screen as a plain picture, changed in the simplest way: what the copy must equal.
struct Picture {
    width: usize,
    height: usize,
    rgb: Vec<u8>,
}

impl Picture {
    fn black(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            rgb: vec![0; width * height * BYTES_PER_PIXEL],
        }
    }

    fn rows(&self, area: Area) -> Vec<u8> {
        (area.y..area.y + area.height)
            .flat_map(|y| {
                let start = (y * self.width + area.x) * BYTES_PER_PIXEL;
                self.rgb[start..start + area.width * BYTES_PER_PIXEL]
                    .iter()
                    .copied()
            })
            .collect()
    }

    fn draw(&mut self, area: Area, pixels: &[u8]) {
        if area.width == 0 {
            return;
        }
        for (row, new) in pixels
            .chunks_exact(area.width * BYTES_PER_PIXEL)
            .enumerate()
        {
            let start = ((area.y + row) * self.width + area.x) * BYTES_PER_PIXEL;
            self.rgb[start..start + new.len()].copy_from_slice(new);
        }
    }
}

/// Does `step` to the screen copy and to the picture.
fn apply(step: &Step, screen: &mut Screen, picture: &mut Picture) {
    match step {
        Step::Write(sent) => {
            let (area, pixels) = sent.onto(screen);
            screen.write(area, &pixels).expect("fitted into the screen");
            picture.draw(area, &pixels);
        }
        Step::Move { from, to } => {
            let (from, _) = from.onto(screen);
            let x = to.0 % (screen.width() - from.width + 1);
            let y = to.1 % (screen.height() - from.height + 1);
            screen
                .move_area(from, x, y)
                .expect("fitted into the screen");
            let moved = picture.rows(from);
            picture.draw(Area { x, y, ..from }, &moved);
        }
        Step::Resize(width, height) => {
            screen.resize(*width, *height).expect("a valid size");
            if (*width, *height) != (picture.width, picture.height) {
                *picture = Picture::black(*width, *height);
            }
        }
    }
}

proptest! {
    /// Whatever a device does, the copy holds the device's picture, a client that redraws
    /// only the reported areas shows it too, and no pixel is drawn twice in one round.
    #[test]
    fn a_client_that_follows_the_changes_shows_the_screen(
        (width, height) in (1..100usize, 1..100usize),
        rounds in prop::collection::vec(prop::collection::vec(step(), 0..6), 1..6),
    ) {
        let mut screen = Screen::new(width, height).expect("a valid size");
        let mut picture = Picture::black(width, height);
        let mut client = Client::of(&screen);
        for round in rounds {
            for step in &round {
                apply(step, &mut screen, &mut picture);
            }
            prop_assert_eq!(&whole(&screen), &picture.rgb);
            let most_drawn = client.follow(&mut screen);
            prop_assert!(most_drawn <= 1, "areas overlap: a pixel was drawn {most_drawn} times");
            prop_assert_eq!(&client.rgb, &picture.rgb);
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
