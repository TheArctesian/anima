//! From each 2-second reading to a point every 10 seconds (MET-014): pure, the time is a
//! parameter. [`gpu`] gives the GPU readings their series and their place among the GPUs.

pub mod gpu;

use std::time::Duration;

use agent_core::points::{Point, SeriesKey, Window, samples};
use agent_core::readings::{GpuReading, Reading};
use chrono::{DateTime, Utc};

/// The previous reading and the open 10-second window.
#[derive(Debug, Default)]
pub struct Sampler {
    previous: Option<(DateTime<Utc>, Reading)>,
    window: Window,
}

impl Sampler {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Takes the sampling made at `at`: the readings of the device and its GPUs (already placed
    /// by [`gpu::Placement`]). Returns the point of the window this sampling closed, if any.
    /// The rates need the reading before, so the first sampling has none; time that did not
    /// move forward gives none either.
    pub fn take(
        &mut self,
        at: DateTime<Utc>,
        reading: Reading,
        gpus: &[GpuReading],
    ) -> Option<Point> {
        let before = self.previous.as_ref();
        let elapsed = before
            .and_then(|(was, _)| (at - *was).to_std().ok())
            .unwrap_or(Duration::ZERO);
        let mut values: Vec<(SeriesKey, f64)> = samples(before.map(|(_, r)| r), &reading, elapsed)
            .iter()
            .filter_map(|sample| Some((SeriesKey::of(sample)?, sample.value)))
            .collect();
        values.extend(gpu::values(gpus));
        self.previous = Some((at, reading));
        self.window.add(at, values)
    }
}

#[cfg(test)]
mod tests {
    use agent_core::readings::{Cpu, Interface, Memory};

    use super::*;

    fn at(seconds: i64) -> DateTime<Utc> {
        DateTime::from_timestamp(1_000_000_000 + seconds, 0).expect("a time")
    }

    fn reading(idle: u64, total: u64, received: u64) -> Reading {
        Reading {
            cpu: Some(Cpu::Ticks { idle, total }),
            memory: Some(Memory {
                total_bytes: 100,
                used_bytes: 25,
            }),
            interfaces: vec![Interface {
                name: "eth0".to_owned(),
                received_bytes: received,
                sent_bytes: 0,
            }],
            ..Reading::default()
        }
    }

    #[test]
    fn met_014_five_samplings_make_one_point_per_10_second_window() {
        let mut sampler = Sampler::new();
        // 1_000_000_000 is a multiple of 10 seconds: the sampling at +0 is a window of its own,
        // the five at +2 to +10 make the next, and +12 opens the third and closes that one.
        let mut points = Vec::new();
        for step in 0..=6 {
            let seconds = step * 2;
            let ticks = u64::try_from(step).expect("small") * 100;
            let taken = sampler.take(at(seconds), reading(ticks, ticks * 2, ticks * 1000), &[]);
            points.extend(taken);
        }
        assert_eq!(points.len(), 2, "two windows closed, the third is open");
        assert_eq!(points[1].at(), at(10));
        let cpu = points[1]
            .values()
            .iter()
            .find(|(key, _)| key.kind().as_str() == "cpu_percent")
            .map(|(_, summary)| *summary)
            .expect("a CPU value");
        assert!((cpu.mean - 50.0).abs() < 1e-9);
        assert_eq!(cpu.max, Some(50.0));
    }

    #[test]
    fn met_014_the_first_sampling_has_no_rate_and_no_cpu_share() {
        let mut sampler = Sampler::new();
        sampler.take(at(0), reading(0, 100, 0), &[]);
        let point = sampler
            .take(at(12), reading(50, 200, 0), &[])
            .expect("closes");
        let kinds: Vec<&str> = point
            .values()
            .keys()
            .map(|key| key.kind().as_str())
            .collect();
        assert!(kinds.contains(&"memory_percent"));
        assert!(
            !kinds.contains(&"cpu_percent"),
            "the first sampling has no CPU share"
        );
    }
}
