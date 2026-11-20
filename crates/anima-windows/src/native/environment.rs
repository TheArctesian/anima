//! Notify Explorer so new processes inherit the updated user environment.
use windows::Win32::Foundation::{LPARAM, WPARAM};
use windows::{
    Win32::UI::WindowsAndMessaging::{
        HWND_BROADCAST, SMTO_ABORTIFHUNG, SendMessageTimeoutW, WM_SETTINGCHANGE,
    },
    core::w,
};

pub(crate) fn changed() {
    // The constant string lives for the complete bounded synchronous broadcast. A hung
    // recipient must not block installation; the persisted value still applies at next login.
    unsafe {
        SendMessageTimeoutW(
            HWND_BROADCAST,
            WM_SETTINGCHANGE,
            WPARAM(0),
            LPARAM(w!("Environment").as_ptr() as isize),
            SMTO_ABORTIFHUNG,
            1000,
            None,
        );
    }
}
