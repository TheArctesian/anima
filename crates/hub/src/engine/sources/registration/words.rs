//! What the operator reads about an attempt: the fixed sentence of each phase and failure, and
//! the GitHub pages it links to (K-E1 Section 7a; the words of the design's Section E). Pure.
//!
//! A sentence may name the App's slug, which is text from GitHub; it never holds a key, a code,
//! a ticket or a state (LOG-007).

use source_exchange::Registration;

use super::attempt::{Failure, Phase, Snapshot};

/// The fixed path of the confirm page a signed-in operator opens.
pub const PAGE_PATH: &str = "/connect/github";

/// The sentence for `snapshot`.
#[must_use]
pub fn message(snapshot: &Snapshot) -> String {
    let slug = snapshot.app.as_ref().map(|app| app.slug.as_str());
    match (snapshot.phase, snapshot.failure) {
        (Phase::WaitingForBrowser, _) => "Opening GitHub in your browser.".to_owned(),
        (Phase::AtGithub, _) => {
            "Waiting for you on GitHub. Press \"Create GitHub App\". If GitHub \
             says the name is taken, change the name there and press it again."
                .to_owned()
        }
        (Phase::AppCreated, _) => format!(
            "The App {} is made. Press Install on GitHub.",
            slug.unwrap_or("for Anima.ing")
        ),
        (Phase::Connected, _) if snapshot.consumer_on => {
            "GitHub is connected. The first sync has started.".to_owned()
        }
        (Phase::Connected, _) => {
            "GitHub is connected. Projects is off, so Anima.ing does not read it \
             yet. Set Projects on in Settings > Extensions."
                .to_owned()
        }
        (Phase::Cancelled, _) => slug.map_or_else(
            || "Cancelled.".to_owned(),
            |slug| format!("Cancelled. The App {slug} is still on GitHub. Delete it there."),
        ),
        (Phase::Failed, failure) => failed(failure.unwrap_or(Failure::ReplyUnusable), slug),
    }
}

/// The sentence of a failure.
fn failed(failure: Failure, slug: Option<&str>) -> String {
    const LISTED: &str = "If GitHub lists a new App that starts with \"anima-\", delete it.";
    match (failure, slug) {
        (Failure::Expired, None) => {
            "The connection was not finished in time. Press Connect GitHub to start again."
                .to_owned()
        }
        (Failure::Expired, Some(slug)) => format!(
            "The App {slug} was made but not installed in time. Anima.ing does not use it. Delete it \
             on GitHub, then start again."
        ),
        (Failure::ExchangeRefused, _) => {
            format!("GitHub did not accept the reply from its page. Start again. {LISTED}")
        }
        (Failure::GithubUnreachable, _) => format!(
            "The hub could not reach GitHub to finish making the App. Start again. {LISTED}"
        ),
        (Failure::ReplyUnusable, Some(slug)) => {
            format!(
                "GitHub's reply could not be used. Delete the App {slug} on GitHub, then start again."
            )
        }
        (Failure::ReplyUnusable, None) => {
            format!("GitHub's reply could not be used. Start again. {LISTED}")
        }
        (Failure::KeyRefused, slug) => format!(
            "GitHub refused the new App's key. Delete the App {} on GitHub, then start again.",
            slug.unwrap_or("it made")
        ),
    }
}

/// `<origin>/connect/github`: holds no value (PRJ-041).
#[must_use]
pub fn page_url(origin: &str) -> String {
    format!("{origin}{PAGE_PATH}")
}

/// Where the operator presses Install, while the App waits for it.
#[must_use]
pub fn install_url(snapshot: &Snapshot, registration: &Registration) -> Option<String> {
    let app = snapshot.app.as_ref()?;
    (snapshot.phase == Phase::AppCreated).then(|| {
        format!(
            "{}{}",
            registration.web_base,
            (registration.install_path)(&app.slug)
        )
    })
}

/// Where the operator deletes an App the hub does not use: set when the attempt failed or was
/// cancelled and an App may exist on GitHub. Without the App's facts, the list of Apps.
#[must_use]
pub fn settings_url(snapshot: &Snapshot, registration: &Registration) -> Option<String> {
    let may_exist = match (snapshot.phase, snapshot.failure) {
        (Phase::Failed | Phase::Cancelled, _) if snapshot.app.is_some() => true,
        (Phase::Failed, Some(failure)) => failure != Failure::Expired,
        _ => false,
    };
    may_exist.then(|| {
        let app = snapshot.app.as_ref();
        let path = (registration.settings_path)(
            app.map(|app| &app.owner),
            app.map(|app| app.slug.as_str()),
        );
        format!("{}{path}", registration.web_base)
    })
}
