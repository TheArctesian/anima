//! What the hub's answer means (K8 Sections 3, 5 and 6): pure, from a status and a body.

use daemon_core::wire::{Refusal, RefusalCode};

/// The answer to one request.
#[derive(Debug, PartialEq)]
pub enum Answer<T> {
    /// The success status, with what the body held.
    Accepted(T),
    /// A refusal of the kind K8 Section 6 lists, with the status that goes with it.
    Refused(RefusalCode),
    /// Anything else: no answer, a 5xx, a redirect, or a status and body K8 does not allow.
    Failed(Cause),
}

/// Why a request failed, in words with no value from the request or the answer (LOG-007).
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum Cause {
    #[error("the hub could not be reached")]
    Network,
    #[error("the hub did not answer within 10 seconds")]
    Timeout,
    #[error("the hub answered with status {0}")]
    Status(u16),
    #[error("the hub's answer did not read")]
    Unreadable,
    /// The gateway of the hosted hubs answered in place of a hub (HMD-059, HM-T31).
    #[error(
        "this address is the hosted Anima.ing hub in a browser, not a hub for a daemon: enrol again with --hub and the address of your own hub, or sign in with the Anima.ing application, which installs the daemon for a hosted hub"
    )]
    HostedAddress,
}

/// Whether `body` is the refusal of the gateway of the hosted hubs for a request with no
/// session of a browser: `{ "error": { "code": "sign_in_needed" } }`.
fn from_gateway(body: &[u8]) -> bool {
    #[derive(serde::Deserialize)]
    struct Gateway {
        error: Code,
    }
    #[derive(serde::Deserialize)]
    struct Code {
        code: String,
    }
    serde_json::from_slice::<Gateway>(body)
        .is_ok_and(|gateway| gateway.error.code == "sign_in_needed")
}

/// The meaning of `status` and `body`. `ok_status` is the success status of the route and
/// `read_ok` reads its body. A refusal counts only when its code goes with its status.
pub fn answer<T>(
    status: u16,
    body: &[u8],
    ok_status: u16,
    read_ok: impl FnOnce(&[u8]) -> Option<T>,
) -> Answer<T> {
    if status == ok_status {
        return read_ok(body).map_or(Answer::Failed(Cause::Unreadable), Answer::Accepted);
    }
    match serde_json::from_slice::<Refusal>(body) {
        Ok(Refusal { error }) if error.status() == status => Answer::Refused(error),
        _ if status == 401 && from_gateway(body) => Answer::Failed(Cause::HostedAddress),
        _ => Answer::Failed(Cause::Status(status)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn push(status: u16, body: &str) -> Answer<()> {
        answer(status, body.as_bytes(), 204, |_| Some(()))
    }

    #[test]
    fn k8_6_a_204_is_accepted_and_each_refusal_reads_by_its_code() {
        assert_eq!(push(204, ""), Answer::Accepted(()));
        for (status, text, code) in [
            (400, "MALFORMED", RefusalCode::Malformed),
            (401, "TOKEN_REFUSED", RefusalCode::TokenRefused),
            (403, "WRONG_DEVICE", RefusalCode::WrongDevice),
            (413, "TOO_LARGE", RefusalCode::TooLarge),
            (422, "OUT_OF_RANGE", RefusalCode::OutOfRange),
            (503, "UNAVAILABLE", RefusalCode::Unavailable),
        ] {
            let body = format!(r#"{{"error":"{text}"}}"#);
            assert_eq!(push(status, &body), Answer::Refused(code), "{text}");
        }
    }

    #[test]
    fn k8_6_a_5xx_a_redirect_a_stray_2xx_and_a_mismatched_code_are_failures() {
        let cases = [
            (500, ""),
            (302, ""),
            (400, r#"{"error":"TOKEN_REFUSED"}"#),
            (400, "not json"),
            (200, ""),
            (201, r#"{"error":"MALFORMED"}"#),
        ];
        for (status, body) in cases {
            assert_eq!(push(status, body), Answer::Failed(Cause::Status(status)));
        }
    }

    #[test]
    fn hm_t31_the_gateway_of_the_hosted_hubs_is_told_apart_from_a_hub_that_refuses() {
        let gateway = r#"{"error":{"code":"sign_in_needed","message":"words"}}"#;
        assert_eq!(push(401, gateway), Answer::Failed(Cause::HostedAddress));
        assert!(Cause::HostedAddress.to_string().contains("--hub"));
        // The same body with a different status, and a different code, go by the status.
        assert_eq!(push(403, gateway), Answer::Failed(Cause::Status(403)));
        let other = r#"{"error":{"code":"hub_not_ready","message":"words"}}"#;
        assert_eq!(push(401, other), Answer::Failed(Cause::Status(401)));
    }

    #[test]
    fn k8_3_a_success_status_whose_body_does_not_read_is_a_failure() {
        let reply = answer(200, b"{", 200, |_| None::<()>);
        assert_eq!(reply, Answer::Failed(Cause::Unreadable));
    }
}
