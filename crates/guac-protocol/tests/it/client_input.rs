//! The client's display and time zone are checked before anything reaches guacd
//! (GW-012).

use guac_protocol::{Display, HandshakeError, HandshakeStep, is_zone_name};

use crate::guacd::{
    ARGS_1_5_5, DISPLAY, K4_PASSWORD_VALUES, connect_of, decode_args, params, reply, started,
    values_of,
};

fn after_refusal(display: Display, timezone: &str) -> Result<HandshakeStep, HandshakeError> {
    let mut handshake = started(params(display, timezone, values_of(&K4_PASSWORD_VALUES)));
    let first = handshake.on_instruction(decode_args(ARGS_1_5_5));
    assert_eq!(
        handshake.on_instruction(decode_args(ARGS_1_5_5)),
        first,
        "stays failed"
    );
    first
}

#[test]
fn gw_012_display_bounds_are_inclusive() {
    let at = |width, height, dpi| Display { width, height, dpi }.within_bounds();
    assert!(at(
        Display::MIN_SIDE_PX,
        Display::MIN_SIDE_PX,
        Display::MIN_DPI
    ));
    assert!(at(
        Display::MAX_SIDE_PX,
        Display::MAX_SIDE_PX,
        Display::MAX_DPI
    ));
    assert!(!at(0, 768, 96));
    assert!(!at(1024, 0, 96));
    assert!(!at(8193, 768, 96));
    assert!(!at(1024, 8193, 96));
    assert!(!at(1024, 768, 47));
    assert!(!at(1024, 768, 481));
}

#[test]
fn gw_012_display_out_of_bounds_is_refused_before_anything_is_sent() {
    let displays = [
        Display {
            width: 0,
            ..DISPLAY
        },
        Display {
            height: 8193,
            ..DISPLAY
        },
        Display { dpi: 47, ..DISPLAY },
        Display {
            dpi: 481,
            ..DISPLAY
        },
    ];
    for display in displays {
        assert_eq!(
            after_refusal(display, "UTC"),
            Err(HandshakeError::InvalidDisplay)
        );
    }
}

#[test]
fn gw_012_zone_names_are_sent_as_the_timezone() {
    let zones = [
        "UTC",
        "Europe/London",
        "America/Argentina/Buenos_Aires",
        "Etc/GMT+5",
        "Etc/GMT-14",
    ];
    let longest = "a".repeat(64);
    for zone in zones.into_iter().chain([longest.as_str()]) {
        assert!(is_zone_name(zone), "{zone:?}");
        let send = reply(
            decode_args(ARGS_1_5_5),
            params(DISPLAY, zone, values_of(&K4_PASSWORD_VALUES)),
        );
        assert_eq!(send[4].opcode, "timezone");
        assert_eq!(send[4].args, [zone]);
        assert_eq!(connect_of(&send).opcode, "connect");
    }
}

#[test]
fn gw_012_time_zone_outside_the_zone_charset_is_refused() {
    let too_long = "a".repeat(65);
    let refused = [
        "",
        too_long.as_str(),
        "UTC\r\n",
        "UTC\n4.size,1.1,1.1,2.96;",
        "\u{1b}[2J",
        "Europe/ London",
        " UTC",
        ":/etc/shadow",
        "UTC:/etc/shadow",
        "4.port,2.22;",
        "Europe/Zürich",
        "東京",
        "UTC\0",
        "../etc/passwd",
        "Etc/../UTC",
        ".",
        "/etc/shadow",
        "/dev/stdin",
        "/",
        "//",
        "Europe//London",
        "Europe/",
        "-UTC",
        "+05",
        "_x",
        "5Etc/UTC",
    ];
    for zone in refused {
        assert!(!is_zone_name(zone), "{zone:?}");
        assert_eq!(
            after_refusal(DISPLAY, zone),
            Err(HandshakeError::InvalidTimezone),
            "{zone:?}"
        );
    }
}
