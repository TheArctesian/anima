//! An RDP device, on `IronRDP`: what the client's keys and pointer mean for it.

mod keys;
mod pointer;

pub use keys::{KeyEvent, Keyboard};
pub use pointer::{Button, Pointer, PointerEvent};
