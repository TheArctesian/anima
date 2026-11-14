//! Turning the one-time code a source hands back after "Create GitHub App" into the App's
//! facts and key (PRJ-039, K-E1 Section 7).
//!
//! The code is untrusted text from a browser redirect: it enters a path only when it is 1 to
//! 200 letters, digits, `-` or `_`, and the path then goes through the same check and the same
//! allowed hosts as every provider request. The POST carries no credential and no body. The
//! reply holds the App's private key and two secrets the hub never reads: it is wiped on drop,
//! and no error or log quotes it (LOG-007).

use std::time::Duration;

use chrono::{DateTime, Utc};
use source_exchange::{Converted, Declaration, Registration, Request};

use super::{
    registration,
    transport::{Content, Fault, Transport},
    verdict::{Verdict, judge},
};

/// How long the hub waits before its one more try after an unreachable source or a 5xx.
pub const RETRY_AFTER: Duration = Duration::from_secs(2);
/// The longest code, in characters.
const MAX_CODE: usize = 200;

/// Why no App came back. Fixed words: never the code or the reply (LOG-007).
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum ConvertFailure {
    /// A 4xx, including a code that is not plain text, which is refused before it is sent.
    #[error("the source refused the code")]
    Refused,
    #[error("the source limits requests")]
    RateLimited(DateTime<Utc>),
    /// No answer, or a 5xx, twice.
    #[error("the source could not be reached")]
    Unreachable,
    /// A reply too large, of a status that is not 201, or one the provider could not read.
    #[error("the source's reply to the conversion could not be used")]
    Unusable,
    #[error("the source cannot make an App for the operator")]
    NotRegistrable,
}

/// One try that did not give an App: worth one more try, or final.
enum Miss {
    Again(ConvertFailure),
    Final(ConvertFailure),
}

impl Miss {
    const fn failure(self) -> ConvertFailure {
        match self {
            Self::Again(failure) | Self::Final(failure) => failure,
        }
    }
}

/// Whether `code` may enter a path: 1 to 200 ASCII letters, digits, `-` or `_`.
#[must_use]
pub fn is_code(code: &str) -> bool {
    (1..=MAX_CODE).contains(&code.len())
        && code
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
}

/// Converts `code` at the source of `declaration` at `now`. An unreachable source or a 5xx is
/// tried once more after `retry_after` ([`RETRY_AFTER`]; a parameter so tests do not sleep).
///
/// # Errors
/// [`ConvertFailure`] for each way it can fail; a code that is not plain is
/// [`ConvertFailure::Refused`] before any request.
pub async fn convert(
    transport: &Transport,
    declaration: &Declaration,
    code: &str,
    (now, retry_after): (DateTime<Utc>, Duration),
) -> Result<Converted, ConvertFailure> {
    let registration = registration(declaration).ok_or(ConvertFailure::NotRegistrable)?;
    if !is_code(code) {
        return Err(ConvertFailure::Refused);
    }
    let target = Request::get(&(registration.conversion_path)(code), &[])
        .map_err(|_| ConvertFailure::Unusable)?;
    let url = transport
        .url(declaration.api_base, &target)
        .map_err(|_| ConvertFailure::Unusable)?;
    let first = once(transport, &registration, url.clone(), now).await;
    let outcome = match first {
        Err(Miss::Again(_)) => {
            tokio::time::sleep(retry_after).await;
            once(transport, &registration, url, now).await
        }
        done => done,
    };
    outcome.map_err(Miss::failure)
}

/// One POST and what its reply says. The reply is dropped, and so wiped, on every path.
async fn once(
    transport: &Transport,
    registration: &Registration,
    url: reqwest::Url,
    now: DateTime<Utc>,
) -> Result<Converted, Miss> {
    let answer = transport.convert(url).await.map_err(|fault| match fault {
        Fault::Network => Miss::Again(ConvertFailure::Unreachable),
        Fault::Refused => Miss::Final(ConvertFailure::Unusable),
    })?;
    match (judge(answer.status, answer.limits, now), answer.status) {
        (Verdict::RateLimited(until), _) => Err(Miss::Final(ConvertFailure::RateLimited(until))),
        (Verdict::Refused, _) | (Verdict::Usable, 400..=499) => {
            Err(Miss::Final(ConvertFailure::Refused))
        }
        (Verdict::Usable, 500..=599) => Err(Miss::Again(ConvertFailure::Unreachable)),
        (Verdict::Usable, 201) => match &answer.content {
            Content::Body(body) => (registration.parse_converted)(body)
                .map_err(|_| Miss::Final(ConvertFailure::Unusable)),
            Content::TooLarge => Err(Miss::Final(ConvertFailure::Unusable)),
        },
        (Verdict::Usable, _) => Err(Miss::Final(ConvertFailure::Unusable)),
    }
}
