//! The state of the device link child that the app runs (2.6, F-2). Pure: an event in, the
//! next state out.

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Phase {
    #[default]
    Stopped,
    Starting,
    Up,
    Down,
}

/// Why the device link is down.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Reason {
    /// Another program holds the saved port. The port is not changed in silence (F-2).
    PortTaken,
    /// The child ended; it is started again after a pause.
    Ended,
    /// This build carries no `anima-link`.
    NotCarried,
    /// The saved port cannot be read.
    NoPort,
}

/// What happened to the child.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    Starting { port: u16 },
    Ready { port: u16 },
    PortTaken,
    Ended,
    Unable(Reason),
    Stopped,
}

/// The device link as the shell knows it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DeviceState {
    pub phase: Phase,
    /// The saved port the child is started with.
    pub port: Option<u16>,
    /// The port the running child said it listens on; only while it is up.
    pub reported: Option<u16>,
    pub reason: Option<Reason>,
}

impl DeviceState {
    /// The state after `event`.
    #[must_use]
    pub fn after(self, event: Event) -> Self {
        match event {
            Event::Starting { port } => Self {
                phase: Phase::Starting,
                port: Some(port),
                reported: None,
                reason: None,
            },
            Event::Ready { port } if self.phase == Phase::Starting => Self {
                phase: Phase::Up,
                reported: Some(port),
                ..self
            },
            Event::Ready { .. } => self,
            Event::PortTaken => self.down(Reason::PortTaken),
            // A child that could not take its port then ends: the port stays the cause.
            Event::Ended if self.reason == Some(Reason::PortTaken) => self,
            Event::Ended => self.down(Reason::Ended),
            Event::Unable(reason) => self.down(reason),
            Event::Stopped => Self {
                phase: Phase::Stopped,
                reported: None,
                reason: None,
                ..self
            },
        }
    }

    fn down(self, reason: Reason) -> Self {
        Self {
            phase: Phase::Down,
            reported: None,
            reason: Some(reason),
            ..self
        }
    }

    /// Whether the child is up and listens on `port` as it reported (F-10).
    pub fn serves(&self, port: u16) -> bool {
        self.phase == Phase::Up && self.reported == Some(port)
    }
}
