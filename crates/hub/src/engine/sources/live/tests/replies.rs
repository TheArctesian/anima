//! PRJ-032, K-E1 Section 13.1: what the cache serves, for how long, and what it lets go.

use std::sync::Arc;

use chrono::{DateTime, TimeDelta, Utc};
use source_exchange::Request;

use super::{account, now};
use crate::{
    engine::sources::live::{
        Keep,
        replies::{BOUNDS, Bounds, Held, Key, Replies},
    },
    ids::SourceAccountId,
};

fn key(account: SourceAccountId, path: &str) -> Key {
    Key::of(account, &Request::get(path, &[]).unwrap())
}

fn held(body: &[u8], at: DateTime<Utc>) -> Held {
    Held {
        status: 200,
        body: Arc::from(body),
        at,
    }
}

fn secs(n: i64) -> TimeDelta {
    TimeDelta::seconds(n)
}

#[test]
fn prj_032_an_answer_is_fresh_inside_its_window_and_not_after() {
    let mut replies = Replies::default();
    let one = key(account(), "/repos/o/r/pulls");
    let keep = Keep::for_seconds(30);
    replies.keep(one.clone(), keep, held(b"[]", now()));

    assert!(replies.fresh(&one, keep, now() + secs(29)).is_some());
    assert!(replies.fresh(&one, keep, now() + secs(30)).is_none());
}

#[test]
fn prj_032_the_query_is_part_of_the_key_in_any_order() {
    let mut replies = Replies::default();
    let id = account();
    let ab = Request::get("/x", &[("a", "1"), ("b", "2")]).unwrap();
    let ba = Request::get("/x", &[("b", "2"), ("a", "1")]).unwrap();
    let other = Request::get("/x", &[("a", "2")]).unwrap();
    let keep = Keep::for_seconds(30);
    replies.keep(Key::of(id, &ab), keep, held(b"1", now()));

    assert!(replies.fresh(&Key::of(id, &ba), keep, now()).is_some());
    assert!(replies.fresh(&Key::of(id, &other), keep, now()).is_none());
    assert!(
        replies
            .fresh(&Key::of(account(), &ab), keep, now())
            .is_none()
    );
}

#[test]
fn prj_032_fresh_reads_again_unless_the_answer_is_under_five_seconds_old() {
    let mut replies = Replies::default();
    let one = key(account(), "/x");
    replies.keep(one.clone(), Keep::for_seconds(60), held(b"1", now()));
    let forced = Keep::for_seconds(60).fresh(true);

    assert!(replies.fresh(&one, forced, now() + secs(4)).is_some());
    assert!(replies.fresh(&one, forced, now() + secs(5)).is_none());
}

#[test]
fn prj_032_a_held_answer_is_served_stale_for_an_hour() {
    let mut replies = Replies::default();
    let one = key(account(), "/x");
    let keep = Keep::for_seconds(30);
    replies.keep(one.clone(), keep, held(b"1", now()));

    assert!(replies.stale(&one, keep, now() + secs(3600)).is_some());
    assert!(replies.stale(&one, keep, now() + secs(3601)).is_none());
}

#[test]
fn prj_032_an_answer_kept_never_is_neither_held_nor_served() {
    let mut replies = Replies::default();
    let one = key(account(), "/repos/o/r/contents/a");
    replies.keep(one.clone(), Keep::never(), held(b"secret file", now()));

    assert_eq!(replies.size(), (0, 0));
    let keep = Keep::for_seconds(30);
    assert!(replies.fresh(&one, keep, now()).is_none());
    assert!(replies.stale(&one, keep, now()).is_none());
}

#[test]
fn prj_032_an_answer_only_when_refuses_is_not_held_and_its_older_answer_goes() {
    let mut replies = Replies::default();
    let one = key(account(), "/repos/o/r/contents/a");
    let keep = Keep::for_seconds(60).only_when(|body| body.starts_with(b"["));
    replies.keep(one.clone(), keep, held(b"[]", now()));
    replies.keep(
        one.clone(),
        keep,
        held(b"{\"content\":\"c2VjcmV0\"}", now()),
    );

    assert_eq!(replies.size(), (0, 0));
    assert!(replies.stale(&one, keep, now()).is_none());
}

#[test]
fn prj_032_a_settled_answer_is_kept_longer() {
    let mut replies = Replies::default();
    let done = key(account(), "/done");
    let busy = key(account(), "/busy");
    let keep = Keep::for_seconds(15).longer_when(600, |body| body == b"completed");
    replies.keep(done.clone(), keep, held(b"completed", now()));
    replies.keep(busy.clone(), keep, held(b"in_progress", now()));

    assert!(replies.fresh(&done, keep, now() + secs(599)).is_some());
    assert!(replies.fresh(&busy, keep, now() + secs(15)).is_none());
}

#[test]
fn prj_032_the_cache_holds_at_most_its_entries_and_lets_the_least_recently_used_go() {
    let bounds = Bounds {
        entries: 3,
        ..BOUNDS
    };
    let mut replies = Replies::new(bounds);
    let id = account();
    let keep = Keep::for_seconds(30);
    for path in ["/a", "/b", "/c"] {
        replies.keep(key(id, path), keep, held(b"1", now()));
    }
    // "/a" is used again, so "/b" is now the least recently used.
    assert!(replies.fresh(&key(id, "/a"), keep, now()).is_some());
    replies.keep(key(id, "/d"), keep, held(b"1", now()));

    assert_eq!(replies.size().0, 3);
    assert!(replies.fresh(&key(id, "/b"), keep, now()).is_none());
    for path in ["/a", "/c", "/d"] {
        assert!(
            replies.fresh(&key(id, path), keep, now()).is_some(),
            "{path}"
        );
    }
}

#[test]
fn prj_032_the_cache_holds_at_most_its_bytes_and_no_body_over_the_body_bound() {
    let mut replies = Replies::new(Bounds {
        entries: 100,
        bytes: 10,
        body: 6,
    });
    let id = account();
    let keep = Keep::for_seconds(30);
    replies.keep(key(id, "/big"), keep, held(b"1234567", now()));
    assert_eq!(replies.size(), (0, 0), "over the body bound");

    replies.keep(key(id, "/a"), keep, held(b"123456", now()));
    replies.keep(key(id, "/b"), keep, held(b"123456", now()));

    assert_eq!(replies.size(), (1, 6));
    assert!(replies.fresh(&key(id, "/b"), keep, now()).is_some());
}

#[test]
fn prj_032_the_real_bounds_are_two_thousand_entries_64_mib_and_512_kib() {
    assert_eq!(BOUNDS.entries, 2_000);
    assert_eq!(BOUNDS.bytes, 64 * 1024 * 1024);
    assert_eq!(BOUNDS.body, 512 * 1024);
    let mut replies = Replies::default();
    let id = account();
    for n in 0..2_001 {
        replies.keep(
            key(id, &format!("/n/{n}")),
            Keep::for_seconds(30),
            held(b"1", now()),
        );
    }
    assert_eq!(replies.size().0, 2_000);
}

#[test]
fn prj_028_forget_drops_exactly_the_paths_below_the_prefix_of_that_account() {
    let mut replies = Replies::default();
    let (id, other) = (account(), account());
    let keep = Keep::for_seconds(30);
    let paths = [
        "/repos/o/r/pulls",
        "/repos/o/r/pulls/12",
        "/repos/o/r/pulls2",
        "/repos/o/r2/pulls",
        "/repos/o/r/issues",
    ];
    for path in paths {
        replies.keep(key(id, path), keep, held(b"1", now()));
        replies.keep(key(other, path), keep, held(b"1", now()));
    }

    replies.forget(id, "/repos/o/r/pulls/");

    let left: Vec<&str> = paths
        .into_iter()
        .filter(|path| replies.fresh(&key(id, path), keep, now()).is_some())
        .collect();
    assert_eq!(
        left,
        [
            "/repos/o/r/pulls2",
            "/repos/o/r2/pulls",
            "/repos/o/r/issues"
        ]
    );
    for path in paths {
        assert!(
            replies.fresh(&key(other, path), keep, now()).is_some(),
            "{path}"
        );
    }
}

#[test]
fn itg_004_a_removed_account_leaves_no_answer_held_and_another_keeps_its_own() {
    let mut replies = Replies::default();
    let (removed, other) = (account(), account());
    let keep = Keep::for_seconds(30);
    for path in ["/repos/o/r/pulls", "/repos/o/r/contents", "/app"] {
        replies.keep(key(removed, path), keep, held(b"1", now()));
        replies.keep(key(other, path), keep, held(b"22", now()));
    }

    replies.forget_account(removed);

    assert_eq!(replies.size(), (3, 6), "only the other account's answers");
    let pulls = key(removed, "/repos/o/r/pulls");
    assert!(replies.stale(&pulls, keep, now()).is_none());
}
