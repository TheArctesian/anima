//! The calls into the Go link library that `link/build-phone.sh` builds (`link/cmd/phonelink`). Each call
//! answers JSON that the library allocated; it is read here and given back to the library at once.

use std::ffi::{CStr, CString, c_char};
use std::path::Path;

use serde::Deserialize;

use super::LinkError;

#[expect(unsafe_code, reason = "the Go link library is a C library (MOB-031)")]
unsafe extern "C" {
    fn AnimaLinkJoin(state: *const c_char, invite: *const c_char) -> *mut c_char;
    fn AnimaLinkStart(state: *const c_char, listen: *const c_char) -> *mut c_char;
    fn AnimaLinkStatus() -> *mut c_char;
    fn AnimaLinkStop();
    fn AnimaLinkFree(text: *mut c_char);
}

/// A join's answer: the fingerprint of this device's key.
#[derive(Deserialize)]
struct Joined {
    fingerprint: String,
}

/// A start's answer: the loopback address of the link.
#[derive(Deserialize)]
struct Started {
    local: String,
}

/// What the library says about the link that runs.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Running {
    #[serde(rename = "running")]
    pub up: bool,
    #[serde(default)]
    pub local: String,
    #[serde(default)]
    pub device_key: String,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub latency_ms: Option<f64>,
}

/// A failed call's answer.
#[derive(Deserialize)]
struct Failure {
    error: String,
}

/// The text that `call` returned, given back to the library.
#[expect(
    unsafe_code,
    reason = "the answer is a C string that the Go library allocated"
)]
fn answer(text: *mut c_char) -> String {
    if text.is_null() {
        return String::new();
    }
    // SAFETY: the library returns a NUL-terminated string from C.CString; it stays valid until AnimaLinkFree.
    let read = unsafe { CStr::from_ptr(text) }
        .to_string_lossy()
        .into_owned();
    // SAFETY: `text` came from the library and is freed once, here.
    unsafe { AnimaLinkFree(text) };
    read
}

/// The answer as `T`, or the error that the library named.
fn read<T: for<'a> Deserialize<'a>>(text: &str) -> Result<T, LinkError> {
    if let Ok(failure) = serde_json::from_str::<Failure>(text) {
        return Err(match failure.error.as_str() {
            "bad-invite" => LinkError::BadInvite,
            "no-answer" => LinkError::NoAnswer,
            "refused" => LinkError::Refused,
            "folder" => LinkError::Folder,
            _ => LinkError::Failed,
        });
    }
    serde_json::from_str(text).map_err(|_| LinkError::Failed)
}

fn c_text(text: &str) -> Result<CString, LinkError> {
    CString::new(text).map_err(|_| LinkError::Failed)
}

fn c_path(path: &Path) -> Result<CString, LinkError> {
    c_text(path.to_str().ok_or(LinkError::Folder)?)
}

/// Uses `invite` with the identity in `state`; the fingerprint of this device's key.
#[expect(unsafe_code, reason = "a call into the Go link library")]
pub fn join(state: &Path, invite: &str) -> Result<String, LinkError> {
    let (state, invite) = (
        c_path(state)?,
        zeroize::Zeroizing::new(c_text(invite)?.into_bytes_with_nul()),
    );
    // SAFETY: both pointers are NUL-terminated strings that live until the call returns.
    let text = answer(unsafe { AnimaLinkJoin(state.as_ptr(), invite.as_ptr().cast()) });
    read::<Joined>(&text).map(|joined| joined.fingerprint)
}

/// Starts the link of `state` on `listen`; the loopback address it listens on.
#[expect(unsafe_code, reason = "a call into the Go link library")]
pub fn start(state: &Path, listen: &str) -> Result<String, LinkError> {
    let (state, listen) = (c_path(state)?, c_text(listen)?);
    // SAFETY: both pointers are NUL-terminated strings that live until the call returns.
    let text = answer(unsafe { AnimaLinkStart(state.as_ptr(), listen.as_ptr()) });
    read::<Started>(&text).map(|started| started.local)
}

/// The link that runs, if one does.
#[expect(unsafe_code, reason = "a call into the Go link library")]
pub fn status() -> Running {
    // SAFETY: the call takes nothing and returns a string of the library.
    let text = answer(unsafe { AnimaLinkStatus() });
    read(&text).unwrap_or_default()
}

/// Stops the link; nothing happens when none runs.
#[expect(unsafe_code, reason = "a call into the Go link library")]
pub fn stop() {
    // SAFETY: the call takes nothing.
    unsafe { AnimaLinkStop() };
}
