//! Keys for an RDP device. Guacamole's `key` carries an X11 keysym; RDP wants the scancode
//! of a key on the device's keyboard, or a Unicode key event for a character.
//!
//! The table is the US keyboard: the layout guacd uses when the hub sets no
//! `server-layout`, which it never does. A character goes as its key when the table has it
//! and Shift is as that key needs it; any other character goes as a Unicode event, so text
//! is right for every layout of the client. With Control, Alt or the logo key down a
//! character always goes as its key, so shortcuts reach the device.
//!
//! Caps Lock and Num Lock are not sent: the client's keysym already says which character
//! is meant, and the session sets the device's locks once at its start.
//!
//! Pure: no I/O.

/// One key event for the device.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyEvent {
    /// A key by its scancode (set 1); `extended` keys have the E0 prefix.
    Scancode {
        code: u8,
        extended: bool,
        pressed: bool,
    },
    /// A character as one UTF-16 unit.
    Unicode { unit: u16, pressed: bool },
}

/// How a key went to the device, so its release goes the same way.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Sent {
    Scancode { code: u8, extended: bool },
    Unicode(char),
    Nothing,
}

/// The most keys held down at once that are remembered. A release that is not remembered
/// is worked out again as a press is.
const MOST_HELD: usize = 32;

const SHIFT_LEFT: u32 = 0xffe1;
const SHIFT_RIGHT: u32 = 0xffe2;
/// Control, Alt, Meta and Super, left and right: the keys that make a shortcut.
const SHORTCUT_KEYS: [u32; 8] = [
    0xffe3, 0xffe4, 0xffe9, 0xffea, 0xffe7, 0xffe8, 0xffeb, 0xffec,
];
/// Keysyms above this are Unicode code points plus this.
const UNICODE_KEYSYMS: u32 = 0x0100_0000;

/// The keys that are down, as the device knows them.
#[derive(Default)]
pub struct Keyboard {
    held: Vec<(u32, Sent)>,
}

impl Keyboard {
    /// The events for one `key` instruction: none for a keysym with no meaning here, two
    /// for a character outside the Basic Multilingual Plane.
    pub fn key(&mut self, keysym: u32, pressed: bool) -> Vec<KeyEvent> {
        let was = self.held.iter().position(|(held, _)| *held == keysym);
        let sent = match (pressed, was) {
            // A key that repeats is sent as it was first sent.
            (_, Some(at)) => self.held[at].1,
            (_, None) => self.how(keysym),
        };
        match (pressed, was) {
            (true, None) if self.held.len() < MOST_HELD => self.held.push((keysym, sent)),
            (false, Some(at)) => {
                self.held.swap_remove(at);
            }
            _ => {}
        }
        events(sent, pressed)
    }

    fn is_down(&self, keysyms: &[u32]) -> bool {
        self.held.iter().any(|(held, _)| keysyms.contains(held))
    }

    /// How `keysym` goes to the device with the keys that are down now.
    fn how(&self, keysym: u32) -> Sent {
        if let Some((code, extended)) = named(keysym) {
            return Sent::Scancode { code, extended };
        }
        let character = character(keysym);
        let key = character.and_then(us_key);
        let shift = self.is_down(&[SHIFT_LEFT, SHIFT_RIGHT]);
        match (key, character) {
            (Some((code, shifted)), _) if shifted == shift || self.is_down(&SHORTCUT_KEYS) => {
                Sent::Scancode {
                    code,
                    extended: false,
                }
            }
            (_, Some(character)) => Sent::Unicode(character),
            (_, None) => Sent::Nothing,
        }
    }
}

fn events(sent: Sent, pressed: bool) -> Vec<KeyEvent> {
    match sent {
        Sent::Scancode { code, extended } => vec![KeyEvent::Scancode {
            code,
            extended,
            pressed,
        }],
        Sent::Unicode(character) => character
            .encode_utf16(&mut [0; 2])
            .iter()
            .map(|unit| KeyEvent::Unicode {
                unit: *unit,
                pressed,
            })
            .collect(),
        Sent::Nothing => Vec::new(),
    }
}

/// The character a keysym stands for: Latin-1 keysyms are their code points, and the
/// client sends every other character as its code point plus [`UNICODE_KEYSYMS`].
fn character(keysym: u32) -> Option<char> {
    let code_point = match keysym {
        0x20..=0x7e | 0xa0..=0xff => keysym,
        UNICODE_KEYSYMS.. => keysym - UNICODE_KEYSYMS,
        _ => return None,
    };
    char::from_u32(code_point).filter(|character| !character.is_control())
}

/// The scancode of the US key that makes `character`, and whether with Shift.
fn us_key(character: char) -> Option<(u8, bool)> {
    /// Each row of the keyboard without Shift and with it, and its first scancode.
    const ROWS: [(&str, &str, u8); 5] = [
        ("1234567890-=", "!@#$%^&*()_+", 0x02),
        ("qwertyuiop[]", "QWERTYUIOP{}", 0x10),
        ("asdfghjkl;'`", "ASDFGHJKL:\"~", 0x1e),
        ("\\zxcvbnm,./", "|ZXCVBNM<>?", 0x2b),
        (" ", "", 0x39),
    ];
    ROWS.iter().find_map(|(plain, shifted, first)| {
        let at = |row: &str| row.chars().position(|key| key == character);
        let (at, shift) = at(plain)
            .map(|at| (at, false))
            .or_else(|| at(shifted).map(|at| (at, true)))?;
        Some((first + u8::try_from(at).ok()?, shift))
    })
}

/// The scancode of a key that is not a character, and whether it is an extended key.
fn named(keysym: u32) -> Option<(u8, bool)> {
    /// Keys with one scancode byte.
    const PLAIN: [(u32, u8); 23] = [
        (0xff08, 0x0e), // BackSpace
        (0xff09, 0x0f), // Tab
        (0xfe20, 0x0f), // ISO_Left_Tab: Tab with Shift down
        (0xff0d, 0x1c), // Return
        (0xff1b, 0x01), // Escape
        (0xff14, 0x46), // Scroll_Lock
        (0xffc8, 0x57), // F11
        (0xffc9, 0x58), // F12
        (SHIFT_LEFT, 0x2a),
        (SHIFT_RIGHT, 0x36),
        (0xffe3, 0x1d), // Control_L
        (0xffe9, 0x38), // Alt_L
        (0xffaa, 0x37), // KP_Multiply
        (0xffab, 0x4e), // KP_Add
        (0xffad, 0x4a), // KP_Subtract
        (0xffae, 0x53), // KP_Decimal
        (0xffb0, 0x52), // KP_0
        (0xffb1, 0x4f), // KP_1
        (0xffb2, 0x50), // KP_2
        (0xffb3, 0x51), // KP_3
        (0xffb4, 0x4b), // KP_4
        (0xffb5, 0x4c), // KP_5
        (0xffb6, 0x4d), // KP_6
    ];
    /// Keys with the E0 prefix.
    const EXTENDED: [(u32, u8); 19] = [
        (0xffff, 0x53), // Delete
        (0xff50, 0x47), // Home
        (0xff51, 0x4b), // Left
        (0xff52, 0x48), // Up
        (0xff53, 0x4d), // Right
        (0xff54, 0x50), // Down
        (0xff55, 0x49), // Page_Up
        (0xff56, 0x51), // Page_Down
        (0xff57, 0x4f), // End
        (0xff63, 0x52), // Insert
        (0xff61, 0x37), // Print
        (0xff67, 0x5d), // Menu
        (0xffe4, 0x1d), // Control_R
        (0xffea, 0x38), // Alt_R
        (0xffe7, 0x5b), // Meta_L
        (0xffeb, 0x5b), // Super_L
        (0xffe8, 0x5c), // Meta_R
        (0xffec, 0x5c), // Super_R
        (0xff8d, 0x1c), // KP_Enter
    ];
    let find = |keys: &[(u32, u8)]| keys.iter().find(|(key, _)| *key == keysym).map(|key| key.1);
    let low = |from: u32| u8::try_from(keysym - from).ok();
    match keysym {
        // F1 to F10.
        0xffbe..=0xffc7 => Some((0x3b + low(0xffbe)?, false)),
        // KP_7 to KP_9.
        0xffb7..=0xffb9 => Some((0x47 + low(0xffb7)?, false)),
        0xffaf => Some((0x35, true)), // KP_Divide
        _ => find(&PLAIN)
            .map(|code| (code, false))
            .or_else(|| find(&EXTENDED).map(|code| (code, true))),
    }
}
