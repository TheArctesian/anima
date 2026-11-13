//! The byte counts of a transfer (K4 Section 9): an upload takes exactly its size, a
//! download reads at most what is left and fails when the device's file does not match.

use super::device_path;
use crate::fleet::events::Severity;
use crate::fleet::sessions::files::{
    trail::sentence,
    transfer::{CHUNK, Direction, Mismatch, Outcome, Overrun, Transfer},
};

fn upload(size: u64) -> Transfer {
    Transfer::new(Direction::Upload, device_path("/a"), size)
}

#[test]
fn ra_032_an_upload_takes_exactly_its_size() {
    let mut transfer = upload(10);
    assert_eq!(transfer.take(4), Ok(0));
    assert_eq!(transfer.take(6), Ok(4));
    assert!(transfer.is_complete());
    assert_eq!(transfer.take(1), Err(Overrun));
    assert_eq!(transfer.moved, 10, "an overrun counts nothing");
    assert_eq!(transfer.take(0), Ok(10));
}

#[test]
fn ra_032_one_frame_past_the_size_is_an_overrun() {
    let mut transfer = upload(10);
    assert_eq!(transfer.take(11), Err(Overrun));
    assert_eq!(transfer.moved, 0);
    assert!(!transfer.is_complete());
    assert_eq!(upload(0).take(1), Err(Overrun));
}

#[test]
fn ra_032_a_download_reads_at_most_what_is_left() {
    let mut transfer = Transfer::new(Direction::Download, device_path("/a"), 40_000);
    assert_eq!(transfer.next_read(), CHUNK);
    assert_eq!(transfer.add_read(32_768, CHUNK), Ok(()));
    assert_eq!(transfer.remaining(), 7_232);
    assert_eq!(transfer.next_read(), 7_232);
    assert_eq!(transfer.add_read(7_232, 7_232), Ok(()));
    assert!(transfer.is_complete());
    let huge = Transfer::new(Direction::Download, device_path("/a"), u64::MAX);
    assert_eq!(huge.next_read(), CHUNK);
}

#[test]
fn ra_032_a_device_that_does_not_match_its_size_fails_the_download() {
    let mut transfer = Transfer::new(Direction::Download, device_path("/a"), 100);
    assert_eq!(transfer.add_read(51, 50), Err(Mismatch), "more than asked");
    assert_eq!(transfer.add_read(0, 50), Err(Mismatch), "ended early");
    assert_eq!(transfer.add_read(100, 100), Ok(()));
    let mut short = Transfer::new(Direction::Download, device_path("/a"), 10);
    assert_eq!(
        short.add_read(11, CHUNK),
        Err(Mismatch),
        "more than is left"
    );
    assert_eq!(short.moved, 0);
}

#[test]
fn log_007_event_sentences_are_fixed_and_failures_warn() {
    for direction in [Direction::Upload, Direction::Download] {
        for outcome in [
            Outcome::Done,
            Outcome::Refused,
            Outcome::Cancelled,
            Outcome::Failed,
        ] {
            let (severity, text) = sentence(direction, outcome);
            let expected = if outcome == Outcome::Failed {
                Severity::Warning
            } else {
                Severity::Info
            };
            assert_eq!(severity, expected);
            assert!(text.ends_with('.') && !text.contains('/'), "{text}");
        }
    }
    assert_eq!(Outcome::Cancelled.as_str(), "cancelled");
}
