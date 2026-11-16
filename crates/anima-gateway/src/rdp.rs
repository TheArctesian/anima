//! An RDP device, on `IronRDP`: the opening of its session, the open session (`device`), its
//! clipboard channel, and what the client's keys and pointer mean for it.

mod certificate;
mod channels;
mod clipboard;
mod device;
mod drawing;
mod input;
mod keys;
mod opening;
mod pointer;
mod resizing;
mod sizing;

pub use clipboard::{ClipboardNotes, Reply, Said, Sharing, reply};
pub use device::{Device, Event, Malformed};
pub use keys::{KeyEvent, Keyboard};
pub use opening::{Connection, OpenError, Opened, Wanted, open};
pub use pointer::{Button, Pointer, PointerEvent};
pub use sizing::{ViewRequests, screen_for};
