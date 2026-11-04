//! Client `size` bounds and number parsing (K4 Section 4, R-018, WP-08 H-6).

use guac_protocol::Instruction;

use super::super::relay::{ClientStep, Relay};

fn size(args: &[&str]) -> ClientStep {
    Relay::default().on_client(Instruction::new("size", args.iter().copied()))
}

fn sized(args: &[&str]) -> ClientStep {
    ClientStep::Forward(Instruction::new("size", args.iter().copied()))
}

#[test]
fn k4_size_is_forwarded_within_bounds_with_or_without_dpi() {
    assert_eq!(size(&["1024", "768"]), sized(&["1024", "768"]));
    assert_eq!(size(&["1", "1"]), sized(&["1", "1"]));
    assert_eq!(size(&["8192", "8192"]), sized(&["8192", "8192"]));
    assert_eq!(size(&["1024", "768", "48"]), sized(&["1024", "768", "48"]));
    assert_eq!(
        size(&["1024", "768", "480"]),
        sized(&["1024", "768", "480"])
    );
}

#[test]
fn k4_size_leading_zeros_are_forwarded_as_plain_numbers() {
    assert_eq!(size(&["01024", "0768"]), sized(&["1024", "768"]));
    assert_eq!(
        size(&["001024", "768"]),
        ClientStep::Drop,
        "more than five digits"
    );
    assert_eq!(size(&["1024", "000768"]), ClientStep::Drop);
}

#[test]
fn k4_size_out_of_bounds_or_not_plain_digits_is_dropped() {
    for args in [
        &["0", "768"][..],
        &["1024", "8193"],
        &["1024", "768", "47"],
        &["1024", "768", "481"],
        &["+1024", "768"],
        &["-1", "768"],
        &["", "768"],
        &[" 1024", "768"],
        &["1024 ", "768"],
        &["0x400", "768"],
        &["１０２４", "768"],
        &["1024"],
        &[],
        &["1024", "768", "96", "1"],
    ] {
        assert_eq!(size(args), ClientStep::Drop, "{args:?}");
    }
}
