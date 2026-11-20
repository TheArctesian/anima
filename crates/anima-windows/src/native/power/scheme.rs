//! Temporary power schemes; the user's original scheme is never edited.
use std::io;
use windows::{
    Win32::{
        Foundation::{HLOCAL, LocalFree},
        System::Power::{
            ACCESS_AC_POWER_SETTING_INDEX, ACCESS_DC_POWER_SETTING_INDEX, GetPwrCapabilities,
            PowerDeleteScheme, PowerDuplicateScheme, PowerGetActiveScheme, PowerSetActiveScheme,
            PowerSettingAccessCheck, PowerWriteACValueIndex, PowerWriteDCValueIndex,
            SYSTEM_POWER_CAPABILITIES,
        },
    },
    core::GUID,
};

static BUTTONS: GUID = GUID::from_u128(0x4f971e89_eebd_4455_a8de_9e59040e7347);
static LID: GUID = GUID::from_u128(0x5ca83367_6e45_459f_a27b_476b1d01c936);

/// Identity of a Windows power scheme.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Scheme(GUID);

impl Scheme {
    /// The active power scheme.
    /// # Errors
    /// Windows cannot read the active scheme.
    pub fn active() -> io::Result<Self> {
        let mut pointer = std::ptr::null_mut();
        // SAFETY: Windows returns a LocalAlloc-owned GUID through this pointer.
        unsafe { PowerGetActiveScheme(None, &raw mut pointer) }
            .ok()
            .map_err(io::Error::other)?;
        allocated(pointer)
    }

    /// A copy with both lid-close actions disabled; it is not activated here.
    /// # Errors
    /// Windows refuses duplication or either setting. A failed copy is removed.
    pub fn without_lid_sleep(self) -> io::Result<Self> {
        let mut pointer = std::ptr::null_mut();
        // SAFETY: source is valid and Windows allocates the output GUID.
        unsafe { PowerDuplicateScheme(None, &raw const self.0, &raw mut pointer) }
            .ok()
            .map_err(io::Error::other)?;
        let copy = allocated(pointer)?;
        if let Err(error) = copy.set_lid() {
            let _ = copy.remove();
            return Err(error);
        }
        Ok(copy)
    }

    fn set_lid(self) -> io::Result<()> {
        // SAFETY: all pointers refer to live GUID values; zero means no lid action.
        unsafe {
            PowerWriteACValueIndex(
                None,
                &raw const self.0,
                Some(&raw const BUTTONS),
                Some(&raw const LID),
                0,
            )
        }
        .ok()
        .map_err(io::Error::other)?;
        // SAFETY: same GUID lifetimes as the AC call.
        let code = unsafe {
            PowerWriteDCValueIndex(
                None,
                &raw const self.0,
                Some(&raw const BUTTONS),
                Some(&raw const LID),
                0,
            )
        };
        windows::Win32::Foundation::WIN32_ERROR(code)
            .ok()
            .map_err(io::Error::other)
    }

    /// Activate this scheme.
    /// # Errors
    /// Windows refuses activation.
    pub fn activate(self) -> io::Result<()> {
        // SAFETY: the GUID remains live throughout the call.
        unsafe { PowerSetActiveScheme(None, Some(&raw const self.0)) }
            .ok()
            .map_err(io::Error::other)
    }

    /// Remove an inactive scheme created by this application.
    /// # Errors
    /// Windows refuses removal (including an active scheme).
    pub fn remove(self) -> io::Result<()> {
        // SAFETY: the GUID remains live throughout the call.
        let status = unsafe { PowerDeleteScheme(None, &raw const self.0) };
        if status.0 == 0 {
            Ok(())
        } else {
            Err(io::Error::from_raw_os_error(
                i32::try_from(status.0).unwrap_or(1),
            ))
        }
    }

    /// Stable journal representation.
    #[must_use]
    pub fn text(self) -> String {
        format!("{:?}", self.0)
    }

    /// Read a journal identity, rejecting malformed text.
    /// # Errors
    /// The text is not a GUID.
    pub fn parse(text: &str) -> io::Result<Self> {
        uuid::Uuid::parse_str(text)
            .map(|id| Self(GUID::from_u128(id.as_u128())))
            .map_err(|_| io::ErrorKind::InvalidData.into())
    }
}

fn allocated(pointer: *mut GUID) -> io::Result<Scheme> {
    if pointer.is_null() {
        return Err(io::ErrorKind::InvalidData.into());
    }
    // SAFETY: successful Power* calls returned a LocalAlloc-owned GUID. Copy, then free once.
    let value = unsafe { *pointer };
    unsafe { LocalFree(Some(HLOCAL(pointer.cast()))) };
    Ok(Scheme(value))
}

/// Hardware has a lid and neither AC nor battery lid policy is locked by the administrator.
#[must_use]
pub fn available() -> bool {
    let mut caps = SYSTEM_POWER_CAPABILITIES::default();
    // SAFETY: capabilities is a writable structure of the API's exact size.
    if !unsafe { GetPwrCapabilities(&raw mut caps) } || !caps.LidPresent {
        return false;
    }
    [ACCESS_AC_POWER_SETTING_INDEX, ACCESS_DC_POWER_SETTING_INDEX]
        .into_iter()
        .all(|access| {
            // SAFETY: LID is a valid static setting GUID.
            unsafe { PowerSettingAccessCheck(access, Some(&raw const LID)) }.is_ok()
        })
}
