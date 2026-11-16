//! The gateway's key and pointer events as RDP fast-path input events.

use ironrdp_pdu::input::MousePdu;
use ironrdp_pdu::input::fast_path::{FastPathInputEvent, KeyboardFlags, SynchronizeFlags};
use ironrdp_pdu::input::mouse::PointerFlags;

use super::keys::KeyEvent;
use super::pointer::{Button, PointerEvent};

/// The device's locks as the key table needs them: Num Lock on, so the keypad makes
/// digits, and Caps Lock off, so a key makes the character the table says.
pub(super) fn locks() -> FastPathInputEvent {
    FastPathInputEvent::SyncEvent(SynchronizeFlags::NUM_LOCK)
}

pub(super) fn key(event: KeyEvent) -> FastPathInputEvent {
    let flags = |pressed: bool, extended: bool| {
        let mut flags = KeyboardFlags::empty();
        flags.set(KeyboardFlags::RELEASE, !pressed);
        flags.set(KeyboardFlags::EXTENDED, extended);
        flags
    };
    match event {
        KeyEvent::Scancode {
            code,
            extended,
            pressed,
        } => FastPathInputEvent::KeyboardEvent(flags(pressed, extended), code),
        KeyEvent::Unicode { unit, pressed } => {
            FastPathInputEvent::UnicodeKeyboardEvent(flags(pressed, false), unit)
        }
    }
}

pub(super) fn pointer(event: PointerEvent, (x, y): (u16, u16)) -> FastPathInputEvent {
    let (flags, wheel) = match event {
        PointerEvent::Move => (PointerFlags::MOVE, 0),
        PointerEvent::Button { button, pressed } => {
            let mut flags = match button {
                Button::Left => PointerFlags::LEFT_BUTTON,
                Button::Middle => PointerFlags::MIDDLE_BUTTON_OR_WHEEL,
                Button::Right => PointerFlags::RIGHT_BUTTON,
            };
            flags.set(PointerFlags::DOWN, pressed);
            (flags, 0)
        }
        PointerEvent::Wheel(units) => (PointerFlags::VERTICAL_WHEEL, units),
    };
    FastPathInputEvent::MouseEvent(MousePdu {
        flags,
        number_of_wheel_rotation_units: wheel,
        x_position: x,
        y_position: y,
    })
}
