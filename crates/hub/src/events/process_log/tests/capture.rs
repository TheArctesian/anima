//! Captures the process log of one test thread as JSON lines.

use std::{
    io,
    sync::{Arc, Mutex, Once},
};

use serde_json::Value;
use tracing::{Event, Metadata, Subscriber, span, subscriber::Interest};

use super::super::subscriber;

#[derive(Clone, Default)]
pub(super) struct Capture(Arc<Mutex<Vec<u8>>>);

impl io::Write for Capture {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Capture {
    pub(super) fn text(&self) -> String {
        String::from_utf8(self.0.lock().unwrap().clone()).unwrap()
    }

    pub(super) fn lines(&self) -> Vec<Value> {
        self.text()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect()
    }
}

/// The test binary's global subscriber: it records nothing, but answers "sometimes" for
/// every callsite. Callsite interest is cached for the whole process, so without it a test
/// thread with no subscriber can cache "never" for a callsite while another thread is
/// capturing, and the capturing thread then loses the event.
struct Undecided;

impl Subscriber for Undecided {
    fn register_callsite(&self, _: &'static Metadata<'static>) -> Interest {
        Interest::sometimes()
    }

    fn enabled(&self, _: &Metadata<'_>) -> bool {
        false
    }

    fn new_span(&self, _: &span::Attributes<'_>) -> span::Id {
        span::Id::from_u64(1)
    }

    fn record(&self, _: &span::Id, _: &span::Record<'_>) {}

    fn record_follows_from(&self, _: &span::Id, _: &span::Id) {}

    fn event(&self, _: &Event<'_>) {}

    fn enter(&self, _: &span::Id) {}

    fn exit(&self, _: &span::Id) {}
}

/// Routes this thread's log to a buffer. Rebuilding the callsite cache after the scoped
/// subscriber is set makes callsites first seen before it ask again.
pub(super) fn capture_log(filter: &str) -> (Capture, tracing::subscriber::DefaultGuard) {
    static UNDECIDED: Once = Once::new();
    UNDECIDED.call_once(|| {
        tracing::subscriber::set_global_default(Undecided).expect("no other global subscriber");
    });
    let capture = Capture::default();
    let writer = capture.clone();
    let log = subscriber(filter, move || writer.clone()).unwrap();
    let guard = tracing::subscriber::set_default(log);
    tracing::callsite::rebuild_interest_cache();
    (capture, guard)
}
