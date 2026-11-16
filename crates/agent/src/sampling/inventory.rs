//! The tool inventory a batch announces: after start and each time a probe result changes, so
//! an unchanged inventory is not sent with every batch (CAP-027, K8 Section 4).

use agent_core::inventory::Inventory;
use agent_core::wire::BatchInventory;

/// The inventory last handed to the pusher.
#[derive(Debug, Default)]
pub struct InventoryWatch {
    announced: Option<Inventory>,
}

impl InventoryWatch {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The `inventory` member to send when `read` differs from the one announced last; `None`
    /// when it is the same, or when nothing could be read (the hub keeps what it has).
    pub fn changed(&mut self, read: Option<Inventory>) -> Option<BatchInventory> {
        let read = read?;
        if self.announced.as_ref() == Some(&read) {
            return None;
        }
        let sent = BatchInventory::from(&read);
        self.announced = Some(read);
        Some(sent)
    }
}

#[cfg(test)]
mod tests {
    use agent_core::inventory::{Architecture, Os, Tool, ToolName};

    use super::*;

    fn mac(cargo: Option<&str>) -> Inventory {
        Inventory {
            architecture: Architecture::Aarch64,
            os: Os::Macos,
            os_version: "27.0".to_owned(),
            tools: vec![Tool {
                name: ToolName::Cargo,
                version: cargo.map(str::to_owned),
            }],
            build_targets: vec!["aarch64-apple-darwin".to_owned()],
        }
    }

    #[test]
    fn cap_027_the_inventory_is_announced_first_then_only_when_a_probe_changes() {
        let mut watch = InventoryWatch::new();
        let first = watch.changed(Some(mac(Some("1.91.0")))).expect("at start");
        assert_eq!(first.tools[0].version.as_deref(), Some("1.91.0"));
        assert_eq!(watch.changed(Some(mac(Some("1.91.0")))), None, "unchanged");
        assert_eq!(watch.changed(None), None, "unread is not empty");
        let upgraded = watch.changed(Some(mac(Some("1.92.0")))).expect("upgraded");
        assert_eq!(upgraded.tools[0].version.as_deref(), Some("1.92.0"));
        let broken = watch.changed(Some(mac(None))).expect("the probe failed");
        assert_eq!(
            broken.tools[0].state,
            Some(agent_core::wire::ToolState::Failed)
        );
    }
}
