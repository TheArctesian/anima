//! Native Windows OS boundaries (WIN2). Callers use safe functions; Win32 pointer and
//! handle lifetimes are confined to `native`. This crate is empty on other platforms.
#[cfg(windows)]
pub mod files;
#[cfg(windows)]
#[expect(
    unsafe_code,
    reason = "Win32 APIs require unsafe calls; native owns allocations and handles and checks results before dereferencing"
)]
mod native;

#[cfg(windows)]
pub use native::console;
