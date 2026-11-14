//! Reading the installations of an App, signed in as the App before an installation is chosen
//! (PRJ-039, K-E1 Sections 7 and 10a).
//!
//! An installation id is untrusted text from a browser redirect: it enters a path only when it
//! is 1 to 20 digits. The App token is the bearer of an ordinary provider `GET`, is never kept
//! and has no `Debug`. A reply is read by the provider and never quoted (LOG-007).

use chrono::{DateTime, Utc};
use ring::rand::SecureRandom;
use source_exchange::{Declaration, Installation, Problem, Registration, Request};

use super::{
    app_key::{AppSigner, digits},
    registration,
    transport::{Answer, Content, Fault, Transport},
    verdict::{Verdict, judge},
};
use crate::fleet::credentials::Secret;

/// The most installations one list asks for.
const PER_PAGE: &str = "100";

/// What the source said about an installation, or the list of them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Lookup<T> {
    /// What the source holds. A suspended installation is found, with `suspended` set.
    Found(T),
    /// A 404, or an id that is not 1 to 20 digits and so was never asked for.
    NotFound,
    /// No answer, a 5xx or a rate limit: worth asking again later.
    Unreachable,
    /// A refused App token, another status, a reply too large or one the provider could not
    /// read; or a source that cannot make an App.
    Unusable,
}

/// The installation `id` of the App that `signer` signs for, at the source of `declaration`.
pub async fn installation(
    transport: &Transport,
    declaration: &Declaration,
    (signer, id): (&AppSigner, &str),
    (now, random): (DateTime<Utc>, &(dyn SecureRandom + Sync)),
) -> Lookup<Installation> {
    let Some(registration) = registration(declaration) else {
        return Lookup::Unusable;
    };
    if !digits(id) {
        return Lookup::NotFound;
    }
    let path = (registration.installation_path)(id);
    let asked = ask(transport, declaration, (signer, &path, &[]), (now, random)).await;
    read(asked, |body| (registration.parse_installation)(body))
}

/// The first 100 installations of the App that `signer` signs for.
pub async fn installations(
    transport: &Transport,
    declaration: &Declaration,
    signer: &AppSigner,
    (now, random): (DateTime<Utc>, &(dyn SecureRandom + Sync)),
) -> Lookup<Vec<Installation>> {
    let Some(Registration {
        installations_path,
        parse_installations,
        ..
    }) = registration(declaration)
    else {
        return Lookup::Unusable;
    };
    let query = [("per_page", PER_PAGE)];
    let asked = ask(
        transport,
        declaration,
        (signer, installations_path, &query),
        (now, random),
    )
    .await;
    read(asked, parse_installations)
}

/// Sends a `GET` of `path` with `query`, signed in as the App, and judges the status: a 200 is
/// the answer, anything else is already the outcome.
async fn ask<T>(
    transport: &Transport,
    declaration: &Declaration,
    (signer, path, query): (&AppSigner, &str, &[(&str, &str)]),
    (now, random): (DateTime<Utc>, &(dyn SecureRandom + Sync)),
) -> Result<Answer, Lookup<T>> {
    let request = Request::get(path, query).map_err(|_| Lookup::Unusable)?;
    let url = transport
        .url(declaration.api_base, &request)
        .map_err(|_| Lookup::Unusable)?;
    let app_token = signer.token(now, random).map_err(|_| Lookup::Unusable)?;
    // Both the JWT and its copy in the secret wipe themselves on drop.
    let bearer = Secret::new(app_token.as_bytes().to_vec());
    drop(app_token);
    let answer = transport
        .get(&request, url, &bearer)
        .await
        .map_err(|fault| match fault {
            Fault::Network => Lookup::Unreachable,
            Fault::Refused => Lookup::Unusable,
        })?;
    match (judge(answer.status, answer.limits, now), answer.status) {
        (Verdict::Usable, 200) => Ok(answer),
        (Verdict::Usable, 404) => Err(Lookup::NotFound),
        (Verdict::RateLimited(_), _) | (Verdict::Usable, 500..=599) => Err(Lookup::Unreachable),
        (Verdict::Refused | Verdict::Usable, _) => Err(Lookup::Unusable),
    }
}

/// The provider's reading of a 200 reply.
fn read<T>(
    asked: Result<Answer, Lookup<T>>,
    parse: impl FnOnce(&[u8]) -> Result<T, Problem>,
) -> Lookup<T> {
    match asked {
        Ok(Answer {
            content: Content::Body(body),
            ..
        }) => parse(&body).map_or(Lookup::Unusable, Lookup::Found),
        Ok(_) => Lookup::Unusable,
        Err(outcome) => outcome,
    }
}
