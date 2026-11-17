//! The tool inventory a batch announces: after start, each time a probe result changes, and
//! once every 24 hours even when unchanged, so an unchanged inventory is not sent with every
//! batch and one lost to a refused batch is sent again (CAP-027, K8 Section 4).

use std::time::{Duration, Instant};

use agent_core::inventory::Inventory;
use agent_core::wire::BatchInventory;

/// The longest an unchanged inventory goes without being announced again (CAP-027).
pub const ANNOUNCE_EVERY: Duration = Duration::from_hours(24);

/// The inventory last handed to the pusher, and when.
#[derive(Debug, Default)]
pub struct InventoryWatch {
    announced: Option<(Inventory, Instant)>,
}

impl InventoryWatch {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The `inventory` member to send at `now`: when `read` differs from the one announced
    /// last, or that was announced 24 hours or more ago; `None` when it is the same and recent,
    /// or when nothing could be read (the hub keeps what it has).
    pub fn changed(&mut self, read: Option<Inventory>, now: Instant) -> Option<BatchInventory> {
        let read = read?;
        if let Some((announced, at)) = &self.announced
            && *announced == read
            && now.saturating_duration_since(*at) < ANNOUNCE_EVERY
        {
            return None;
        }
        let sent = BatchInventory::from(&read);
        self.announced = Some((read, now));
        Some(sent)
    }
}

#[cfg(test)]
mod tests {
    use agent_core::inventory::{Architecture, Os, Reason, Tool, ToolName, Verdict};

    use super::*;

    fn mac(cargo: Verdict) -> Inventory {
        Inventory {
            architecture: Architecture::Aarch64,
            os: Os::Macos,
            os_version: "27.0".to_owned(),
            tools: vec![Tool {
                name: ToolName::Cargo,
                verdict: cargo,
            }],
            build_targets: vec!["aarch64-apple-darwin".to_owned()],
        }
    }

    fn cargo(version: &str) -> Inventory {
        mac(Verdict::Usable(version.to_owned()))
    }

    #[test]
    fn cap_027_the_inventory_is_announced_first_then_only_when_a_probe_changes() {
        let start = Instant::now();
        let mut watch = InventoryWatch::new();
        let first = watch
            .changed(Some(cargo("1.91.0")), start)
            .expect("at start");
        assert_eq!(first.tools[0].version.as_deref(), Some("1.91.0"));
        let later = start + Duration::from_mins(15);
        assert_eq!(
            watch.changed(Some(cargo("1.91.0")), later),
            None,
            "unchanged"
        );
        assert_eq!(watch.changed(None, later), None, "unread is not empty");
        let upgraded = watch
            .changed(Some(cargo("1.92.0")), later)
            .expect("upgraded");
        assert_eq!(upgraded.tools[0].version.as_deref(), Some("1.92.0"));
        let stuck = Some(mac(Verdict::NotUsable(Reason::TimedOut)));
        let broken = watch.changed(stuck, later).expect("the probe failed");
        assert_eq!(broken.tools[0].state, Some(Reason::TimedOut));
    }

    #[test]
    fn cap_027_an_unchanged_inventory_is_announced_again_after_24_hours() {
        let start = Instant::now();
        let mut watch = InventoryWatch::new();
        watch
            .changed(Some(cargo("1.91.0")), start)
            .expect("at start");
        let almost = start + Duration::from_secs(24 * 60 * 60 - 1);
        assert_eq!(watch.changed(Some(cargo("1.91.0")), almost), None);
        let day = start + ANNOUNCE_EVERY;
        assert!(
            watch.changed(Some(cargo("1.91.0")), day).is_some(),
            "once a day"
        );
        assert_eq!(
            watch.changed(Some(cargo("1.91.0")), day + Duration::from_mins(15)),
            None
        );
        let upgraded = watch.changed(Some(cargo("1.92.0")), day + Duration::from_mins(30));
        assert!(upgraded.is_some(), "a change still goes at once");
        let next_day = day + Duration::from_mins(30) + ANNOUNCE_EVERY;
        assert!(
            watch.changed(Some(cargo("1.92.0")), next_day).is_some(),
            "counted from the last"
        );
    }
}
