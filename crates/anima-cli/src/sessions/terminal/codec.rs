//! The terminal tunnel's text messages (K4 Section 8.4). Pure.
//!
//! The client sends `resize` and `ping`. The hub's messages are a closed set, read strictly:
//! `{"type":"ping"}` and `{"type":"end","reason":R}` with a K1 `SessionEndReason`, each key
//! once, nothing else. Anything else is ignored, as the reference client does: a newer hub may
//! add messages, and the close code still tells how the session ended. Nothing of a message is
//! ever printed (the caller sees only the parsed value).

use serde::Deserialize;

use super::ending::EndReason;

/// The size the hub accepts on each side, in cells (K4 Section 8.4).
pub const CELLS: std::ops::RangeInclusive<u16> = 1..=1000;
/// No hub message of the closed set is longer; a longer text is not parsed at all.
pub const MAX_TEXT: usize = 256;
/// The largest input frame sent; the hub takes up to 64 KiB (K4 Section 8.5).
pub const MAX_INPUT_FRAME: usize = 32 * 1024;
/// The client's `ping`.
pub const PING: &str = r#"{"type":"ping"}"#;

/// A terminal size in character cells.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CellSize {
    pub cols: u16,
    pub rows: u16,
}

impl CellSize {
    /// The size when the terminal does not say: 80 by 24.
    pub const DEFAULT: Self = Self { cols: 80, rows: 24 };

    /// Each side within [`CELLS`].
    #[must_use]
    pub fn clamped(self) -> Self {
        let side = |value: u16| value.clamp(*CELLS.start(), *CELLS.end());
        Self {
            cols: side(self.cols),
            rows: side(self.rows),
        }
    }
}

/// The `resize` message for `size`, clamped.
#[must_use]
pub fn resize(size: CellSize) -> String {
    let CellSize { cols, rows } = size.clamped();
    format!(r#"{{"type":"resize","cols":{cols},"rows":{rows}}}"#)
}

/// A message from the hub's closed set.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HubMessage {
    Ping,
    End(EndReason),
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "lowercase", deny_unknown_fields)]
enum Wire {
    Ping {},
    End { reason: String },
}

/// The hub message `text` holds, or `None` for anything outside the closed set.
#[must_use]
pub fn decode(text: &str) -> Option<HubMessage> {
    if text.len() > MAX_TEXT || !text.trim_start().starts_with('{') {
        return None;
    }
    match serde_json::from_str::<Wire>(text).ok()? {
        Wire::Ping {} => Some(HubMessage::Ping),
        Wire::End { reason } => EndReason::parse(&reason).map(HubMessage::End),
    }
}

/// `bytes` cut into frames of at most [`MAX_INPUT_FRAME`], in order.
pub fn input_frames(bytes: &[u8]) -> impl Iterator<Item = Vec<u8>> + '_ {
    bytes.chunks(MAX_INPUT_FRAME).map(<[u8]>::to_vec)
}

/// The size the hub holds: a size is sent only when it differs from the last one sent.
#[derive(Debug, Default)]
pub struct Sizing {
    sent: Option<CellSize>,
}

impl Sizing {
    /// The `resize` to send for `size`, or `None` when the hub already has it.
    pub fn update(&mut self, size: CellSize) -> Option<String> {
        let size = size.clamped();
        if self.sent == Some(size) {
            return None;
        }
        self.sent = Some(size);
        Some(resize(size))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn k4_8_4_resize_is_clamped_to_one_to_a_thousand() {
        assert_eq!(
            resize(CellSize {
                cols: 132,
                rows: 43
            }),
            r#"{"type":"resize","cols":132,"rows":43}"#
        );
        assert_eq!(
            resize(CellSize {
                cols: 0,
                rows: 5000
            }),
            r#"{"type":"resize","cols":1,"rows":1000}"#
        );
    }

    #[test]
    fn k4_8_4_a_size_the_hub_has_is_not_sent_again() {
        let mut sizing = Sizing::default();
        assert!(sizing.update(CellSize::DEFAULT).is_some());
        assert!(sizing.update(CellSize::DEFAULT).is_none());
        assert!(sizing.update(CellSize { cols: 81, rows: 24 }).is_some());
    }

    #[test]
    fn k4_8_4_only_the_closed_set_is_read() {
        assert_eq!(decode(r#"{"type":"ping"}"#), Some(HubMessage::Ping));
        assert_eq!(
            decode(r#" { "type" : "end", "reason" : "HOST_KEY_MISMATCH" } "#),
            Some(HubMessage::End(EndReason::HostKeyMismatch))
        );
        for text in [
            r#"{"type":"ping","x":1}"#,
            r#"{"type":"end"}"#,
            r#"{"type":"end","reason":"SOMETHING_NEW"}"#,
            r#"{"type":"end","reason":"OPERATOR","reason":"OPERATOR"}"#,
            r#"{"type":"resize","cols":1,"rows":1}"#,
            r#"[{"type":"ping"}]"#,
            r#""ping""#,
            "",
            "{",
        ] {
            assert_eq!(decode(text), None, "{text}");
        }
        let long = format!(r#"{{"type":"ping"{}}}"#, " ".repeat(MAX_TEXT));
        assert_eq!(decode(&long), None);
    }

    #[test]
    fn k4_8_5_input_is_cut_under_the_frame_limit_in_order() {
        let bytes: Vec<u8> = (0..=255).cycle().take(MAX_INPUT_FRAME * 2 + 7).collect();
        let frames: Vec<Vec<u8>> = input_frames(&bytes).collect();
        assert_eq!(frames.len(), 3);
        assert!(frames.iter().all(|frame| frame.len() <= MAX_INPUT_FRAME));
        assert_eq!(frames.concat(), bytes);
    }

    /// A small deterministic generator, so the property test needs no extra crate.
    fn next(state: &mut u64) -> u64 {
        *state ^= *state << 13;
        *state ^= *state >> 7;
        *state ^= *state << 17;
        *state
    }

    #[test]
    fn k4_7_no_hub_text_panics_the_decoder_and_only_known_messages_pass() {
        let pieces = [
            "{",
            "}",
            "\"type\"",
            ":",
            "\"ping\"",
            "\"end\"",
            "\"reason\"",
            ",",
            "\"OPERATOR\"",
            "[",
            "]",
            "1e9",
            "-1",
            "null",
            " ",
            "\u{1b}",
            "\"\\u0000\"",
            "\u{fffd}",
            "\"",
        ];
        let mut state = 0x9e37_79b9_7f4a_7c15_u64;
        for _ in 0..20_000 {
            let count = next(&mut state) % 12;
            let mut text = String::new();
            for _ in 0..count {
                let index = usize::try_from(next(&mut state) % pieces.len() as u64).unwrap_or(0);
                text.push_str(pieces[index]);
            }
            match decode(&text) {
                None | Some(HubMessage::Ping) => {}
                Some(HubMessage::End(reason)) => assert!(text.contains(reason.as_str())),
            }
        }
    }
}
