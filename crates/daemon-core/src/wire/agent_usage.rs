//! The `agentUsage` member of a batch (K8 Section 4; R-135, AIS-018, AIS-019, AIS-024), and the
//! hub's pure check of it.

use std::collections::BTreeSet;

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use crate::agent_usage::{
    AgentUsage, MAX_COUNT, MAX_MODELS, MAX_SESSION_DAYS, ModelTokens, SessionDay, Source, Tokens,
    UsageState, is_model, is_repository, is_session_id, is_word,
};

/// The use of the AI agent programs as a batch spells it: what the daemon can read, and the
/// session days whose counts changed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchAgentUsage {
    pub state: String,
    pub session_days: Vec<BatchSessionDay>,
}

/// The counts of one session on one UTC day. `source` is text, so an entry of a source this
/// hub does not know can be dropped.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchSessionDay {
    pub source: String,
    pub id: String,
    #[serde(with = "super::times::days")]
    pub day: NaiveDate,
    #[serde(with = "super::times")]
    pub started_at: DateTime<Utc>,
    #[serde(with = "super::times")]
    pub ended_at: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,
    pub prompts: u64,
    pub tool_calls: u64,
    pub models: Vec<BatchModel>,
}

/// The tokens of one model on one session day.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchModel {
    pub model: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_tokens: u64,
    pub cache_write_tokens: u64,
}

impl BatchAgentUsage {
    /// The member a daemon in `state` sends with `session_days`; the caller gives at most
    /// [`MAX_SESSION_DAYS`], and none unless it reads.
    #[must_use]
    pub fn new(state: UsageState, session_days: &[SessionDay]) -> Self {
        Self {
            state: state.as_str().to_owned(),
            session_days: session_days.iter().map(BatchSessionDay::from).collect(),
        }
    }

    /// What the hub stores: the state when it knows it, and the session days of the sources it
    /// knows; `None` when the member breaks K8 Section 4: a `state` or a `source` that is not 1
    /// to 32 bytes of `[a-z_]`; session days with a known `state` other than `reading`; more
    /// than 16 session days, or two with one source, ID and day; an entry that breaks
    /// [`BatchSessionDay::fits`].
    #[must_use]
    pub fn accepted(self) -> Option<AgentUsage> {
        let state = UsageState::parse(&self.state);
        let keys: BTreeSet<(&str, &str, NaiveDate)> = self
            .session_days
            .iter()
            .map(|entry| (entry.source.as_str(), entry.id.as_str(), entry.day))
            .collect();
        let silent = matches!(state, Some(UsageState::SystemUnit | UsageState::TurnedOff));
        let fits = is_word(&self.state)
            && self.session_days.len() <= MAX_SESSION_DAYS
            && keys.len() == self.session_days.len()
            && (!silent || self.session_days.is_empty())
            && self.session_days.iter().all(BatchSessionDay::fits);
        if !fits {
            return None;
        }
        let session_days = self
            .session_days
            .into_iter()
            .filter_map(BatchSessionDay::known)
            .collect();
        Some(AgentUsage {
            state,
            session_days,
        })
    }
}

impl BatchSessionDay {
    /// Whether the entry keeps the K8 rules: a `source` of `[a-z_]`, an ID of 1 to 64 bytes of
    /// `[0-9A-Za-z_-]`, a start not after the end and a `day` between their dates, a
    /// repository by [`is_repository`], counts of at most 2^53 − 1, and at most 16 models with
    /// unique names by [`is_model`].
    #[must_use]
    pub fn fits(&self) -> bool {
        let names: BTreeSet<&str> = self.models.iter().map(|one| one.model.as_str()).collect();
        is_word(&self.source)
            && is_session_id(&self.id)
            && self.started_at <= self.ended_at
            && (self.started_at.date_naive()..=self.ended_at.date_naive()).contains(&self.day)
            && self.repository.as_deref().is_none_or(is_repository)
            && self.prompts <= MAX_COUNT
            && self.tool_calls <= MAX_COUNT
            && self.models.len() <= MAX_MODELS
            && names.len() == self.models.len()
            && self
                .models
                .iter()
                .all(|one| is_model(&one.model) && one.tokens().fits())
    }

    /// The session day of a source this side knows; `None` for any other.
    fn known(self) -> Option<SessionDay> {
        Some(SessionDay {
            source: Source::parse(&self.source)?,
            id: self.id,
            day: self.day,
            started_at: self.started_at,
            ended_at: self.ended_at,
            repository: self.repository,
            prompts: self.prompts,
            tool_calls: self.tool_calls,
            models: self
                .models
                .into_iter()
                .map(|one| ModelTokens {
                    tokens: one.tokens(),
                    model: one.model,
                })
                .collect(),
        })
    }
}

impl BatchModel {
    const fn tokens(&self) -> Tokens {
        Tokens {
            input: self.input_tokens,
            output: self.output_tokens,
            cache_read: self.cache_read_tokens,
            cache_write: self.cache_write_tokens,
        }
    }
}

impl From<&SessionDay> for BatchSessionDay {
    fn from(day: &SessionDay) -> Self {
        Self {
            source: day.source.as_str().to_owned(),
            id: day.id.clone(),
            day: day.day,
            started_at: day.started_at,
            ended_at: day.ended_at,
            repository: day.repository.clone(),
            prompts: day.prompts,
            tool_calls: day.tool_calls,
            models: day
                .models
                .iter()
                .map(|one| BatchModel {
                    model: one.model.clone(),
                    input_tokens: one.tokens.input,
                    output_tokens: one.tokens.output,
                    cache_read_tokens: one.tokens.cache_read,
                    cache_write_tokens: one.tokens.cache_write,
                })
                .collect(),
        }
    }
}
