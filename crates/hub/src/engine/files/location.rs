//! The location metadata of a file (FIL-003): read at upload when the operator keeps it, and
//! removed from each copy that the hub sends.

mod exif;
mod jpeg;
#[cfg(test)]
mod tests;

pub use exif::Position;
pub use jpeg::{CLEANED_TYPES, Cleaner, is_jpeg};
