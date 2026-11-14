//! The three presses from the command line (PRJ-039, PRJ-041): start the attempt, give the
//! operator the way into the browser, and print each phase until the attempt ends.
//!
//! The change subscription opens before the attempt starts, so no phase is missed. At a
//! terminal the command asks for the one-time link and shows it; elsewhere it never asks and
//! shows the page that holds no value.

use serde_json::Value;

use super::attempt::Attempt;
use super::documents::{
    GithubConnectChanged, GithubConnectOpen, GithubConnectStart, GithubConnectState,
    github_connect_changed, github_connect_open, github_connect_start, github_connect_state,
};
use super::lines::{
    INTERRUPTED, LINK_INTRO, PAGE_INTRO, RESTARTED, STILL_OPEN, outcome, phase_line,
};
use super::report::{OneTimeLink, Report};
use crate::failure::{Failure, exit};
use crate::hub::Hub;
use crate::hub::subscription::Subscription;

/// What the operator asked for.
#[derive(Debug, Default)]
pub struct Request {
    pub label: Option<String>,
    pub organisation: Option<String>,
}

type Changes = Subscription<github_connect_changed::ResponseData>;

/// Runs the flow to its end and returns the final attempt as the hub's JSON.
///
/// # Errors
/// The hub's refusal (`CONFLICT` at start, `FORBIDDEN` for an API key); [`Failure::Ended`] for
/// an attempt that failed or was cancelled (exit 1, or 7 for `GITHUB_UNREACHABLE`) and for
/// Ctrl-C (exit 130).
pub async fn connect(
    hub: &Hub,
    request: Request,
    report: &mut Report<'_>,
) -> Result<Value, Failure> {
    let changes =
        Subscription::open::<GithubConnectChanged>(hub, github_connect_changed::Variables).await?;
    let started = start(hub, request).await?;
    if report.terminal {
        let link = open(hub, &started.id).await?;
        report.link(LINK_INTRO, &link);
    } else {
        report.page(PAGE_INTRO, &started.page_url);
    }
    tokio::select! {
        ended = follow(changes, hub, &started, report) => ended,
        _ = tokio::signal::ctrl_c() => Err(Failure::Ended {
            message: STILL_OPEN.to_owned(),
            status: INTERRUPTED,
        }),
    }
}

async fn start(hub: &Hub, request: Request) -> Result<Attempt, Failure> {
    let input = github_connect_start::StartGithubConnectInput {
        label: request.label,
        organisation: request.organisation,
    };
    let answer = hub
        .run::<GithubConnectStart>(github_connect_start::Variables { input: Some(input) })
        .await?;
    Ok(Attempt::read(&answer.start_github_connect)?.0)
}

async fn open(hub: &Hub, id: &str) -> Result<OneTimeLink, Failure> {
    let answer = hub
        .run::<GithubConnectOpen>(github_connect_open::Variables { id: id.to_owned() })
        .await?;
    Ok(OneTimeLink::new(answer.open_github_connect.url))
}

/// Prints each new phase of `started` until it ends.
async fn follow(
    mut changes: Changes,
    hub: &Hub,
    started: &Attempt,
    report: &mut Report<'_>,
) -> Result<Value, Failure> {
    let mut shown = started.phase;
    while let Some(answer) = changes.next().await {
        let (attempt, value) = Attempt::read(&answer?.github_connect_changed)?;
        if attempt.id != started.id || attempt.phase == shown {
            continue;
        }
        shown = attempt.phase;
        if let Some(line) = phase_line(&attempt) {
            report.progress(&line);
        }
        if attempt.phase.ended() {
            outcome(&attempt)?;
            return Ok(value);
        }
    }
    after_the_stream(hub, started, report).await
}

/// The subscription ended with the attempt not seen to end: ask the hub what became of it.
async fn after_the_stream(
    hub: &Hub,
    started: &Attempt,
    report: &mut Report<'_>,
) -> Result<Value, Failure> {
    let answer = hub
        .run::<GithubConnectState>(github_connect_state::Variables)
        .await?;
    let Some(current) = answer.github_connect else {
        return Err(lost(RESTARTED));
    };
    let (attempt, value) = Attempt::read(&current)?;
    if attempt.id != started.id {
        return Err(lost("Another GitHub connection replaced this one."));
    }
    if !attempt.phase.ended() {
        return Err(lost(
            "The hub stopped sending changes. `anima source connect github --state` shows where \
             the connection is.",
        ));
    }
    if let Some(line) = phase_line(&attempt) {
        report.progress(&line);
    }
    outcome(&attempt)?;
    Ok(value)
}

fn lost(words: &str) -> Failure {
    Failure::Ended {
        message: words.to_owned(),
        status: exit::FAILURE,
    }
}
