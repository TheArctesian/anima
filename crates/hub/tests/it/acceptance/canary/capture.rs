//! The process log of every thread while the canary test runs: this thread's own default, and a
//! global default for the threads that have none, such as the blocking pool that hashes a
//! password (LOG-007).
//!
//! The global log is installed once for the test binary, at TRACE, and is enabled only while a
//! [`Watch`] lives, so the other tests' threads pay one flag check per event, not a formatted
//! TRACE line; no other `it` test sets a global subscriber. Another test's lines that reach the
//! capture meanwhile cannot hold a canary made for this run.

use std::{
    io,
    sync::{
        Mutex, Once, PoisonError,
        atomic::{AtomicBool, Ordering},
    },
};

use anima_hub::fleet::events::process_log;
use tracing::{
    Dispatch, Event, Metadata, Subscriber,
    span::{Attributes, Id, Record},
    subscriber::{DefaultGuard, Interest},
};

use crate::logs::Capture;

/// Where the global log writes: the capture of the live [`Watch`], or nowhere.
static SINK: Mutex<Option<Capture>> = Mutex::new(None);

/// Whether a [`Watch`] lives; the global log is disabled otherwise.
static WATCHING: AtomicBool = AtomicBool::new(false);

/// The global log: the hub's process log, enabled only while [`WATCHING`].
struct Gated<S>(S);

impl<S: Subscriber> Subscriber for Gated<S> {
    fn register_callsite(&self, _: &'static Metadata<'static>) -> Interest {
        Interest::sometimes()
    }

    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        WATCHING.load(Ordering::SeqCst) && self.0.enabled(metadata)
    }

    fn new_span(&self, span: &Attributes<'_>) -> Id {
        self.0.new_span(span)
    }

    fn record(&self, span: &Id, values: &Record<'_>) {
        self.0.record(span, values);
    }

    fn record_follows_from(&self, span: &Id, follows: &Id) {
        self.0.record_follows_from(span, follows);
    }

    fn event(&self, event: &Event<'_>) {
        self.0.event(event);
    }

    fn enter(&self, span: &Id) {
        self.0.enter(span);
    }

    fn exit(&self, span: &Id) {
        self.0.exit(span);
    }

    fn clone_span(&self, id: &Id) -> Id {
        self.0.clone_span(id)
    }

    fn try_close(&self, id: Id) -> bool {
        self.0.try_close(id)
    }
}

/// The global log's writer, which hands each line to [`SINK`].
struct Sink;

impl io::Write for Sink {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        match SINK.lock().unwrap_or_else(PoisonError::into_inner).as_mut() {
            Some(capture) => capture.write(bytes),
            None => Ok(bytes.len()),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// The capture of every thread's log while it lives.
pub struct Watch {
    pub capture: Capture,
    _thread: DefaultGuard,
    _second: Dispatch,
}

impl Drop for Watch {
    fn drop(&mut self) {
        WATCHING.store(false, Ordering::SeqCst);
        *SINK.lock().unwrap_or_else(PoisonError::into_inner) = None;
    }
}

/// Starts capturing the log of every thread at TRACE.
pub fn everywhere() -> Watch {
    static GLOBAL: Once = Once::new();
    GLOBAL.call_once(|| {
        let subscriber = Gated(process_log::subscriber("trace", || Sink).unwrap());
        tracing::subscriber::set_global_default(subscriber).expect("no other global log");
    });
    let capture = Capture::default();
    *SINK.lock().unwrap_or_else(PoisonError::into_inner) = Some(capture.clone());
    WATCHING.store(true, Ordering::SeqCst);
    let writer = capture.clone();
    let subscriber = process_log::subscriber("trace", move || writer.clone()).unwrap();
    // As in agent::secrecy: a second dispatcher keeps another test thread from caching this
    // test's callsites as disabled.
    let second = Dispatch::new(tracing::subscriber::NoSubscriber::default());
    let thread = tracing::subscriber::set_default(subscriber);
    tracing::callsite::rebuild_interest_cache();
    Watch {
        capture,
        _thread: thread,
        _second: second,
    }
}
