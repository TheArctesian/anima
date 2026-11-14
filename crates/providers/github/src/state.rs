//! The provider's slice of the sync state (SRC-004): where a run is and what it has seen.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

const VERSION: u8 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Phase {
    List,
    Signs,
    Done,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Step {
    Tree,
    Deployments,
}

/// A repository whose signs are still to be read.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Pending {
    pub id: String,
    pub name: String,
    pub branch: String,
    pub pushed_at: Option<String>,
    pub step: Step,
}

/// `{"v":1,"phase","page","listed","pending","seen"}`. `seen` maps a repository id to the
/// `pushed_at` its signs were last read at.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Slice {
    pub v: u8,
    pub phase: Phase,
    pub page: u32,
    pub listed: Vec<String>,
    pub pending: Vec<Pending>,
    pub seen: BTreeMap<String, Option<String>>,
}

impl Slice {
    pub fn fresh() -> Self {
        Self {
            v: VERSION,
            phase: Phase::List,
            page: 1,
            listed: Vec::new(),
            pending: Vec::new(),
            seen: BTreeMap::new(),
        }
    }

    /// The slice in `state`; a fresh one when it is empty, unreadable or of another version.
    pub fn read(state: &Value) -> Self {
        serde_json::from_value::<Self>(state.clone())
            .ok()
            .filter(|slice| slice.v == VERSION)
            .unwrap_or_else(Self::fresh)
    }

    pub fn to_value(&self) -> Value {
        serde_json::to_value(self).unwrap_or(Value::Null)
    }

    /// Moves to `Done` when no repository is left to read.
    pub fn settle(&mut self) {
        if self.phase == Phase::Signs && self.pending.is_empty() {
            self.phase = Phase::Done;
        }
    }
}
