//! Strict decimal numbers from peer text (GW-011).

/// A decimal number of 1..=`max_digits` ASCII digits; no sign, no `+`, no space.
/// `u32::from_str` alone would accept a leading `+`. `max_digits` is at most 9, so it fits.
pub(super) fn decimal(text: &str, max_digits: usize) -> Option<u32> {
    if text.is_empty() || text.len() > max_digits || !text.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    text.parse().ok()
}
