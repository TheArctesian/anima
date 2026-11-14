//! The return from "Create GitHub App": the code becomes the App's facts and key (K-E1
//! Section 7a, PRJ-039).
//!
//! The bound browser's cookie and the state S1 are checked, and S1 is spent, before the code is
//! converted, so a replay converts nothing. The key moves at once into the attempt; GitHub's
//! two other secrets are never read. "App created" is audited as soon as GitHub made an App, so
//! the operator can still delete it after a cancel or a restart.
//!
//! The conversion, the audit entry and the step to `APP_CREATED` run in a task of their own, so
//! a browser that leaves does not stop them; a [`Settle`] fails the attempt if that task is
//! dropped or panics before the App is kept.

use source_exchange::{Converted, Declaration};
use uuid::Uuid;
use zeroize::Zeroizing;

use super::{
    GITHUB,
    attempt::{App, Exchange, Failure, Refusal, Starter},
    attempts::{Attempts, Settle},
    binding::{Minted, Step},
    trail,
};
use crate::{
    engine::sources::{
        Scheduling,
        acquisition::{
            app_key::AppSigner,
            conversion::{ConvertFailure, RETRY_AFTER, convert},
        },
    },
    fleet::events::Trail,
};

/// Where the browser goes after `/created`.
pub enum Created {
    /// To GitHub's install page with the state S2; the URL holds S2.
    Install(Zeroizing<String>),
    /// The App is made and the request did not bring the spent S1: the waiting page, no change.
    Waiting,
    /// Refused; the attempt is as it was.
    Refused(Refusal),
    /// The App could not be made or used; the attempt failed.
    Failed,
}

/// The browser with `cookie` came back with `code` and `state`.
pub async fn created(
    scheduling: &Scheduling,
    attempts: &Attempts,
    cookie: Option<&str>,
    (code, state): (Option<&str>, Option<&str>),
) -> Created {
    let now = scheduling.clock.now();
    let begun = attempts.step(None, now, |held| {
        let exchange = held.begin_exchange(cookie, state)?;
        let slug = held.snapshot().app.map(|app| app.slug);
        Ok((exchange, held.id(), held.request().by.clone(), slug))
    });
    let (exchange, id, by, slug) = match begun {
        Ok(begun) => begun,
        Err(refusal) => return Created::Refused(refusal),
    };
    // `begin_exchange` found the cookie to be the bound browser's.
    let s2 = Minted::state(cookie.unwrap_or_default(), Step::Install);
    match exchange {
        Exchange::Waiting => Created::Waiting,
        Exchange::Made => slug
            .and_then(|slug| location(scheduling, &slug, &s2))
            .map_or(Created::Refused(Refusal::Conflict), Created::Install),
        Exchange::Fresh => {
            let code = Zeroizing::new(code.unwrap_or_default().to_owned());
            let task = tokio::spawn(exchanged(
                scheduling.clone(),
                attempts.clone(),
                (id, by),
                (code, s2),
            ));
            task.await.unwrap_or(Created::Failed)
        }
    }
}

/// The exchange for attempt `id`, in its own task: convert, audit, keep the App.
async fn exchanged(
    scheduling: Scheduling,
    attempts: Attempts,
    (id, by): (Uuid, Starter),
    (code, s2): (Zeroizing<String>, Minted),
) -> Created {
    // Does nothing once the exchange has ended; fails it if this task is lost before then.
    let _settle = Settle::new(
        attempts.clone(),
        id,
        scheduling.clock.clone(),
        |held, now| held.exchange_failed(Failure::GithubUnreachable, None, now),
    );
    let converted = converted(&scheduling, &code).await;
    made(&scheduling, &attempts, (id, by), (converted, &s2)).await
}

/// The App and its key from GitHub's reply, or the failure.
async fn converted(scheduling: &Scheduling, code: &str) -> Result<Converted, Failure> {
    let declaration: Declaration = *scheduling
        .registry
        .declaration(GITHUB)
        .ok_or(Failure::ReplyUnusable)?;
    let now = scheduling.clock.now();
    convert(
        &scheduling.transport,
        &declaration,
        code,
        (now, RETRY_AFTER),
    )
    .await
    .map_err(|failure| {
        // Fixed words only: never the code or the reply (LOG-007).
        tracing::warn!(%failure, "github app could not be made");
        match failure {
            ConvertFailure::Refused => Failure::ExchangeRefused,
            ConvertFailure::RateLimited(_) | ConvertFailure::Unreachable => {
                Failure::GithubUnreachable
            }
            ConvertFailure::Unusable | ConvertFailure::NotRegistrable => Failure::ReplyUnusable,
        }
    })
}

/// Keeps the App made for attempt `id` and sends the browser to Install with `s2`; or fails
/// the attempt.
async fn made(
    scheduling: &Scheduling,
    attempts: &Attempts,
    (id, by): (Uuid, Starter),
    (converted, s2): (Result<Converted, Failure>, &Minted),
) -> Created {
    let converted = match converted {
        Ok(converted) => converted,
        Err(failure) => return failed(attempts, id, (failure, None), scheduling),
    };
    let app = App {
        id: converted.app_id,
        slug: converted.slug,
        owner: converted.owner,
    };
    let key = converted.private_key_pem;
    audit(scheduling, by, &app).await;
    let Some(location) = (AppSigner::read(app.id, key.as_bytes()).ok())
        .and_then(|_| location(scheduling, &app.slug, s2))
    else {
        return failed(
            attempts,
            id,
            (Failure::ReplyUnusable, Some(app)),
            scheduling,
        );
    };
    let now = scheduling.clock.now();
    // A fresh App whose attempt ended meanwhile fails the page; the attempt keeps its facts.
    match attempts.step(Some(id), now, |held| held.created(app, key, s2.hash())) {
        Ok(()) => Created::Install(location),
        Err(_) => Created::Failed,
    }
}

fn failed(
    attempts: &Attempts,
    id: Uuid,
    (failure, app): (Failure, Option<App>),
    scheduling: &Scheduling,
) -> Created {
    let now = scheduling.clock.now();
    let _ = attempts.step(Some(id), now, |held| {
        held.exchange_failed(failure, app, now);
        Ok(())
    });
    Created::Failed
}

/// GitHub's install page for the App `slug`, with the state `s2`; `None` when the hub cannot
/// make Apps on GitHub.
fn location(scheduling: &Scheduling, slug: &str, s2: &Minted) -> Option<Zeroizing<String>> {
    let registration = super::github(&scheduling.registry)?;
    Some(Zeroizing::new(format!(
        "{}{}?state={}",
        registration.web_base,
        (registration.install_path)(slug),
        s2.text()
    )))
}

/// Writes "App created" for the operator who started the attempt. A failed write is logged
/// with fixed words and the flow goes on: the attempt still shows the App.
async fn audit(scheduling: &Scheduling, by: Starter, app: &App) {
    let now = scheduling.clock.now();
    let trail = Trail::new(&scheduling.bus, (by.operator, by.client), now);
    let written = async {
        let mut tx = scheduling.pool.begin().await?;
        trail
            .write(&mut tx, trail::app_created(app))
            .await
            .map_err(|crate::fleet::events::Error::Database(error)| error)?;
        tx.commit().await
    };
    if let Err(error) = written.await {
        tracing::warn!(%error, "github app could not be audited");
    }
}
