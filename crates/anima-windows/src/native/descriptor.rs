use super::identity::{self, Identity};
use std::{fs::File, io, os::windows::io::AsRawHandle};
use windows::{
    Win32::{
        Foundation::{HANDLE, HLOCAL, LocalFree},
        Security::{
            ACCESS_ALLOWED_ACE, ACE_HEADER, ACL,
            Authorization::{
                ConvertStringSecurityDescriptorToSecurityDescriptorW, GetSecurityInfo,
                SDDL_REVISION_1, SE_FILE_OBJECT, SetSecurityInfo,
            },
            DACL_SECURITY_INFORMATION, GetAce, GetSecurityDescriptorDacl, INHERIT_ONLY_ACE,
            OWNER_SECURITY_INFORMATION, PROTECTED_DACL_SECURITY_INFORMATION, PSECURITY_DESCRIPTOR,
            PSID,
        },
    },
    core::{BOOL, PCWSTR},
};

struct Descriptor(PSECURITY_DESCRIPTOR);
impl Drop for Descriptor {
    fn drop(&mut self) {
        unsafe {
            let _ = LocalFree(Some(HLOCAL(self.0.0)));
        }
    }
}
fn failure(error: windows::core::Error) -> io::Error {
    io::Error::other(error)
}

pub(crate) fn check(file: &File, private: bool) -> io::Result<bool> {
    let user = Identity::current().map_err(failure)?;
    let mut owner = PSID::default();
    let mut acl = std::ptr::null_mut();
    let mut descriptor = PSECURITY_DESCRIPTOR::default();
    unsafe {
        GetSecurityInfo(
            HANDLE(file.as_raw_handle()),
            SE_FILE_OBJECT,
            OWNER_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION,
            Some(&raw mut owner),
            None,
            Some(&raw mut acl),
            None,
            Some(&raw mut descriptor),
        )
        .ok()
        .map_err(failure)?;
    }
    let _descriptor = Descriptor(descriptor);
    if acl.is_null() || !(user.trusted(owner) || (!private && identity::installer(owner))) {
        return Ok(false);
    }
    let denied = if private {
        u32::MAX
    } else if file.metadata()?.is_dir() {
        0x000d_0040 | 0x5000_0000
    } else {
        0x000d_0156 | 0x5000_0000
    };
    for index in 0..u32::from(unsafe { (*acl).AceCount }) {
        if !ace_allows(acl, index, &user, denied)? {
            return Ok(false);
        }
    }
    Ok(true)
}

fn ace_allows(acl: *const ACL, index: u32, user: &Identity, denied: u32) -> io::Result<bool> {
    let mut pointer = std::ptr::null_mut();
    unsafe {
        GetAce(acl, index, &raw mut pointer).map_err(failure)?;
    }
    let header = unsafe { &*pointer.cast::<ACE_HEADER>() };
    if u32::from(header.AceFlags) & INHERIT_ONLY_ACE.0 != 0 || header.AceType == 1 {
        return Ok(true);
    }
    if header.AceType != 0 {
        return Ok(false);
    }
    let ace = unsafe { &*pointer.cast::<ACCESS_ALLOWED_ACE>() };
    let sid = PSID(std::ptr::from_ref(&ace.SidStart).cast_mut().cast());
    // DELETE, WRITE_DAC, WRITE_OWNER, FILE_WRITE_DATA/ADD_FILE, APPEND_DATA/ADD_SUBDIR,
    // WRITE_EA, DELETE_CHILD, WRITE_ATTRIBUTES, GENERIC_ALL, GENERIC_WRITE.
    // Windows owns and updates system programs through TrustedInstaller. It is trusted
    // for integrity checks, but does not grant access to the user's private secret files.
    Ok(user.trusted(sid)
        || ace.Mask & denied == 0
        || (denied != u32::MAX && identity::installer(sid)))
}

pub(crate) fn protect(file: &File) -> io::Result<()> {
    let user = Identity::current().map_err(failure)?;
    let text = format!(
        "D:P(A;OICI;FA;;;{})(A;OICI;FA;;;SY)(A;OICI;FA;;;BA)",
        user.text().map_err(failure)?
    );
    set_acl(file, &text)
}

fn set_acl(file: &File, text: &str) -> io::Result<()> {
    let wide: Vec<u16> = text.encode_utf16().chain([0]).collect();
    let mut descriptor = PSECURITY_DESCRIPTOR::default();
    unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            PCWSTR(wide.as_ptr()),
            SDDL_REVISION_1,
            &raw mut descriptor,
            None,
        )
        .map_err(failure)?;
    }
    let descriptor = Descriptor(descriptor);
    let mut present = BOOL::default();
    let mut defaulted = BOOL::default();
    let mut acl = std::ptr::null_mut();
    unsafe {
        GetSecurityDescriptorDacl(
            descriptor.0,
            &raw mut present,
            &raw mut acl,
            &raw mut defaulted,
        )
        .map_err(failure)?;
        if !present.as_bool() || acl.is_null() {
            return Err(io::ErrorKind::PermissionDenied.into());
        }
        SetSecurityInfo(
            HANDLE(file.as_raw_handle()),
            SE_FILE_OBJECT,
            DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION,
            None,
            None,
            Some(acl),
            None,
        )
        .ok()
        .map_err(failure)
    }
}

/// Copy a checked source ACL onto an empty replacement before writing any bytes.
pub(crate) fn copy_acl(source: &File, destination: &File) -> io::Result<()> {
    if !check(source, false)? {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    let mut acl = std::ptr::null_mut();
    let mut descriptor = PSECURITY_DESCRIPTOR::default();
    unsafe {
        GetSecurityInfo(
            HANDLE(source.as_raw_handle()),
            SE_FILE_OBJECT,
            DACL_SECURITY_INFORMATION,
            None,
            None,
            Some(&raw mut acl),
            None,
            Some(&raw mut descriptor),
        )
        .ok()
        .map_err(failure)?;
    }
    let _descriptor = Descriptor(descriptor);
    if acl.is_null() {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    unsafe {
        SetSecurityInfo(
            HANDLE(destination.as_raw_handle()),
            SE_FILE_OBJECT,
            DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION,
            None,
            None,
            Some(acl),
            None,
        )
        .ok()
        .map_err(failure)
    }
}

#[cfg(test)]
#[path = "descriptor_tests.rs"]
mod tests;

pub(crate) fn single_name(file: &File) -> io::Result<bool> {
    use windows::Win32::Storage::FileSystem::{
        BY_HANDLE_FILE_INFORMATION, GetFileInformationByHandle,
    };
    let mut properties = BY_HANDLE_FILE_INFORMATION::default();
    // SAFETY: the file owns the borrowed handle; the initialized output lives through the call.
    unsafe { GetFileInformationByHandle(HANDLE(file.as_raw_handle()), &raw mut properties) }
        .map_err(failure)?;
    Ok(properties.nNumberOfLinks == 1)
}
