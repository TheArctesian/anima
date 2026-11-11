//! The queue between the relay and the disk, pure (K4 §11): parts of 64 KiB, the 5-second
//! flush, the 64-part bound and the size limit.

use std::time::Duration;

use proptest::prelude::*;
use tokio::time::Instant;

use crate::sessions::recording::{
    format::MAX_PART,
    model::StopReason,
    recorder::{FLUSH_AFTER, Job, Next, Pushed, Queue},
};

const LARGE: u64 = u64::MAX;

fn part(next: Next) -> Vec<u8> {
    match next {
        Next::Write(Job::Part(part)) => part.to_vec(),
        other => panic!("expected a part, got {}", describe(&other)),
    }
}

fn last(next: Next) -> (Vec<u8>, Option<StopReason>) {
    match next {
        Next::Write(Job::Last { tail, stopped }) => (tail.to_vec(), stopped),
        other => panic!("expected the last part, got {}", describe(&other)),
    }
}

fn describe(next: &Next) -> String {
    match next {
        Next::Write(Job::Part(part)) => format!("a part of {} bytes", part.len()),
        Next::Write(Job::Last { tail, stopped }) => {
            format!("the last part of {} bytes, {stopped:?}", tail.len())
        }
        Next::Wait(due) => format!("a wait until {due:?}"),
    }
}

#[test]
fn k4_11_a_part_is_ready_at_64_kib_without_waiting() {
    let start = Instant::now();
    let mut queue = Queue::new(LARGE);
    assert_eq!(
        queue.push(&vec![1; MAX_PART - 1], start),
        Pushed::Kept { wake: true }
    );
    assert!(matches!(queue.next(start), Next::Wait(Some(due)) if due == start + FLUSH_AFTER));
    assert_eq!(queue.push(&[2], start), Pushed::Kept { wake: true });
    let written = part(queue.next(start));
    assert_eq!(written.len(), MAX_PART);
    assert_eq!(written.last(), Some(&2));
    assert!(
        matches!(queue.next(start), Next::Wait(None)),
        "nothing left"
    );
}

#[test]
fn k4_11_the_open_part_is_written_5_seconds_after_its_first_byte() {
    let start = Instant::now();
    let mut queue = Queue::new(LARGE);
    queue.push(b"first", start);
    assert_eq!(
        queue.push(b"second", start + Duration::from_secs(3)),
        Pushed::Kept { wake: false },
        "the writer already waits for this part"
    );
    let almost = start + FLUSH_AFTER - Duration::from_nanos(1);
    assert!(matches!(queue.next(almost), Next::Wait(Some(_))));
    assert_eq!(part(queue.next(start + FLUSH_AFTER)), b"firstsecond");
    assert!(matches!(queue.next(start + FLUSH_AFTER), Next::Wait(None)));
}

#[test]
fn k4_11_bytes_after_a_full_part_start_a_new_timer() {
    let start = Instant::now();
    let later = start + Duration::from_secs(4);
    let mut queue = Queue::new(LARGE);
    queue.push(b"a", start);
    queue.push(&vec![b'b'; MAX_PART], later);
    assert_eq!(part(queue.next(later)).len(), MAX_PART);
    assert!(matches!(queue.next(later), Next::Wait(Some(due)) if due == later + FLUSH_AFTER));
    assert_eq!(part(queue.next(later + FLUSH_AFTER)), b"b");
}

#[test]
fn k4_11_a_65th_ready_part_stops_with_write_failed_and_drops_the_push() {
    let now = Instant::now();
    let mut queue = Queue::new(LARGE);
    // 64 is K4 §11's bound, written out so a changed constant fails here.
    for _ in 0..64 {
        assert!(matches!(
            queue.push(&vec![7; MAX_PART], now),
            Pushed::Kept { .. }
        ));
    }
    queue.push(b"tail", now);
    assert_eq!(
        queue.push(&vec![8; MAX_PART], now),
        Pushed::Stopped(StopReason::WriteFailed)
    );
    assert_eq!(queue.push(b"later", now), Pushed::Dropped);
    for _ in 0..64 {
        assert_eq!(part(queue.next(now)), vec![7; MAX_PART]);
    }
    assert_eq!(
        last(queue.next(now)),
        (b"tail".to_vec(), Some(StopReason::WriteFailed)),
        "what was kept before the stop is written"
    );
}

#[test]
fn k4_11_the_size_limit_keeps_exactly_max_bytes() {
    let now = Instant::now();
    let mut queue = Queue::new(10);
    assert!(matches!(queue.push(b"0123", now), Pushed::Kept { .. }));
    assert!(matches!(queue.push(b"456789", now), Pushed::Kept { .. }));
    assert_eq!(
        queue.push(b"x", now),
        Pushed::Stopped(StopReason::SizeLimit)
    );
    assert_eq!(
        last(queue.next(now)),
        (b"0123456789".to_vec(), Some(StopReason::SizeLimit))
    );
}

#[test]
fn k4_11_a_push_that_would_pass_the_limit_is_dropped_whole() {
    let now = Instant::now();
    let mut queue = Queue::new(10);
    queue.push(b"01234", now);
    assert_eq!(
        queue.push(b"567890", now),
        Pushed::Stopped(StopReason::SizeLimit)
    );
    assert_eq!(last(queue.next(now)).0, b"01234");
}

#[test]
fn k4_11_a_disk_failure_drops_what_was_not_written() {
    let now = Instant::now();
    let mut queue = Queue::new(LARGE);
    queue.push(&vec![1; MAX_PART + 3], now);
    queue.fail(StopReason::DiskFull);
    queue.fail(StopReason::WriteFailed);
    assert_eq!(
        last(queue.next(now)),
        (Vec::new(), Some(StopReason::DiskFull))
    );
    assert_eq!(queue.push(b"later", now), Pushed::Dropped);
}

#[test]
fn k4_11_closing_writes_the_open_part_as_the_last() {
    let now = Instant::now();
    let mut queue = Queue::new(LARGE);
    queue.push(b"open", now);
    queue.close();
    assert_eq!(queue.push(b"after", now), Pushed::Dropped);
    assert_eq!(last(queue.next(now)), (b"open".to_vec(), None));
}

proptest! {
    /// Whatever the pushes, the parts hold exactly the kept bytes in order, each before the
    /// last holds 1 to 64 KiB, and the last fits a part.
    #[test]
    fn k4_11_parts_carry_the_kept_bytes_in_order(
        pushes in prop::collection::vec((0..3 * MAX_PART, any::<u8>(), 0_u64..7), 0..12),
        max_bytes in 0_u64..400_000,
    ) {
        let start = Instant::now();
        let mut queue = Queue::new(max_bytes);
        let mut kept = Vec::new();
        let mut written = Vec::new();
        for (index, (length, byte, seconds)) in pushes.into_iter().enumerate() {
            let now = start + Duration::from_secs(seconds + 7 * index as u64);
            let bytes = vec![byte; length];
            if let Pushed::Kept { .. } = queue.push(&bytes, now) {
                kept.extend_from_slice(&bytes);
            }
            loop {
                match queue.next(now + Duration::from_secs(1)) {
                    Next::Write(Job::Part(part)) => {
                        prop_assert!((1..=MAX_PART).contains(&part.len()));
                        written.extend_from_slice(&part);
                    }
                    // A stop: the kept tail is the last part; later nexts give empty tails.
                    Next::Write(Job::Last { tail, .. }) => {
                        written.extend_from_slice(&tail);
                        break;
                    }
                    Next::Wait(_) => break,
                }
            }
        }
        queue.close();
        let (tail, _) = last(queue.next(start));
        prop_assert!(tail.len() <= MAX_PART);
        written.extend_from_slice(&tail);
        prop_assert!(kept.len() as u64 <= max_bytes);
        prop_assert_eq!(written, kept);
    }
}
