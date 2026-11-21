//! The limits of each name (Section 12.7): connections at one time, new connections each
//! second, and bytes in each period. The numbers are parameters: the operator decides them with
//! the prices (OD-HOST-40). Time is a parameter too, in milliseconds from any fixed start. Pure.

use std::collections::HashMap;

/// The limits of one name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    /// Connections open at one time.
    pub connections: u32,
    /// New connections each second, with a burst of the same number.
    pub new_per_second: u32,
    /// Bytes in both directions in each period; 0: no limit.
    pub bytes_per_period: u64,
    /// The length of a period, in milliseconds.
    pub period_ms: u64,
}

/// Why a connection was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    Connections,
    Rate,
    Bytes,
}

#[derive(Clone, Copy, Debug, Default)]
struct State {
    open: u32,
    /// New connections that the bucket still holds, in thousandths.
    tokens_milli: u64,
    refilled_ms: u64,
    period_start_ms: u64,
    bytes: u64,
    started: bool,
}

/// The state of the limits of each name.
#[derive(Debug)]
pub struct Limiter {
    limits: Limits,
    names: HashMap<String, State>,
}

impl Limiter {
    #[must_use]
    pub fn new(limits: Limits) -> Self {
        Self {
            limits,
            names: HashMap::new(),
        }
    }

    /// Admits one new connection to `name` at `now_ms`, or says which limit refuses it.
    ///
    /// # Errors
    /// The [`Refusal`] of the first limit that is reached.
    pub fn admit(&mut self, name: &str, now_ms: u64) -> Result<(), Refusal> {
        let limits = self.limits;
        let state = self.state(name, now_ms);
        if state.open >= limits.connections {
            return Err(Refusal::Connections);
        }
        if limits.bytes_per_period > 0 && state.bytes >= limits.bytes_per_period {
            return Err(Refusal::Bytes);
        }
        let elapsed = now_ms.saturating_sub(state.refilled_ms);
        let full = u64::from(limits.new_per_second) * 1000;
        state.tokens_milli =
            (state.tokens_milli + elapsed * u64::from(limits.new_per_second)).min(full);
        state.refilled_ms = now_ms;
        if state.tokens_milli < 1000 {
            return Err(Refusal::Rate);
        }
        state.tokens_milli -= 1000;
        state.open += 1;
        Ok(())
    }

    /// A connection to `name` ended.
    pub fn release(&mut self, name: &str) {
        if let Some(state) = self.names.get_mut(name) {
            state.open = state.open.saturating_sub(1);
        }
    }

    /// Adds `bytes` to the period of `name`; whether the name is still under its limit.
    pub fn add_bytes(&mut self, name: &str, bytes: u64, now_ms: u64) -> bool {
        let limit = self.limits.bytes_per_period;
        let state = self.state(name, now_ms);
        state.bytes = state.bytes.saturating_add(bytes);
        limit == 0 || state.bytes < limit
    }

    /// The state of `name`, with a new period when the last one ended.
    fn state(&mut self, name: &str, now_ms: u64) -> &mut State {
        let limits = self.limits;
        let state = self.names.entry(name.to_owned()).or_default();
        if !state.started {
            *state = State {
                tokens_milli: u64::from(limits.new_per_second) * 1000,
                refilled_ms: now_ms,
                period_start_ms: now_ms,
                started: true,
                ..State::default()
            };
        }
        if now_ms.saturating_sub(state.period_start_ms) >= limits.period_ms {
            state.period_start_ms = now_ms;
            state.bytes = 0;
        }
        state
    }
}
