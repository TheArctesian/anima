//! The agent's log lines, in one place: what happened, by code or cause, never a token, a
//! code or a body (LOG-007). Called from the I/O edge only.

use agent_core::wire::RefusalCode;

use crate::hub::answer::Cause;
use crate::token_store::TokenStoreError;

pub fn batch_dropped(code: RefusalCode, points: usize) {
    tracing::warn!(
        code = code.as_str(),
        points,
        "the hub refused a batch; dropped it"
    );
}

pub fn push_failed(cause: Cause) {
    tracing::warn!(%cause, "push failed; keeping the points");
}

pub fn hub_cannot_store() {
    tracing::warn!("the hub cannot store now; keeping the points");
}

pub fn hub_back() {
    tracing::info!("the hub answers again");
}

pub fn token_refused(code: RefusalCode) {
    tracing::error!(
        code = code.as_str(),
        "the hub refused the agent's token; pushing stopped, sampling goes on"
    );
}

pub fn token_accepted_again() {
    tracing::info!("the hub accepts the token again");
}

pub fn token_store_changed() {
    tracing::info!("the token store changed; pushing again");
}

pub fn renewed() {
    tracing::info!("the agent token was renewed");
}

pub fn renewal_not_stored(error: &TokenStoreError) {
    tracing::error!(%error, "the new token could not be stored; keeping the current one");
}

pub fn renewal_refused(code: RefusalCode) {
    tracing::warn!(code = code.as_str(), "renewal refused");
}

pub fn renewal_failed(cause: Cause) {
    tracing::warn!(%cause, "renewal failed; trying again in an hour");
}

pub fn running(device: uuid::Uuid) {
    tracing::info!(%device, "the agent is running");
}

pub fn stopping() {
    tracing::info!("the agent is stopping");
}

pub fn sampling_panicked() {
    tracing::error!("sampling stopped: its task panicked");
}

pub fn sampling_stopped() {
    tracing::error!("sampling stopped before the agent was told to stop");
}
