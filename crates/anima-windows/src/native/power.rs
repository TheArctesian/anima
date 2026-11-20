//! Process-owned idle-sleep requests. Closing the handle releases the request, including
//! when Windows tears down a crashed process; no persistent power policy is changed.
pub mod scheme;

use std::{
    io,
    os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle},
};
use windows::{
    Win32::{
        Foundation::HANDLE,
        System::{
            Power::{
                GetSystemPowerStatus, PowerClearRequest, PowerCreateRequest,
                PowerRequestSystemRequired, PowerSetRequest, SYSTEM_POWER_STATUS,
            },
            Threading::{POWER_REQUEST_CONTEXT_SIMPLE_STRING, REASON_CONTEXT, REASON_CONTEXT_0},
        },
    },
    core::PWSTR,
};

/// An idle-sleep hold. It leaves display sleep and explicit user sleep decisions to Windows.
#[derive(Debug)]
pub struct Request(OwnedHandle);

impl Request {
    /// Whether this object still owns its active request handle.
    #[must_use]
    pub fn active(&self) -> bool {
        !self.0.as_raw_handle().is_null()
    }

    /// Request that Windows keep this process's computer running while it works.
    /// # Errors
    /// Windows refuses to create or activate the request.
    pub fn idle() -> io::Result<Self> {
        let mut reason: Vec<u16> = "Anima: keep this computer awake\0".encode_utf16().collect();
        let context = REASON_CONTEXT {
            Version: 0,
            Flags: POWER_REQUEST_CONTEXT_SIMPLE_STRING,
            Reason: REASON_CONTEXT_0 {
                SimpleReasonString: PWSTR(reason.as_mut_ptr()),
            },
        };
        // SAFETY: the context and terminated string stay alive until Windows copies them.
        let handle = unsafe { PowerCreateRequest(&raw const context) }.map_err(io::Error::other)?;
        // SAFETY: PowerCreateRequest returned a new owned handle, closed exactly once.
        let owned = unsafe { OwnedHandle::from_raw_handle(handle.0) };
        // SAFETY: owned keeps this request handle alive through activation.
        unsafe { PowerSetRequest(handle, PowerRequestSystemRequired) }.map_err(io::Error::other)?;
        Ok(Self(owned))
    }
}

impl Drop for Request {
    fn drop(&mut self) {
        // SAFETY: this request was activated once; the owned handle is still open here.
        let _ = unsafe {
            PowerClearRequest(HANDLE(self.0.as_raw_handle()), PowerRequestSystemRequired)
        };
    }
}

/// Current battery percentage only when discharging, with a known battery and percentage.
/// Unknown power readings and computers without batteries return None.
#[must_use]
pub fn battery_percent() -> Option<u8> {
    let mut status = SYSTEM_POWER_STATUS::default();
    // SAFETY: status points to a writable structure of the API's exact type.
    unsafe { GetSystemPowerStatus(&raw mut status) }.ok()?;
    battery(
        status.ACLineStatus,
        status.BatteryFlag,
        status.BatteryLifePercent,
    )
}

fn battery(ac: u8, flags: u8, percent: u8) -> Option<u8> {
    (ac == 0 && flags != 255 && flags & 128 == 0 && percent <= 100).then_some(percent)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_known_discharging_battery_has_a_floor_reading() {
        assert_eq!(battery(0, 1, 42), Some(42));
        assert_eq!(battery(0, 4, 0), Some(0));
        for (ac, flags, percent) in [
            (1, 1, 42),
            (255, 1, 42),
            (0, 255, 42),
            (0, 128, 42),
            (0, 1, 255),
        ] {
            assert_eq!(battery(ac, flags, percent), None);
        }
    }

    #[test]
    fn windows_request_can_be_acquired_and_released() {
        let request = Request::idle().unwrap();
        drop(request);
    }
}
