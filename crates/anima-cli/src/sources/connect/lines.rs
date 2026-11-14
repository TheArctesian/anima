//! The words of the command, each a pure function of the attempt (K-E1 design, Sections E and F).
//! Text from GitHub is cleaned where a line is printed. Links that GitHub made are shown only
//! when they point at `github.com`.

use super::attempt::{Attempt, Phase, Reason};
use crate::failure::{Failure, exit};

/// The only place a link from the hub's answer is shown to be followed (design Section F).
const GITHUB: &str = "https://github.com/";

/// Why the command stopped with the connection still open.
pub const STILL_OPEN: &str = "The connection is still open on the hub. `anima source connect \
                              github --cancel` stops it.";

pub const NONE_OPEN: &str = "No GitHub connection is in progress.";

/// What `connect` prints before the one-time link, which comes on the next line.
pub const LINK_INTRO: &str = "Open this link in your browser. It works once, for 5 minutes:";

/// What `connect` prints before the page link, off a terminal.
pub const PAGE_INTRO: &str =
    "Open this page in a browser that is signed in to Anima and press Continue.";

/// The exit code of a command stopped by Ctrl-C.
pub const INTERRUPTED: u8 = 130;

fn github_link(link: Option<&str>) -> Option<&str> {
    link.filter(|link| link.starts_with(GITHUB))
}

/// The line for a phase that has just begun; `None` when the phase has no line of its own.
#[must_use]
pub fn phase_line(attempt: &Attempt) -> Option<String> {
    match attempt.phase {
        Phase::AtGithub => {
            Some("Waiting for you to press \"Create GitHub App\" on GitHub.".to_owned())
        }
        Phase::AppCreated => Some(app_created(attempt)),
        Phase::Connected => Some(connected(attempt)),
        _ => None,
    }
}

fn app_created(attempt: &Attempt) -> String {
    let slug = attempt.app_slug.as_deref().unwrap_or("for Anima");
    let owner = attempt.owner_login.as_deref().unwrap_or("you");
    let sentence =
        format!("GitHub App {slug} is made for {owner}. Waiting for you to press \"Install\".");
    match github_link(attempt.install_url.as_deref()) {
        Some(link) => format!("{sentence}\n  {link}"),
        None => sentence,
    }
}

/// `GitHub is connected as "<label>".` and what the hub says comes next: the first sync, or
/// that Projects is off.
fn connected(attempt: &Attempt) -> String {
    let label = attempt
        .account
        .as_ref()
        .map_or("", |account| &account.label);
    let next = attempt
        .message
        .as_deref()
        .and_then(|message| message.strip_prefix("GitHub is connected. "))
        .unwrap_or("");
    format!("GitHub is connected as \"{label}\". {next}")
        .trim_end()
        .to_owned()
}

/// The words of an attempt that did not connect: the hub's fixed sentence and, when an App may
/// be left on GitHub, the page where it is deleted.
#[must_use]
pub fn ending_words(attempt: &Attempt) -> String {
    let words = attempt
        .message
        .clone()
        .unwrap_or_else(|| "The GitHub connection did not finish.".to_owned());
    match github_link(attempt.app_settings_url.as_deref()) {
        Some(link) => format!("{words}\n  {link}"),
        None => words,
    }
}

/// How the command ends for an attempt that is over: `Ok` when connected.
///
/// # Errors
/// [`Failure::Ended`] for a failed or cancelled attempt: exit 7 for `GITHUB_UNREACHABLE`, else 1.
pub fn outcome(attempt: &Attempt) -> Result<(), Failure> {
    match attempt.phase {
        Phase::Connected => Ok(()),
        _ => Err(Failure::Ended {
            message: ending_words(attempt),
            status: if attempt.failure == Some(Reason::GithubUnreachable) {
                exit::UNAVAILABLE
            } else {
                exit::FAILURE
            },
        }),
    }
}

/// The attempt as `--state` prints it: the phase, the hub's sentence and the links that apply.
#[must_use]
pub fn state_text(attempt: &Attempt) -> String {
    let mut parts = vec![format!(
        "{}: {}",
        attempt.phase.name(),
        attempt.message.as_deref().unwrap_or_default()
    )];
    for link in [&attempt.install_url, &attempt.app_settings_url] {
        parts.extend(github_link(link.as_deref()).map(str::to_owned));
    }
    parts.join("\n  ")
}

/// The hub dropped the attempt: it restarted (design Section E).
pub const RESTARTED: &str = "The hub restarted and dropped the connection. If you already made \
                             the App on GitHub, delete it, then start again.";

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sources::connect::attempt::Account;

    fn attempt(phase: Phase) -> Attempt {
        Attempt {
            id: "a".to_owned(),
            phase,
            page_url: "https://hub/connect/github".to_owned(),
            app_slug: Some("anima-1".to_owned()),
            owner_login: Some("me".to_owned()),
            install_url: Some("https://github.com/apps/anima-1/installations/new".to_owned()),
            app_settings_url: None,
            failure: None,
            message: None,
            account: None,
        }
    }

    #[test]
    fn prj_039_the_app_line_names_the_slug_and_links_only_to_github() {
        let line = phase_line(&attempt(Phase::AppCreated)).expect("a line");
        assert!(
            line.starts_with("GitHub App anima-1 is made for me."),
            "{line}"
        );
        assert!(line.contains("https://github.com/apps/anima-1"), "{line}");
        let mut elsewhere = attempt(Phase::AppCreated);
        elsewhere.install_url = Some("https://evil.example/".to_owned());
        let line = phase_line(&elsewhere).expect("a line");
        assert!(!line.contains("evil"), "{line}");
    }

    #[test]
    fn prj_039_connected_names_the_label_and_keeps_the_hubs_projects_off_sentence() {
        let mut done = attempt(Phase::Connected);
        done.account = Some(Account {
            label: "mine".to_owned(),
        });
        done.message = Some("GitHub is connected. The first sync has started.".to_owned());
        assert_eq!(
            phase_line(&done).as_deref(),
            Some("GitHub is connected as \"mine\". The first sync has started.")
        );
        done.message =
            Some("GitHub is connected. Projects is off, so Anima does not read it yet.".to_owned());
        assert!(
            phase_line(&done)
                .expect("a line")
                .ends_with("does not read it yet.")
        );
    }

    #[test]
    fn prj_041_github_unreachable_is_exit_7_and_other_failures_exit_1() {
        let mut failed = attempt(Phase::Failed);
        failed.failure = Some(Reason::GithubUnreachable);
        assert_eq!(outcome(&failed).expect_err("failed").exit_code(), 7);
        failed.failure = Some(Reason::Expired);
        assert_eq!(outcome(&failed).expect_err("failed").exit_code(), 1);
        assert!(outcome(&attempt(Phase::Connected)).is_ok());
    }
}
