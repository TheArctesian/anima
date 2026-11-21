use crate::model_work::supervision::{Next, Supervision};

#[test]
fn the_waits_double_then_stay_at_five_minutes() {
    let waits: Vec<u64> = (0..12).map(Supervision::wait).collect();
    assert_eq!(waits, [1, 2, 4, 8, 16, 32, 64, 128, 256, 300, 300, 300]);
}

#[test]
fn five_ends_in_one_hour_fail_the_server() {
    let mut record = Supervision::default();
    record.started();
    let mut now = 0;
    for expected in [1, 2, 4, 8] {
        assert_eq!(record.ended(now), Next::RestartAt(now + expected));
        now += expected + 1;
    }
    assert_eq!(record.ended(now), Next::Failed);
    assert!(record.is_failed());
    assert_eq!(record.ended(now + 7200), Next::Failed);
}

#[test]
fn twenty_ends_in_a_day_fail_the_server() {
    let mut record = Supervision::default();
    let mut last = Next::Failed;
    for index in 0..20u64 {
        last = record.ended(index * 3_000);
        if index < 19 {
            assert!(matches!(last, Next::RestartAt(_)), "{index}");
        }
    }
    assert_eq!(last, Next::Failed);
}

#[test]
fn only_a_new_start_that_stays_ready_ten_minutes_clears_the_counts() {
    let mut record = Supervision::default();
    record.ended(0);
    record.ended(10);
    record.ready(20);
    record.tick(20 + 3_600);
    assert_eq!(record.restarts(), 2, "no server_start came");
    record.started();
    record.ready(4_000);
    record.tick(4_000 + 599);
    assert_eq!(record.restarts(), 2);
    record.tick(4_000 + 600);
    assert_eq!(record.restarts(), 0);
}
