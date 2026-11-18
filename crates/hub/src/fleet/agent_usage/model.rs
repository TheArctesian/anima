//! What the operator asks of the agent usage and what the hub answers (AIS-020, AIS-021,
//! AIS-024).
#![expect(
    clippy::unused_async_trait_impl,
    reason = "the async-graphql Enum derive generates async trait impls"
)]

use async_graphql::Enum;
use chrono::NaiveDate;
use daemon_core::agent_usage::{Source, UsageState};

use crate::ids::DeviceId;

/// How many days the range holds when the operator gives none (AIS-020).
pub(super) const DEFAULT_DAYS: u64 = 30;

/// `AgentUsageDimension`: a dimension the sums are grouped by.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Enum)]
#[graphql(name = "AgentUsageDimension")]
pub enum Dimension {
    Device,
    Source,
    Model,
    Repository,
    /// The UTC day.
    Day,
}

impl Dimension {
    const fn bit(self) -> u8 {
        match self {
            Self::Device => 1,
            Self::Source => 2,
            Self::Model => 4,
            Self::Repository => 8,
            Self::Day => 16,
        }
    }
}

/// The dimensions a query groups by; none: one group of everything.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DimensionSet(u8);

impl DimensionSet {
    /// The set of `dimensions`; a dimension named twice counts once.
    #[must_use]
    pub fn of(dimensions: &[Dimension]) -> Self {
        Self(dimensions.iter().fold(0, |set, one| set | one.bit()))
    }

    /// Whether the set holds `dimension`.
    #[must_use]
    pub const fn has(self, dimension: Dimension) -> bool {
        self.0 & dimension.bit() != 0
    }
}

/// One query: the filter, the days from `first_day` to `last_day` (both included) and the
/// dimensions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ask {
    pub device: Option<DeviceId>,
    pub source: Option<Source>,
    /// `host/owner/name`, matched exactly.
    pub repository: Option<String>,
    pub first_day: NaiveDate,
    pub last_day: NaiveDate,
    pub by: DimensionSet,
}

/// The values of one group: `None` where the dimension is not asked for, and where the
/// session has no value (no repository, no model).
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct GroupKey {
    pub device: Option<DeviceId>,
    pub source: Option<Source>,
    pub model: Option<String>,
    pub repository: Option<String>,
    pub day: Option<NaiveDate>,
}

/// Token sums. `f64`: a sum of counts of at most 2^53 − 1 each, exact while it stays under
/// 2^53.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TokenSums {
    pub input: f64,
    pub output: f64,
    pub cache_read: f64,
    pub cache_write: f64,
}

impl TokenSums {
    /// All four counts together.
    #[must_use]
    pub fn all(self) -> f64 {
        self.input + self.output + self.cache_read + self.cache_write
    }

    #[must_use]
    pub fn plus(self, more: Self) -> Self {
        Self {
            input: self.input + more.input,
            output: self.output + more.output,
            cache_read: self.cache_read + more.cache_read,
            cache_write: self.cache_write + more.cache_write,
        }
    }
}

/// The sums of one group.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Sums {
    /// The sessions with use in the group; a session on two days counts on each day.
    pub sessions: i64,
    pub prompts: f64,
    pub tool_calls: f64,
    pub tokens: TokenSums,
}

impl Sums {
    #[must_use]
    pub fn plus(self, more: Self) -> Self {
        Self {
            sessions: self.sessions.saturating_add(more.sessions),
            prompts: self.prompts + more.prompts,
            tool_calls: self.tool_calls + more.tool_calls,
            tokens: self.tokens.plus(more.tokens),
        }
    }
}

/// The sums of one group for one model, as the store reads them: the tokens of the model, and
/// the sessions, prompts and tool calls of the sessions whose model it is. Summed over the
/// models, each count is counted once.
#[derive(Clone, Debug, PartialEq)]
pub struct Grain {
    /// `model` is always the grain's model, whether or not the query groups by model.
    pub key: GroupKey,
    pub sums: Sums,
}

/// One group of the answer, with the estimate of its cost (AIS-021).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Group {
    pub key: GroupKey,
    pub sums: Sums,
    /// The tokens of models with a price, times their public list prices; `None` when no
    /// token of the group has a price. An estimate, never the bill of a subscription.
    pub estimated_cost_usd: Option<f64>,
    /// The tokens of models with no known price: not in the estimate.
    pub unpriced_tokens: f64,
}

/// The usage state a device's daemon sent last (AIS-024).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DeviceState {
    pub device: DeviceId,
    pub state: UsageState,
}

/// The answer to an [`Ask`].
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Report {
    /// In no set order.
    pub groups: Vec<Group>,
    /// Everything the filter admits, with no dimension.
    pub total: Group,
    /// Of each device the device filter admits.
    pub states: Vec<DeviceState>,
}
