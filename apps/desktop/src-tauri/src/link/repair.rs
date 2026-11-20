//! The repair of a kept link while the app runs (HMD-058): something outside the app can remove
//! the unit of the user or end its link (a cleaning tool deletes a launch agent that it does
//! not know). When the watched link stays down, the app installs and starts the unit again,
//! a few times for each run at most, so that a unit that cannot run is not started for ever.

/// How many looks in a row find no link before a repair: about 30 s at one look each 2 s
/// ([`super::device::WATCH_EVERY`]).
pub const DOWN_LOOKS: u32 = 15;
/// How many repairs one run of the app tries.
pub const TRIES: u32 = 3;

/// The count of the looks that found no link, and of the repairs of this run.
#[derive(Debug, Default)]
pub struct Repair {
    down: u32,
    tried: u32,
}

impl Repair {
    /// Records one look at the link. True when the unit must be put back now: the link was
    /// down for [`DOWN_LOOKS`] looks in a row and this run has a try left. The count of the
    /// looks starts again after a repair, so the next one is as long after it.
    pub fn due(&mut self, up: bool) -> bool {
        if up {
            self.down = 0;
            return false;
        }
        self.down += 1;
        if self.down < DOWN_LOOKS || self.tried >= TRIES {
            return false;
        }
        self.down = 0;
        self.tried += 1;
        true
    }
}
