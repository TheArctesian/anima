use windows::{
    Win32::{
        Foundation::{CloseHandle, HANDLE, HLOCAL, LocalFree},
        Security::{
            Authorization::ConvertSidToStringSidW, EqualSid, GetTokenInformation, IsWellKnownSid,
            LookupAccountNameW, PSID, SID_NAME_USE, TOKEN_QUERY, TOKEN_USER, TokenUser,
            WinBuiltinAdministratorsSid, WinLocalSystemSid,
        },
        System::Threading::{GetCurrentProcess, OpenProcessToken},
    },
    core::{PCWSTR, PWSTR, w},
};

// TOKEN_USER and a maximum-size SID fit in this aligned buffer. An unexpected larger
// token fails at GetTokenInformation rather than exposing uninitialised storage.
pub(super) struct Identity(Box<[usize; 64]>);
impl Identity {
    pub(super) fn current() -> windows::core::Result<Self> {
        let mut token = HANDLE::default();
        unsafe {
            OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &raw mut token)?;
        }
        let mut result = Self(Box::new([0; 64]));
        let mut size = 0;
        let read = unsafe {
            GetTokenInformation(
                token,
                TokenUser,
                Some(result.0.as_mut_ptr().cast()),
                512,
                &raw mut size,
            )
        };
        unsafe {
            let _ = CloseHandle(token);
        }
        read?;
        Ok(result)
    }
    fn sid(&self) -> PSID {
        unsafe { (*self.0.as_ptr().cast::<TOKEN_USER>()).User.Sid }
    }
    pub(super) fn trusted(&self, sid: PSID) -> bool {
        if sid.0.is_null() {
            return false;
        }
        unsafe {
            EqualSid(sid, self.sid()).is_ok()
                || IsWellKnownSid(sid, WinLocalSystemSid).as_bool()
                || IsWellKnownSid(sid, WinBuiltinAdministratorsSid).as_bool()
        }
    }
    pub(super) fn text(&self) -> windows::core::Result<String> {
        let mut text = PWSTR::null();
        unsafe {
            ConvertSidToStringSidW(self.sid(), &raw mut text)?;
        }
        let converted = unsafe { text.to_string() };
        unsafe {
            let _ = LocalFree(Some(HLOCAL(text.0.cast())));
        }
        converted.map_err(|_| windows::core::Error::from_thread())
    }
}

pub(super) fn installer(sid: PSID) -> bool {
    if sid.0.is_null() {
        return false;
    }
    let mut buffer = [0_usize; 64];
    let mut size = 512;
    let mut domain = [0_u16; 256];
    let mut domain_size = 256;
    let mut kind = SID_NAME_USE::default();
    let found = PSID(buffer.as_mut_ptr().cast());
    unsafe {
        LookupAccountNameW(
            PCWSTR::null(),
            w!("NT SERVICE\\TrustedInstaller"),
            Some(found),
            &raw mut size,
            Some(PWSTR(domain.as_mut_ptr())),
            &raw mut domain_size,
            &raw mut kind,
        )
        .is_ok()
            && EqualSid(sid, found).is_ok()
    }
}
