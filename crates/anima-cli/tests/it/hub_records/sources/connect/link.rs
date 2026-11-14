//! The one-time link: asked for only at a terminal, printed once, handed to the opener once and
//! in no other output (PRJ-041, MCP-004, LOG-007).

use anima_cli::failure::Failure;
use anima_cli::hub::{Hub, Origin, Token};
use anima_cli::sources::connect::flow::{self, Request};
use anima_cli::sources::connect::report::{OneTimeLink, Report};
use serde_json::Value;
use zeroize::Zeroizing;

use super::{LINK, TICKET, asked, attempt, changes, connected, failed, scripted, signed_in};
use crate::kit::fake_hub::FakeHub;
use crate::kit::subscription_hub::Script;
use crate::kit::terminal::Terminal;

#[tokio::test]
async fn prj_041_at_a_terminal_the_link_is_asked_for_and_shown_once() {
    let (hub, _) = scripted(Script::Next(changes(&[attempt("AT_GITHUB"), connected()]))).await;
    let anima = signed_in();
    let args = [
        "source",
        "connect",
        "github",
        "--no-browser",
        "--hub",
        &hub.origin,
    ];
    let mut terminal = Terminal::start(&anima, &args);
    terminal
        .expect("Open this link in your browser. It works once, for 5 minutes:")
        .await;
    terminal.expect(LINK).await;
    let (code, screen) = terminal.finish().await;
    assert_eq!(code, 0, "{screen}");
    assert_eq!(screen.matches(TICKET).count(), 1, "{screen}");
    assert_eq!(asked(&hub, "GithubConnectOpen"), 1);
}

/// What a run through the library printed and what the opener was given.
struct Library {
    outcome: Result<Value, Failure>,
    notes: String,
    opened: Vec<String>,
}

async fn through_the_library(hub: &FakeHub, browser: bool, opener_fails: bool) -> Library {
    let token = Token::new(Zeroizing::new("login-token".to_owned())).expect("token");
    let origin = Origin::parse(&hub.origin).expect("origin");
    let hub = Hub::new(origin, Some(token)).expect("a client");
    let mut notes = Vec::new();
    let mut opened = Vec::new();
    let mut open = |url: &str| {
        opened.push(url.to_owned());
        if opener_fails {
            Err(std::io::Error::other("no browser here"))
        } else {
            Ok(())
        }
    };
    let outcome = {
        let mut report = Report {
            notes: &mut notes,
            terminal: true,
            json: false,
            browser: browser.then_some(&mut open),
        };
        flow::connect(&hub, Request::default(), &mut report).await
    };
    Library {
        outcome,
        notes: String::from_utf8(notes).expect("text"),
        opened,
    }
}

#[tokio::test]
async fn prj_041_the_link_is_printed_once_and_handed_to_the_opener_once() {
    let (hub, _) = scripted(Script::Next(changes(&[attempt("AT_GITHUB"), connected()]))).await;
    let run = through_the_library(&hub, true, false).await;
    run.outcome.expect("connected");
    assert_eq!(run.opened, [LINK]);
    assert_eq!(run.notes.matches(TICKET).count(), 1, "{}", run.notes);
}

#[tokio::test]
async fn prj_041_no_browser_prints_the_link_and_opens_nothing() {
    let (hub, _) = scripted(Script::Next(changes(&[connected()]))).await;
    let run = through_the_library(&hub, false, false).await;
    run.outcome.expect("connected");
    assert_eq!(run.opened.len(), 0);
    assert_eq!(run.notes.matches(TICKET).count(), 1, "{}", run.notes);
}

#[tokio::test]
async fn prj_041_a_failed_opener_is_not_an_error_and_the_link_is_still_printed() {
    let (hub, _) = scripted(Script::Next(changes(&[connected()]))).await;
    let run = through_the_library(&hub, true, true).await;
    run.outcome.expect("connected");
    assert_eq!(run.opened.len(), 1);
    assert!(run.notes.contains(LINK), "{}", run.notes);
}

#[tokio::test]
async fn prj_041_the_link_is_in_no_failure_text_and_no_debug_output() {
    let over = failed("EXPIRED", "The connection was not finished in time.", false);
    let (hub, _) = scripted(Script::Next(changes(&[over]))).await;
    let run = through_the_library(&hub, false, false).await;
    let failure = run.outcome.expect_err("failed");
    for text in [failure.to_string(), format!("{failure:?}")] {
        assert!(
            !text.contains(TICKET) && !text.contains("ticket="),
            "{text}"
        );
    }
    assert_eq!(
        format!("{:?}", OneTimeLink::new(LINK.to_owned())),
        "OneTimeLink(..)"
    );
}
