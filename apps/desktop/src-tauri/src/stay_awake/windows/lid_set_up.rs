//! Lid holds are available on laptops whose power policy permits temporary lid settings.
use super::platform::Platform;
#[path = "../lid_set_up/model.rs"]
mod model;
pub use model::{Lid, SetUpError};

pub fn lid(_: &Platform) -> Lid {
    if anima_windows::power::scheme::available() {
        Lid::Ready
    } else {
        Lid::Unsupported
    }
}
pub fn set_up(_: &Platform) -> Result<(), SetUpError> {
    Err(SetUpError::Unsupported)
}
pub fn remove(_: &Platform, _: bool) -> Result<(), SetUpError> {
    Err(SetUpError::Unsupported)
}
