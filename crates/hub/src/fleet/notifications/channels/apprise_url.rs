//! Which stateless Apprise URLs the hub sends (N-4, WP-52 handoff, D-17, D-18).
//!
//! The hub hands these URLs to the Apprise API, which then connects on its own; the hub's
//! target rule and deny list never see those connections. So the hub permits only URLs whose
//! service talks to one fixed public host, with query keys that change how the message looks,
//! never where it goes or what Apprise reads. The firewall around Apprise contains it too; the
//! hub does not rely on that.
//!
//! - **Schemes** ([`SCHEMES`]): Discord, Telegram, Slack, Pushover, Pushbullet, Google Chat,
//!   `PagerDuty` and Opsgenie. Each posts to its vendor's fixed API host. Every scheme that takes
//!   a host from the URL is refused: `json`, `jsons`, `form`, `forms`, `xml`, `xmls`,
//!   `apprise`, `apprises` (post to any URL), `ntfy`, `gotify`, `mailto`, `matrix`,
//!   `mattermost`, `rocket`, `mqtt`, `syslog` and the rest, and `http`/`https` (Apprise maps
//!   some native URLs to services). Self-hosted targets use the hub's own webhook, ntfy and
//!   e-mail channels, whose connections pass the target rule (P3-003).
//! - **Query keys** ([`KEYS`]): display and routing options of those services only. Anything
//!   else is refused, among them `template`, `attach` and `attachment` (read a file or URL),
//!   `avatar_url`, `image`, `url` and `href` (hand over or read a URL or a local image), `verify`
//!   (turns TLS verification off), `cto` and `rto` (timeouts), `store` (writes to disk), and
//!   the `+`, `-` and `:` keys of the custom JSON, form and XML services.
//!
//! The rules are written for how Apprise reads a URL: it splits several URLs at a comma or a
//! space, takes the query after the first `?`, splits it at `&` and `;`, percent-decodes each
//! key, trims it and lowers its case. So a URL with a space, a comma, a control character, a
//! second `://` or a second `?` is refused, and a key is compared after percent-decoding: it
//! must be exactly an allowed key in ASCII, any case. A key Apprise would read differently from
//! the hub is not on the list, so it is refused.
//!
//! The comma rule is defence in depth: a second URL after a comma needs its own `://`, which
//! the one-scheme rule refuses already. The comma is refused on its own anyway, so the rule
//! does not lean on how Apprise reads what follows a comma that has no scheme.

/// Services that post only to their vendor's fixed host.
const SCHEMES: [&str; 8] = [
    "discord",
    "tgram",
    "slack",
    "pover",
    "pbul",
    "gchat",
    "pagerduty",
    "opsgenie",
];

/// Query keys that change how a message looks or which recipient of the same service gets it.
const KEYS: [&str; 17] = [
    "format", "overflow", "emojis", "to", "tts", "avatar", "footer", "thread", "botname", "silent",
    "preview", "topic", "priority", "sound", "retry", "expire", "blocks",
];

/// The longest URL accepted, in bytes.
const URL_MAX: usize = 2048;

/// Why a URL is refused; the operator reads it, so it never quotes the URL.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::fleet::notifications) enum Refusal {
    /// Too long, empty, or holds a space, comma, control character, second scheme or second
    /// query.
    Shape,
    /// A scheme outside [`SCHEMES`].
    Scheme,
    /// A query key outside [`KEYS`].
    Key,
}

impl Refusal {
    pub(in crate::fleet::notifications) const fn message(self) -> &'static str {
        match self {
            Self::Shape => {
                "Enter one Apprise URL per entry, with no spaces or commas, at most 2048 characters."
            }
            Self::Scheme => {
                "Use a Discord, Telegram, Slack, Pushover, Pushbullet, Google Chat, PagerDuty or \
                 Opsgenie URL. For other targets add a webhook, ntfy or e-mail channel."
            }
            Self::Key => {
                "Remove options the hub does not send (such as template, attach or verify)."
            }
        }
    }
}

/// `Ok` when the hub may hand `url` to Apprise (N-4). Called at save and again before each
/// send.
///
/// # Errors
/// The first rule the URL breaks.
pub(in crate::fleet::notifications) fn permitted(url: &str) -> Result<(), Refusal> {
    let shape_ok = !url.is_empty()
        && url.len() <= URL_MAX
        && !url.chars().any(|c| {
            c.is_control() || c.is_whitespace() || c == ',' || crate::text::is_invisible(c)
        })
        && url.matches("://").count() == 1
        && url.matches('?').count() <= 1;
    if !shape_ok {
        return Err(Refusal::Shape);
    }
    let (scheme, rest) = url.split_once("://").ok_or(Refusal::Shape)?;
    if !SCHEMES
        .iter()
        .any(|allowed| scheme.eq_ignore_ascii_case(allowed))
    {
        return Err(Refusal::Scheme);
    }
    let Some((_, query)) = rest.split_once('?') else {
        return Ok(());
    };
    let keys_ok = query
        .split(['&', ';'])
        .filter(|pair| !pair.is_empty())
        .all(|pair| {
            let key = pair.split_once('=').map_or(pair, |(key, _)| key);
            decoded(key)
                .is_some_and(|key| KEYS.iter().any(|allowed| key.eq_ignore_ascii_case(allowed)))
        });
    if keys_ok { Ok(()) } else { Err(Refusal::Key) }
}

/// The percent-decoded text, or `None` for a bad escape or bytes that are not UTF-8.
fn decoded(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while let Some(&byte) = bytes.get(index) {
        if byte == b'%' {
            let hex = bytes.get(index + 1..index + 3)?;
            let hex = std::str::from_utf8(hex).ok()?;
            out.push(u8::from_str_radix(hex, 16).ok()?);
            index += 3;
        } else {
            out.push(byte);
            index += 1;
        }
    }
    String::from_utf8(out).ok()
}
