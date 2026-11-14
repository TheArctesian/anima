//! The HTTP routes of Connect GitHub (K-E1 Section 7a, K7): fixed paths, no path parameters,
//! each logged by its template only, behind the permitted-networks layer.
//!
//! `GET /connect/github` (with a ticket, or the fixed confirm page), `POST /connect/github/open`
//! (the browser session from Anima's own origin), `GET /connect/github/start`, `/created` and
//! `/installed`. Every answer carries `no-store`, `no-referrer`, `nosniff`, HSTS, COOP and a
//! policy of its own. The binding cookie is the only thing that ties a browser to the attempt.

use axum::{
    Router,
    extract::{RawQuery, State},
    http::{HeaderMap, HeaderName, HeaderValue, StatusCode, header},
    middleware,
    response::{IntoResponse as _, Response},
    routing::{get, post},
};
use ring::rand::SystemRandom;
use source_exchange::{ManifestFacts, Webhook};
use zeroize::Zeroizing;

use super::{
    attempt::{Finished, Phase, Refusal},
    attempts::Attempts,
    binding::{Minted, Step},
    exchange::{Created, created},
    github,
    installation::verified,
    pages::{self, Notice},
    query::value,
    words::PAGE_PATH,
};
use crate::{
    engine::sources::Scheduling,
    fleet::access::{PublicOrigin, SameOrigin, TokenKind, cookie_value, identify},
};

/// The binding cookie (K7).
const COOKIE: &str = "__Host-anima-connect";
/// What every `Set-Cookie` of [`COOKIE`] carries besides its value and `Max-Age`.
const ATTRIBUTES: &str = "Path=/; Secure; HttpOnly; SameSite=Lax";
/// How long the cookie lives, in seconds: past the attempt's 55 minutes.
const COOKIE_SECONDS: u32 = 3600;
/// How the manifest treats the webhook (K-E1 Section 1; the live check may change it).
const WEBHOOK: Webhook = Webhook::InactiveAtHub;
const START_PATH: &str = "/connect/github/start";

/// What the routes share: the parts a run needs, the attempt and the hub's public origin.
#[derive(Clone)]
pub struct GithubFlow {
    pub scheduling: Scheduling,
    pub attempts: Attempts,
    pub origin: PublicOrigin,
}

/// The Connect GitHub routes, with the headers every answer carries.
pub fn router(flow: GithubFlow) -> Router {
    Router::new()
        .route(PAGE_PATH, get(page))
        .route("/connect/github/open", post(open))
        .route(START_PATH, get(start))
        .route("/connect/github/created", get(back_from_create))
        .route("/connect/github/installed", get(back_from_install))
        .layer(middleware::map_response(protect))
        .with_state(flow)
}

/// Sets the fixed headers on every answer, and the plain policy unless the page has its own.
async fn protect(mut response: Response) -> Response {
    let headers = response.headers_mut();
    let fixed = [
        (header::CACHE_CONTROL, "no-store"),
        (header::REFERRER_POLICY, "no-referrer"),
        (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
        (header::STRICT_TRANSPORT_SECURITY, "max-age=31536000"),
        (
            HeaderName::from_static("cross-origin-opener-policy"),
            "same-origin",
        ),
    ];
    for (name, text) in fixed {
        headers.insert(name, HeaderValue::from_static(text));
    }
    if !headers.contains_key(header::CONTENT_SECURITY_POLICY) {
        headers.insert(
            header::CONTENT_SECURITY_POLICY,
            HeaderValue::from_static(pages::PLAIN_POLICY),
        );
    }
    response
}

/// A page with `status` and `body`.
fn html(status: StatusCode, body: String) -> Response {
    (
        status,
        [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
        body,
    )
        .into_response()
}

fn notice(status: StatusCode, notice: Notice) -> Response {
    html(status, pages::notice(notice))
}

/// A notice that also clears the cookie: the done and failure pages.
fn last(status: StatusCode, notice: Notice) -> Response {
    let mut response = self::notice(status, notice);
    let cleared = format!("{COOKIE}=; Max-Age=0; {ATTRIBUTES}");
    if let Ok(value) = HeaderValue::from_str(&cleared) {
        response.headers_mut().insert(header::SET_COOKIE, value);
    }
    response
}

/// `303` to `location`, which may hold a one-time value: marked sensitive.
fn see_other(location: &str, cookie: Option<HeaderValue>) -> Response {
    let Ok(mut value) = HeaderValue::from_str(location) else {
        return notice(StatusCode::INTERNAL_SERVER_ERROR, Notice::TryAgain);
    };
    value.set_sensitive(true);
    let mut response = StatusCode::SEE_OTHER.into_response();
    response.headers_mut().insert(header::LOCATION, value);
    if let Some(cookie) = cookie {
        response.headers_mut().insert(header::SET_COOKIE, cookie);
    }
    response
}

/// The `Set-Cookie` value that binds the browser with `binding`; marked sensitive.
fn set_cookie(binding: &Minted) -> Option<HeaderValue> {
    let text = Zeroizing::new(format!(
        "{COOKIE}={}; Max-Age={COOKIE_SECONDS}; {ATTRIBUTES}",
        binding.text()
    ));
    let mut value = HeaderValue::from_str(&text).ok()?;
    value.set_sensitive(true);
    Some(value)
}

/// Binds this browser to the attempt with `claim`, then sends it to the form page.
fn bind(
    attempts: &Attempts,
    now: chrono::DateTime<chrono::Utc>,
    claim: impl FnOnce(&mut super::attempt::Attempt, &Minted) -> Result<(), Refusal>,
) -> Result<Response, Refusal> {
    let binding = Minted::new(&SystemRandom::new()).map_err(|_| Refusal::Conflict)?;
    attempts.step(None, now, |held| claim(held, &binding))?;
    Ok(see_other(START_PATH, set_cookie(&binding)))
}

/// `GET /connect/github`: the ticket binds this browser; without one, the fixed confirm page.
async fn page(State(flow): State<GithubFlow>, RawQuery(raw): RawQuery) -> Response {
    let Some(ticket) = value(raw.as_deref(), "ticket") else {
        return html(StatusCode::OK, pages::confirm());
    };
    let now = flow.scheduling.clock.now();
    bind(&flow.attempts, now, |held, binding| {
        held.claim_ticket(ticket, binding.hash(), now)
    })
    .unwrap_or_else(|_| notice(StatusCode::BAD_REQUEST, Notice::LinkEnded))
}

/// `POST /connect/github/open`: a browser signed in to Anima, from Anima's own page, binds itself.
async fn open(State(flow): State<GithubFlow>, headers: HeaderMap) -> Response {
    let Some(from) = SameOrigin::of(&headers, &flow.origin) else {
        return notice(StatusCode::FORBIDDEN, Notice::NotFromAnima);
    };
    let now = flow.scheduling.clock.now();
    let Ok(identity) = identify(&flow.scheduling.pool, &headers, Some(&from), now).await else {
        return notice(StatusCode::SERVICE_UNAVAILABLE, Notice::TryAgain);
    };
    let signed_in = identity.is_some_and(|identity| {
        identity.kind() == TokenKind::Browser
            && identity.operator().is_some_and(|operator| {
                operator.actor.kind() != crate::fleet::events::ActorKind::Agent
            })
    });
    if !signed_in {
        return notice(StatusCode::UNAUTHORIZED, Notice::SignIn);
    }
    bind(&flow.attempts, now, |held, binding| {
        held.claim(binding.hash())
    })
    .unwrap_or_else(|_| notice(StatusCode::CONFLICT, Notice::NothingWaiting))
}

/// `GET /connect/github/start`: the bound browser gets the form that posts the manifest to
/// GitHub with its state S1, the same on every hit, so a second hit does not spoil the first.
async fn start(State(flow): State<GithubFlow>, headers: HeaderMap) -> Response {
    let cookie = cookie_value(&headers, COOKIE);
    let Some(registration) = github(&flow.scheduling.registry) else {
        return notice(StatusCode::INTERNAL_SERVER_ERROR, Notice::TryAgain);
    };
    // Used only once `send_to_github` has found the cookie to be the bound browser's.
    let s1 = Minted::state(cookie.unwrap_or_default(), Step::Create);
    let now = flow.scheduling.clock.now();
    let sent = flow.attempts.step(None, now, |held| {
        held.send_to_github(cookie, s1.hash())?;
        let request = held.request();
        Ok((request.name.clone(), request.organisation.clone()))
    });
    let (name, organisation) = match sent {
        Ok(sent) => sent,
        Err(Refusal::Conflict) => return notice(StatusCode::CONFLICT, Notice::StepPassed),
        Err(_) => return notice(StatusCode::BAD_REQUEST, Notice::NotThisBrowser),
    };
    let Some(form_path) = (registration.form_path)(organisation.as_deref()) else {
        return notice(StatusCode::BAD_REQUEST, Notice::NothingWaiting);
    };
    let facts = ManifestFacts {
        name: &name,
        origin: flow.origin.as_str(),
        webhook: WEBHOOK,
    };
    let manifest = (registration.manifest)(&facts);
    let action = format!("{}{form_path}?state={}", registration.web_base, s1.text());
    let mut response = html(StatusCode::OK, pages::to_github(&action, &manifest));
    if let Ok(policy) = HeaderValue::from_str(&pages::to_github_policy(registration.web_base)) {
        response
            .headers_mut()
            .insert(header::CONTENT_SECURITY_POLICY, policy);
    }
    response
}

/// `GET /connect/github/created?code&state`: the bound browser is back from "Create GitHub App".
async fn back_from_create(
    State(flow): State<GithubFlow>,
    headers: HeaderMap,
    RawQuery(raw): RawQuery,
) -> Response {
    let cookie = cookie_value(&headers, COOKIE);
    let (code, state) = (
        value(raw.as_deref(), "code"),
        value(raw.as_deref(), "state"),
    );
    match created(&flow.scheduling, &flow.attempts, cookie, (code, state)).await {
        Created::Install(location) => see_other(&location, None),
        Created::Waiting => notice(StatusCode::OK, Notice::Finishing),
        Created::Refused(Refusal::Conflict) => notice(StatusCode::CONFLICT, Notice::StepPassed),
        Created::Refused(_) => notice(StatusCode::BAD_REQUEST, Notice::NotThisBrowser),
        Created::Failed => last(StatusCode::OK, Notice::Failed),
    }
}

/// `GET /connect/github/installed?installation_id&setup_action&state`: the browser is back from
/// Install. The ID is checked with GitHub before anything is stored. The bound browser with a
/// state that does not match only wakes the poll; any other browser changes nothing.
async fn back_from_install(
    State(flow): State<GithubFlow>,
    headers: HeaderMap,
    RawQuery(raw): RawQuery,
) -> Response {
    let cookie = cookie_value(&headers, COOKIE);
    let (id, state) = (
        value(raw.as_deref(), "installation_id"),
        value(raw.as_deref(), "state"),
    );
    let now = flow.scheduling.clock.now();
    let seen = flow.attempts.step(None, now, |held| {
        // An ended attempt has spent its state: the cookie alone shows its last page.
        let back = if held.ended() {
            held.bound(cookie)
        } else {
            held.back_from_install(cookie, state)
        };
        let known = (held.bound(cookie), back);
        Ok((known, held.id(), held.snapshot().phase, held.waiting_app()))
    });
    let Ok(((_, true), attempt, phase, waiting)) = seen else {
        if matches!(seen, Ok(((true, _), ..))) {
            flow.attempts.wake();
        }
        return notice(StatusCode::OK, Notice::MaybeInstalled);
    };
    match (phase, waiting, id) {
        (Phase::Connected, ..) => last(StatusCode::OK, Notice::Connected),
        (Phase::Failed | Phase::Cancelled, ..) => last(StatusCode::OK, Notice::Failed),
        (_, Some((app, key)), Some(id)) => {
            let Some(found) = verified(&flow.scheduling, (&app, &key), id).await else {
                return notice(StatusCode::OK, Notice::Finishing);
            };
            match super::finish::finish(&flow.scheduling, &flow.attempts, (attempt, found)).await {
                Some(Finished::Connected { .. }) => last(StatusCode::OK, Notice::Connected),
                Some(Finished::Failed(_)) => last(StatusCode::OK, Notice::Failed),
                Some(Finished::Again) | None => notice(StatusCode::OK, Notice::Finishing),
            }
        }
        _ => notice(StatusCode::OK, Notice::Finishing),
    }
}
