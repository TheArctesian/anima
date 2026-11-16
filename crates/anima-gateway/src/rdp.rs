//! An RDP device, on `IronRDP`: the opening of its session, its clipboard channel, and
//! what the client's keys and pointer mean for it.

mod certificate;
mod clipboard;
mod keys;
mod opening;
mod pointer;

pub use clipboard::{Clipboard, Said, TEXT};
pub use keys::{KeyEvent, Keyboard};
pub use opening::{Connection, OpenError, Opened, Wanted, open};
pub use pointer::{Button, Pointer, PointerEvent};
