//! The `inventory` member of a batch (K8 Section 4; CAP-020, CAP-024, CAP-029), and the hub's
//! pure check of it.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::inventory::{
    Architecture, Inventory, MAX_BUILD_TARGETS, MAX_TOOLS, Os, Tool, ToolName, is_build_target,
    is_tool_name, is_version,
};

/// The device's tool inventory as a batch spells it. It replaces the stored one whole.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchInventory {
    pub architecture: Architecture,
    pub os: Os,
    pub os_version: String,
    pub tools: Vec<BatchTool>,
    pub build_targets: Vec<String>,
}

/// One tool found: a version, or the state `failed` and no version.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BatchTool {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<ToolState>,
}

/// Why a found tool has no version: a fixed code, never text from the device. A state a newer
/// agent knows and this hub does not reads as `failed`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ToolState {
    /// The probe failed, timed out or printed no version it could read.
    #[serde(other)]
    Failed,
}

impl From<&Inventory> for BatchInventory {
    fn from(inventory: &Inventory) -> Self {
        Self {
            architecture: inventory.architecture,
            os: inventory.os,
            os_version: inventory.os_version.clone(),
            tools: inventory.tools.iter().map(BatchTool::from).collect(),
            build_targets: inventory.build_targets.clone(),
        }
    }
}

impl From<&Tool> for BatchTool {
    fn from(tool: &Tool) -> Self {
        Self {
            name: tool.name.as_str().to_owned(),
            version: tool.version.clone(),
            state: tool.version.is_none().then_some(ToolState::Failed),
        }
    }
}

impl BatchInventory {
    /// The inventory the hub stores, without the tools whose name it does not know; `None` when
    /// the member breaks K8 Section 4: an `osVersion` or a version that is not 1 to 64 bytes of
    /// `[0-9A-Za-z.+_-]`; more than 64 tools or two with one name; a name that is not 1 to 64
    /// bytes of `[0-9a-z_.-]`; a tool with both a version and a state, or neither; more than 128
    /// build targets, two equal, or one that is not 1 to 64 bytes of `[0-9a-z_.-]`.
    #[must_use]
    pub fn accepted(self) -> Option<Inventory> {
        let names: BTreeSet<&str> = self.tools.iter().map(|tool| tool.name.as_str()).collect();
        let targets: BTreeSet<&str> = self.build_targets.iter().map(String::as_str).collect();
        let fits = is_version(&self.os_version)
            && self.tools.len() <= MAX_TOOLS
            && names.len() == self.tools.len()
            && self.tools.iter().all(BatchTool::fits)
            && self.build_targets.len() <= MAX_BUILD_TARGETS
            && targets.len() == self.build_targets.len()
            && targets.iter().all(|target| is_build_target(target));
        if !fits {
            return None;
        }
        let mut tools: Vec<Tool> = self
            .tools
            .into_iter()
            .filter_map(|tool| {
                Some(Tool {
                    name: ToolName::parse(&tool.name)?,
                    version: tool.version,
                })
            })
            .collect();
        tools.sort_by_key(|tool| tool.name);
        let mut build_targets = self.build_targets;
        build_targets.sort_unstable();
        Some(Inventory {
            architecture: self.architecture,
            os: self.os,
            os_version: self.os_version,
            tools,
            build_targets,
        })
    }
}

impl BatchTool {
    fn fits(&self) -> bool {
        let told = match (&self.version, self.state) {
            (Some(version), None) => is_version(version),
            (None, Some(ToolState::Failed)) => true,
            _ => false,
        };
        told && is_tool_name(&self.name)
    }
}
