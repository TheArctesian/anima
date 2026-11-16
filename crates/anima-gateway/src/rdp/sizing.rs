//! When the device is asked for a screen of the size of the client's view (RA-004). A
//! window that is dragged sends many sizes: the device is asked once the view has kept
//! its size for a moment, and asked again a few times when it could not be asked.
//!
//! Pure: the time is a parameter, counted from the session's start.

use std::time::Duration;

use ironrdp_displaycontrol::pdu::MonitorLayoutEntry;

/// How long the view must keep its size before the device is asked for it.
const VIEW_SETTLES: Duration = Duration::from_millis(300);
/// How long to wait before the device is asked again, when it could not be asked.
const ASK_AGAIN: Duration = Duration::from_secs(1);
/// How often the device is asked again for one size.
const MOST_ASKS_AGAIN: u8 = 5;

/// The screen size RDP allows that is nearest to a view: each side from 200 to 8192
/// pixels and an even width.
#[must_use]
pub fn screen_for(view: (usize, usize)) -> (u16, u16) {
    let side = |side: usize| u32::try_from(side).unwrap_or(u32::MAX);
    let (width, height) = MonitorLayoutEntry::adjust_display_size(side(view.0), side(view.1));
    let side = |side: u32| u16::try_from(side).unwrap_or(u16::MAX);
    (side(width), side(height))
}

/// The size the device has not been asked for yet, and when to ask.
#[derive(Default)]
pub struct ViewRequests {
    waiting: Option<Waiting>,
}

struct Waiting {
    size: (u16, u16),
    ask_at: Duration,
    asks_again: u8,
}

impl ViewRequests {
    /// The client's view has this size now.
    pub fn view_changed(&mut self, view: (usize, usize), now: Duration) {
        self.waiting = Some(Waiting {
            size: screen_for(view),
            ask_at: now + VIEW_SETTLES,
            asks_again: MOST_ASKS_AGAIN,
        });
    }

    /// When to call [`ViewRequests::due`] next; `None` while nothing waits.
    #[must_use]
    pub fn ask_at(&self) -> Option<Duration> {
        self.waiting.as_ref().map(|waiting| waiting.ask_at)
    }

    /// The size to ask the device for now, whose screen has the size `screen`. `None`
    /// before the time, and when the device's screen already has the size.
    pub fn due(&mut self, screen: (usize, usize), now: Duration) -> Option<(u16, u16)> {
        let waiting = self
            .waiting
            .as_ref()
            .filter(|waiting| waiting.ask_at <= now)?;
        let size = waiting.size;
        if (usize::from(size.0), usize::from(size.1)) == screen {
            self.waiting = None;
            return None;
        }
        Some(size)
    }

    /// The device was asked for the size that was due.
    pub fn asked(&mut self) {
        self.waiting = None;
    }

    /// The device could not be asked for the size that was due: it is asked again later,
    /// a few times.
    pub fn not_asked(&mut self, now: Duration) {
        self.waiting = self.waiting.take().and_then(|waiting| {
            Some(Waiting {
                ask_at: now + ASK_AGAIN,
                asks_again: waiting.asks_again.checked_sub(1)?,
                ..waiting
            })
        });
    }
}
