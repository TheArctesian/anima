//! Windows owns the idle request handle, so process termination releases the hold.
use super::platform::Platform;
use anima_windows::power::Request;
use std::{io, path::Path};

#[derive(Debug)]
pub struct Hold {
    request: Request,
}
impl Hold {
    pub fn alive(&mut self) -> bool {
        // The hold has no child to exit: its request exists for this object's lifetime.
        self.request.active()
    }
}
pub fn idle(_: &Platform) -> io::Result<Hold> {
    Request::idle().map(|request| Hold { request })
}

pub use super::lid_guard::LidHold;
pub fn lid(_: &Platform, marker: &Path) -> io::Result<LidHold> {
    super::lid_guard::start(marker)
}
pub fn restore(_: &Platform, marker: &Path) -> io::Result<()> {
    super::lid_guard::restore(marker)
}
pub fn release_lid(_: &Platform, marker: &Path, hold: LidHold) -> io::Result<()> {
    drop(hold);
    super::lid_guard::restore(marker)
}
