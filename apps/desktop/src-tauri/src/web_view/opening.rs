//! The arguments of `web_view_open`, read whole and checked field by field before any web
//! view exists. Pure: no I/O.

use serde::Deserialize;
use tauri::Rect;

use super::{
    Fingerprint, WebViewError,
    address::{Embedding, WebUrl, embedding},
    bounds::Bounds,
    storage::DeviceId,
    tabs::{Tab, TabId},
};

/// `{ tabId, deviceId, url, pin, bounds }` as the client sends it; no other field.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Opening {
    tab_id: String,
    device_id: String,
    url: String,
    pin: Option<String>,
    bounds: Bounds,
}

/// A checked opening: the tab to reserve and where its web view goes.
#[derive(Debug, Clone)]
pub struct NewTab {
    pub id: TabId,
    pub device: DeviceId,
    pub rect: Rect,
    pub tab: Tab,
}

impl Opening {
    /// Reads the command's whole argument object. A wrong shape is `arguments`, whatever
    /// the field, so no value is echoed back.
    pub fn read(body: &serde_json::Value) -> Result<Self, WebViewError> {
        Self::deserialize(body).map_err(|_| WebViewError::Arguments)
    }

    /// Checks every field and the D-6 rule for `https` given whether the platform `pinning`.
    pub fn check(self, pinning: bool) -> Result<NewTab, WebViewError> {
        let id = TabId::parse(&self.tab_id)?;
        let device = DeviceId::parse(&self.device_id)?;
        let url = WebUrl::parse(&self.url)?;
        let pin = self
            .pin
            .as_deref()
            .map(str::parse::<Fingerprint>)
            .transpose()?;
        let rect = self.bounds.rect()?;
        let (origin, pin) = match embedding(&url, pin, pinning)? {
            Embedding::Plain => (None, None),
            Embedding::Pinned { origin, pin } => (Some(origin), pin),
        };
        let tab = Tab {
            url,
            origin,
            pin,
            popped_out: false,
        };
        Ok(NewTab {
            id,
            device,
            rect,
            tab,
        })
    }
}
