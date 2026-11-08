//! The samples of each known GPU from one collection's answers (MET-006, MET-007, MET-009).
//! Pure.

use agent_core::readings::GpuReading;

use super::{
    answers::Answer,
    discovery::{Discovered, Found},
};
use crate::metrics::{GpuVendor, MetricKind, Sample, rates::float};

/// Busy share, VRAM share and bytes, temperature and power of each GPU in `known` whose
/// vendor answered with its number, labelled with the GPU's device number. A GPU with no
/// answer this time, or a value its reading lacks, gives no sample, never zero; VRAM only for
/// a GPU found with VRAM (MET-007). A GPU the device reports that is not in `known` waits for
/// the next discovery.
pub(in crate::metrics::collector) fn samples(
    known: &Discovered,
    answers: &[(GpuVendor, Answer)],
) -> Vec<Sample> {
    let mut out = Vec::new();
    for found in &known.gpus {
        if let Some(reading) = reading_of(found, answers) {
            add(&mut out, found, reading);
        }
    }
    out
}

/// The reading of `found` among the answers of its vendor.
fn reading_of<'a>(found: &Found, answers: &'a [(GpuVendor, Answer)]) -> Option<&'a GpuReading> {
    answers
        .iter()
        .filter(|(vendor, _)| *vendor == found.gpu.vendor)
        .find_map(|(_, answer)| match answer {
            Answer::Gpus(readings) => readings
                .iter()
                .find(|reading| reading.index == found.source),
            Answer::Unknown => None,
        })
}

fn add(out: &mut Vec<Sample>, found: &Found, reading: &GpuReading) {
    let index = found.gpu.index;
    let sample = |kind, value| Sample::gpu(kind, index, value);
    out.extend(
        reading
            .busy_percent
            .and_then(|busy| sample(MetricKind::GpuPercent, busy)),
    );
    if let Some(vram) = reading.vram.filter(|_| found.gpu.has_vram) {
        let used = vram.used_bytes.min(vram.total_bytes);
        let share = float(used) / float(vram.total_bytes) * 100.0;
        out.extend(sample(MetricKind::VramPercent, share));
        out.extend(sample(MetricKind::VramBytes, float(used)));
    }
    out.extend(
        reading
            .temperature_c
            .and_then(|celsius| sample(MetricKind::GpuTemperatureC, celsius)),
    );
    out.extend(
        reading
            .power_w
            .and_then(|watts| sample(MetricKind::GpuPowerW, watts)),
    );
}
