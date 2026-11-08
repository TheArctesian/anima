//! The metrics agent's secrets, which are never an access bearer (R-092 D-1, K8, MET-013).
//!
//! Agent tokens and enrolment codes live in `metrics`, not in `access.api_tokens`, so the guard
//! refuses them by their spelling before any lookup. Pure.

/// An agent token: this prefix and 43 base64url characters (32 bytes, K8).
const AGENT_TOKEN_PREFIX: &str = "anima_agent_";
const AGENT_TOKEN_CHARS: usize = 43;

/// An enrolment code: this prefix and 22 base64url characters (16 bytes, K8).
const ENROLMENT_CODE_PREFIX: &str = "anima_enrol_";
const ENROLMENT_CODE_CHARS: usize = 22;

/// Whether `presented` is spelled exactly as an agent token or an enrolment code: the prefix,
/// then exactly the K8 number of base64url characters. An API key is `anima_` and 43
/// characters, so one whose text happens to begin `agent_` or `enrol_` is never taken for an
/// agent secret.
pub(in crate::access) fn is_agent_secret(presented: &str) -> bool {
    spelled(presented, AGENT_TOKEN_PREFIX, AGENT_TOKEN_CHARS)
        || spelled(presented, ENROLMENT_CODE_PREFIX, ENROLMENT_CODE_CHARS)
}

fn spelled(presented: &str, prefix: &str, chars: usize) -> bool {
    presented.strip_prefix(prefix).is_some_and(|body| {
        body.len() == chars
            && body
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
    })
}
