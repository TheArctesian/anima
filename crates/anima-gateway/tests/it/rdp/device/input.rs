//! The client's fast-path input, as the test device notes it.

use anima_gateway::rdp::{Button, KeyEvent, PointerEvent};
use ironrdp_core::decode;
use ironrdp_pdu::input::MousePdu;
use ironrdp_pdu::input::fast_path::{FastPathInput, FastPathInputEvent, KeyboardFlags};
use ironrdp_pdu::input::mouse::PointerFlags;

use super::Seen;

/// What one frame says. A frame that is not fast-path input says nothing to the device.
pub(super) fn heard(frame: &[u8]) -> Vec<Seen> {
    let Ok(input) = decode::<FastPathInput>(frame) else {
        return Vec::new();
    };
    input.input_events().iter().filter_map(seen).collect()
}

fn seen(event: &FastPathInputEvent) -> Option<Seen> {
    let pressed = |flags: &KeyboardFlags| !flags.contains(KeyboardFlags::RELEASE);
    match event {
        FastPathInputEvent::KeyboardEvent(flags, code) => Some(Seen::Key(KeyEvent::Scancode {
            code: *code,
            extended: flags.contains(KeyboardFlags::EXTENDED),
            pressed: pressed(flags),
        })),
        FastPathInputEvent::UnicodeKeyboardEvent(flags, unit) => {
            Some(Seen::Key(KeyEvent::Unicode {
                unit: *unit,
                pressed: pressed(flags),
            }))
        }
        FastPathInputEvent::MouseEvent(mouse) => Some(Seen::Pointer {
            x: mouse.x_position,
            y: mouse.y_position,
            event: pointer(*mouse)?,
        }),
        FastPathInputEvent::SyncEvent(locks) => Some(Seen::Locks(locks.bits())),
        // The gateway sends none of these.
        FastPathInputEvent::MouseEventEx(_)
        | FastPathInputEvent::MouseEventRel(_)
        | FastPathInputEvent::QoeEvent(_) => None,
    }
}

/// The pointer event of one mouse PDU: a turn of the vertical wheel, a button, or a move.
/// The horizontal wheel has no event in the gateway.
fn pointer(mouse: MousePdu) -> Option<PointerEvent> {
    let flags = mouse.flags;
    if flags.contains(PointerFlags::VERTICAL_WHEEL) {
        return Some(PointerEvent::Wheel(mouse.number_of_wheel_rotation_units));
    }
    let buttons = [
        (PointerFlags::LEFT_BUTTON, Button::Left),
        (PointerFlags::MIDDLE_BUTTON_OR_WHEEL, Button::Middle),
        (PointerFlags::RIGHT_BUTTON, Button::Right),
    ];
    if let Some((_, button)) = buttons.into_iter().find(|(bit, _)| flags.contains(*bit)) {
        let pressed = flags.contains(PointerFlags::DOWN);
        return Some(PointerEvent::Button { button, pressed });
    }
    flags
        .contains(PointerFlags::MOVE)
        .then_some(PointerEvent::Move)
}
