//! What the harness observes: every server hit, page report, IPC call, navigation decision and
//! certificate challenge arrives here as one `Event`, in order, over a bounded queue.

use std::time::Duration;

use tokio::sync::mpsc;
use tokio::time::Instant;

use crate::pin::Verdict;

/// Bound on queued events. A full queue drops the event and says so on stderr.
const QUEUE: usize = 512;

#[derive(Clone, Debug)]
pub enum Event {
    /// A test server answered a GET. `cookie` is the request's `Cookie` header.
    Served {
        server: String,
        path: String,
        query: String,
        cookie: String,
    },
    /// A page posted a result to `/report?name=...`.
    Report {
        server: String,
        name: String,
        body: serde_json::Value,
    },
    /// The spike command ran; `label` is the calling web view.
    Invoked { label: String, from: String },
    /// The navigation filter of a device web view decided on a URL.
    Navigation {
        label: String,
        url: String,
        allowed: bool,
    },
    /// The trust hook answered a server-trust challenge.
    Challenge {
        label: String,
        host: String,
        port: u16,
        verdict: Verdict,
        system_trusted: bool,
    },
    /// The trust hook was (or was not) installed on the TLS web view.
    Hook { result: Result<String, String> },
}

#[derive(Clone)]
pub struct Events(mpsc::Sender<Event>);

impl Events {
    pub fn send(&self, event: Event) {
        if self.0.try_send(event).is_err() {
            eprintln!("od7: event queue full or closed; an event was dropped");
        }
    }
}

/// The ordered record of events, with waits that look at events after a mark.
pub struct Journal {
    queue: mpsc::Receiver<Event>,
    seen: Vec<Event>,
}

pub fn channel() -> (Events, Journal) {
    let (sender, queue) = mpsc::channel(QUEUE);
    (
        Events(sender),
        Journal {
            queue,
            seen: Vec::new(),
        },
    )
}

impl Journal {
    /// The position of the next event; pass it to `until` or `since`.
    pub fn mark(&mut self) -> usize {
        self.drain();
        self.seen.len()
    }

    /// Waits up to `wait` for an event after `from` that matches.
    pub async fn until(
        &mut self,
        from: usize,
        wait: Duration,
        matches: impl Fn(&Event) -> bool,
    ) -> Option<Event> {
        let deadline = Instant::now() + wait;
        loop {
            self.drain();
            if let Some(found) = self.seen[from..].iter().find(|event| matches(event)) {
                return Some(found.clone());
            }
            match tokio::time::timeout_at(deadline, self.queue.recv()).await {
                Ok(Some(event)) => self.seen.push(event),
                Ok(None) | Err(_) => return None,
            }
        }
    }

    /// Collects events for `wait`, so late arrivals are counted.
    pub async fn settle(&mut self, wait: Duration) {
        let deadline = Instant::now() + wait;
        while let Ok(Some(event)) = tokio::time::timeout_at(deadline, self.queue.recv()).await {
            self.seen.push(event);
        }
    }

    /// Every event after `from` that matches.
    pub fn since(&mut self, from: usize, matches: impl Fn(&Event) -> bool) -> Vec<Event> {
        self.drain();
        self.seen[from..]
            .iter()
            .filter(|event| matches(event))
            .cloned()
            .collect()
    }

    fn drain(&mut self) {
        while let Ok(event) = self.queue.try_recv() {
            self.seen.push(event);
        }
    }
}

/// The report `name` from `server` whose body field `who` equals `who` (or any, if `None`).
pub fn is_report(event: &Event, server: &str, name: &str, who: Option<&str>) -> bool {
    match event {
        Event::Report {
            server: s,
            name: n,
            body,
        } => {
            s == server
                && n == name
                && who.is_none_or(|w| body.get("who").and_then(|v| v.as_str()) == Some(w))
        }
        _ => false,
    }
}
