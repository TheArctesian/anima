//! What a program says of the account that it uses: its name, its plan and its limits (THR-051,
//! THR-115, THR-132). Codex gives them by `account/read`, the limit methods and their notices;
//! Claude Code gives only its limits, in `rate_limit_event`.
//!
//! These facts stay at the machine in stage 1b: the runner keeps them in memory and gives them
//! only to a client of the same user on the local socket. They go to no log, no notification, no
//! update of a thread and not to the hub. A token is never one of them: no field of a token is
//! read here.

use std::fmt;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The most limits kept of one report.
const MAX_LIMITS: usize = 8;
/// The most bytes of one text field of a report.
const MAX_FIELD: usize = 256;

/// A field that can name a person (an address, a user ID, a workspace). Its `Debug` shows no
/// text, so that a log line made from an event cannot hold it (LOG-007).
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Private(String);

impl Private {
    fn of(value: &Value) -> Option<Self> {
        let text = value.as_str().filter(|text| !text.trim().is_empty())?;
        Some(Self(crate::held(text, MAX_FIELD)))
    }

    /// The text, for the person on this computer only.
    #[must_use]
    pub fn reveal(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for Private {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("Private(..)")
    }
}

/// One limit of the account: a window of time and how much of it is used.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Limit {
    /// The name of the window as the program gives it (`five_hour`, `seven_day`), or its length
    /// in minutes (`300 min`) when the program gives only that.
    pub window: String,
    /// How much of the window is used; `None` when the program says only whether it is reached.
    pub used_percent: Option<u8>,
    /// The time of the reset, in seconds from 1970.
    pub resets_at: Option<i64>,
    /// The program said that the limit is reached.
    pub reached: bool,
}

/// One report of a program about its account. A part that the report does not give is `None`
/// or empty: the runner keeps the part from an earlier report. Its `Debug` shows no field, so
/// that no log line made from it holds one (THR-051).
#[derive(Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountFacts {
    /// The identifier that the vendor gives the account (THR-132).
    pub id: Option<Private>,
    /// The name of the account: often an address of the person.
    pub name: Option<Private>,
    pub plan: Option<String>,
    pub limits: Vec<Limit>,
    /// The program said that it is signed in to no account: the earlier account is gone.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub signed_out: bool,
}

impl fmt::Debug for AccountFacts {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("AccountFacts(..)")
    }
}

impl AccountFacts {
    /// The facts of a newer report over these: each part that it gives replaces the old one;
    /// a limit replaces the limit of the same window.
    pub fn merge(&mut self, newer: Self) {
        if newer.signed_out {
            *self = newer;
            return;
        }
        if newer.id.is_some() {
            self.id = newer.id;
        }
        if newer.name.is_some() {
            self.name = newer.name;
        }
        if newer.plan.is_some() {
            self.plan = newer.plan;
        }
        for limit in newer.limits {
            self.limits.retain(|old| old.window != limit.window);
            self.limits.push(limit);
        }
        self.limits.truncate(MAX_LIMITS);
    }

    fn is_empty(&self) -> bool {
        !self.signed_out
            && self.id.is_none() && self.name.is_none() && self.plan.is_none() && self.limits.is_empty()
    }

    fn some(self) -> Option<Self> {
        (!self.is_empty()).then_some(self)
    }
}

fn plan(value: &Value) -> Option<String> {
    value
        .as_str()
        .filter(|plan| !plan.is_empty() && *plan != "unknown")
        .map(|plan| crate::held(plan, MAX_FIELD))
}

fn percent(used: f64) -> u8 {
    // A value out of range from the program is held at the ends.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "the value is clamped to 0..=100 first"
    )]
    let whole = used.clamp(0.0, 100.0).round() as u8;
    whole
}

fn signed_out() -> AccountFacts {
    AccountFacts {
        signed_out: true,
        ..AccountFacts::default()
    }
}

/// The `result` of `account/read`: `{"account": {"email", "planType", "workspaceName"}, "userId"}`;
/// `"account": null` when Codex is signed in to no account.
#[must_use]
pub fn codex_account(result: &Value) -> Option<AccountFacts> {
    let account = &result["account"];
    if account.is_null() {
        return Some(signed_out());
    }
    let name = Private::of(&account["email"]).or_else(|| Private::of(&account["workspaceName"]));
    AccountFacts {
        id: Private::of(&result["userId"]),
        name,
        plan: plan(&account["planType"]),
        ..AccountFacts::default()
    }
    .some()
}

/// The `params` of `account/updated`: the plan only; `"authMode": null` when Codex signed out.
#[must_use]
pub fn codex_account_updated(params: &Value) -> Option<AccountFacts> {
    if params.get("authMode").is_some_and(Value::is_null) {
        return Some(signed_out());
    }
    AccountFacts {
        plan: plan(&params["planType"]),
        ..AccountFacts::default()
    }
    .some()
}

/// A `RateLimitSnapshot` of Codex (`params.rateLimits` of the notice, `result.rateLimits` of
/// `account/rateLimits/read`): a primary and a secondary window.
#[must_use]
pub fn codex_limits(snapshot: &Value) -> Option<AccountFacts> {
    let reached = !snapshot["rateLimitReachedType"].is_null();
    let limits = ["primary", "secondary"]
        .iter()
        .filter_map(|key| codex_window(key, &snapshot[key], reached))
        .collect();
    AccountFacts {
        plan: plan(&snapshot["planType"]),
        limits,
        ..AccountFacts::default()
    }
    .some()
}

fn codex_window(key: &str, window: &Value, reached: bool) -> Option<Limit> {
    let used = window["usedPercent"].as_f64()?;
    let minutes = window["windowDurationMins"].as_i64();
    Some(Limit {
        window: minutes.map_or_else(|| key.to_owned(), |minutes| format!("{minutes} min")),
        used_percent: Some(percent(used)),
        resets_at: window["resetsAt"].as_i64(),
        reached: reached && used >= 100.0,
    })
}

/// The `rate_limit_info` of a `rate_limit_event` of Claude Code (measured 2026-11-21 with
/// 2.1.311): `unifiedWindows` holds each window with a `utilization` from 0 to 1. A report with
/// no `unifiedWindows` gives one limit from its `rateLimitType`, `status` and `resetsAt`.
#[must_use]
pub fn claude_limits(report: &Value) -> Option<AccountFacts> {
    let mut limits: Vec<Limit> = report["unifiedWindows"]
        .as_object()
        .into_iter()
        .flatten()
        .filter_map(|(name, window)| claude_window(name, window))
        .take(MAX_LIMITS)
        .collect();
    if limits.is_empty()
        && let Some(name) = report["rateLimitType"].as_str()
    {
        limits.extend(claude_window(name, report));
    }
    AccountFacts {
        limits,
        ..AccountFacts::default()
    }
    .some()
}

/// One window of Claude Code. Only `rejected` is a reached limit: `allowed_warning` is not.
fn claude_window(name: &str, window: &Value) -> Option<Limit> {
    let used = window["utilization"].as_f64().map(|used| percent(used * 100.0));
    let status = window["status"].as_str();
    if used.is_none() && status.is_none() {
        return None;
    }
    Some(Limit {
        window: crate::held(name, MAX_FIELD),
        used_percent: used,
        resets_at: window["resetsAt"].as_i64(),
        reached: status == Some("rejected"),
    })
}
