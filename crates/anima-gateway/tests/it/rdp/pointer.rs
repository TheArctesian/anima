//! The client's button mask becomes one RDP event for each change.

use anima_gateway::rdp::{Button, Pointer, PointerEvent};

fn button(button: Button, pressed: bool) -> PointerEvent {
    PointerEvent::Button { button, pressed }
}

#[test]
fn a_move_with_no_change_of_buttons_is_only_a_move() {
    let mut pointer = Pointer::default();

    assert_eq!(pointer.moved(0), [PointerEvent::Move]);
    assert_eq!(
        pointer.moved(1),
        [PointerEvent::Move, button(Button::Left, true)]
    );
    // A drag: the button stays down and is not sent again.
    assert_eq!(pointer.moved(1), [PointerEvent::Move]);
    assert_eq!(
        pointer.moved(0),
        [PointerEvent::Move, button(Button::Left, false)]
    );
}

#[test]
fn each_button_of_the_mask_is_its_rdp_button() {
    let mut pointer = Pointer::default();

    assert_eq!(
        pointer.moved(2),
        [PointerEvent::Move, button(Button::Middle, true)]
    );
    assert_eq!(
        pointer.moved(4),
        [
            PointerEvent::Move,
            button(Button::Middle, false),
            button(Button::Right, true),
        ]
    );
}

#[test]
fn a_wheel_bit_is_one_step_when_it_goes_down_and_nothing_when_it_goes_up() {
    let mut pointer = Pointer::default();

    assert_eq!(
        pointer.moved(8),
        [PointerEvent::Move, PointerEvent::Wheel(120)]
    );
    assert_eq!(pointer.moved(0), [PointerEvent::Move]);
    assert_eq!(
        pointer.moved(16),
        [PointerEvent::Move, PointerEvent::Wheel(-120)]
    );
    assert_eq!(pointer.moved(16), [PointerEvent::Move]);
    // Bits above the wheel have no meaning.
    assert_eq!(pointer.moved(0xe0), [PointerEvent::Move]);
}
