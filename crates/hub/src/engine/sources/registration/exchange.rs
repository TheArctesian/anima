//! The return from "Create GitHub App": the code becomes the App's facts and key (K-E1
//! Section 7a, PRJ-039).
//!
//! The bound browser's cookie and the state S1 are checked, and S1 is spent, before the code is
//! converted, so a replay converts nothing. The key moves at once into the attempt; GitHub's
//! two other secrets are never read. "App created" is audited as soon as GitHub made an App, so
//! the operator can still delete it after a cancel or a restart.

use ring::rand::SystemRandom;
use source_exchange::{Converted, Declaration};
use uuid::Uuid;
use zeroize::Zeroizing;

use super::{
    GITHUB,
    attempt::{App, Exchange, Failure, Refusal, Starter},
    attempts::Attempts,
    binding::Minted,
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
    /// To GitHub's install page, with a new state S2; the URL holds S2.
    Install(Zeroizing<String>),
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
        Ok((exchange, held.id(), held.request().by.clone()))
    });
    match begun {
        Err(refusal) => Created::Refused(refusal),
        Ok((Exchange::Made, ..)) => install(scheduling, attempts, None, cookie),
        Ok((Exchange::Fresh, id, by)) => {
            let converted = converted(scheduling, code.unwrap_or_default()).await;
            made(scheduling, attempts, (id, by), converted).await
        }
    }
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

/// Keeps the App made for attempt `id` and sends the browser to Install; or fails the attempt.
async fn made(
    scheduling: &Scheduling,
    attempts: &Attempts,
    (id, by): (Uuid, Starter),
    converted: Result<Converted, Failure>,
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
    if AppSigner::read(app.id, key.as_bytes()).is_err() {
        return failed(
            attempts,
            id,
            (Failure::ReplyUnusable, Some(app)),
            scheduling,
        );
    }
    install(scheduling, attempts, Some((id, app, key)), None)
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

/// Mints S2 and sends the browser to Install: for the App just made (`fresh`), or again for
/// the bound browser with `cookie`. A fresh App whose attempt ended meanwhile fails the page.
fn install(
    scheduling: &Scheduling,
    attempts: &Attempts,
    fresh: Option<(Uuid, App, Zeroizing<String>)>,
    cookie: Option<&str>,
) -> Created {
    let (Some(registration), Ok(s2)) = (
        super::github(&scheduling.registry),
        Minted::new(&SystemRandom::new()),
    ) else {
        return Created::Refused(Refusal::Conflict);
    };
    let now = scheduling.clock.now();
    let was_fresh = fresh.is_some();
    let stepped = match fresh {
        Some((id, app, key)) => attempts.step(Some(id), now, |held| {
            let slug = app.slug.clone();
            held.created(app, key, s2.hash()).map(|()| slug)
        }),
        None => attempts.step(None, now, |held| {
            held.send_to_install(cookie, s2.hash())?;
            let slug = held.snapshot().app.map(|app| app.slug);
            slug.ok_or(Refusal::Conflict)
        }),
    };
    match stepped {
        Ok(slug) => Created::Install(Zeroizing::new(format!(
            "{}{}?state={}",
            registration.web_base,
            (registration.install_path)(&slug),
            s2.text()
        ))),
        Err(_) if was_fresh => Created::Failed,
        Err(refusal) => Created::Refused(refusal),
    }
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
