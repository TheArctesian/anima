//! The `accounts` member of `agentUsage` (K8 Section 4; AIS-022, R-NEW-accountgroup): the key, the
//! plan and the limit windows of each account, and the hub's pure check of them.

use std::collections::BTreeSet;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::agent_usage::account::{
    Account, Limit, MAX_ACCOUNTS, MAX_LIMITS, is_key, is_plan, is_window,
};
use crate::agent_usage::{Source, is_word};

/// One account as a batch spells it. `source` is text, so an account of a source this hub
/// does not know can be dropped.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchAccount {
    pub source: String,
    pub key: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plan: Option<String>,
    #[serde(default)]
    pub limits: Vec<BatchLimit>,
}

/// One limit window of an account.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchLimit {
    pub window: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub used_percent: Option<u8>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "optional_time"
    )]
    pub resets_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub reached: bool,
}

/// The accounts of the known sources; `None` when the list breaks K8 Section 4: more than
/// [`MAX_ACCOUNTS`] entries, two of one source, or an entry that breaks [`BatchAccount::fits`].
#[must_use]
pub fn accepted(accounts: Vec<BatchAccount>) -> Option<Vec<Account>> {
    let sources: BTreeSet<&str> = accounts.iter().map(|one| one.source.as_str()).collect();
    let fits = accounts.len() <= MAX_ACCOUNTS
        && sources.len() == accounts.len()
        && accounts.iter().all(BatchAccount::fits);
    fits.then(|| {
        accounts
            .into_iter()
            .filter_map(BatchAccount::known)
            .collect()
    })
}

impl BatchAccount {
    /// Whether the entry keeps the K8 rules: a `source` of `[a-z_]`, a key by [`is_key`], a plan
    /// by [`is_plan`], and at most [`MAX_LIMITS`] windows with unique names by [`is_window`] and
    /// a used share of at most 100.
    #[must_use]
    pub fn fits(&self) -> bool {
        let windows: BTreeSet<&str> = self.limits.iter().map(|one| one.window.as_str()).collect();
        is_word(&self.source)
            && is_key(&self.key)
            && self.plan.as_deref().is_none_or(is_plan)
            && self.limits.len() <= MAX_LIMITS
            && windows.len() == self.limits.len()
            && self.limits.iter().all(|one| {
                is_window(&one.window) && one.used_percent.is_none_or(|used| used <= 100)
            })
    }

    fn known(self) -> Option<Account> {
        Some(Account {
            source: Source::parse(&self.source)?,
            key: self.key,
            plan: self.plan,
            limits: self
                .limits
                .into_iter()
                .map(|one| Limit {
                    window: one.window,
                    used_percent: one.used_percent,
                    resets_at: one.resets_at,
                    reached: one.reached,
                })
                .collect(),
        })
    }
}

impl From<&Account> for BatchAccount {
    fn from(account: &Account) -> Self {
        Self {
            source: account.source.as_str().to_owned(),
            key: account.key.clone(),
            plan: account.plan.clone(),
            limits: account
                .limits
                .iter()
                .map(|one| BatchLimit {
                    window: one.window.clone(),
                    used_percent: one.used_percent,
                    resets_at: one.resets_at,
                    reached: one.reached,
                })
                .collect(),
        }
    }
}

/// An optional K8 time (Section 1).
mod optional_time {
    use chrono::{DateTime, Utc};
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(
        time: &Option<DateTime<Utc>>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        match time {
            Some(time) => crate::wire::times::serialize(time, serializer),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<DateTime<Utc>>, D::Error> {
        #[derive(Deserialize)]
        struct Time(#[serde(with = "crate::wire::times")] DateTime<Utc>);
        Ok(Option::<Time>::deserialize(deserializer)?.map(|Time(time)| time))
    }
}
