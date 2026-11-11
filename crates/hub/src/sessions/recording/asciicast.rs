//! The asciicast v2 encoder for SSH recordings (K4 Section 11, RA-041, D-12). Pure: it
//! reads no clock and does no I/O; each call returns the bytes to write.
//!
//! The file is a header line, then one JSON line per event: `[t,"o",text]` for output and
//! `[t,"r","CxR"]` for a resize, `t` in seconds since the start. There is no input event.
//! Output bytes go through a streaming UTF-8 decoder: a sequence split across two frames is
//! joined, and bad bytes become U+FFFD exactly as `String::from_utf8_lossy` would on the
//! whole stream.
#![cfg_attr(
    not(test),
    expect(dead_code, reason = "used by the recorder in WP-48 T4")
)]

use std::time::Duration;

use chrono::{DateTime, Utc};
use serde_json::json;

/// An asciicast v2 recording being written: the decoder's carried bytes and the last event time.
pub(super) struct Cast {
    /// The start of an unfinished multi-byte sequence from the last frame (at most 3 bytes).
    carried: Vec<u8>,
    /// The latest event time, so time never goes backwards.
    last: Duration,
}

impl Cast {
    /// Starts a recording at `started_at` with the first terminal size; returns it with the
    /// header line.
    pub(super) fn start(cols: u16, rows: u16, started_at: DateTime<Utc>) -> (Self, Vec<u8>) {
        let header = json!({
            "version": 2,
            "width": cols,
            "height": rows,
            "timestamp": started_at.timestamp(),
        });
        (
            Self {
                carried: Vec::new(),
                last: Duration::ZERO,
            },
            line(&header),
        )
    }

    /// The output event for a frame, `elapsed` after the start. Empty when the frame holds
    /// only the start of a split sequence.
    pub(super) fn output(&mut self, elapsed: Duration, frame: &[u8]) -> Vec<u8> {
        let text = self.decode(frame);
        if text.is_empty() {
            return Vec::new();
        }
        let t = self.time(elapsed);
        line(&json!([t, "o", text]))
    }

    /// The resize event, `elapsed` after the start.
    pub(super) fn resize(&mut self, elapsed: Duration, cols: u16, rows: u16) -> Vec<u8> {
        let t = self.time(elapsed);
        line(&json!([t, "r", format!("{cols}x{rows}")]))
    }

    /// The last output event: an incomplete sequence still carried becomes U+FFFD. Empty when
    /// nothing is carried.
    pub(super) fn finish(mut self, elapsed: Duration) -> Vec<u8> {
        if self.carried.is_empty() {
            return Vec::new();
        }
        let t = self.time(elapsed);
        line(&json!([t, "o", "\u{FFFD}"]))
    }

    /// Seconds since the start, never before the previous event.
    fn time(&mut self, elapsed: Duration) -> f64 {
        self.last = self.last.max(elapsed);
        let micros = self.last.as_micros();
        #[expect(
            clippy::cast_precision_loss,
            reason = "microseconds fit f64 exactly for any recording shorter than 285 years"
        )]
        let seconds = micros as f64 / 1_000_000.0;
        seconds
    }

    /// Joins the carried bytes with `frame` and decodes all but an incomplete tail.
    fn decode(&mut self, frame: &[u8]) -> String {
        let mut bytes = std::mem::take(&mut self.carried);
        bytes.extend_from_slice(frame);
        let mut text = String::new();
        let mut rest = bytes.as_slice();
        loop {
            match std::str::from_utf8(rest) {
                Ok(valid) => {
                    text.push_str(valid);
                    return text;
                }
                Err(error) => {
                    let (valid, after) = rest.split_at(error.valid_up_to());
                    text.push_str(&String::from_utf8_lossy(valid));
                    if let Some(bad) = error.error_len() {
                        text.push('\u{FFFD}');
                        rest = &after[bad..];
                    } else {
                        self.carried = after.to_vec();
                        return text;
                    }
                }
            }
        }
    }
}

/// One JSON value and its newline.
fn line(value: &serde_json::Value) -> Vec<u8> {
    let mut bytes = value.to_string().into_bytes();
    bytes.push(b'\n');
    bytes
}

#[cfg(test)]
mod tests;
