//! Which cookie and site store a device's web views use (D-2, WEB-006): one per device, never
//! the main window's. Pure: no I/O.

use uuid::Uuid;

use super::WebViewError;

/// The folder under the app's data folder that holds the Linux stores.
#[cfg(any(test, not(target_os = "macos")))]
pub const STORE_FOLDER: &str = "web";

/// A device, by the hub's `DeviceId` UUID.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeviceId(Uuid);

impl DeviceId {
    /// Reads the hub's hyphenated UUID form; no other form, so one device has one store.
    pub fn parse(text: &str) -> Result<Self, WebViewError> {
        let canonical = text.len() == 36 && text.as_bytes().get(8) == Some(&b'-');
        let uuid = Uuid::try_parse(text).map_err(|_| WebViewError::DeviceId)?;
        if canonical {
            Ok(Self(uuid))
        } else {
            Err(WebViewError::DeviceId)
        }
    }

    /// macOS: the `WKWebsiteDataStore` identifier, the UUID's 16 bytes.
    #[cfg(any(test, target_os = "macos"))]
    pub fn store_identifier(self) -> [u8; 16] {
        *self.0.as_bytes()
    }

    /// Linux: the store's folder name under [`STORE_FOLDER`], the lower-case hyphenated
    /// UUID. Hex digits and hyphens only, so it cannot leave its parent folder.
    #[cfg(any(test, not(target_os = "macos")))]
    pub fn folder_name(self) -> String {
        self.0.hyphenated().to_string()
    }
}
