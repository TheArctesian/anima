//! How a source makes its own App for the operator, and what the hub reads back (PRJ-039).
//!
//! Pure types and function pointers: the hub supplies entropy, the origin and the transport.

use zeroize::Zeroizing;

use crate::exchange::Problem;

/// The provider's side of "Connect in three presses": the form to post, the manifest in it, the
/// pages the operator presses on, and how to read the replies.
#[derive(Clone, Copy, Debug)]
pub struct Registration {
    /// Where the operator's browser goes, for example `https://github.com`.
    pub web_base: &'static str,
    /// The App name from 7 bytes of entropy the hub supplies.
    pub name: fn(entropy: &[u8; 7]) -> String,
    /// The manifest the form posts, as JSON text.
    pub manifest: fn(&ManifestFacts<'_>) -> String,
    /// The form path below `web_base`. `None`: the organisation is not a valid login.
    pub form_path: fn(organisation: Option<&str>) -> Option<String>,
    /// The page where the operator installs the App. Text that is not a plain slug gives a fixed
    /// page on `web_base`.
    pub install_path: fn(slug: &str) -> String,
    /// The App's settings page, or the list of Apps when no slug is known.
    pub settings_path: fn(owner: Option<&Owner>, slug: Option<&str>) -> String,
    /// The conversion path below `api_base`. Always a path `Request::get` accepts.
    pub conversion_path: fn(code: &str) -> String,
    /// Reads the conversion reply.
    pub parse_converted: fn(&[u8]) -> Result<Converted, Problem>,
    /// The path of one installation below `api_base`, for the App's own token.
    pub installation_path: fn(id: &str) -> String,
    /// The path that lists the App's installations, below `api_base`.
    pub installations_path: &'static str,
    pub parse_installation: fn(&[u8]) -> Result<Installation, Problem>,
    pub parse_installations: fn(&[u8]) -> Result<Vec<Installation>, Problem>,
}

/// What goes into the manifest besides what the provider knows.
#[derive(Clone, Copy, Debug)]
pub struct ManifestFacts<'a> {
    pub name: &'a str,
    /// The hub's public origin, with no trailing slash.
    pub origin: &'a str,
    pub webhook: Webhook,
}

/// How the manifest treats the webhook, in the order the live check tries them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Webhook {
    /// A URL at the hub and the hook switched off.
    InactiveAtHub,
    /// No webhook key at all.
    Omitted,
    /// A fixed URL elsewhere and the hook switched off.
    InactivePlaceholder,
}

/// What the provider returned when the manifest was converted. No `Debug`: it holds the key,
/// which wipes itself when dropped.
pub struct Converted {
    pub app_id: u64,
    pub slug: String,
    pub owner: Owner,
    pub private_key_pem: Zeroizing<String>,
}

/// Who owns the App.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Owner {
    pub id: u64,
    pub login: String,
    pub organisation: bool,
}

/// One installation of the App.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Installation {
    pub id: u64,
    pub app_id: u64,
    pub account_id: u64,
    pub suspended: bool,
}
