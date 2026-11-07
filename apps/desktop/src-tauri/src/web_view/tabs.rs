//! The open device web views, by tab ID: what each was opened on, its pin, whether it has
//! its own window and which reservation of the ID it is. No I/O; the lock is held only to
//! copy in or out, never across a call to the web view or an `await`.

use std::{
    collections::HashMap,
    sync::{Mutex, MutexGuard, PoisonError},
};

use super::{Fingerprint, Pin, PinnedOrigin, WebUrl, WebViewError};

/// The most device web views open at once; each is a `WebKit` process.
pub const TABS_MAX: usize = 32;

const TAB_ID_MAX: usize = 128;

/// A tab ID as the client makes it: 1 to 128 of `A-Z a-z 0-9 _ -`. It becomes part of the
/// labels `web-<tab id>` and `pop-<tab id>`, so no other character may reach a label.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TabId(String);

impl TabId {
    pub fn parse(text: &str) -> Result<Self, WebViewError> {
        let allowed = |byte: u8| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-';
        if (1..=TAB_ID_MAX).contains(&text.len()) && text.bytes().all(allowed) {
            Ok(Self(text.to_owned()))
        } else {
            Err(WebViewError::TabId)
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The device web view's label. No capability names it, so it has no IPC (D-3).
    pub fn view_label(&self) -> String {
        format!("web-{}", self.0)
    }

    /// The label of the window the web view moves to when popped out.
    pub fn window_label(&self) -> String {
        format!("pop-{}", self.0)
    }
}

/// One open device web view.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tab {
    /// The address it was opened on; a new pin loads it again.
    pub url: WebUrl,
    /// The `https` origin a pin belongs to; `None` for `http`.
    pub origin: Option<PinnedOrigin>,
    pub pin: Option<Pin>,
    /// Whether it has moved to its own window.
    pub popped_out: bool,
}

/// One reservation of a tab ID. A tab closed and opened again under the same ID has a new
/// generation, so work begun for the old one (an open still waiting, a closing pop-out window)
/// cannot touch the new one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Generation(u64);

#[derive(Debug, Default)]
struct Open {
    tabs: HashMap<TabId, (Generation, Tab)>,
    /// The generation the next reservation takes.
    next: u64,
}

/// The open device web views. Bounded by [`TABS_MAX`].
#[derive(Debug, Default)]
pub struct Tabs(Mutex<Open>);

impl Tabs {
    /// A poisoned lock still holds a consistent map: every change is one insert, update or
    /// remove.
    fn open(&self) -> MutexGuard<'_, Open> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Takes `id` for a new web view, if it is free and the bound allows.
    pub fn reserve(&self, id: TabId, tab: Tab) -> Result<Generation, WebViewError> {
        let mut open = self.open();
        if open.tabs.contains_key(&id) {
            return Err(WebViewError::TabOpen);
        }
        if open.tabs.len() >= TABS_MAX {
            return Err(WebViewError::TabLimit);
        }
        let generation = Generation(open.next);
        open.next += 1;
        open.tabs.insert(id, (generation, tab));
        Ok(generation)
    }

    pub fn get(&self, id: &TabId) -> Result<Tab, WebViewError> {
        self.open()
            .tabs
            .get(id)
            .map(|(_, tab)| tab.clone())
            .ok_or(WebViewError::UnknownTab)
    }

    /// Whether `id` is still the reservation `generation`, not closed and not opened again.
    pub fn holds(&self, id: &TabId, generation: Generation) -> bool {
        self.open()
            .tabs
            .get(id)
            .is_some_and(|(held, _)| *held == generation)
    }

    /// Pins the certificate the operator accepted to the tab's `https` origin.
    pub fn set_pin(&self, id: &TabId, fingerprint: Fingerprint) -> Result<Tab, WebViewError> {
        let mut open = self.open();
        let (_, tab) = open.tabs.get_mut(id).ok_or(WebViewError::UnknownTab)?;
        let origin = tab.origin.clone().ok_or(WebViewError::PinScheme)?;
        tab.pin = Some(Pin {
            origin,
            fingerprint,
        });
        Ok(tab.clone())
    }

    /// Marks the tab as moved to its own window. `None` if it already was; otherwise the
    /// reservation that moved, for the window to end when it closes.
    pub fn pop_out(&self, id: &TabId) -> Result<Option<Generation>, WebViewError> {
        let mut open = self.open();
        let (generation, tab) = open.tabs.get_mut(id).ok_or(WebViewError::UnknownTab)?;
        Ok((!std::mem::replace(&mut tab.popped_out, true)).then_some(*generation))
    }

    /// Takes back a pop-out that did not happen, if `id` is still that reservation.
    pub fn undo_pop_out(&self, id: &TabId, generation: Generation) {
        if let Some((held, tab)) = self.open().tabs.get_mut(id)
            && *held == generation
        {
            tab.popped_out = false;
        }
    }

    /// Ends the tab, whichever reservation it is: the operator closed it.
    pub fn remove(&self, id: &TabId) -> Option<Tab> {
        self.open().tabs.remove(id).map(|(_, tab)| tab)
    }

    /// Ends the tab only if it is still the reservation `generation`.
    pub fn release(&self, id: &TabId, generation: Generation) -> Option<Tab> {
        let mut open = self.open();
        let held = open
            .tabs
            .get(id)
            .is_some_and(|(held, _)| *held == generation);
        held.then(|| open.tabs.remove(id))
            .flatten()
            .map(|(_, tab)| tab)
    }
}
