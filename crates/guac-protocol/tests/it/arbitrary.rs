//! Proptest generators shared by the property tests.

use proptest::prelude::*;

/// Any Unicode text of up to `max_chars` characters but NUL: guacd ends an
/// element at a NUL, so the decoder and the handshake both refuse it.
pub fn text(max_chars: usize) -> impl Strategy<Value = String> {
    let non_nul = any::<char>().prop_filter("no NUL", |c| *c != '\0');
    prop::collection::vec(non_nul, 0..=max_chars).prop_map(String::from_iter)
}
