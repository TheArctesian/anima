//! Check 1 (WEB-006): device A, device B and the default store (the main view's) share no
//! cookie or `localStorage`. All three load the same origin, so only the store separates them.

use std::time::Duration;

use super::device::{self, STORE_A, STORE_B};
use super::{Harness, Nonces, Results};
use crate::event::{Event, Journal, is_report};

const WAIT: Duration = Duration::from_secs(8);

/// What one web view saw: `document.cookie`, `localStorage['spike']` and the `Cookie` header.
struct Seen {
    who: &'static str,
    cookie: String,
    local: String,
    header: String,
}

impl Seen {
    fn holds(&self, nonce: &str) -> bool {
        self.cookie.contains(nonce) || self.local.contains(nonce) || self.header.contains(nonce)
    }

    fn has_both_parts(&self, nonce: &str) -> bool {
        self.cookie.contains(&format!("spike_js={nonce}"))
            && self.cookie.contains(&format!("spike_http={nonce}"))
            && self.local == nonce
    }
}

impl std::fmt::Display for Seen {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}: cookie={:?} localStorage={:?} header={:?}",
            self.who, self.cookie, self.local, self.header
        )
    }
}

pub async fn check(
    harness: &Harness,
    journal: &mut Journal,
    results: &mut Results,
    nonces: &Nonces,
) {
    match observe(harness, journal, nonces).await {
        Ok(seen) => judge(results, nonces, &seen),
        Err(reason) => results.record(false, "store-isolated", reason),
    }
}

async fn observe(
    harness: &Harness,
    journal: &mut Journal,
    nonces: &Nonces,
) -> Result<[Seen; 3], String> {
    let from = journal.mark();
    let set = |who: &str, n: &str| harness.http_url(&format!("/store/set?who={who}&n={n}"));
    let a = device::open(harness, "web-a", Some(STORE_A), set("a", &nonces.device))?;
    let main = device::open(
        harness,
        "web-default",
        None,
        set("default", &nonces.default),
    )?;
    for who in ["a", "default"] {
        journal
            .until(from, WAIT, |e| is_report(e, "http", "store-set", Some(who)))
            .await
            .ok_or(format!("no store-set report from {who}"))?;
    }
    let from = journal.mark();
    let read = |who: &str| harness.http_url(&format!("/store/read?who={who}"));
    device::open(harness, "web-b", Some(STORE_B), read("b"))?;
    a.navigate(read("a")).map_err(|e| e.to_string())?;
    main.navigate(read("default")).map_err(|e| e.to_string())?;
    let mut seen = Vec::new();
    for who in ["a", "b", "default"] {
        seen.push(read_back(journal, from, who).await?);
    }
    let [a, b, main] = <[Seen; 3]>::try_from(seen).map_err(|_| "three reads".to_owned())?;
    Ok([a, b, main])
}

async fn read_back(journal: &mut Journal, from: usize, who: &'static str) -> Result<Seen, String> {
    let report = journal
        .until(from, WAIT, |e| {
            is_report(e, "http", "store-read", Some(who))
        })
        .await
        .ok_or(format!("no store-read report from {who}"))?;
    let Event::Report { body, .. } = report else {
        unreachable!("is_report matched")
    };
    let field = |name: &str| {
        body.get(name)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_owned()
    };
    // The GET for this read carried the cookie header the store holds for the origin.
    let query = format!("who={who}");
    let header = journal
        .since(from, |e| matches!(e, Event::Served { path, query: q, .. } if path == "/store/read" && *q == query))
        .into_iter()
        .find_map(|e| match e {
            Event::Served { cookie, .. } => Some(cookie),
            _ => None,
        })
        .unwrap_or_default();
    Ok(Seen {
        who,
        cookie: field("cookie"),
        local: field("ls"),
        header,
    })
}

fn judge(results: &mut Results, nonces: &Nonces, [a, b, main]: &[Seen; 3]) {
    let (device, default) = (nonces.device.as_str(), nonces.default.as_str());
    results.record(
        a.has_both_parts(device) && main.has_both_parts(default),
        "store-own-data",
        format!("control, each store reads back what it set | {a} | {main}"),
    );
    results.record(
        !b.holds(device) && !b.holds(default),
        "store-device-isolated",
        format!("device B sees neither device A's nor the default store's data | {b}"),
    );
    results.record(
        !a.holds(default) && !main.holds(device),
        "store-device-vs-main",
        "device A does not see the default (main) store's data and the default store does not see device A's",
    );
}
