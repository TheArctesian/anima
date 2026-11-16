//! The pointer for an RDP device. Guacamole's `mouse` carries the position and a mask of
//! the buttons that are down, with the wheel as two buttons that go down for one step. RDP
//! wants an event for each change.
//!
//! Pure: no I/O.

/// A mouse button RDP knows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Button {
    Left,
    Middle,
    Right,
}

/// One pointer event for the device, at the position of its `mouse` instruction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PointerEvent {
    Move,
    Button {
        button: Button,
        pressed: bool,
    },
    /// The wheel by this many units: 120 is one step away from the user.
    Wheel(i16),
}

/// Guacamole's mask bits, in order: left, middle, right, wheel up, wheel down.
const BUTTONS: [(u8, Button); 3] = [
    (0x01, Button::Left),
    (0x02, Button::Middle),
    (0x04, Button::Right),
];
const WHEEL_UP: u8 = 0x08;
const WHEEL_DOWN: u8 = 0x10;
/// One step of the wheel, in RDP's units.
const WHEEL_STEP: i16 = 120;

/// The buttons that are down, as the device knows them.
#[derive(Default)]
pub struct Pointer {
    mask: u8,
}

impl Pointer {
    /// The events for one `mouse` instruction with this button mask: the move, then each
    /// button that changed and each step of the wheel.
    pub fn moved(&mut self, mask: u8) -> Vec<PointerEvent> {
        let changed = self.mask ^ mask;
        self.mask = mask;
        let mut events = vec![PointerEvent::Move];
        for (bit, button) in BUTTONS {
            if changed & bit != 0 {
                events.push(PointerEvent::Button {
                    button,
                    pressed: mask & bit != 0,
                });
            }
        }
        // A wheel bit goes down for a step and up again: the step is the going down.
        for (bit, units) in [(WHEEL_UP, WHEEL_STEP), (WHEEL_DOWN, -WHEEL_STEP)] {
            if changed & mask & bit != 0 {
                events.push(PointerEvent::Wheel(units));
            }
        }
        events
    }
}
