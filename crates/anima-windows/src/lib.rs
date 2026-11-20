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

#[cfg(windows)]
pub mod search_path;

#[cfg(windows)]
pub use native::process;

#[cfg(windows)]
pub use native::job;

#[cfg(windows)]
pub use native::socket;

#[cfg(windows)]
pub use native::power;

#[cfg(windows)]
pub mod shutdown;
