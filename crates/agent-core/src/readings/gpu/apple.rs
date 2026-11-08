//! Reader of `ioreg -r -d 1 -c IOAccelerator` on Apple silicon (MET-016).

use super::{GpuReading, GpuVendor};
use crate::readings::{label, lines, number};

/// The name of a GPU whose entry has no usable `model`.
const FALLBACK_NAME: &str = "Apple GPU";

/// One reading per `IOAccelerator` entry, indexed in the order they print. From each entry:
/// `"Device Utilization %"` is busy, `"In use system memory"` is the memory the GPU holds, and
/// `"model"` is the name; a number is read up to its first character that is not a digit or
/// a point, so `1e3` gives 1. There is no VRAM: the GPU shares system memory (MET-007). An entry
/// with none of the three is skipped.
#[must_use]
pub fn gpus(output: &[u8]) -> Vec<GpuReading> {
    let mut entries: Vec<Entry<'_>> = Vec::new();
    for line in lines(output) {
        if line.trim_start_matches(['|', ' ']).starts_with("+-o ") || entries.is_empty() {
            entries.push(Entry::default());
        }
        if let Some(entry) = entries.last_mut() {
            entry.read(line);
        }
    }
    entries
        .into_iter()
        .filter(|entry| entry.model.is_some() || entry.busy.is_some() || entry.memory.is_some())
        .zip(0..)
        .map(|(entry, index)| GpuReading {
            index,
            name: entry.model.unwrap_or(FALLBACK_NAME).to_owned(),
            vendor: GpuVendor::Apple,
            busy_percent: entry.busy,
            vram: None,
            temperature_c: None,
            power_w: None,
            shared_memory_bytes: entry.memory,
        })
        .collect()
}

#[derive(Default)]
struct Entry<'a> {
    model: Option<&'a str>,
    busy: Option<f64>,
    memory: Option<u64>,
}

impl<'a> Entry<'a> {
    fn read(&mut self, line: &'a str) {
        if self.model.is_none() {
            self.model = value(line, "model")
                .and_then(|text| text.strip_prefix('"')?.split_once('"'))
                .and_then(|(model, _)| label(model));
        }
        if self.busy.is_none() {
            self.busy = value(line, "Device Utilization %")
                .and_then(|text| number(leading(text, |c| c.is_ascii_digit() || c == '.')));
        }
        if self.memory.is_none() {
            self.memory = value(line, "In use system memory")
                .and_then(|text| leading(text, |c| c.is_ascii_digit()).parse().ok());
        }
    }
}

/// The text after `"key" =` in `line`, without the spaces before it.
fn value<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    let after = line.match_indices(key).find_map(|(at, _)| {
        let before = line.get(..at)?;
        let after = line.get(at + key.len()..)?;
        before.ends_with('"').then_some(())?;
        after.strip_prefix('"')
    })?;
    Some(after.trim_start().strip_prefix('=')?.trim_start())
}

fn leading(text: &str, keep: fn(char) -> bool) -> &str {
    let end = text.find(|c| !keep(c)).unwrap_or(text.len());
    text.get(..end).unwrap_or_default()
}
