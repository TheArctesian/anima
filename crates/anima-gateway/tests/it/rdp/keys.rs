//! Keysyms from the client become the keys of a US keyboard, or Unicode characters.

use anima_gateway::rdp::{KeyEvent, Keyboard};
use proptest::prelude::*;

const SHIFT: u32 = 0xffe1;
const CONTROL: u32 = 0xffe3;

fn key(code: u8, pressed: bool) -> KeyEvent {
    KeyEvent::Scancode {
        code,
        extended: false,
        pressed,
    }
}

fn extended(code: u8, pressed: bool) -> KeyEvent {
    KeyEvent::Scancode {
        code,
        extended: true,
        pressed,
    }
}

fn unicode(character: char, pressed: bool) -> KeyEvent {
    let unit = u16::try_from(u32::from(character)).expect("one unit");
    KeyEvent::Unicode { unit, pressed }
}

/// The scancodes of `text` typed on a new keyboard, each key pressed alone.
fn typed(text: &str) -> Vec<KeyEvent> {
    let mut keyboard = Keyboard::default();
    text.chars()
        .flat_map(|character| keyboard.key(u32::from(character), true))
        .collect()
}

#[test]
fn each_plain_key_of_the_us_keyboard_is_its_scancode() {
    let rows = [
        (
            "`1234567890-=",
            vec![0x29, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13],
        ),
        (
            "qwertyuiop[]\\",
            vec![16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 0x2b],
        ),
        (
            "asdfghjkl;'",
            vec![30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40],
        ),
        (
            "zxcvbnm,./ ",
            vec![44, 45, 46, 47, 48, 49, 50, 51, 52, 53, 0x39],
        ),
    ];
    for (row, codes) in rows {
        let expected: Vec<_> = codes.into_iter().map(|code| key(code, true)).collect();
        assert_eq!(typed(row), expected, "{row}");
    }
}

#[test]
fn a_shifted_character_is_its_key_while_shift_is_down() {
    let mut keyboard = Keyboard::default();

    assert_eq!(keyboard.key(SHIFT, true), [key(0x2a, true)]);
    assert_eq!(keyboard.key(u32::from('@'), true), [key(0x03, true)]);
    assert_eq!(keyboard.key(u32::from('@'), false), [key(0x03, false)]);
    assert_eq!(keyboard.key(u32::from('Q'), true), [key(0x10, true)]);
    assert_eq!(keyboard.key(u32::from('~'), true), [key(0x29, true)]);
    assert_eq!(keyboard.key(u32::from('|'), true), [key(0x2b, true)]);
    assert_eq!(keyboard.key(SHIFT, false), [key(0x2a, false)]);
}

#[test]
fn a_character_whose_key_needs_another_shift_state_goes_as_unicode() {
    let mut keyboard = Keyboard::default();
    // Caps Lock on the client: a capital with no Shift down.
    assert_eq!(keyboard.key(u32::from('A'), true), [unicode('A', true)]);
    assert_eq!(keyboard.key(u32::from('A'), false), [unicode('A', false)]);
    // Another layout: `2` is made with Shift there.
    keyboard.key(SHIFT, true);
    assert_eq!(keyboard.key(u32::from('2'), true), [unicode('2', true)]);
}

#[test]
fn a_key_is_released_the_way_it_was_pressed_whatever_shift_does_meanwhile() {
    let mut keyboard = Keyboard::default();
    keyboard.key(SHIFT, true);
    keyboard.key(u32::from('A'), true);
    keyboard.key(SHIFT, false);

    assert_eq!(keyboard.key(u32::from('A'), false), [key(0x1e, false)]);
}

#[test]
fn characters_outside_the_table_go_as_unicode() {
    let mut keyboard = Keyboard::default();

    assert_eq!(keyboard.key(0xf6, true), [unicode('ö', true)]);
    assert_eq!(
        keyboard.key(0x0100_0000 + 0x20ac, true),
        [unicode('€', true)]
    );
    // Outside the Basic Multilingual Plane: two units.
    assert_eq!(
        keyboard.key(0x0100_0000 + 0x1f600, true),
        [
            KeyEvent::Unicode {
                unit: 0xd83d,
                pressed: true
            },
            KeyEvent::Unicode {
                unit: 0xde00,
                pressed: true
            },
        ]
    );
}

#[test]
fn a_shortcut_sends_the_key_whatever_shift_state_the_character_needs() {
    let mut keyboard = Keyboard::default();
    keyboard.key(CONTROL, true);

    // Control and C, with Caps Lock on at the client.
    assert_eq!(keyboard.key(u32::from('C'), true), [key(0x2e, true)]);
    // A character with no US key still goes as itself.
    assert_eq!(keyboard.key(0xf6, true), [unicode('ö', true)]);
}

#[test]
fn keys_that_are_not_characters_have_their_scancodes() {
    let mut keyboard = Keyboard::default();
    let mut press = |keysym| keyboard.key(keysym, true);

    assert_eq!(press(0xff0d), [key(0x1c, true)]); // Return
    assert_eq!(press(0xff1b), [key(0x01, true)]); // Escape
    assert_eq!(press(0xff08), [key(0x0e, true)]); // BackSpace
    assert_eq!(press(0xffbe), [key(0x3b, true)]); // F1
    assert_eq!(press(0xffc7), [key(0x44, true)]); // F10
    assert_eq!(press(0xffc9), [key(0x58, true)]); // F12
    assert_eq!(press(0xffff), [extended(0x53, true)]); // Delete
    assert_eq!(press(0xff51), [extended(0x4b, true)]); // Left
    assert_eq!(press(0xffe4), [extended(0x1d, true)]); // Control_R
    assert_eq!(press(0xffeb), [extended(0x5b, true)]); // Super_L
    assert_eq!(press(0xffb9), [key(0x49, true)]); // KP_9
    assert_eq!(press(0xffaf), [extended(0x35, true)]); // KP_Divide
}

#[test]
fn the_lock_keys_and_unknown_keysyms_send_nothing() {
    let mut keyboard = Keyboard::default();

    for keysym in [
        0xffe5,
        0xff7f,
        0xfe03,
        0x0,
        0x7f,
        0x0100_0000 + 0x07,
        0x0111_0000,
    ] {
        assert_eq!(keyboard.key(keysym, true), [], "{keysym:#x}");
        assert_eq!(keyboard.key(keysym, false), [], "{keysym:#x}");
    }
}

proptest! {
    /// Whatever the client sends, each release answers its press in kind, and no more
    /// keys are remembered than the limit.
    #[test]
    fn any_keys_are_released_as_they_were_pressed(
        keys in proptest::collection::vec((prop_oneof![0x20u32..0x100, 0xff00u32..0x1_0000], any::<bool>()), 0..200)
    ) {
        let mut keyboard = Keyboard::default();
        let mut down: Vec<(u32, Vec<KeyEvent>)> = Vec::new();
        for (keysym, pressed) in keys {
            let events = keyboard.key(keysym, pressed);
            let held = down.iter().position(|(held, _)| *held == keysym);
            match (pressed, held) {
                (true, None) if down.len() < 32 => down.push((keysym, events)),
                (false, Some(at)) => {
                    let (_, pressed_as) = down.swap_remove(at);
                    let released: Vec<_> = pressed_as.into_iter().map(|event| match event {
                        KeyEvent::Scancode { code, extended, .. } => KeyEvent::Scancode { code, extended, pressed: false },
                        KeyEvent::Unicode { unit, .. } => KeyEvent::Unicode { unit, pressed: false },
                    }).collect();
                    prop_assert_eq!(events, released);
                }
                _ => {}
            }
        }
    }
}
