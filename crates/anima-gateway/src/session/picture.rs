//! The picture a session sends: the copy of the device's screen, scaled to the client's
//! view, as frames no faster than the client draws them. VNC and RDP sessions share it.

use tokio::io::{AsyncRead, AsyncWrite};
use tokio::time::Instant;

use super::hub::Hub;
use super::opening::{Allowed, refuse};
use super::{Ended, status};
use crate::frames::{self, Formats, Next, Pace, Pacing};
use crate::screen::{BadSize, Screen};
use crate::view::View;

pub(super) struct Picture {
    /// The copy of the device's screen, which the device's updates are drawn into.
    pub(super) screen: Screen,
    view: View,
    pacing: Pacing,
    formats: Formats,
    started: Instant,
}

impl Picture {
    /// A black screen of the device's `size`.
    pub(super) fn new(
        size: (usize, usize),
        allowed: &Allowed,
        pace: Pace,
    ) -> Result<Self, BadSize> {
        let screen = Screen::new(size.0, size.1)?;
        Ok(Self {
            view: View::new(&screen, allowed.view),
            screen,
            pacing: Pacing::new(pace),
            formats: allowed.formats,
            started: Instant::now(),
        })
    }

    /// When a frame that the frame rate holds back may go. A frame that the client holds
    /// back goes after the client's next `sync`, which is an instruction.
    pub(super) fn due(&self) -> Option<Instant> {
        match self.pacing.next(self.started.elapsed()) {
            Next::At(time) if self.view.has_changes(&self.screen) => Some(self.started + time),
            _ => None,
        }
    }

    /// The client drew the frame with this timestamp.
    pub(super) fn drawn(&mut self, timestamp: &str) {
        self.pacing.answered(timestamp);
    }

    /// The client's view has this size now.
    pub(super) fn set_view(&mut self, view: (usize, usize)) {
        self.view.set_client(view, &mut self.screen);
    }

    /// The pixel of the device's screen under a pixel of the client's view.
    pub(super) fn to_device(&self, at: (usize, usize)) -> (u16, u16) {
        let (x, y) = self.view.to_device(at, &self.screen);
        let on_wire = |at: usize| u16::try_from(at).unwrap_or(u16::MAX);
        (on_wire(x), on_wire(y))
    }

    /// Sends a frame when the screen changed and one may go.
    pub(super) async fn send_frame<S>(&mut self, hub: &mut Hub<S>) -> Result<(), Ended>
    where
        S: AsyncRead + AsyncWrite + Send + 'static,
    {
        if !self.view.has_changes(&self.screen) {
            return Ok(());
        }
        let Some(timestamp) = self.pacing.send(self.started.elapsed()) else {
            return Ok(());
        };
        let (shown, changes) = self.view.take_changes(&mut self.screen);
        let Ok(frame) = frames::frame(shown, &changes, self.formats, timestamp) else {
            return Err(refuse(hub, status::SERVER_ERROR, Ended::Fault).await);
        };
        hub.send(&frame).await.map_err(|_| Ended::ByHub)
    }
}
