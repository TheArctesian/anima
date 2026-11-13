//! The `amdgpu` files of several cards as `grep -H . <files>` prints them, one card at a time
//! through the agent's per-file readers (MET-016). Pure.

use std::collections::BTreeMap;

use agent_core::readings::{
    GpuReading,
    gpu::amd::{self, CardFiles},
};

use crate::fleet::metrics::MAX_LABELS;

/// The start of each path the command reads, before the card number.
const CARDS: &str = "/sys/class/drm/card";

/// Which of a card's files a line holds.
#[derive(Clone, Copy)]
enum File {
    Busy,
    VramUsed,
    VramTotal,
}

/// The lines each file of one card printed, newline included, empty when it printed none.
#[derive(Default)]
struct Card<'a> {
    busy: &'a [u8],
    vram_used: &'a [u8],
    vram_total: &'a [u8],
}

/// One reading per card number found, in card order, at most [`MAX_LABELS`] cards. A line
/// that is not `/sys/class/drm/card<number>/device/<file>:<value>` for one of the three files
/// is skipped, and so is a repeat of a file already read. Sysfs gives no name, so the reader
/// names each card.
pub(in crate::fleet::metrics::collector) fn cards(output: &[u8]) -> Vec<GpuReading> {
    let mut cards: BTreeMap<u32, Card<'_>> = BTreeMap::new();
    let complete = output.split_inclusive(|byte| *byte == b'\n');
    for (number, file, value) in complete.filter_map(line) {
        if cards.len() >= MAX_LABELS && !cards.contains_key(&number) {
            continue;
        }
        let card = cards.entry(number).or_default();
        let slot = match file {
            File::Busy => &mut card.busy,
            File::VramUsed => &mut card.vram_used,
            File::VramTotal => &mut card.vram_total,
        };
        if slot.is_empty() {
            *slot = value;
        }
    }
    cards
        .into_iter()
        .map(|(number, card)| {
            let files = CardFiles {
                busy_percent: card.busy,
                vram_used: card.vram_used,
                vram_total: card.vram_total,
            };
            amd::card(number, "", files)
        })
        .collect()
}

/// The card number, the file and the value (with its newline) of one complete line.
fn line(line: &[u8]) -> Option<(u32, File, &[u8])> {
    if !line.ends_with(b"\n") {
        return None;
    }
    let colon = line.iter().position(|byte| *byte == b':')?;
    let (path, value) = (line.get(..colon)?, line.get(colon + 1..)?);
    let (number, name) = std::str::from_utf8(path)
        .ok()?
        .strip_prefix(CARDS)?
        .split_once("/device/")?;
    if number.is_empty() || !number.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let file = match name {
        "gpu_busy_percent" => File::Busy,
        "mem_info_vram_used" => File::VramUsed,
        "mem_info_vram_total" => File::VramTotal,
        _ => return None,
    };
    Some((number.parse().ok()?, file, value))
}
