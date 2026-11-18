//! The pages the hub serves to the operator's browser during Connect GitHub (K-E1 Section 7a,
//! K7). Pure.
//!
//! Every page is fixed text. The one exception is the form that posts the manifest to GitHub:
//! its action (GitHub's page with the hub's own state) and the manifest JSON, both escaped. No
//! value from GitHub or from a query string is ever written into a page.

use base64::{Engine as _, engine::general_purpose::STANDARD};
use sha2::{Digest as _, Sha256};

mod installed;

/// The one script the hub serves: it posts the form to GitHub at once.
pub const SCRIPT: &str = "document.forms[0].submit();";

/// The policy of every page but the one that posts the manifest.
pub const PLAIN_POLICY: &str = "default-src 'none'; style-src 'unsafe-inline'; \
    form-action 'self'; base-uri 'none'; frame-ancestors 'none'";

/// A page of fixed words.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Notice {
    /// A ticket that is wrong, used or old.
    LinkEnded,
    /// `/open` from a browser with no Anima.ing session.
    SignIn,
    /// `/open` from a page that is not Anima.ing's.
    NotFromAnima,
    /// `/open` with no attempt waiting for a browser.
    NothingWaiting,
    /// A page of the flow from a browser that is not the bound one, or with the wrong state.
    NotThisBrowser,
    /// A page of the flow at a step that has passed.
    StepPassed,
    /// The hub could not check something; the operator tries again.
    TryAgain,
    /// The connection is stored.
    Connected,
    /// The App is made, and the hub has not yet seen the installation.
    Finishing,
    /// A hit on `/installed` from a browser that is not bound: it only wakes the poll.
    MaybeInstalled,
    /// The attempt failed.
    Failed,
}

impl Notice {
    const fn words(self) -> &'static str {
        match self {
            Self::LinkEnded => {
                "This link was already used or has ended. Go back to Anima.ing and \
                press Connect GitHub again."
            }
            Self::SignIn => "Sign in to Anima.ing in this browser, then open the link again.",
            Self::NotFromAnima => {
                "This request did not come from Anima.ing's page. Open the link \
                from Anima.ing again."
            }
            Self::NothingWaiting => "No GitHub connection is waiting. Start one in Anima.ing.",
            Self::NotThisBrowser => {
                "This page belongs to a GitHub connection in another \
                browser, or the link is old. Go back to Anima.ing."
            }
            Self::StepPassed => "This step is done. Go back to Anima.ing to see what comes next.",
            Self::TryAgain => "Anima.ing could not finish this step. Try again in a moment.",
            Self::Connected => "You can close this tab and go back to Anima.ing.",
            Self::Finishing => {
                "The App is made. Anima.ing finishes the connection when GitHub \
                shows it is installed. You can close this tab."
            }
            Self::MaybeInstalled => {
                "If you installed the App, Anima.ing finishes the connection. \
                You can close this tab."
            }
            Self::Failed => {
                "Anima.ing could not finish connecting GitHub. Go back to Anima.ing to see \
                what to do next."
            }
        }
    }
}

/// `text` with the five characters that matter in HTML text and attribute values escaped.
#[must_use]
pub fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            other => out.push(other),
        }
    }
    out
}

fn document(main: &str) -> String {
    format!(
        "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\">\
         <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\
         <title>Connect GitHub to Anima.ing</title><style>body{{font-family:system-ui,sans-serif;\
         max-width:32rem;margin:4rem auto;padding:0 1rem;line-height:1.5}}</style></head>\
         <body><h1>Connect GitHub to Anima.ing</h1>{main}</body></html>"
    )
}

/// The page of fixed words for `notice`.
#[must_use]
pub fn notice(notice: Notice) -> String {
    document(&format!("<p>{}</p>", notice.words()))
}

/// The browser's result when it returns from GitHub (PRJ-039, UR-7).
#[must_use]
pub fn installed(notice: Notice) -> String {
    let heading = if notice == Notice::Connected {
        "GitHub is connected."
    } else {
        "Connect GitHub to Anima.ing"
    };
    installed::document(heading, notice.words())
}

/// The confirm page a signed-in operator opens from the link with no value in it: the same
/// page whether or not an attempt exists.
#[must_use]
pub fn confirm() -> String {
    document(
        "<p>Anima.ing makes a GitHub App in your account and keeps its key in the hub. Continue in \
         a browser that is signed in to Anima.ing.</p>\
         <form method=\"post\" action=\"/connect/github/open\">\
         <button type=\"submit\">Continue to GitHub</button></form>",
    )
}

/// The page that posts `manifest` to `action` at once, with a button for a browser without
/// scripts.
#[must_use]
pub fn to_github(action: &str, manifest: &str) -> String {
    document(&format!(
        "<p>Sending you to GitHub…</p><form method=\"post\" action=\"{}\">\
         <input type=\"hidden\" name=\"manifest\" value=\"{}\">\
         <button type=\"submit\">Continue to GitHub</button></form><script>{SCRIPT}</script>",
        escape(action),
        escape(manifest)
    ))
}

/// The policy of the page that posts the manifest: only the hashed [`SCRIPT`] runs, and the
/// form goes only to `web_base` (GitHub).
#[must_use]
pub fn to_github_policy(web_base: &str) -> String {
    let hash = STANDARD.encode(Sha256::digest(SCRIPT.as_bytes()));
    format!(
        "default-src 'none'; script-src 'sha256-{hash}'; style-src 'unsafe-inline'; \
         form-action {web_base}; base-uri 'none'; frame-ancestors 'none'"
    )
}

#[cfg(test)]
mod tests {
    use super::{Notice, confirm, escape, notice, to_github};

    #[test]
    fn prj_039_the_form_escapes_its_action_and_the_manifest() {
        let page = to_github(
            "https://github.com/x?a=1&b=\"2\"",
            r#"{"name":"<script>'x'</script>"}"#,
        );
        assert!(page.contains("action=\"https://github.com/x?a=1&amp;b=&quot;2&quot;\""));
        assert!(
            page.contains("&quot;name&quot;:&quot;&lt;script&gt;&#39;x&#39;&lt;/script&gt;&quot;")
        );
        assert_eq!(
            page.matches("<script>").count(),
            1,
            "only the hub's own script"
        );
        assert_eq!(escape("plain-text_1"), "plain-text_1");
    }

    #[test]
    fn prj_041_the_fixed_pages_hold_no_value() {
        assert!(confirm().contains("action=\"/connect/github/open\""));
        assert!(!confirm().contains("<script"));
        assert!(notice(Notice::LinkEnded).contains("already used or has ended"));
    }
}
